//! Panasonic DIGA Dynamic HDD Capacity Unlock & `src/hdd_cmd.c` Patcher
//!
//! Empirically derived from `MAIN3060NP` (`0x574E499C`) and `EH55_MAIN_3410PQ` (`0x574DD1FC`):
//! Panasonic's MEIHDFS / LBA48 storage stack supports full physical drive capacities natively,
//! but `src/hdd_cmd.c` deliberately clamps the detected 2 KB block count (`PhysicalBlocks2KB`)
//! against a software ceiling `MaxProductLBA` (200 GB = `0x05D21DBA` on DMR-EH55 vs. 400 GB =
//! `0x0BA43B74` on `M7920` when option bit `0x8000` at `DevInfo(option)+0x8A` is set).
//!
//! Changing the clamp branch at `+0x5B` from `bls +12` (`0xC6 0x0C`) to `bra +12` (`0xCA 0x0C`)
//! and raising the 3 `MaxProductLBA` constant slots (`+0x0E`, `+0x29`, `+0x44`) unlocks dynamic
//! physical auto-detection for ANY HDD capacity (250 GB, 320 GB, 500 GB, 1 TB, 2 TB+) via
//! ATA `IDENTIFY DEVICE` LBA48.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use crate::constants::*;
use crate::crypto::{ExtrheadCryptMode, pack_extrhead, unpack_extrhead};
use crate::nvram::{calculate_hashes, resolve_input_path};

/// Default virtual load address for `MAIN` firmware binaries (`0x57000000`).
pub const MAIN_DEFAULT_BASE_VA: u32 = 0x5700_0000;

/// Expected `MaxProductLBA` constants in `src/hdd_cmd.c` (2 KB blocks = 2048 bytes/block).
pub const HDD_CMD_M7920_400GB_BLOCKS: u32 = 0x0BA4_3B74; // 195,312,500 * 2KB = 400.00 GB
pub const HDD_CMD_M7920_200GB_BLOCKS: u32 = 0x05D2_1DBA; //  97,656,250 * 2KB = 200.00 GB
pub const DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB: u32 = 0x3FFF_FFFF; // ~2.199 TB (2KB blocks)

/// Fixed byte offsets inside the 114-byte (`0x72`) `src/hdd_cmd.c` clamp window.
const WIN_LEN: usize = 0x72;
const PREFIX_SIG: [u8; 14] = [
    0xFD, 0xAA, 0x00, 0x8A, 0x00, 0x00, 0xFB, 0xE9, 0x00, 0x80, 0xC8, 0x1D, 0xFC, 0xCC,
];
const OFF_SLOT_400GB: usize = 0x0E;
const OFF_MAX_LBA_VAR_1: usize = 0x14;
const OFF_SLOT_200GB: usize = 0x29;
const OFF_SLOT_FALLBACK: usize = 0x44;
const OFF_PHYS_VAR_1: usize = 0x56;
const OFF_CLAMP_BRANCH_OP: usize = 0x5B;
const OPCODE_BLS: u8 = 0xC6;
const OPCODE_BRA: u8 = 0xCA;

/// Official RTSD service menu preset (`Change HDD Capacity` @ `0x574CA054` / `0x574C267B`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RtsdCapacityPreset {
    pub menu_index: u8,
    pub label: String,
    pub capacity_gb: u32,
    pub blocks_2kb: u32,
    pub blocks_2kb_hex: String,
    pub sectors_512b: u64,
}

/// Complete forensic report for `src/hdd_cmd.c` HDD capacity ceiling inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainCapacityInspectionReport {
    pub input_path: String,
    pub container_format: String,
    pub build_name: Option<String>,
    pub raw_main_size: usize,
    pub raw_main_sha256: String,
    pub patch_site_file_offset: usize,
    pub patch_site_va_hex: String,
    pub max_product_lba_var_va_hex: String,
    pub physical_blocks_var_va_hex: String,
    pub clamp_branch_opcode_hex: String,
    pub is_dynamic_unclamped: bool,
    pub slot_400gb_blocks_2kb: u32,
    pub slot_400gb_hex: String,
    pub slot_400gb_gb: f64,
    pub slot_200gb_blocks_2kb: u32,
    pub slot_200gb_hex: String,
    pub slot_200gb_gb: f64,
    pub slot_fallback_blocks_2kb: u32,
    pub slot_fallback_hex: String,
    pub slot_fallback_gb: f64,
    pub effective_default_limit_gb: f64,
    pub rtsd_presets: Vec<RtsdCapacityPreset>,
}

