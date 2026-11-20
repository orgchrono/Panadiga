#![deny(unsafe_code)]

//! Forensic 142-sector Spansion `S29JL064H` (`IC5001`, 8 MiB) analyzer.
//!
//! Consolidates `analyze_eh55_nor_dump.py`: validates full `8×8K + 126×64K + 8×8K` sector
//! geometry, Shannon entropy, `% non-0xFF`, printable signatures, per-sector SHA-256,
//! Loader `ver0.61` strings, and `SA134..SA141` EEMUL NVRAM health via [`audit_nvram_slice`].

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{
    AM34_EH55_FLASH_BGO_BASE, AM34_EH55_FLASH_BGO_SIZE, AM34_FLASH_CRITICAL_IPL_SIZE,
    AM34_FLASH_UNCACHED_BASE, FLASH_ERASED_BYTE, SPANSION_FLASH_SIZE_BYTES,
    SPANSION_MAIN_SECTOR_SIZE_BYTES, SPANSION_MICROSECTOR_BASE_INDEX, SPANSION_MICROSECTOR_SIZE,
    SPANSION_NVRAM_BASE_OFFSET, SPANSION_NVRAM_SIZE_BYTES, SPANSION_S29JL064H_SECTOR_COUNT,
};
use crate::nvram::{MicroSectorAudit, audit_nvram_slice, calculate_hashes};

#[cfg(test)]
mod tests;

/// Physical Spansion `S29JL064H` partition role for a sector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpansionRegionKind {
    BootIpl,
    UpperNor,
    BgoRegion,
    EemulNvram,
}

/// Per-sector telemetry for one of the 142 sectors (`SA0..=SA141`) on `IC5001`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpansionSectorSummary {
    pub sa: usize,
    pub offset: usize,
    pub phys_va: u32,
    pub size_bytes: usize,
    pub region: SpansionRegionKind,
    pub is_blank: bool,
    pub non_ff_bytes: usize,
    pub non_ff_percent: f64,
    pub shannon_entropy: f64,
    pub sha256: String,
    pub printable_signature: Option<String>,
    pub last_non_ff_offset: Option<usize>,
}

/// NVRAM (`SA134..=SA141`, 64 KiB) health summary inside an 8 MiB `IC5001` dump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpansionNvramHealth {
    pub base_offset: usize,
    pub total_bytes: usize,
    pub non_ff_bytes: usize,
    pub non_zero_bytes: usize,
    pub active_microsectors: usize,
    pub is_factory_erased: bool,
    pub microsectors: Vec<MicroSectorAudit>,
}

/// Complete 142-sector Spansion `S29JL064H` forensic audit report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpansionNorAuditReport {
    pub total_bytes: usize,
    pub sha256: String,
    pub total_sectors: usize,
    pub populated_sectors: usize,
    pub blank_sectors: usize,
    pub sectors: Vec<SpansionSectorSummary>,
    pub nvram_health: SpansionNvramHealth,
    pub loader_signatures: Vec<String>,
}

/// Builds the `(sa_idx, start_offset, size_bytes)` geometry table for `S29JL064H` (142 sectors = 8 MiB).
pub fn spansion_s29jl064h_sector_table() -> Vec<(usize, usize, usize)> {
    let mut sectors = Vec::with_capacity(SPANSION_S29JL064H_SECTOR_COUNT);
    let mut off = 0usize;
    let mut sa = 0usize;
    for (count, sz) in [
        (8, SPANSION_MICROSECTOR_SIZE),
        (126, SPANSION_MAIN_SECTOR_SIZE_BYTES),
        (8, SPANSION_MICROSECTOR_SIZE),
    ] {
        for _ in 0..count {
            sectors.push((sa, off, sz));
            off += sz;
            sa += 1;
        }
    }
    debug_assert_eq!(off, SPANSION_FLASH_SIZE_BYTES);
    debug_assert_eq!(sectors.len(), SPANSION_S29JL064H_SECTOR_COUNT);
    sectors
}

fn shannon_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for &b in data {
        counts[usize::from(b)] += 1;
    }
    let n = data.len() as f64;
    counts.into_iter().fold(0.0f64, |ent, c| {
        if c == 0 {
            ent
        } else {
            let p = (c as f64) / n;
            ent - p * p.log2()
        }
    })
}

fn first_printable_signature(data: &[u8]) -> Option<String> {
    let mut run_start: Option<usize> = None;
    for (idx, &b) in data.iter().enumerate() {
        if (0x20..=0x7E).contains(&b) {
            let start = *run_start.get_or_insert(idx);
            if idx - start + 1 >= 32 {
                return Some(String::from_utf8_lossy(&data[start..=idx]).into_owned());
            }
        } else if let Some(start) = run_start.take()
            && idx - start >= 6
        {
            return Some(String::from_utf8_lossy(&data[start..idx]).into_owned());
        }
    }
    if let Some(start) = run_start
        && data.len() - start >= 6
    {
        return Some(String::from_utf8_lossy(&data[start..]).into_owned());
    }
    None
}

