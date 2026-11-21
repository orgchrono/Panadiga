#![deny(unsafe_code)]

//! Live guarded application of a CoW-verified [`BgoRepairPlan`] over `PANAMON1>` / Pico `BRIDGE`.

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::EH55_BGO_SECTOR_STRIDE;
use super::repair_plan::{BgoRepairOp, BgoRepairPlan, validate_bgo_op_bounds};
use crate::pico::panamon1::{Panamon1Client, Panamon1Transport};
use crate::pico::{StreamProgressTelemetry, compute_rate_and_eta};

mod backup;
#[cfg(test)]
mod tests;

use backup::{
    emit_progress, emit_progress_telemetry, read_and_hash_bgo_sectors, write_verified_cow_backup,
};

/// Execution phase emitted to UI/CLI progress callbacks during live BGO repair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BgoApplyStage {
    PreReadVerify,
    BackupWrite,
    UnlockUpperNor,
    EraseAndProgram,
    LockUpperNor,
    PostReadVerify,
    BootTrace,
    Complete,
}

/// Structured live BGO apply progress update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgoLiveApplyProgress {
    pub stage: BgoApplyStage,
    pub current: usize,
    pub total: usize,
    pub detail: String,
    #[serde(flatten)]
    pub telemetry: StreamProgressTelemetry,
}

/// Final live BGO repair audit record returned by [`apply_bgo_repair_plan_live`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgoLiveApplyReport {
    pub backup_path: String,
    pub backup_sha256: String,
    pub pre_bgo_sha256: String,
    pub post_bgo_sha256: String,
    pub erased_sectors: usize,
    pub written_bytes: usize,
    pub boot_trace_output: Option<String>,
    pub boot_normal_ok: Option<bool>,
}