/// Patch execution summary for `src/hdd_cmd.c` (+ optional `EXTRHEAD` / `MEIHDFS1.0` MBR update).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainCapacityPatchReport {
    pub input_path: String,
    pub output_path: String,
    pub container_format: String,
    pub mode: String,
    pub target_gb: Option<u32>,
    pub applied_blocks_2kb: u32,
    pub applied_blocks_2kb_hex: String,
    pub applied_sectors_512b: u64,
    pub mbr_superblocks_updated: bool,
    pub repacked_container_size: Option<usize>,
    pub original_sha256: String,
    pub patched_sha256: String,
    pub inspection_before: MainCapacityInspectionReport,
    pub inspection_after: MainCapacityInspectionReport,
}

/// Supported patch strategies for the HDD capacity limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HddCapacityPatchMode {
    /// Bypass the software clamp (`bls` -> `bra`) and raise slots so ANY physical drive works at 100%.
    DynamicUnclamped,
    /// Keep the clamp active (`bls`) and set all 3 slots to a specific preset or custom GB capacity.
    FixedCapacity,
    /// Restore factory DMR-EH55 defaults (`bls +12`, `0x0BA43B74` / `0x05D21DBA`).
    Factory200Gb,
}

impl std::str::FromStr for HddCapacityPatchMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "dynamic" | "unclamped" | "auto" => Ok(Self::DynamicUnclamped),
            "cap" | "fixed" | "preset" | "limit" => Ok(Self::FixedCapacity),
            "factory" | "factory_200gb" | "default" | "restore" => Ok(Self::Factory200Gb),
            other => bail!(
                "Unsupported capacity patch mode '{other}': expected dynamic, cap, or factory_200gb"
            ),
        }
    }
}

/// Returns the 9 official RTSD `Change HDD Capacity` presets recovered from `MAIN` firmware.
pub fn official_rtsd_presets() -> Vec<RtsdCapacityPreset> {
    let raw: &[(u8, &str, u32, u32)] = &[
        (1, "400GB HDD (RTSD)", 400, 0x0BA3_FB74),
        (2, "300GB HDD (RTSD)", 300, 0x08BA_EC97),
        (3, "250GB HDD (RTSD)", 250, 0x0746_6528),
        (4, "200GB HDD (RTSD)", 200, 0x05D1_DDBA),
        (5, "160GB HDD (RTSD)", 160, 0x04A7_D7C8),
        (6, "137GB HDD (RTSD)", 137, 0x03FC_7AC3),
        (7, "120GB HDD (RTSD)", 120, 0x037D_D1D6),
        (8, "100GB HDD (RTSD)", 100, 0x02E8_CEDD),
        (9, " 80GB HDD (RTSD)", 80, 0x0253_CBE4),
    ];
    raw.iter()
        .map(|&(idx, label, gb, blocks)| RtsdCapacityPreset {
            menu_index: idx,
            label: label.to_string(),
            capacity_gb: gb,
            blocks_2kb: blocks,
            blocks_2kb_hex: format!("0x{blocks:08X}"),
            sectors_512b: (blocks as u64) * 4,
        })
        .collect()
}

/// Maps a nominal GB size to exact 2 KB blocks (`MaxProductLBA`), matching Panasonic's RTSD table
/// for factory presets (`80..400 GB`) and computing SI decimal blocks `(gb * 1e9) / 2048` otherwise.
pub fn blocks_2kb_for_gb(target_gb: u32) -> Result<u32> {
    if target_gb < 20 {
        bail!("Target capacity {target_gb} GB is too small (minimum 20 GB required for MEIHDFS)");
    }
    let blocks = match target_gb {
        400 => HDD_CMD_M7920_400GB_BLOCKS,
        300 => 0x08BA_EC97,
        250 => 0x0746_6528,
        200 => HDD_CMD_M7920_200GB_BLOCKS,
        160 => 0x04A7_D7C8,
        137 => 0x03FC_7AC3,
        120 => 0x037D_D1D6,
        100 => 0x02E8_CEDD,
        80 => 0x0253_CBE4,
        other => {
            let computed = ((other as u64) * 1_000_000_000) / 2048;
            if computed > 0x7FFF_FFFF {
                0x7FFF_FFFF
            } else {
                computed as u32
            }
        }
    };
    Ok(blocks)
}

#[inline]
fn blocks_2kb_to_gb(blocks: u32) -> f64 {
    let gb = ((blocks as f64) * 2048.0) / 1_000_000_000.0;
    (gb * 100.0).round() / 100.0
}

