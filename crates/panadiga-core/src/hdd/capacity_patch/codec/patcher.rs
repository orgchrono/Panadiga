#![deny(unsafe_code)]

//! In-place capacity clamp patching, EXTRHEAD repacking, and MEIHDFS MBR capacity synchronization.

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use crate::constants::*;
use crate::crypto::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, ExtrheadHeader, UnpackedExtrhead,
    pack_extrhead_with_options, unpack_extrhead,
};

use super::super::specs::{
    DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB, HDD_CMD_M7920_200GB_BLOCKS, HDD_CMD_M7920_400GB_BLOCKS,
    HDD_CMD_M7920_FALLBACK_BLOCKS, HddCapacityPatchMode, MainCapacityInspectionReport,
    OFF_CLAMP_BRANCH_OP, OFF_SLOT_200GB, OFF_SLOT_400GB, OFF_SLOT_FALLBACK, OPCODE_BLS, OPCODE_BRA,
    blocks_2kb_for_gb,
};
use super::loader::{find_hdd_cmd_clamp_site, inspect_unpacked_capacity};

/// Applies the selected HDD capacity patch directly onto an unpacked `MAIN` buffer in place.
pub fn apply_capacity_patch_in_place(
    main_buf: &mut [u8],
    mode: HddCapacityPatchMode,
    target_gb: Option<u32>,
) -> Result<u32> {
    let site = find_hdd_cmd_clamp_site(main_buf)
        .context("Cannot patch capacity: src/hdd_cmd.c clamp sequence not found in MAIN binary")?;

    let (branch_op, slot_400_val, slot_200_val, slot_fb_val) = match mode {
        HddCapacityPatchMode::DynamicUnclamped => {
            let blocks = match target_gb {
                Some(gb) => blocks_2kb_for_gb(gb)?,
                None => DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB,
            };
            (OPCODE_BRA, blocks, blocks, blocks)
        }
        HddCapacityPatchMode::FixedCapacity => {
            let gb = target_gb.context(
                "Mode 'cap' requires --target-gb <GB> (e.g., 250, 300, 320, 400, 500, 1000)",
            )?;
            let blocks = blocks_2kb_for_gb(gb)?;
            (OPCODE_BLS, blocks, blocks, blocks)
        }
        HddCapacityPatchMode::Factory200Gb => (
            OPCODE_BLS,
            HDD_CMD_M7920_400GB_BLOCKS,
            HDD_CMD_M7920_200GB_BLOCKS,
            HDD_CMD_M7920_FALLBACK_BLOCKS,
        ),
    };

    LittleEndian::write_u32(&mut main_buf[site + OFF_SLOT_400GB..], slot_400_val);
    LittleEndian::write_u32(&mut main_buf[site + OFF_SLOT_200GB..], slot_200_val);
    LittleEndian::write_u32(&mut main_buf[site + OFF_SLOT_FALLBACK..], slot_fb_val);
    main_buf[site + OFF_CLAMP_BRANCH_OP] = branch_op;

    Ok(slot_200_val)
}

pub(crate) fn apply_optional_passthrough(
    main_buf: &mut [u8],
    passthrough_mode: Option<&str>,
) -> Result<Option<String>> {
    let Some(raw_mode) = passthrough_mode else {
        return Ok(None);
    };
    let mode: crate::re::MainPassthroughMode = raw_mode.parse()?;
    crate::re::apply_main_passthrough_in_place(main_buf, mode)?;
    Ok(Some(mode.as_str().to_string()))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn patch_and_repack_extrhead(
    unpacked: &mut UnpackedExtrhead,
    is_plain: bool,
    round_key: &[u8; 32],
    container_format: &str,
    in_label: &str,
    mode: HddCapacityPatchMode,
    target_gb: Option<u32>,
    passthrough_mode: Option<&str>,
) -> Result<(MainCapacityInspectionReport, u32, Option<String>, Vec<u8>)> {
    let before = inspect_unpacked_capacity(unpacked, container_format, in_label)?;
    let applied = apply_capacity_patch_in_place(&mut unpacked.decompressed, mode, target_gb)?;
    let applied_pt = apply_optional_passthrough(&mut unpacked.decompressed, passthrough_mode)?;
    let repacked = repack_from_header(
        &unpacked.decompressed,
        &unpacked.header,
        is_plain,
        round_key,
    )?;
    Ok((before, applied, applied_pt, repacked))
}

pub(crate) fn verify_and_inspect_repacked(
    container_bytes: &[u8],
    container_format: &str,
    out_label: &str,
) -> Result<MainCapacityInspectionReport> {
    let verified = unpack_extrhead(container_bytes, ExtrheadCryptMode::Auto)
        .with_context(|| format!("Verifying repacked {container_format} checksum"))?;
    if !verified.checksum_valid {
        bail!("Internal verification failed: repacked {container_format} checksum mismatch");
    }
    inspect_unpacked_capacity(&verified, container_format, out_label)
}

fn repack_from_header(
    payload: &[u8],
    hdr: &ExtrheadHeader,
    is_plain: bool,
    round_key: &[u8; 32],
) -> Result<Vec<u8>> {
    let mode = if is_plain {
        ExtrheadCryptMode::Plain
    } else {
        ExtrheadCryptMode::Auto
    };
    let algo = match hdr.algo {
        0 => ExtrheadCompressionAlgo::Zlib,
        1 => ExtrheadCompressionAlgo::Gzip,
        2 => ExtrheadCompressionAlgo::Lzss,
        3 => ExtrheadCompressionAlgo::Lzma,
        _ => ExtrheadCompressionAlgo::Zlib,
    };
    pack_extrhead_with_options(
        payload,
        &hdr.tag,
        hdr.dst_addr,
        &hdr.build_name,
        mode,
        algo,
        round_key,
        hdr.adler_step.max(1),
        hdr.flag,
    )
}

pub(crate) fn maybe_update_procedural_mbr_capacity(
    image_bytes: &mut [u8],
    new_total_sectors: u64,
) -> bool {
    if image_bytes.len() < (LBA_MIRROR_MBR_OFFSET as usize) + LBA_SECTOR_SIZE_BYTES {
        return false;
    }
    let existing_total = LittleEndian::read_u64(&image_bytes[24..32]);
    let existing_p3_start = LittleEndian::read_u64(&image_bytes[88..96]);
    if existing_total == 0 && existing_p3_start == 0 {
        return false;
    }
    let p3_start = if existing_p3_start > 0 {
        existing_p3_start
    } else {
        196_608
    };
    let p3_sectors = new_total_sectors.saturating_sub(p3_start);

    for offset in [0usize, LBA_MIRROR_MBR_OFFSET as usize] {
        LittleEndian::write_u64(
            &mut image_bytes[offset + 24..offset + 32],
            new_total_sectors,
        );
        LittleEndian::write_u64(&mut image_bytes[offset + 96..offset + 104], p3_sectors);
    }
    true
}
