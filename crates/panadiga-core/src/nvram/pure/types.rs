//! Domain DTOs and Spansion NOR Flash geometry resolution for NVRAM & Upper NOR operations.

use anyhow::{Result, bail};

use crate::constants::{FLASH_16MB_NVRAM_SIZE_BYTES, SPANSION_NVRAM_SIZE_BYTES};

/// Audit metrics for an individual 8 KB micro-sector.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MicroSectorAudit {
    pub sector_name: String,
    pub offset: usize,
    pub active_bytes: usize,
    pub total_bytes: usize,
    pub header_hex: String,
}

/// Comprehensive audit summary of the flash cleaning operation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NvramCleanReport {
    pub input_file: String,
    pub output_file: String,
    pub backup_file: String,
    pub original_md5: String,
    pub original_sha256: String,
    pub clean_md5: String,
    pub clean_sha256: String,
    pub total_flash_bytes: usize,
    pub nvram_offset_start: usize,
    pub nvram_offset_end: usize,
    pub nvram_non_ff_bytes: usize,
    pub nvram_non_zero_bytes: usize,
    pub microsectors: Vec<MicroSectorAudit>,
    pub bootloader_immutable: bool,
}

/// Pure functional output containing cleaned flash bytes, extracted backup, and audit report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PureNvramCleanOutput {
    pub cleaned_bytes: Vec<u8>,
    pub backup_nvram_bytes: Vec<u8>,
    pub report: NvramCleanReport,
}

/// Comprehensive audit summary for a Copy-on-Write Upper NOR App/BGO flash patch.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpperNorPatchReport {
    pub input_file: String,
    pub output_file: String,
    pub backup_file: String,
    pub original_md5: String,
    pub original_sha256: String,
    pub patched_md5: String,
    pub patched_sha256: String,
    pub backup_sector_sha256: String,
    pub patched_range_before_sha256: String,
    pub patched_range_after_sha256: String,
    pub total_flash_bytes: usize,
    pub patch_offset_start: usize,
    pub patch_offset_end: usize,
    pub patch_length: usize,
    pub backup_sector_start: usize,
    pub backup_sector_end: usize,
    pub first_sector_name: String,
    pub last_sector_name: String,
    pub critical_ipl_override_used: bool,
    pub critical_ipl_untouched: bool,
    pub nvram_untouched: bool,
}

/// Pure functional output containing patched flash bytes, pre-patch 64 KiB sector backup, and audit report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PureUpperNorPatchOutput {
    pub patched_bytes: Vec<u8>,
    pub backup_sector_bytes: Vec<u8>,
    pub report: UpperNorPatchReport,
}

pub(super) struct FlashLayoutGeometry {
    pub flash_len: usize,
    pub nvram_base_offset: usize,
    pub base_sector_idx: usize,
}

pub(super) fn resolve_flash_layout(flash_len: usize) -> Result<FlashLayoutGeometry> {
    let valid_capacities = [
        2 * 1024 * 1024,
        4 * 1024 * 1024,
        8 * 1024 * 1024,
        16 * 1024 * 1024,
    ];
    if !valid_capacities.contains(&flash_len) {
        bail!(
            "Invalid flash dump size: {} bytes. Expected standard Panasonic DIGA NOR Flash capacity (2 MB, 4 MB, 8 MB or 16 MB).",
            flash_len
        );
    }
    let is_16mb = flash_len == 16 * 1024 * 1024;
    let nvram_size = if is_16mb {
        FLASH_16MB_NVRAM_SIZE_BYTES
    } else {
        SPANSION_NVRAM_SIZE_BYTES
    };
    let base_sector_idx = match flash_len {
        8_388_608 => 134, // S29JL064H Top-Boot SA134..SA141
        4_194_304 => 70,  // S29JL032H Top-Boot SA70..SA77
        2_097_152 => 38,  // S29AL016J Top-Boot SA38..SA45
        _ => 254,         // S29GL128 / S29JL128 Top-Boot SA254..SA269 (16 sectors)
    };
    Ok(FlashLayoutGeometry {
        flash_len,
        nvram_base_offset: flash_len - nvram_size,
        base_sector_idx,
    })
}
