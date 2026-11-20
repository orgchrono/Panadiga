#![deny(unsafe_code)]

//! Minimal diff & [`build_bgo_repair_plan`] generator for EH55 BGO `SA26..=SA70`.

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use super::reconstruct::reconstruct_corrupt_bgo_in_cow;
use super::simulate::{push_contiguous_writes, validate_and_simulate_bgo_plan};
use super::{BgoRepairOp, BgoRepairPlan, hash_bgo_target_sectors};
use crate::constants::{
    AM34_EH55_FLASH_BGO_BASE, AM34_EH55_FLASH_BGO_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE,
};
use crate::nvram::calculate_hashes;
use crate::re::eh55_bgo::{
    EH55_BGO_FIRST_SA, EH55_BGO_LAST_SA, EH55_BGO_SECTOR_STRIDE, audit_eh55_bgo_nor_8mb,
    eh55_bgo_sector_offset, verify_nor_one_to_zero_compatible,
};

/// Builds a deterministic, CoW-verified [`BgoRepairPlan`] from `corrupt_nor` and an optional healthy reference image.
pub fn build_bgo_repair_plan(
    corrupt_nor: &[u8],
    reference_healthy_nor: Option<&[u8]>,
) -> Result<BgoRepairPlan> {
    if corrupt_nor.len() != AM34_FLASH_SIZE as usize {
        bail!(
            "Expected {AM34_FLASH_SIZE}-byte 8 MiB NOR dump, got {} bytes",
            corrupt_nor.len()
        );
    }

    let (cow_target, missing_f2_ids, assigned_spare_sas) = match reference_healthy_nor {
        Some(ref_nor) => {
            if ref_nor.len() != AM34_FLASH_SIZE as usize {
                bail!(
                    "Expected {AM34_FLASH_SIZE}-byte reference 8 MiB NOR dump, got {} bytes",
                    ref_nor.len()
                );
            }
            if !audit_eh55_bgo_nor_8mb(ref_nor)?.healthy {
                bail!("Reference NOR image itself fails audit_eh55_bgo_nor_8mb");
            }
            let pre_audit = audit_eh55_bgo_nor_8mb(corrupt_nor)?;
            let mut cow = corrupt_nor.to_vec();
            let bgo_lo = eh55_bgo_sector_offset(EH55_BGO_FIRST_SA)?;
            let bgo_hi = bgo_lo + AM34_EH55_FLASH_BGO_SIZE as usize;
            cow[bgo_lo..bgo_hi].copy_from_slice(&ref_nor[bgo_lo..bgo_hi]);
            (cow, pre_audit.p2_missing_ids, Vec::new())
        }
        None => reconstruct_corrupt_bgo_in_cow(corrupt_nor)?,
    };

    let mut ops = Vec::new();
    let mut target_set = BTreeSet::new();
    let mut erased_sas = Vec::new();
    let mut total_write_bytes = 0usize;

    for sa in EH55_BGO_FIRST_SA..=EH55_BGO_LAST_SA {
        let base = eh55_bgo_sector_offset(sa)?;
        let phys_base = AM34_FLASH_UNCACHED_BASE + base as u32;
        let orig_sec = &corrupt_nor[base..base + EH55_BGO_SECTOR_STRIDE];
        let new_sec = &cow_target[base..base + EH55_BGO_SECTOR_STRIDE];
        if orig_sec == new_sec {
            continue;
        }
        target_set.insert(sa);
        let pre_len = ops.len();
        if verify_nor_one_to_zero_compatible(orig_sec, new_sec).is_ok() {
            push_contiguous_writes(&mut ops, sa, phys_base, new_sec, false, |idx, b| {
                orig_sec[idx] != b
            });
        } else {
            erased_sas.push(sa);
            ops.push(BgoRepairOp::EraseSector {
                sa,
                phys_va: phys_base,
            });
            push_contiguous_writes(&mut ops, sa, phys_base, new_sec, true, |_, b| b != 0xFF);
        }
        for op in &ops[pre_len..] {
            if let BgoRepairOp::WriteBytes { data, .. } = op {
                total_write_bytes += data.len();
            }
        }
    }

    let (sim, post_audit) = validate_and_simulate_bgo_plan(corrupt_nor, &ops)?;
    let target_sectors: Vec<usize> = target_set.into_iter().collect();
    let (_, source_nor_sha256) = calculate_hashes(corrupt_nor);
    let (_, expected_post_nor_sha256) = calculate_hashes(&sim);
    let (_, source_bgo_sha256) = hash_bgo_target_sectors(corrupt_nor, &target_sectors)?;
    let (_, expected_post_sha256) = hash_bgo_target_sectors(&sim, &target_sectors)?;

    Ok(BgoRepairPlan {
        bgo_window_start_va: AM34_EH55_FLASH_BGO_BASE,
        bgo_window_end_va: AM34_EH55_FLASH_BGO_BASE + AM34_EH55_FLASH_BGO_SIZE,
        target_sectors,
        source_nor_sha256,
        source_bgo_sha256,
        expected_post_sha256,
        expected_post_nor_sha256,
        missing_f2_ids,
        assigned_spare_sas,
        erased_sas,
        total_write_bytes,
        ops,
        post_audit,
        repaired_nor_image: Some(sim),
    })
}