fn classify_sector_region(sa: usize, off: usize) -> SpansionRegionKind {
    let bgo_lo = (AM34_EH55_FLASH_BGO_BASE - AM34_FLASH_UNCACHED_BASE) as usize;
    let bgo_hi = bgo_lo + AM34_EH55_FLASH_BGO_SIZE as usize;
    if off < AM34_FLASH_CRITICAL_IPL_SIZE as usize {
        SpansionRegionKind::BootIpl
    } else if sa >= SPANSION_MICROSECTOR_BASE_INDEX {
        SpansionRegionKind::EemulNvram
    } else if (bgo_lo..bgo_hi).contains(&off) {
        SpansionRegionKind::BgoRegion
    } else {
        SpansionRegionKind::UpperNor
    }
}

fn extract_loader_signatures(loader: &[u8]) -> Vec<String> {
    const PATTERNS: [&[u8]; 7] = [
        b"ver0.", b"BGO", b"nfrom", b"EEMUL", b"U99", b"Loader", b"PANAMON",
    ];
    let mut hits = Vec::new();
    for pat in PATTERNS {
        if let Some(pos) = loader.windows(pat.len()).position(|w| w == pat) {
            let s = pos.saturating_sub(8);
            let e = (pos + 40).min(loader.len());
            let snippet: String = loader[s..e]
                .iter()
                .map(|&b| {
                    if (0x20..=0x7E).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            hits.push(format!("0x{pos:06X}: {snippet}"));
        }
    }
    hits
}

/// Performs a full 142-sector Spansion `S29JL064H` audit on an 8 MiB `IC5001` dump.
pub fn analyze_spansion_nor_sectors(nor_8mb: &[u8]) -> Result<SpansionNorAuditReport> {
    if nor_8mb.len() != SPANSION_FLASH_SIZE_BYTES {
        bail!(
            "Expected {SPANSION_FLASH_SIZE_BYTES}-byte 8 MiB NOR dump, got {} bytes",
            nor_8mb.len()
        );
    }
    let (_, sha256) = calculate_hashes(nor_8mb);
    let table = spansion_s29jl064h_sector_table();
    let mut sectors = Vec::with_capacity(table.len());
    let mut blank_sectors = 0usize;

    for (sa, off, size_bytes) in table {
        let sec = &nor_8mb[off..off + size_bytes];
        let ff_count = sec.iter().filter(|&&b| b == FLASH_ERASED_BYTE).count();
        let non_ff_bytes = size_bytes - ff_count;
        let is_blank = non_ff_bytes == 0;
        if is_blank {
            blank_sectors += 1;
        }
        let non_ff_percent = (non_ff_bytes as f64) * 100.0 / (size_bytes as f64);
        let (_, sec_sha256) = calculate_hashes(sec);
        let last_non_ff_offset = sec.iter().rposition(|&b| b != FLASH_ERASED_BYTE);
        let printable_signature = (!is_blank)
            .then(|| first_printable_signature(sec))
            .flatten();
        sectors.push(SpansionSectorSummary {
            sa,
            offset: off,
            phys_va: AM34_FLASH_UNCACHED_BASE + off as u32,
            size_bytes,
            region: classify_sector_region(sa, off),
            is_blank,
            non_ff_bytes,
            non_ff_percent,
            shannon_entropy: if is_blank { 0.0 } else { shannon_entropy(sec) },
            sha256: sec_sha256,
            printable_signature,
            last_non_ff_offset,
        });
    }

    let nvram_slice = &nor_8mb[SPANSION_NVRAM_BASE_OFFSET..SPANSION_FLASH_SIZE_BYTES];
    let (nvram_non_ff, nvram_non_zero, microsectors) = audit_nvram_slice(
        nvram_slice,
        SPANSION_NVRAM_BASE_OFFSET,
        SPANSION_MICROSECTOR_BASE_INDEX,
    );
    let active_microsectors = microsectors.iter().filter(|m| m.active_bytes > 0).count();
    let nvram_health = SpansionNvramHealth {
        base_offset: SPANSION_NVRAM_BASE_OFFSET,
        total_bytes: SPANSION_NVRAM_SIZE_BYTES,
        non_ff_bytes: nvram_non_ff,
        non_zero_bytes: nvram_non_zero,
        active_microsectors,
        is_factory_erased: nvram_non_ff == 0,
        microsectors,
    };

    let populated_sectors = SPANSION_S29JL064H_SECTOR_COUNT - blank_sectors;
    let loader_signatures =
        extract_loader_signatures(&nor_8mb[..AM34_FLASH_CRITICAL_IPL_SIZE as usize]);

    Ok(SpansionNorAuditReport {
        total_bytes: nor_8mb.len(),
        sha256,
        total_sectors: SPANSION_S29JL064H_SECTOR_COUNT,
        populated_sectors,
        blank_sectors,
        sectors,
        nvram_health,
        loader_signatures,
    })
}
