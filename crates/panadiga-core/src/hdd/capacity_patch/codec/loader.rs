#![deny(unsafe_code)]

//! Loader and inspection helpers for `src/hdd_cmd.c` capacity clamping in `MAIN` binaries & HDD P1.

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use crate::constants::*;
use crate::crypto::extrhead::detect_extrhead_key_and_double;
use crate::crypto::{ExtrheadCryptMode, UnpackedExtrhead, unpack_extrhead};
use crate::nvram::calculate_hashes;

use super::super::specs::{
    DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB, MAIN_DEFAULT_BASE_VA, MainCapacityInspectionReport,
    OFF_CLAMP_BRANCH_OP, OFF_MAX_LBA_VAR_1, OFF_PHYS_VAR_1, OFF_SLOT_200GB, OFF_SLOT_400GB,
    OFF_SLOT_FALLBACK, OPCODE_BLS, OPCODE_BRA, PREFIX_SIG, WIN_LEN, blocks_2kb_to_gb,
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
        round_key: [u8; 32],
    },
    FullHddP1 {
        image_bytes: Vec<u8>,
        p1_container_len: usize,
        unpacked: UnpackedExtrhead,
        is_plain: bool,
        round_key: [u8; 32],
    },
}

fn detect_extrhead_slice_key(slice: &[u8]) -> Option<(bool, [u8; 32], bool)> {
    if slice.len() < PANA_EXTRHEAD_HEADER_SIZE {
        return None;
    }
    if slice.starts_with(b"EXTRHEAD") {
        return Some((true, PANA_FEISTEL_ROUND_KEY_32B, false));
    }
    detect_extrhead_key_and_double(&slice[..8], None).map(|(rk, dbl)| (false, rk, dbl))
}

fn is_full_hdd_with_p1(raw: &[u8]) -> bool {
    let p1 = LBA_PARTITION_1_OFFSET as usize;
    raw.len() >= (LBA_PARTITION_2_OFFSET as usize)
        && raw[..8] == PANASONIC_HDD_MBR_SIGNATURE
        && (raw[p1..].starts_with(&PANASONIC_HDD_PARTITION_MAGIC)
            || detect_extrhead_slice_key(&raw[p1..]).is_some())
}

fn detect_extrhead_container_len(slice: &[u8]) -> Result<(usize, bool, [u8; 32])> {
    if slice.len() < PANA_EXTRHEAD_HEADER_SIZE {
        bail!("EXTRHEAD slice shorter than {PANA_EXTRHEAD_HEADER_SIZE} bytes");
    }
    let (is_plain, rk, double_encrypted) = detect_extrhead_slice_key(slice)
        .context("EXTRHEAD P1 header failed multi-key Feistel auto-detection")?;
    let mut hdr = [0u8; PANA_EXTRHEAD_HEADER_SIZE];
    hdr.copy_from_slice(&slice[..PANA_EXTRHEAD_HEADER_SIZE]);
    if !is_plain {
        crate::crypto::feistel_decrypt_buffer(&mut hdr, &rk, &PANA_FEISTEL_SBOX_256);
        if double_encrypted {
            crate::crypto::feistel_decrypt_buffer(&mut hdr, &rk, &PANA_FEISTEL_SBOX_256);
        }
    }
    let parsed = crate::crypto::parse_extrhead_header(&hdr)?;
    let footer_span = if parsed.footer_offset >= PANA_EXTRHEAD_HEADER_SIZE as u32 {
        parsed.footer_offset as usize + PANA_EXTRHEAD_FOOTER_SIZE
    } else {
        PANA_EXTRHEAD_HEADER_SIZE + parsed.src_len as usize + PANA_EXTRHEAD_FOOTER_SIZE
    };
    let aligned = (footer_span + 7) & !7;
    let c_len = if aligned > slice.len() {
        slice.len() & !7
    } else {
        aligned
    };
    Ok((c_len, is_plain, rk))
}

pub(crate) fn load_main_source(raw: Vec<u8>) -> Result<LoadedMainSource> {
    if is_full_hdd_with_p1(&raw) {
        let p1_off = LBA_PARTITION_1_OFFSET as usize;
        let p2_off = LBA_PARTITION_2_OFFSET as usize;
        let max_p1 = &raw[p1_off..p2_off];
        let (c_len, is_plain, round_key) = detect_extrhead_container_len(max_p1)?;
        let unpacked = unpack_extrhead(&max_p1[..c_len], ExtrheadCryptMode::Auto)?;
        return Ok(LoadedMainSource::FullHddP1 {
            image_bytes: raw,
            p1_container_len: c_len,
            unpacked,
            is_plain,
            round_key,
        });
    }

    if let Some((is_plain, round_key, _)) = detect_extrhead_slice_key(&raw) {
        let unpacked = unpack_extrhead(&raw, ExtrheadCryptMode::Auto)?;
        return Ok(LoadedMainSource::Extrhead {
            unpacked,
            is_plain,
            round_key,
        });
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
