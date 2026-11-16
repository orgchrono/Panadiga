//! Pure in-memory buffer codecs for `src/hdd_cmd.c` capacity clamping, `EXTRHEAD`, & MBR superblocks.

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use crate::constants::*;
use crate::crypto::{
    ExtrheadCryptMode, ExtrheadHeader, UnpackedExtrhead, pack_extrhead, unpack_extrhead,
};
use crate::nvram::calculate_hashes;

use super::specs::{
    DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB, HDD_CMD_M7920_200GB_BLOCKS, HDD_CMD_M7920_400GB_BLOCKS,
    HddCapacityPatchMode, MAIN_DEFAULT_BASE_VA, MainCapacityInspectionReport, OFF_CLAMP_BRANCH_OP,
    OFF_MAX_LBA_VAR_1, OFF_PHYS_VAR_1, OFF_SLOT_200GB, OFF_SLOT_400GB, OFF_SLOT_FALLBACK,
    OPCODE_BLS, OPCODE_BRA, PREFIX_SIG, WIN_LEN, blocks_2kb_for_gb, blocks_2kb_to_gb,
    official_rtsd_presets,
};

/// Locates the 114-byte `src/hdd_cmd.c` capacity-clamping basic block inside an unpacked `MAIN` image.
pub fn find_hdd_cmd_clamp_site(main_buf: &[u8]) -> Option<usize> {
    if main_buf.len() < WIN_LEN {
        return None;
    }
    for pos in 0..=(main_buf.len() - WIN_LEN) {
        let win = &main_buf[pos..pos + WIN_LEN];
        if win[0x00..0x0E] != PREFIX_SIG || win[0x12..0x14] != [0xFC, 0x81] {
            continue;
        }
        if win[0x27..0x29] != [0xFC, 0xCC] || win[0x2D..0x2F] != [0xFC, 0x81] {
            continue;
        }
        if win[0x42..0x44] != [0xFC, 0xCC] || win[0x48..0x4A] != [0xFC, 0x81] {
            continue;
        }
        if win[0x4E..0x50] != [0xFC, 0xA5] || win[0x54..0x56] != [0xFC, 0xA4] {
            continue;
        }
        let branch_op = win[OFF_CLAMP_BRANCH_OP];
        if win[0x5A] != 0xA1
            || (branch_op != OPCODE_BLS && branch_op != OPCODE_BRA)
            || win[0x5C..0x61] != [0x0C, 0x29, 0xFF, 0xFC, 0x85]
        {
            continue;
        }
        if win[0x65..0x6B] != [0xCA, 0x0A, 0x28, 0xFF, 0xFC, 0x81] {
            continue;
        }
        return Some(pos);
    }
    None
}

pub(crate) enum LoadedMainSource {
    RawMain {
        main_bytes: Vec<u8>,
        base_va: u32,
    },
    Extrhead {
        unpacked: UnpackedExtrhead,
        is_plain: bool,
    },
    FullHddP1 {
        image_bytes: Vec<u8>,
        p1_container_len: usize,
        unpacked: UnpackedExtrhead,
        is_plain: bool,
    },
}

fn is_full_hdd_with_p1(raw: &[u8]) -> bool {
    raw.len() >= (LBA_PARTITION_2_OFFSET as usize)
        && raw[..8] == PANASONIC_HDD_MBR_SIGNATURE
        && (raw[LBA_PARTITION_1_OFFSET as usize..].starts_with(b"EXTRHEAD")
            || raw[LBA_PARTITION_1_OFFSET as usize..].starts_with(&PANASONIC_HDD_PARTITION_MAGIC))
}

fn detect_extrhead_container_len(slice: &[u8]) -> Result<usize> {
    if slice.len() < 0x40 {
        bail!("EXTRHEAD slice shorter than 64 bytes");
    }
    let is_plain = slice.starts_with(b"EXTRHEAD");
    let mut hdr = [0u8; 0x40];
    hdr.copy_from_slice(&slice[..0x40]);
    if !is_plain {
        crate::crypto::feistel_decrypt_buffer(
            &mut hdr,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );
    }
    let parsed = crate::crypto::parse_extrhead_header(&hdr)?;
    let footer_span = if parsed.footer_offset >= 0x40 {
        parsed.footer_offset as usize + 8
    } else {
        0x40 + parsed.src_len as usize + 8
    };
    let aligned = (footer_span + 7) & !7;
    if aligned > slice.len() {
        Ok(slice.len() & !7)
    } else {
        Ok(aligned)
    }
}

