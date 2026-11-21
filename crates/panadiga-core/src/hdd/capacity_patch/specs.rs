#![deny(unsafe_code)]

//! Pure immutable RTSD HDD capacity presets, DTO types, and total block-math conversions.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Default virtual load address for `MAIN` firmware binaries (`0x57000000`).
pub const MAIN_DEFAULT_BASE_VA: u32 = crate::constants::AM34_MAIN_BASE_VA;

/// Expected `MaxProductLBA` constants in `src/hdd_cmd.c` (2 KB blocks = 2048 bytes/block).
pub const HDD_CMD_M7920_400GB_BLOCKS: u32 = 0x0BA4_3B74; // 195,312,500 * 2KB = 400.00 GB
pub const HDD_CMD_M7920_200GB_BLOCKS: u32 = 0x05D2_1DBA; //  97,656,250 * 2KB = 200.00 GB
pub const HDD_CMD_M7920_FALLBACK_BLOCKS: u32 = crate::constants::PANA_HDD_CMD_M7920_FALLBACK_BLOCKS;
pub const DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB: u32 = 0x3FFF_FFFF; // ~2.199 TB (2KB blocks)
/// Extended 4 TiB ceiling (`0x7FFF_FFFF` 2KB blocks) requiring explicit opt-in via `allow_extended_4tb`.
pub const EXTENDED_4TB_MAX_BLOCKS_2KB: u32 = 0x7FFF_FFFF;

/// Fixed byte offsets inside the 114-byte (`0x72`) `src/hdd_cmd.c` clamp window.
pub(crate) const WIN_LEN: usize = 0x72;
pub(crate) const PREFIX_SIG: [u8; 14] = [
    0xFD, 0xAA, 0x00, 0x8A, 0x00, 0x00, 0xFB, 0xE9, 0x00, 0x80, 0xC8, 0x1D, 0xFC, 0xCC,
];
pub(crate) const OFF_SLOT_400GB: usize = 0x0E;
pub(crate) const OFF_MAX_LBA_VAR_1: usize = 0x14;
pub(crate) const OFF_SLOT_200GB: usize = 0x29;
pub(crate) const OFF_SLOT_FALLBACK: usize = 0x44;
pub(crate) const OFF_PHYS_VAR_1: usize = 0x56;
pub(crate) const OFF_CLAMP_BRANCH_OP: usize = 0x5B;
pub(crate) const OPCODE_BLS: u8 = 0xC6;
pub(crate) const OPCODE_BRA: u8 = 0xCA;

/// Audited 114-byte (`0x72`) `src/hdd_cmd.c` basic block template (`EH55_MAIN_3410PQ.bin` factory bytes).
#[rustfmt::skip]
pub(crate) const HDD_CMD_CLAMP_WINDOW_TEMPLATE: [u8; WIN_LEN] = [
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passthrough_mode: Option<String>,
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

const RTSD_PRESET_SPECS: &[(u8, &str, u32, u32)] = &[
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

/// Returns the 9 official RTSD `Change HDD Capacity` presets recovered from `MAIN` firmware.
pub fn official_rtsd_presets() -> Vec<RtsdCapacityPreset> {
    RTSD_PRESET_SPECS
        .iter()
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
/// for factory presets (`80..400 GB`), the verified `SAMSUNG SV0511D` 5.1 GB IDE bench drive (`5 GB` -> `0x0024C388`),
/// and computing SI decimal blocks `(gb * 1e9) / 2048` up to `DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB` (`0x3FFF_FFFF`, ~2.199 TB)
/// unless `allow_extended_4tb` explicitly authorizes up to `0x7FFF_FFFF`.
pub fn blocks_2kb_for_gb_bounded(target_gb: u32, allow_extended_4tb: bool) -> Result<u32> {
    if target_gb != 5 && target_gb < 20 {
        bail!("Target capacity {target_gb} GB is too small (minimum 20 GB required for MEIHDFS)");
    }
    if let Some(&(_, _, _, rtsd_blocks)) = RTSD_PRESET_SPECS
        .iter()
        .find(|&&(_, _, gb, _)| gb == target_gb)
    {
        return Ok(rtsd_blocks);
    }
    let blocks = match target_gb {
        // SAMSUNG SV0511D (9,965,088 LBAs): ((9,965,088 & !3) - 0x50000) >> 2 = 2,409,352 (2 KB blocks)
        5 => 0x0024_C388,
        other => {
            let computed = ((other as u64) * 1_000_000_000) / 2048;
            if !allow_extended_4tb && computed > u64::from(DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB) {
                bail!(
                    "Target capacity {other} GB ({computed} 2KB blocks) exceeds 2 TiB limit 0x{DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB:08X}; pass allow_extended_4tb=true to unlock up to 0x{EXTENDED_4TB_MAX_BLOCKS_2KB:08X}"
                );
            }
            if computed > u64::from(EXTENDED_4TB_MAX_BLOCKS_2KB) {
                EXTENDED_4TB_MAX_BLOCKS_2KB
            } else {
                computed as u32
            }
        }
    };
    Ok(blocks)
}

/// Maps a nominal GB size to exact 2 KB blocks (`MaxProductLBA`) bounded at `0x3FFF_FFFF` (~2.199 TB).
pub fn blocks_2kb_for_gb(target_gb: u32) -> Result<u32> {
    blocks_2kb_for_gb_bounded(target_gb, false)
}

#[inline]
pub(crate) fn blocks_2kb_to_gb(blocks: u32) -> f64 {
    let gb = ((blocks as f64) * 2048.0) / 1_000_000_000.0;
    (gb * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtsd_presets_and_gb_conversion_are_monotone_and_bounded() {
        let presets = official_rtsd_presets();
        assert_eq!(presets.len(), 9);
        for preset in &presets {
            assert_eq!(
                blocks_2kb_for_gb(preset.capacity_gb).unwrap(),
                preset.blocks_2kb
            );
        }
        assert_eq!(blocks_2kb_for_gb(400).unwrap(), 0x0BA3_FB74);
        assert_eq!(blocks_2kb_for_gb(200).unwrap(), 0x05D1_DDBA);
        assert_eq!(blocks_2kb_for_gb(250).unwrap(), 0x0746_6528);
        assert_eq!(blocks_2kb_for_gb(5).unwrap(), 0x0024_C388);
        assert!(blocks_2kb_for_gb(10).is_err());
        assert!(blocks_2kb_for_gb(500).unwrap() > blocks_2kb_for_gb(400).unwrap());

        let two_tb = blocks_2kb_for_gb(2000).unwrap();
        assert!(two_tb <= DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB);
        assert_eq!(two_tb, 0x3A35_2944);
        assert!(blocks_2kb_for_gb(2500).is_err());
        assert!(blocks_2kb_for_gb_bounded(2500, true).unwrap() > DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB);
        assert_eq!(
            blocks_2kb_for_gb_bounded(5000, true).unwrap(),
            EXTENDED_4TB_MAX_BLOCKS_2KB
        );
    }
}