/// Locates the 114-byte `src/hdd_cmd.c` capacity-clamping basic block inside an unpacked `MAIN` image.
pub fn find_hdd_cmd_clamp_site(main_buf: &[u8]) -> Option<usize> {
    if main_buf.len() < WIN_LEN {
        return None;
    }
    for pos in 0..=(main_buf.len() - WIN_LEN) {
        let win = &main_buf[pos..pos + WIN_LEN];
        if win[0x00..0x0E] != PREFIX_SIG {
            continue;
        }
        if win[0x12..0x14] != [0xFC, 0x81] {
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

enum LoadedMainSource {
    RawMain {
        main_bytes: Vec<u8>,
        base_va: u32,
    },
    Extrhead {
        unpacked: crate::crypto::UnpackedExtrhead,
        is_plain: bool,
        padded_len: usize,
    },
    FullHddP1 {
        image_bytes: Vec<u8>,
        p1_container_len: usize,
        unpacked: crate::crypto::UnpackedExtrhead,
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

fn load_main_source(raw: Vec<u8>) -> Result<LoadedMainSource> {
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
        let padded_len = raw.len();
        return Ok(LoadedMainSource::Extrhead {
            unpacked,
            is_plain,
            padded_len,
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

/// Applies the selected HDD capacity patch directly onto an unpacked `MAIN` buffer in place.
/// Returns the applied block count (`blocks_2kb`) and `(branch_opcode, slot400, slot200, slot_fallback)`.
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

fn inspect_unpacked_capacity(
    unpacked: &crate::crypto::UnpackedExtrhead,
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

/// Inspects a file (`RAW_MAIN_BINARY`, `EXTRHEAD_CONTAINER`, or `FULL_HDD_IMAGE_P1`) on disk.
pub fn inspect_main_hdd_capacity(input_path: &Path) -> Result<MainCapacityInspectionReport> {
    let resolved = resolve_input_path(input_path)?;
    let raw = fs::read(&resolved)
        .with_context(|| format!("Failed reading firmware input: {}", resolved.display()))?;
    let label = resolved.to_string_lossy().into_owned();

    match load_main_source(raw)? {
        LoadedMainSource::RawMain {
            main_bytes,
            base_va,
        } => inspect_main_buffer_capacity(&main_bytes, base_va, "RAW_MAIN_BINARY", None, &label),
        LoadedMainSource::Extrhead { unpacked, .. } => {
            inspect_unpacked_capacity(&unpacked, "EXTRHEAD_CONTAINER", &label)
        }
        LoadedMainSource::FullHddP1 { unpacked, .. } => {
            inspect_unpacked_capacity(&unpacked, "FULL_HDD_IMAGE_P1", &label)
        }
    }
}

fn patch_and_repack_extrhead(
    unpacked: &mut crate::crypto::UnpackedExtrhead,
    is_plain: bool,
    container_format: &str,
    in_label: &str,
    mode: HddCapacityPatchMode,
    target_gb: Option<u32>,
) -> Result<(MainCapacityInspectionReport, u32, Vec<u8>)> {
    let before = inspect_unpacked_capacity(unpacked, container_format, in_label)?;
    let applied = apply_capacity_patch_in_place(&mut unpacked.decompressed, mode, target_gb)?;
    let repacked = repack_from_header(&unpacked.decompressed, &unpacked.header, is_plain)?;
    Ok((before, applied, repacked))
}

fn verify_and_inspect_repacked(
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
    hdr: &crate::crypto::ExtrheadHeader,
    is_plain: bool,
) -> Result<Vec<u8>> {
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

fn maybe_update_procedural_mbr_capacity(image_bytes: &mut [u8], new_total_sectors: u64) -> bool {
    if image_bytes.len() < (LBA_MIRROR_MBR_OFFSET as usize) + LBA_SECTOR_SIZE_BYTES {
        return false;
    }
    // Real Panasonic dumps keep bytes 16..512 strictly zeroed (`zeros_after_header_clean`);
    // procedural mkfs.meihdfs images store explicit total_sectors at LBA 0 byte 24.
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

/// Computes the default output path (`<stem>_<suffix>.<ext>`) when the caller omits an explicit output file.
pub fn default_capacity_patched_output_path(
    input_path: &Path,
    mode_str: &str,
    target_gb: Option<u32>,
) -> std::path::PathBuf {
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("main_firmware");
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("bin");
    let suffix = match (mode_str.trim().to_ascii_lowercase().as_str(), target_gb) {
        ("factory" | "factory_200gb" | "default" | "restore", _) => "FACTORY_200GB".to_string(),
        (_, Some(gb)) => format!("PATCHED_{gb}GB"),
        _ => "PATCHED_DYNAMIC".to_string(),
    };
    parent.join(format!("{stem}_{suffix}.{ext}"))
}

/// Applies a dynamic or preset HDD capacity patch and writes the output artifact (`RAW`, `EXTRHEAD`, or full HDD image).
pub fn patch_main_hdd_capacity(
    input_path: &Path,
    output_path: &Path,
    mode_str: &str,
    target_gb: Option<u32>,
) -> Result<MainCapacityPatchReport> {
    let mode: HddCapacityPatchMode = mode_str.parse()?;
    let resolved_in = resolve_input_path(input_path)?;
    let raw = fs::read(&resolved_in)
        .with_context(|| format!("Failed reading firmware input: {}", resolved_in.display()))?;
    let (_, original_sha256) = calculate_hashes(&raw);
    let in_label = resolved_in.to_string_lossy().into_owned();

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Creating output directory {}", parent.display()))?;
    }

    let (container_format, before, after, applied_blocks, mbr_updated, repacked_len, out_bytes) =
        match load_main_source(raw)? {
            LoadedMainSource::RawMain {
                mut main_bytes,
                base_va,
            } => {
                let before = inspect_main_buffer_capacity(
                    &main_bytes,
                    base_va,
                    "RAW_MAIN_BINARY",
                    None,
                    &in_label,
                )?;
                let applied = apply_capacity_patch_in_place(&mut main_bytes, mode, target_gb)?;
                let after = inspect_main_buffer_capacity(
                    &main_bytes,
                    base_va,
                    "RAW_MAIN_BINARY",
                    None,
                    &output_path.to_string_lossy(),
                )?;
                (
                    "RAW_MAIN_BINARY".to_string(),
                    before,
                    after,
                    applied,
                    false,
                    None,
                    main_bytes,
                )
            }
            LoadedMainSource::Extrhead {
                mut unpacked,
                is_plain,
                padded_len,
            } => {
                let (before, applied, repacked) = patch_and_repack_extrhead(
                    &mut unpacked,
                    is_plain,
                    "EXTRHEAD_CONTAINER",
                    &in_label,
                    mode,
                    target_gb,
                )?;
                let repacked_sz = repacked.len();
                let after = verify_and_inspect_repacked(
                    &repacked,
                    "EXTRHEAD_CONTAINER",
                    &output_path.to_string_lossy(),
                )?;
                let _ = padded_len;
                (
                    "EXTRHEAD_CONTAINER".to_string(),
                    before,
                    after,
                    applied,
                    false,
                    Some(repacked_sz),
                    repacked,
                )
            }
            LoadedMainSource::FullHddP1 {
                mut image_bytes,
                p1_container_len,
                mut unpacked,
                is_plain,
            } => {
                let (before, applied, repacked) = patch_and_repack_extrhead(
                    &mut unpacked,
                    is_plain,
                    "FULL_HDD_IMAGE_P1",
                    &in_label,
                    mode,
                    target_gb,
                )?;
                let repacked_sz = repacked.len();

                let p1_off = LBA_PARTITION_1_OFFSET as usize;
                let p2_off = LBA_PARTITION_2_OFFSET as usize;
                if p1_off + repacked_sz > p2_off {
                    bail!(
                        "Repacked Partition 1 ({} bytes) exceeds 32 MB gap before Partition 2 ({} bytes)",
                        repacked_sz,
                        p2_off - p1_off
                    );
                }
                if repacked_sz < p1_container_len {
                    image_bytes[p1_off..p1_off + p1_container_len].fill(0);
                }
                image_bytes[p1_off..p1_off + repacked_sz].copy_from_slice(&repacked);

                let mbr_updated = if let Some(gb) = target_gb {
                    maybe_update_procedural_mbr_capacity(
                        &mut image_bytes,
                        (gb as u64 * 1_000_000_000) / 512,
                    )
                } else {
                    maybe_update_procedural_mbr_capacity(&mut image_bytes, (applied as u64) * 4)
                };

                let after = verify_and_inspect_repacked(
                    &image_bytes[p1_off..p1_off + repacked_sz],
                    "FULL_HDD_IMAGE_P1",
                    &output_path.to_string_lossy(),
                )?;
                (
                    "FULL_HDD_IMAGE_P1".to_string(),
                    before,
                    after,
                    applied,
                    mbr_updated,
                    Some(repacked_sz),
                    image_bytes,
                )
            }
        };

    let (_, patched_sha256) = calculate_hashes(&out_bytes);
    fs::write(output_path, &out_bytes).with_context(|| {
        format!(
            "Writing patched firmware output to {}",
            output_path.display()
        )
    })?;

    Ok(MainCapacityPatchReport {
        input_path: in_label,
        output_path: output_path.to_string_lossy().into_owned(),
        container_format,
        mode: mode_str.to_string(),
        target_gb,
        applied_blocks_2kb: applied_blocks,
        applied_blocks_2kb_hex: format!("0x{applied_blocks:08X}"),
        applied_sectors_512b: (applied_blocks as u64) * 4,
        mbr_superblocks_updated: mbr_updated,
        repacked_container_size: repacked_len,
        original_sha256,
        patched_sha256,
        inspection_before: before,
        inspection_after: after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_synthetic_hdd_cmd_main() -> Vec<u8> {
        let mut buf = vec![0u8; 0x300];
        // Exact 114-byte (`WIN_LEN`) `src/hdd_cmd.c` sequence matching 3410PQ @ 0x574dd1fc
        let sig: [u8; WIN_LEN] = [
            0xFD, 0xAA, 0x00, 0x8A, 0x00, 0x00, 0xFB, 0xE9, 0x00, 0x80, 0xC8, 0x1D, 0xFC, 0xCC,
            0x74, 0x3B, 0xA4, 0x0B, // +0x0E: 0x0BA43B74 (400GB)
            0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, // MaxProductLBA = 0x5311B9AC
            0xFC, 0xCC, 0x57, 0xE9, 0xA5, 0x57, 0xDD, 0xC0, 0x2B, 0x01, 0x00, 0x00, 0x00, 0xCA,
            0x29, 0xFC, 0xCC, 0xBA, 0x1D, 0xD2, 0x05, // +0x29: 0x05D21DBA (200GB)
            0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xCC, 0x71, 0xE9, 0xA5, 0x57, 0xDD, 0xA5,
            0x2B, 0x01, 0x00, 0x00, 0x00, 0xCA, 0x0E, 0xFC, 0xCC, 0xC8, 0xD7, 0xA6,
            0x04, // +0x44: 0x04A6D7C8 (160GB fallback)
            0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xA5, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xA4,
            0x80, 0xBC, 0x11, 0x53, // PhysicalBlocks = 0x5311BC80
            0xA1, 0xC6, 0x0C, // +0x5A: cmp d1, d0; +0x5B: bls +12
            0x29, 0xFF, 0xFC, 0x85, 0x80, 0xBC, 0x11, 0x53, 0xCA, 0x0A, 0x28, 0xFF, 0xFC, 0x81,
            0x80, 0xBC, 0x11, 0x53, 0xFC, 0xA4, 0x80,
        ];
        buf[0x80..0x80 + sig.len()].copy_from_slice(&sig);
        buf
    }

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

        // Patch 1: Dynamic Unclamped (Auto-detect ANY size up to ~2.2 TB)
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

        // Patch 2: Specific 250 GB preset (0x07466528 = 122,053,928 * 2KB blocks)
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

        // Patch 3: Restore factory defaults
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

    #[test]
    fn test_extrhead_container_roundtrip_capacity_patch() {
        let main = build_synthetic_hdd_cmd_main();
        let packed = pack_extrhead(
            &main,
            "MAIN",
            MAIN_DEFAULT_BASE_VA,
            "TEST3410PQ",
            ExtrheadCryptMode::Main,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
            0x0101,
        )
        .expect("Pack synthetic EXTRHEAD");

        let tmp = std::env::temp_dir().join("panadiga_cap_test");
        let _ = fs::create_dir_all(&tmp);
        let in_file = tmp.join("test_p1_in.bin");
        let out_file = tmp.join("test_p1_out.bin");
        fs::write(&in_file, &packed).expect("Write input");

        let report = patch_main_hdd_capacity(&in_file, &out_file, "dynamic", None)
            .expect("Patch EXTRHEAD container");
        assert_eq!(report.container_format, "EXTRHEAD_CONTAINER");
        assert!(report.inspection_after.is_dynamic_unclamped);

        let inspect_out = inspect_main_hdd_capacity(&out_file).expect("Inspect output EXTRHEAD");
        assert!(inspect_out.is_dynamic_unclamped);
        let _ = fs::remove_dir_all(&tmp);
    }
}
