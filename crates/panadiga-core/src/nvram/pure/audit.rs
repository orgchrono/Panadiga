//! Pure hashing, derived NVRAM sector breakdown, C0 dump inspection, and live Pico NVRAM capture audit.

use anyhow::{Result, bail};
use md5::{Digest as _, Md5};
use sha2::Sha256;

use super::types::{
    MicroSectorAudit, NvramCleanReport, nvram_sectors_for_capacity, resolve_flash_layout,
};
use crate::constants::{
    FLASH_ERASED_BYTE, SPANSION_FLASH_SIZE_BYTES, SPANSION_MICROSECTOR_SIZE,
    SPANSION_NVRAM_BASE_OFFSET,
};
use crate::ssot::{DerivedSector, HardwareProfileRegistry, fingerprint_nor_or_firmware};

const PARALLEL_HASH_THRESHOLD_BYTES: usize = 64 * 1024;

/// Computes MD5 and SHA-256 hashes of a byte slice (in parallel above 64 KiB).
pub fn calculate_hashes(data: &[u8]) -> (String, String) {
    let compute_md5 = || {
        let mut md5_hasher = Md5::new();
        md5_hasher.update(data);
        hex::encode(md5_hasher.finalize())
    };
    let compute_sha256 = || {
        let mut sha_hasher = Sha256::new();
        sha_hasher.update(data);
        hex::encode(sha_hasher.finalize())
    };

    if data.len() >= PARALLEL_HASH_THRESHOLD_BYTES {
        rayon::join(compute_md5, compute_sha256)
    } else {
        (compute_md5(), compute_sha256())
    }
}

/// Computes activity counters and exact derived sector audits over `[nvram_base..]`.
pub fn audit_nvram_sectors(
    nvram_slice: &[u8],
    sectors: &[DerivedSector],
) -> (usize, usize, Vec<MicroSectorAudit>) {
    let non_ff = nvram_slice
        .iter()
        .filter(|&&b| b != FLASH_ERASED_BYTE)
        .count();
    let non_zero = nvram_slice.iter().filter(|&&b| b != 0x00).count();
    let base_offset = sectors.first().map_or(0, |s| s.start_offset as usize);
    let mut microsectors = Vec::with_capacity(sectors.len());

    for sec in sectors {
        let rel_start = (sec.start_offset as usize).saturating_sub(base_offset);
        if rel_start >= nvram_slice.len() {
            break;
        }
        let sec_size = sec.size_bytes as usize;
        let rel_end = (rel_start + sec_size).min(nvram_slice.len());
        let sec_slice = &nvram_slice[rel_start..rel_end];
        let used = sec_slice
            .iter()
            .filter(|&&b| b != FLASH_ERASED_BYTE)
            .count();
        let hdr_len = sec_slice.len().min(8);
        let header_hex = hex::encode_upper(&sec_slice[..hdr_len]);

        microsectors.push(MicroSectorAudit {
            sector_name: format!("SA{}", sec.index),
            offset: sec.start_offset as usize,
            active_bytes: used,
            total_bytes: sec_slice.len(),
            header_hex,
        });
    }

    (non_ff, non_zero, microsectors)
}

/// Compatibility helper computing NVRAM breakdown for uniform 8 KiB sector chunks.
pub fn audit_nvram_slice(
    nvram_slice: &[u8],
    nvram_base_offset: usize,
    base_sector_idx: usize,
) -> (usize, usize, Vec<MicroSectorAudit>) {
    let sector_count = nvram_slice.len().div_ceil(SPANSION_MICROSECTOR_SIZE).max(1);
    let sectors: Vec<DerivedSector> = (0..sector_count)
        .map(|i| {
            let start = nvram_base_offset + i * SPANSION_MICROSECTOR_SIZE;
            DerivedSector {
                index: (base_sector_idx + i) as u32,
                start_offset: start as u32,
                end_offset_inclusive: (start + SPANSION_MICROSECTOR_SIZE - 1) as u32,
                size_bytes: SPANSION_MICROSECTOR_SIZE as u32,
                bank_name: "nvram".to_string(),
            }
        })
        .collect();
    audit_nvram_sectors(nvram_slice, &sectors)
}