/// Applies a verified [`BgoRepairPlan`] over `PANAMON1>` with pre-read SHA-256 check, CoW backup,
/// RAII `Panamon1NorUnlockGuard`, read-back verification, and optional `BOOTTRACE <ms> NOJIG`.
pub fn apply_bgo_repair_plan_live<T: Panamon1Transport>(
    client: &mut Panamon1Client<T>,
    plan: &BgoRepairPlan,
    backup_dir: &Path,
    confirm_upper_nor: bool,
    run_boot_trace_ms: Option<u32>,
    progress: &mut dyn FnMut(BgoLiveApplyProgress),
) -> Result<BgoLiveApplyReport> {
    if !confirm_upper_nor {
        bail!("Upper NOR mutation refused fail-closed: confirm_upper_nor must be true");
    }
    if plan.target_sectors.is_empty() && plan.ops.is_empty() {
        bail!("BgoRepairPlan contains no operations to apply");
    }

    let mut planned_targets = Vec::with_capacity(plan.ops.len());
    for op in &plan.ops {
        let (phys_va, len) = match op {
            BgoRepairOp::EraseSector { phys_va, .. } => (*phys_va, EH55_BGO_SECTOR_STRIDE),
            BgoRepairOp::WriteBytes { phys_va, data, .. } => (*phys_va, data.len()),
        };
        let _ = validate_bgo_op_bounds(phys_va, len)?;
        planned_targets.push((phys_va, len));
    }

    let _ = client.ensure_panamon1_bridge(Some("JIG:2500"), 6000)?;
    let (pre_bytes, pre_bgo_sha256) = read_and_hash_bgo_sectors(
        client,
        &plan.target_sectors,
        BgoApplyStage::PreReadVerify,
        progress,
    )?;
    if pre_bgo_sha256 != plan.source_bgo_sha256 {
        bail!(
            "Live BGO target sectors SHA-256 mismatch before repair: live={pre_bgo_sha256} != plan={}",
            plan.source_bgo_sha256
        );
    }

    let (backup_path, backup_sha256) =
        write_verified_cow_backup(backup_dir, &pre_bytes, &pre_bgo_sha256)?;
    emit_progress(
        progress,
        BgoApplyStage::BackupWrite,
        1,
        1,
        format!("Saved CoW backup {backup_path}"),
    );

    emit_progress(
        progress,
        BgoApplyStage::UnlockUpperNor,
        0,
        1,
        "Issuing U CONFIRM_UPPER_NOR",
    );
    let mut erased_sectors = 0usize;
    let mut written_bytes = 0usize;
    let total_write_bytes: usize = plan
        .ops
        .iter()
        .map(|op| match op {
            BgoRepairOp::EraseSector { .. } => 0,
            BgoRepairOp::WriteBytes { data, .. } => data.len(),
        })
        .sum();
    let op_started = Instant::now();
    {
        let mut guard = client.unlock_upper_nor(&planned_targets)?;
        let total_ops = plan.ops.len();
        for (idx, op) in plan.ops.iter().enumerate() {
            let (bytes_per_sec, eta_seconds) = compute_rate_and_eta(
                op_started.elapsed().as_millis(),
                written_bytes,
                total_write_bytes,
            );
            let make_tel = |sa: usize| StreamProgressTelemetry {
                active_sa: Some(sa),
                bytes_done: Some(written_bytes),
                bytes_total: Some(total_write_bytes),
                bytes_per_sec,
                eta_seconds,
            };
            match op {
                BgoRepairOp::EraseSector { sa, phys_va } => {
                    emit_progress_telemetry(
                        progress,
                        BgoApplyStage::EraseAndProgram,
                        idx,
                        total_ops,
                        format!("Erasing SA{sa} @ 0x{phys_va:08X}"),
                        make_tel(*sa),
                    );
                    guard.erase_sector(*phys_va)?;
                    erased_sectors += 1;
                }
                BgoRepairOp::WriteBytes {
                    sa, phys_va, data, ..
                } => {
                    emit_progress_telemetry(
                        progress,
                        BgoApplyStage::EraseAndProgram,
                        idx,
                        total_ops,
                        format!("Programming {} B in SA{sa} @ 0x{phys_va:08X}", data.len()),
                        make_tel(*sa),
                    );
                    guard.write_nor_bytes(*phys_va, data)?;
                    written_bytes += data.len();
                }
            }
        }
        emit_progress(
            progress,
            BgoApplyStage::LockUpperNor,
            0,
            1,
            "Issuing U LOCK",
        );
        guard.lock()?;
    }

    let (_, post_bgo_sha256) = read_and_hash_bgo_sectors(
        client,
        &plan.target_sectors,
        BgoApplyStage::PostReadVerify,
        progress,
    )?;
    if post_bgo_sha256 != plan.expected_post_sha256 {
        bail!(
            "Post-write BGO read-back SHA-256 mismatch: live={post_bgo_sha256} != expected={}",
            plan.expected_post_sha256
        );
    }

    let (boot_trace_output, boot_normal_ok) = if let Some(bt_ms) = run_boot_trace_ms {
        emit_progress(
            progress,
            BgoApplyStage::BootTrace,
            0,
            1,
            format!("Running BOOTTRACE {bt_ms} NOJIG"),
        );
        let _ = client.exit_bridge_raw()?;
        let bt_out = client.transport_mut().pico_cli_exchange(
            &format!("BOOTTRACE {bt_ms} NOJIG"),
            &["BOOTTRACE: OK"],
            Duration::from_millis(u64::from(bt_ms).saturating_add(5000)),
        )?;
        let ok = !bt_out.contains("BGO driver init failed") && bt_out.contains("BOOTTRACE: OK");
        (Some(bt_out), Some(ok))
    } else {
        (None, None)
    };

    emit_progress(
        progress,
        BgoApplyStage::Complete,
        1,
        1,
        "BGO FTL repair verified",
    );
    Ok(BgoLiveApplyReport {
        backup_path,
        backup_sha256,
        pre_bgo_sha256,
        post_bgo_sha256,
        erased_sectors,
        written_bytes,
        boot_trace_output,
        boot_normal_ok,
    })
}
