//! Pure hashing, NVRAM micro-sector breakdown, and live Pico NVRAM capture inspection.

use anyhow::{Result, bail};
use md5::{Digest as _, Md5};
use sha2::Sha256;

use super::types::{MicroSectorAudit, NvramCleanReport};
use crate::constants::{
    FLASH_ERASED_BYTE, SPANSION_FLASH_SIZE_BYTES, SPANSION_MICROSECTOR_BASE_INDEX,
    SPANSION_MICROSECTOR_SIZE, SPANSION_NVRAM_BASE_OFFSET,
};

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

/// Shared helper computing NVRAM microsector breakdown and byte activity counts.
pub fn audit_nvram_slice(
    nvram_slice: &[u8],
    nvram_base_offset: usize,
    base_sector_idx: usize,
) -> (usize, usize, Vec<MicroSectorAudit>) {
    let non_ff = nvram_slice
        .iter()
        .filter(|&&b| b != FLASH_ERASED_BYTE)
        .count();
    let non_zero = nvram_slice.iter().filter(|&&b| b != 0x00).count();
    let sector_count = nvram_slice.len().div_ceil(SPANSION_MICROSECTOR_SIZE);
    let mut microsectors = Vec::with_capacity(sector_count);

    for idx in 0..sector_count {
        let sec_offset = idx * SPANSION_MICROSECTOR_SIZE;
        let sec_end = (sec_offset + SPANSION_MICROSECTOR_SIZE).min(nvram_slice.len());
        let sec_slice = &nvram_slice[sec_offset..sec_end];
        let used = sec_slice
            .iter()
            .filter(|&&b| b != FLASH_ERASED_BYTE)
            .count();
        let hdr_len = sec_slice.len().min(8);
        let header_hex = hex::encode_upper(&sec_slice[..hdr_len]);
        let sec_name = format!("SA{}", base_sector_idx + idx);

        microsectors.push(MicroSectorAudit {
            sector_name: sec_name,
            offset: nvram_base_offset + sec_offset,
            active_bytes: used,
            total_bytes: sec_slice.len(),
            header_hex,
        });
    }

    (non_ff, non_zero, microsectors)
}

/// Inspects an in-circuit live NVRAM slice (`<= 64 KB` or full flash) captured via Pico P7501 UART / GDB RSP.
/// Preserves exact sector numbering (defaults to `SA134..SA141` on 8 MB / `0x807F0000`) and integrity hashes.
pub fn inspect_partial_nvram_capture(
    nvram_bytes: &[u8],
    base_bus_address: u32,
    source_label: &str,
) -> Result<NvramCleanReport> {
    if nvram_bytes.is_empty() {
        bail!("Captured NVRAM buffer is empty.");
    }
    let (orig_md5, orig_sha256) = calculate_hashes(nvram_bytes);
    let (total_flash_bytes, nvram_offset_start, base_sector_idx) = match base_bus_address {
        0x801F_0000 => (2 * 1024 * 1024, 0x001F_0000, 38usize),
        0x803F_0000 => (4 * 1024 * 1024, 0x003F_0000, 70usize),
        0x80FE_0000 | 0x80FF_0000 => (16 * 1024 * 1024, 0x00FE_0000, 254usize),
        _ => (
            SPANSION_FLASH_SIZE_BYTES,
            SPANSION_NVRAM_BASE_OFFSET,
            SPANSION_MICROSECTOR_BASE_INDEX,
        ),
    };
    let (non_ff, non_zero, microsectors) =
        audit_nvram_slice(nvram_bytes, nvram_offset_start, base_sector_idx);

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
