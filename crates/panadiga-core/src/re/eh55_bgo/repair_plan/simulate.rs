#![deny(unsafe_code)]

//! Copy-on-Write simulation and monotonic `1 -> 0` verification for BGO repair plans.

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use super::{BgoRepairOp, validate_bgo_op_bounds};
use crate::constants::{AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};
use crate::re::eh55_bgo::{
    EH55_BGO_SECTOR_STRIDE, Eh55BgoAuditReport, audit_eh55_bgo_nor_8mb, eh55_bgo_sector_offset,
    verify_nor_one_to_zero_compatible,
};

pub(super) fn push_contiguous_writes(
    ops: &mut Vec<BgoRepairOp>,
    sa: usize,
    phys_base: u32,
    new_sec: &[u8],
    requires_erase_first: bool,
    should_emit: impl Fn(usize, u8) -> bool,
) {
    let mut i = 0usize;
    while i < new_sec.len() {
        if !should_emit(i, new_sec[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < new_sec.len() && should_emit(i, new_sec[i]) {
            i += 1;
        }
        ops.push(BgoRepairOp::WriteBytes {
            sa,
            phys_va: phys_base + start as u32,
            data: new_sec[start..i].to_vec(),
            requires_erase_first,
        });
    }
}

/// Validates bounds (`SA26..=SA70`), monotonic `1 -> 0` transitions on non-erased sectors,
/// simulates all ops in CoW memory, and verifies [`audit_eh55_bgo_nor_8mb`] passes with zero errors.
pub fn validate_and_simulate_bgo_plan(
    corrupt_nor: &[u8],
    ops: &[BgoRepairOp],
) -> Result<(Vec<u8>, Eh55BgoAuditReport)> {
    if corrupt_nor.len() != AM34_FLASH_SIZE as usize {
        bail!(
            "Expected {AM34_FLASH_SIZE}-byte 8 MiB NOR dump, got {} bytes",
            corrupt_nor.len()
        );
    }
    let mut sim = corrupt_nor.to_vec();
    let mut erased_sas = BTreeSet::new();

    for op in ops {
        match op {
            BgoRepairOp::EraseSector { sa, phys_va } => {
                let expected_sa = validate_bgo_op_bounds(*phys_va, EH55_BGO_SECTOR_STRIDE)?;
                let expected_off = eh55_bgo_sector_offset(*sa)?;
                if expected_sa != *sa || *phys_va != AM34_FLASH_UNCACHED_BASE + expected_off as u32
                {
                    bail!(
                        "EraseSector SA{sa} phys_va 0x{phys_va:08X} misaligned (expected 0x{:08X})",
                        AM34_FLASH_UNCACHED_BASE + expected_off as u32
                    );
                }
                sim[expected_off..expected_off + EH55_BGO_SECTOR_STRIDE].fill(0xFF);
                erased_sas.insert(*sa);
            }
            BgoRepairOp::WriteBytes {
                sa,
                phys_va,
                data,
                requires_erase_first,
            } => {
                if data.is_empty() {
                    continue;
                }
                let addr_sa = validate_bgo_op_bounds(*phys_va, data.len())?;
                if addr_sa != *sa {
                    bail!("WriteBytes spans/mismatches sector: op says SA{sa}, VA is SA{addr_sa}");
                }
                let sec_off = eh55_bgo_sector_offset(*sa)?;
                let flash_off = (*phys_va - AM34_FLASH_UNCACHED_BASE) as usize;
                if flash_off + data.len() > sec_off + EH55_BGO_SECTOR_STRIDE {
                    bail!("WriteBytes at 0x{phys_va:08X} overflows sector SA{sa}");
                }
                if !erased_sas.contains(sa) {
                    if *requires_erase_first {
                        bail!("WriteBytes requires_erase_first=true for SA{sa} before EraseSector");
                    }
                    verify_nor_one_to_zero_compatible(
                        &sim[flash_off..flash_off + data.len()],
                        data,
                    )?;
                }
                sim[flash_off..flash_off + data.len()].copy_from_slice(data);
            }
        }
    }

    let post_audit = audit_eh55_bgo_nor_8mb(&sim)?;
    if !post_audit.healthy
        || !post_audit.p2_missing_ids.is_empty()
        || !post_audit.corrupt_sas.is_empty()
        || post_audit.l2p_tag_mismatches != 0
        || post_audit.fat_mirror_mismatches != 0
    {
        bail!(
            "Simulated BGO repair failed post-audit (healthy={}, missing_f2={:?}, corrupt_sas={:?}, l2p_err={}, fat_err={})",
            post_audit.healthy,
            post_audit.p2_missing_ids,
            post_audit.corrupt_sas,
            post_audit.l2p_tag_mismatches,
            post_audit.fat_mirror_mismatches
        );
    }

    Ok((sim, post_audit))
}