/// Strictly read-only (C0) pure inspection of an in-memory flash dump without mutating or gating writes.
pub fn inspect_flash_nvram_slice(
    flash_bytes: &[u8],
    input_label: &str,
) -> Result<NvramCleanReport> {
    let fp = fingerprint_nor_or_firmware(flash_bytes);
    let resolved_opt = fp
        .detected_model
        .as_deref()
        .and_then(|m| HardwareProfileRegistry::builtin().resolve_model(m).ok());
    let layout = resolve_flash_layout(
        flash_bytes.len(),
        resolved_opt.as_ref().filter(|r| {
            r.flash_chip
                .as_ref()
                .is_some_and(|c| c.total_bytes as usize == flash_bytes.len() && c.writable)
        }),
    )?;

    let (orig_md5, orig_sha256) = calculate_hashes(flash_bytes);
    let nvram_slice = &flash_bytes[layout.nvram_base_offset..layout.nvram_end_offset];
    let (non_ff, non_zero, microsectors) = audit_nvram_sectors(nvram_slice, &layout.nvram_sectors);

    Ok(NvramCleanReport {
        input_file: input_label.to_string(),
        output_file: "<read-only-inspect>".to_string(),
        backup_file: "<none>".to_string(),
        original_md5: orig_md5.clone(),
        original_sha256: orig_sha256.clone(),
        clean_md5: orig_md5,
        clean_sha256: orig_sha256,
        total_flash_bytes: layout.flash_len,
        nvram_offset_start: layout.nvram_base_offset,
        nvram_offset_end: layout.nvram_end_offset,
        nvram_non_ff_bytes: non_ff,
        nvram_non_zero_bytes: non_zero,
        microsectors,
        bootloader_immutable: true,
    })
}

/// Inspects an in-circuit live NVRAM slice (`<= 64 KB` or `128 KB`) captured via Pico P7501 UART / GDB RSP.
/// Preserves exact datasheet-derived sector numbering (`SA31..SA34`, `SA63..SA70`, `SA134..SA141`, `SA127`).
pub fn inspect_partial_nvram_capture(
    nvram_bytes: &[u8],
    base_bus_address: u32,
    source_label: &str,
) -> Result<NvramCleanReport> {
    if nvram_bytes.is_empty() {
        bail!("Captured NVRAM buffer is empty.");
    }
    let (orig_md5, orig_sha256) = calculate_hashes(nvram_bytes);
    let (total_flash_bytes, nvram_offset_start) = match base_bus_address {
        0x801F_0000 | 0x841F_0000 => (2 * 1024 * 1024, 0x001F_0000usize),
        0x803F_0000 | 0x843F_0000 => (4 * 1024 * 1024, 0x003F_0000usize),
        0x80FE_0000 | 0x84FE_0000 | 0x80FF_0000 | 0x84FF_0000 => {
            (16 * 1024 * 1024, 0x00FE_0000usize)
        }
        _ => (SPANSION_FLASH_SIZE_BYTES, SPANSION_NVRAM_BASE_OFFSET),
    };

    let sectors = nvram_sectors_for_capacity(total_flash_bytes)?;
    let (non_ff, non_zero, microsectors) = audit_nvram_sectors(nvram_bytes, &sectors);

    Ok(NvramCleanReport {
        input_file: source_label.to_string(),
        output_file: "<live-in-circuit-capture>".to_string(),
        backup_file: "<live-memory>".to_string(),
        original_md5: orig_md5.clone(),
        original_sha256: orig_sha256.clone(),
        clean_md5: orig_md5,
        clean_sha256: orig_sha256,
        total_flash_bytes,
        nvram_offset_start,
        nvram_offset_end: nvram_offset_start + nvram_bytes.len(),
        nvram_non_ff_bytes: non_ff,
        nvram_non_zero_bytes: non_zero,
        microsectors,
        bootloader_immutable: true,
    })
}
