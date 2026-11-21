#![deny(unsafe_code)]

//! Core traits, Windows block-device adapter, geometry helpers, and DTOs for
//! safe Panasonic DMR-EH55 physical HDD inspection and flashing.

use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::LBA_SECTOR_SIZE_BYTES;

use super::super::discovery::list_hard_disk_drives;
use super::super::raw_device::infer_bus_type_id;
pub use super::super::raw_device::{BUS_TYPE_NVME, BlockFlashDevice, WindowsPhysicalFlashDevice};
use super::super::types::PhysicalDriveInfo;

/// High-LBA backup step (`0x20000` sectors = `64 MiB`) subtracted from 4-sector-aligned disk LBAs.
pub const EH55_MIRROR_MAIN_BACKSTEP_LBAS: u64 = 0x20000;
/// Streaming transfer / verification chunk size (`1 MiB`, 2048 sectors).
pub const EH55_FLASH_CHUNK_BYTES: usize = 1024 * 1024;
/// Number of sectors captured in each of the two 32 KiB backup regions (`LBA 0..63` + `LBA 57344..57407`).
pub const EH55_BACKUP_SECTOR_COUNT: usize = 64;

pub(super) const DEFAULT_EXPECTED_GB_RANGE: (u32, u32) = (240, 260);
pub(super) const MIN_ALLOWED_GB: u32 = 5;
pub(super) const MAX_ALLOWED_GB: u32 = 2200;

/// Computes the exact EH55 high-LBA `MAIN` mirror sector `((disk_bytes / 512) & !3) - 0x20000`.
pub fn compute_eh55_mirror_main_lba(disk_bytes: u64) -> Result<u64> {
    let total_lbas = disk_bytes / (LBA_SECTOR_SIZE_BYTES as u64);
    let aligned_lbas = total_lbas & !3u64;
    if aligned_lbas <= EH55_MIRROR_MAIN_BACKSTEP_LBAS {
        bail!(
            "Disk too small ({disk_bytes} B / {aligned_lbas} aligned LBAs) for EH55 mirror offset -0x{:X}",
            EH55_MIRROR_MAIN_BACKSTEP_LBAS
        );
    }
    Ok(aligned_lbas - EH55_MIRROR_MAIN_BACKSTEP_LBAS)
}

/// Enriched target disk safety & geometry assessment for safe EH55 flashing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SafeTargetDiskInfo {
    pub device_path: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub size_gb: f64,
    pub bus_type: String,
    pub bus_type_id: u32,
    pub is_nvme_blocked: bool,
    pub is_os_blocked: bool,
    pub safe_to_flash: bool,
    pub lockout_reason: Option<String>,
    pub mirror_main_lba: Option<u64>,
}

/// Enriches discovered [`PhysicalDriveInfo`] records with NVMe/OS lockout flags and `mirror_main_lba`.
#[must_use]
pub fn enrich_safe_target_disks(drives: &[PhysicalDriveInfo]) -> Vec<SafeTargetDiskInfo> {
    drives
        .iter()
        .map(|d| {
            let bus_type_id = infer_bus_type_id(&d.bus_type);
            let is_nvme_blocked = bus_type_id == BUS_TYPE_NVME;
            let is_drive_0 = super::super::block_ops::is_unconditional_os_boot_disk(&d.device_path);
            let is_os_blocked = d.is_system_drive || !d.safe_to_write || is_drive_0;
            let safe_to_flash = !is_os_blocked && !is_nvme_blocked;
            let lockout_reason = if is_nvme_blocked {
                Some(
                    "SAFETY LOCKOUT: BusTypeNvme (17) rejected for EH55 IDE/PATA target"
                        .to_string(),
                )
            } else if is_os_blocked {
                d.lockout_reason.clone().or_else(|| {
                    Some("SAFETY LOCKOUT: OS / PhysicalDrive0 target blocked".to_string())
                })
            } else {
                None
            };
            SafeTargetDiskInfo {
                device_path: d.device_path.clone(),
                display_name: d.display_name.clone(),
                size_bytes: d.size_bytes,
                size_gb: d.size_gb,
                bus_type: d.bus_type.clone(),
                bus_type_id,
                is_nvme_blocked,
                is_os_blocked,
                safe_to_flash,
                lockout_reason,
                mirror_main_lba: compute_eh55_mirror_main_lba(d.size_bytes).ok(),
            }
        })
        .collect()
}

/// Lists hard disk drives via `super::super::discovery::list_hard_disk_drives()` and classifies NVMe/OS/mirror safety.
#[must_use]
pub fn inspect_safe_target_disks() -> Vec<SafeTargetDiskInfo> {
    let drives = list_hard_disk_drives().unwrap_or_default();
    enrich_safe_target_disks(&drives)
}

/// Deterministic pipeline phases emitted during `execute_eh55_hdd_flash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Eh55FlashStage {
    Preflight,
    Backup64k,
    WipeLayout,
    Write100m,
    WriteMirror,
    VerifySha256,
    Done,
}

/// Structured progress event emitted during EH55 HDD flashing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Eh55FlashProgress {
    pub stage: Eh55FlashStage,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub detail: String,
}

/// Input parameters for `execute_eh55_hdd_flash`.
#[derive(Debug, Clone)]
pub struct Eh55HddFlashRequest {
    pub device_path: String,
    pub confirmation_token: String,
    pub image_path: PathBuf,
    pub mirror_p1_path: Option<PathBuf>,
    pub extract_p1_mirror_from_image: bool,
    pub backup_dir: PathBuf,
    pub expected_gb_range: Option<(u32, u32)>,
    pub detected_os_disks: Option<Vec<String>>,
}

/// Full audit report returned after successful EH55 HDD flash and SHA-256 read-back verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Eh55HddFlashReport {
    pub device_path: String,
    pub disk_bytes: u64,
    pub bus_type_id: u32,
    pub mirror_main_lba: u64,
    pub backup_bin_path: String,
    pub backup_manifest_path: String,
    pub backup_64k_sha256: String,
    pub primary_bytes_written: usize,
    pub primary_sha256: String,
    pub mirror_bytes_written: usize,
    pub mirror_sha256: Option<String>,
    pub readback_verified: bool,
}
