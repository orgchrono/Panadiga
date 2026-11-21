#![deny(unsafe_code)]

//! Pre/post NOR readback hashing and CoW backup helpers for live BGO repair.

use std::fs;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};

use super::{BgoApplyStage, BgoLiveApplyProgress};
use crate::constants::AM34_FLASH_UNCACHED_BASE;
use crate::nvram::calculate_hashes;
use crate::pico::panamon1::{Panamon1Client, Panamon1Transport};
use crate::pico::{StreamProgressTelemetry, compute_rate_and_eta};
use crate::re::eh55_bgo::{EH55_BGO_SECTOR_STRIDE, eh55_bgo_sector_offset};

pub(super) fn emit_progress(
    cb: &mut dyn FnMut(BgoLiveApplyProgress),
    stage: BgoApplyStage,
    current: usize,
    total: usize,
    detail: impl Into<String>,
) {
    emit_progress_telemetry(
        cb,
        stage,
        current,
        total,
        detail,
        StreamProgressTelemetry::default(),
    );
}

pub(super) fn emit_progress_telemetry(
    cb: &mut dyn FnMut(BgoLiveApplyProgress),
    stage: BgoApplyStage,
    current: usize,
    total: usize,
    detail: impl Into<String>,
    telemetry: StreamProgressTelemetry,
) {
    cb(BgoLiveApplyProgress {
        stage,
        current,
        total,
        detail: detail.into(),
        telemetry,
    });
}

pub(super) fn read_and_hash_bgo_sectors<T: Panamon1Transport>(
    client: &mut Panamon1Client<T>,
    target_sectors: &[usize],
    stage: BgoApplyStage,
    progress: &mut dyn FnMut(BgoLiveApplyProgress),
) -> Result<(Vec<u8>, String)> {
    let total = target_sectors.len();
    let total_bytes = total * EH55_BGO_SECTOR_STRIDE;
    let started = Instant::now();
    let mut buf = Vec::with_capacity(total_bytes);
    for (idx, &sa) in target_sectors.iter().enumerate() {
        let phys_va = AM34_FLASH_UNCACHED_BASE + eh55_bgo_sector_offset(sa)? as u32;
        let (bytes_per_sec, eta_seconds) =
            compute_rate_and_eta(started.elapsed().as_millis(), buf.len(), total_bytes);
        emit_progress_telemetry(
            progress,
            stage,
            idx,
            total,
            format!("Reading SA{sa} @ 0x{phys_va:08X}"),
            StreamProgressTelemetry {
                active_sa: Some(sa),
                bytes_done: Some(buf.len()),
                bytes_total: Some(total_bytes),
                bytes_per_sec,
                eta_seconds,
            },
        );
        buf.extend_from_slice(&client.mem_read_bytes(phys_va, EH55_BGO_SECTOR_STRIDE)?);
    }
    let (_, sha256) = calculate_hashes(&buf);
    let (bytes_per_sec, _) =
        compute_rate_and_eta(started.elapsed().as_millis(), buf.len(), total_bytes);
    emit_progress_telemetry(
        progress,
        stage,
        total,
        total,
        format!("SHA-256={sha256}"),
        StreamProgressTelemetry {
            active_sa: target_sectors.last().copied(),
            bytes_done: Some(buf.len()),
            bytes_total: Some(total_bytes),
            bytes_per_sec,
            eta_seconds: Some(0),
        },
    );
    Ok((buf, sha256))
}

pub(super) fn write_verified_cow_backup(
    backup_dir: &Path,
    pre_bytes: &[u8],
    pre_sha: &str,
) -> Result<(String, String)> {
    fs::create_dir_all(backup_dir)
        .with_context(|| format!("Creating BGO backup dir {}", backup_dir.display()))?;
    let prefix = &pre_sha[..pre_sha.len().min(12)];
    let backup_path = backup_dir.join(format!("bgo_pre_repair_{prefix}.bin"));
    let backup_sha_path = backup_dir.join(format!("bgo_pre_repair_{prefix}.sha256"));
    fs::write(&backup_path, pre_bytes)
        .with_context(|| format!("Writing BGO CoW backup {}", backup_path.display()))?;
    fs::write(&backup_sha_path, format!("{pre_sha}\n"))
        .with_context(|| format!("Writing BGO CoW backup SHA {}", backup_sha_path.display()))?;
    let read_back = fs::read(&backup_path)
        .with_context(|| format!("Verifying BGO CoW backup {}", backup_path.display()))?;
    let (_, verified_sha) = calculate_hashes(&read_back);
    if verified_sha != pre_sha {
        bail!("CoW backup SHA-256 verification failed");
    }
    Ok((backup_path.display().to_string(), verified_sha))
}
