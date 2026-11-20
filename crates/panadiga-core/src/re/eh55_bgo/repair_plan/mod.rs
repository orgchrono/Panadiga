#![deny(unsafe_code)]

//! Deterministic BGO FTL repair plan builder & CoW structural verifier (`SA26..=SA70`).

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::{EH55_BGO_FIRST_SA, EH55_BGO_SECTOR_STRIDE, eh55_bgo_sector_offset};
use crate::constants::{AM34_EH55_FLASH_BGO_BASE, AM34_EH55_FLASH_BGO_SIZE};
use crate::nvram::calculate_hashes;
use crate::pico::rsp_codec::policy::{NorWritePolicy, is_nor_write_blocked_with_policy};

pub mod planner;
mod reconstruct;
pub mod simulate;

#[cfg(test)]
pub(super) mod tests;

pub use planner::build_bgo_repair_plan;
pub use simulate::validate_and_simulate_bgo_plan;

/// Single atomic NOR operation inside an EH55 BGO FTL repair plan (`SA26..=SA70` only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BgoRepairOp {
    EraseSector {
        sa: usize,
        phys_va: u32,
    },
    WriteBytes {
        sa: usize,
        phys_va: u32,
        data: Vec<u8>,
        requires_erase_first: bool,
    },
}

/// Verified, CoW-simulated repair plan for the DMR-EH55 45-sector BGO FTL (`0x8413_0000..0x8440_0000`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgoRepairPlan {
    pub bgo_window_start_va: u32,
    pub bgo_window_end_va: u32,
    pub target_sectors: Vec<usize>,
    pub source_nor_sha256: String,
    pub source_bgo_sha256: String,
    pub expected_post_sha256: String,
    pub expected_post_nor_sha256: String,
    pub missing_f2_ids: Vec<u16>,
    pub assigned_spare_sas: Vec<usize>,
    pub erased_sas: Vec<usize>,
    pub total_write_bytes: usize,
    pub ops: Vec<BgoRepairOp>,
    pub post_audit: super::Eh55BgoAuditReport,
    #[serde(skip)]
    pub repaired_nor_image: Option<Vec<u8>>,
}

pub(super) fn read_le_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

/// Validates that `(phys_va, len)` lies strictly within `0x8413_0000..0x8440_0000` (`SA26..=SA70`)
/// and passes [`is_nor_write_blocked_with_policy`] under [`NorWritePolicy::UpperNorAppUnlocked`].
pub fn validate_bgo_op_bounds(phys_va: u32, len: usize) -> Result<usize> {
    let bgo_lo = AM34_EH55_FLASH_BGO_BASE;
    let bgo_hi = AM34_EH55_FLASH_BGO_BASE + AM34_EH55_FLASH_BGO_SIZE;
    let end = u64::from(phys_va) + len.max(1) as u64;
    if u64::from(phys_va) < u64::from(bgo_lo) || end > u64::from(bgo_hi) {
        bail!(
            "BGO safety lockout (E05): address 0x{phys_va:08X} (+{len} B) outside SA26..=SA70 (0x{bgo_lo:08X}..0x{bgo_hi:08X})"
        );
    }
    if is_nor_write_blocked_with_policy(phys_va, len.max(1), NorWritePolicy::UpperNorAppUnlocked) {
        bail!("Hardware Safety Lockout (E05): target 0x{phys_va:08X} touches protected IPL");
    }
    let rel = (phys_va - AM34_EH55_FLASH_BGO_BASE) as usize;
    Ok(EH55_BGO_FIRST_SA + (rel / EH55_BGO_SECTOR_STRIDE))
}

/// Concatenates the 64 KiB sector payloads of `target_sectors` (sorted) and computes their SHA-256.
pub fn hash_bgo_target_sectors(
    nor_8mb: &[u8],
    target_sectors: &[usize],
) -> Result<(Vec<u8>, String)> {
    let mut joined = Vec::with_capacity(target_sectors.len() * EH55_BGO_SECTOR_STRIDE);
    for &sa in target_sectors {
        let base = eh55_bgo_sector_offset(sa)?;
        joined.extend_from_slice(&nor_8mb[base..base + EH55_BGO_SECTOR_STRIDE]);
    }
    let (_, sha256) = calculate_hashes(&joined);
    Ok((joined, sha256))
}