pub(crate) fn load_main_source(raw: Vec<u8>) -> Result<LoadedMainSource> {
    if is_full_hdd_with_p1(&raw) {
        let p1_off = LBA_PARTITION_1_OFFSET as usize;
        let p2_off = LBA_PARTITION_2_OFFSET as usize;
        let max_p1 = &raw[p1_off..p2_off];
        let c_len = detect_extrhead_container_len(max_p1)?;
        let is_plain = max_p1.starts_with(b"EXTRHEAD");
        let unpacked = unpack_extrhead(&max_p1[..c_len], ExtrheadCryptMode::Auto)?;
        return Ok(LoadedMainSource::FullHddP1 {
            image_bytes: raw,
            p1_container_len: c_len,
            unpacked,
            is_plain,
        });
    }

    if raw.starts_with(b"EXTRHEAD") || raw.starts_with(&PANASONIC_HDD_PARTITION_MAGIC) {
        let is_plain = raw.starts_with(b"EXTRHEAD");
        let unpacked = unpack_extrhead(&raw, ExtrheadCryptMode::Auto)?;
        return Ok(LoadedMainSource::Extrhead { unpacked, is_plain });
    }

    Ok(LoadedMainSource::RawMain {
        main_bytes: raw,
        base_va: MAIN_DEFAULT_BASE_VA,
    })
}

/// Inspects an unpacked `MAIN` buffer for its `src/hdd_cmd.c` HDD capacity clamp configuration.
pub fn inspect_main_buffer_capacity(
    main_buf: &[u8],
    base_va: u32,
    container_format: &str,
    build_name: Option<String>,
    input_label: &str,
) -> Result<MainCapacityInspectionReport> {
    let site = find_hdd_cmd_clamp_site(main_buf).context(
        "Could not locate src/hdd_cmd.c MaxProductLBA clamp sequence in MAIN binary. \
         Expected EH55/EH65 M7920 firmware (3060NP / 3410PQ series).",
    )?;

    let win = &main_buf[site..site + WIN_LEN];
    let slot_400gb = LittleEndian::read_u32(&win[OFF_SLOT_400GB..OFF_SLOT_400GB + 4]);
    let slot_200gb = LittleEndian::read_u32(&win[OFF_SLOT_200GB..OFF_SLOT_200GB + 4]);
    let slot_fallback = LittleEndian::read_u32(&win[OFF_SLOT_FALLBACK..OFF_SLOT_FALLBACK + 4]);
    let max_lba_var = LittleEndian::read_u32(&win[OFF_MAX_LBA_VAR_1..OFF_MAX_LBA_VAR_1 + 4]);
    let phys_var = LittleEndian::read_u32(&win[OFF_PHYS_VAR_1..OFF_PHYS_VAR_1 + 4]);
    let branch_op = win[OFF_CLAMP_BRANCH_OP];
    let is_dynamic_unclamped = branch_op == OPCODE_BRA;

    let (_, main_sha256) = calculate_hashes(main_buf);
    let patch_va = base_va.wrapping_add(site as u32);
    let effective_gb = if is_dynamic_unclamped {
        blocks_2kb_to_gb(DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB)
    } else {
        blocks_2kb_to_gb(slot_200gb)
    };

    Ok(MainCapacityInspectionReport {
        input_path: input_label.to_string(),
        container_format: container_format.to_string(),
        build_name,
        raw_main_size: main_buf.len(),
        raw_main_sha256: main_sha256,
        patch_site_file_offset: site,
        patch_site_va_hex: format!("0x{patch_va:08X}"),
        max_product_lba_var_va_hex: format!("0x{max_lba_var:08X}"),
        physical_blocks_var_va_hex: format!("0x{phys_var:08X}"),
        clamp_branch_opcode_hex: format!("0x{branch_op:02X}"),
        is_dynamic_unclamped,
        slot_400gb_blocks_2kb: slot_400gb,
        slot_400gb_hex: format!("0x{slot_400gb:08X}"),
        slot_400gb_gb: blocks_2kb_to_gb(slot_400gb),
        slot_200gb_blocks_2kb: slot_200gb,
        slot_200gb_hex: format!("0x{slot_200gb:08X}"),
        slot_200gb_gb: blocks_2kb_to_gb(slot_200gb),
        slot_fallback_blocks_2kb: slot_fallback,
        slot_fallback_hex: format!("0x{slot_fallback:08X}"),
        slot_fallback_gb: blocks_2kb_to_gb(slot_fallback),
        effective_default_limit_gb: effective_gb,
        rtsd_presets: official_rtsd_presets(),
    })
}

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
        HddCapacityPatchMode::Factory200Gb => {
            let fb = 0x05D0_DDBA;
            (
                OPCODE_BLS,
                HDD_CMD_M7920_400GB_BLOCKS,
                HDD_CMD_M7920_200GB_BLOCKS,
                fb,
            )
        }
    };

    LittleEndian::write_u32(&mut main_buf[site + OFF_SLOT_400GB..], slot_400_val);
    LittleEndian::write_u32(&mut main_buf[site + OFF_SLOT_200GB..], slot_200_val);
    LittleEndian::write_u32(&mut main_buf[site + OFF_SLOT_FALLBACK..], slot_fb_val);
    main_buf[site + OFF_CLAMP_BRANCH_OP] = branch_op;

    Ok(slot_200_val)
}

