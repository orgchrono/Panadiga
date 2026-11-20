#![deny(unsafe_code)]

//! Pre/post NOR readback hashing and CoW backup helpers for live BGO repair.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::{BgoApplyStage, BgoLiveApplyProgress};
use crate::constants::AM34_FLASH_UNCACHED_BASE;
use crate::nvram::calculate_hashes;
use crate::pico::panamon1::{Panamon1Client, Panamon1Transport};
use crate::re::eh55_bgo::{EH55_BGO_SECTOR_STRIDE, eh55_bgo_sector_offset};

pub(super) fn emit_progress(
    cb: &mut dyn FnMut(BgoLiveApplyProgress),
    stage: BgoApplyStage,
    current: usize,
    total: usize,
    detail: impl Into<String>,
) {
    cb(BgoLiveApplyProgress {
        stage,
        current,
        total,
        detail: detail.into(),
    });
}

pub(super) fn read_and_hash_bgo_sectors<T: Panamon1Transport>(
    client: &mut Panamon1Client<T>,
    target_sectors: &[usize],
    stage: BgoApplyStage,
    progress: &mut dyn FnMut(BgoLiveApplyProgress),
) -> Result<(Vec<u8>, String)> {
    let total = target_sectors.len();
    let mut buf = Vec::with_capacity(total * EH55_BGO_SECTOR_STRIDE);
    for (idx, &sa) in target_sectors.iter().enumerate() {
        let phys_va = AM34_FLASH_UNCACHED_BASE + eh55_bgo_sector_offset(sa)? as u32;
        emit_progress(
            progress,
            stage,
            idx,
            total,
            format!("Reading SA{sa} @ 0x{phys_va:08X}"),
        );
        buf.extend_from_slice(&client.mem_read_bytes(phys_va, EH55_BGO_SECTOR_STRIDE)?);
    }
    let (_, sha256) = calculate_hashes(&buf);
    emit_progress(progress, stage, total, total, format!("SHA-256={sha256}"));
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