pub(crate) fn inspect_unpacked_capacity(
    unpacked: &UnpackedExtrhead,
    container_format: &str,
    label: &str,
) -> Result<MainCapacityInspectionReport> {
    inspect_main_buffer_capacity(
        &unpacked.decompressed,
        unpacked.header.dst_addr,
        container_format,
        Some(unpacked.header.build_name.clone()),
        label,
    )
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

pub(crate) fn patch_and_repack_extrhead(
    unpacked: &mut UnpackedExtrhead,
    is_plain: bool,
    container_format: &str,
    in_label: &str,
    mode: HddCapacityPatchMode,
    target_gb: Option<u32>,
    passthrough_mode: Option<&str>,
) -> Result<(MainCapacityInspectionReport, u32, Option<String>, Vec<u8>)> {
    let before = inspect_unpacked_capacity(unpacked, container_format, in_label)?;
    let applied = apply_capacity_patch_in_place(&mut unpacked.decompressed, mode, target_gb)?;
    let applied_pt = apply_optional_passthrough(&mut unpacked.decompressed, passthrough_mode)?;
    let repacked = repack_from_header(&unpacked.decompressed, &unpacked.header, is_plain)?;
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

fn repack_from_header(payload: &[u8], hdr: &ExtrheadHeader, is_plain: bool) -> Result<Vec<u8>> {
    let mode = if is_plain {
        ExtrheadCryptMode::Plain
    } else {
        ExtrheadCryptMode::Auto
    };
    pack_extrhead(
        payload,
        &hdr.tag,
        hdr.dst_addr,
        &hdr.build_name,
        mode,
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

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;

    pub(crate) fn build_synthetic_hdd_cmd_main() -> Vec<u8> {
        let mut buf = vec![0u8; 0x300];
        let sig: [u8; WIN_LEN] = [
            0xFD, 0xAA, 0x00, 0x8A, 0x00, 0x00, 0xFB, 0xE9, 0x00, 0x80, 0xC8, 0x1D, 0xFC, 0xCC,
            0x74, 0x3B, 0xA4, 0x0B, 0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xCC, 0x57, 0xE9,
            0xA5, 0x57, 0xDD, 0xC0, 0x2B, 0x01, 0x00, 0x00, 0x00, 0xCA, 0x29, 0xFC, 0xCC, 0xBA,
            0x1D, 0xD2, 0x05, 0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xCC, 0x71, 0xE9, 0xA5,
            0x57, 0xDD, 0xA5, 0x2B, 0x01, 0x00, 0x00, 0x00, 0xCA, 0x0E, 0xFC, 0xCC, 0xC8, 0xD7,
            0xA6, 0x04, 0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xA5, 0xAC, 0xB9, 0x11, 0x53,
            0xFC, 0xA4, 0x80, 0xBC, 0x11, 0x53, 0xA1, 0xC6, 0x0C, 0x29, 0xFF, 0xFC, 0x85, 0x80,
            0xBC, 0x11, 0x53, 0xCA, 0x0A, 0x28, 0xFF, 0xFC, 0x81, 0x80, 0xBC, 0x11, 0x53, 0xFC,
            0xA4, 0x80,
        ];
        buf[0x80..0x80 + sig.len()].copy_from_slice(&sig);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::build_synthetic_hdd_cmd_main;
    use super::*;

    #[test]
    fn test_inspect_and_patch_dynamic_and_preset_250gb_and_restore() {
        let mut main = build_synthetic_hdd_cmd_main();

        let rep0 = inspect_main_buffer_capacity(
            &main,
            MAIN_DEFAULT_BASE_VA,
            "RAW_MAIN_BINARY",
            None,
            "synthetic",
        )
        .expect("Should locate clamp site");
        assert_eq!(rep0.patch_site_file_offset, 0x80);
        assert!(!rep0.is_dynamic_unclamped);
        assert_eq!(rep0.slot_200gb_blocks_2kb, HDD_CMD_M7920_200GB_BLOCKS);
        assert_eq!(rep0.slot_400gb_blocks_2kb, HDD_CMD_M7920_400GB_BLOCKS);

        let applied_dyn =
            apply_capacity_patch_in_place(&mut main, HddCapacityPatchMode::DynamicUnclamped, None)
                .expect("Dynamic patch succeeds");
        assert_eq!(applied_dyn, DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB);
        let rep_dyn = inspect_main_buffer_capacity(
            &main,
            MAIN_DEFAULT_BASE_VA,
            "RAW_MAIN_BINARY",
            None,
            "synthetic",
        )
        .expect("Should still match signature after dynamic patch");
        assert!(rep_dyn.is_dynamic_unclamped);
        assert_eq!(rep_dyn.clamp_branch_opcode_hex, "0xCA");

        let applied_250 = apply_capacity_patch_in_place(
            &mut main,
            HddCapacityPatchMode::FixedCapacity,
            Some(250),
        )
        .expect("250 GB patch succeeds");
        assert_eq!(applied_250, 0x0746_6528);
        let rep_250 = inspect_main_buffer_capacity(
            &main,
            MAIN_DEFAULT_BASE_VA,
            "RAW_MAIN_BINARY",
            None,
            "synthetic",
        )
        .expect("Should detect 250 GB preset");
        assert!(!rep_250.is_dynamic_unclamped);
        assert_eq!(rep_250.slot_200gb_blocks_2kb, 0x0746_6528);
        assert_eq!(rep_250.slot_400gb_blocks_2kb, 0x0746_6528);
        assert_eq!(rep_250.slot_fallback_blocks_2kb, 0x0746_6528);

        apply_capacity_patch_in_place(&mut main, HddCapacityPatchMode::Factory200Gb, None)
            .expect("Factory restore succeeds");
        let rep_fac = inspect_main_buffer_capacity(
            &main,
            MAIN_DEFAULT_BASE_VA,
            "RAW_MAIN_BINARY",
            None,
            "synthetic",
        )
        .expect("Should detect factory restore");
        assert_eq!(rep_fac.slot_200gb_blocks_2kb, HDD_CMD_M7920_200GB_BLOCKS);
        assert_eq!(rep_fac.slot_400gb_blocks_2kb, HDD_CMD_M7920_400GB_BLOCKS);
        assert_eq!(rep_fac.clamp_branch_opcode_hex, "0xC6");
    }
}
