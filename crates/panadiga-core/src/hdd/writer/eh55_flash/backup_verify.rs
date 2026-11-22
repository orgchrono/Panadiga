#![deny(unsafe_code)]

//! Mandatory 64 KiB pre-write CoW backup, image header validation, chunked
//! writing, and full read-back SHA-256 verification helpers for EH55 flashing.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::constants::{
    LBA_MIRROR_MBR_OFFSET, LBA_MIRROR_MBR_SECTOR, LBA_PARTITION_1_ALIGNED_SIZE,
    LBA_PARTITION_1_OFFSET, LBA_SECTOR_SIZE_BYTES, PANASONIC_HDD_MBR_SIGNATURE,
    STANDARD_MBR_BOOT_SIGNATURE,
};
use crate::nvram::calculate_hashes;

use super::super::block_ops::{is_unconditional_os_boot_disk, verify_safety_lockout_with_os_disks};
use super::super::discovery::detect_host_os_disks;
use super::types::{
    BUS_TYPE_NVME, BlockFlashDevice, DEFAULT_EXPECTED_GB_RANGE, EH55_BACKUP_SECTOR_COUNT,
    EH55_FLASH_CHUNK_BYTES, Eh55FlashProgress, Eh55FlashStage, Eh55HddFlashRequest, MAX_ALLOWED_GB,
    MIN_ALLOWED_GB, compute_eh55_mirror_main_lba,
};

/// Validated target geometry & safety-lockout parameters shared across HDD flash writers.
pub(in crate::hdd::writer) struct HddPreflightParams<'a> {
    pub device_path: &'a str,
    pub confirmation_token: &'a str,
    pub expected_gb_range: Option<(u32, u32)>,
    pub detected_os_disks: Option<&'a [String]>,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::hdd::writer) struct HddTargetGeometry {
    pub disk_bytes: u64,
    pub total_sectors: u64,
    pub bus_type_id: u32,
    pub mirror_main_lba: u64,
    pub mirror_offset_bytes: u64,
}

/// Shared fail-closed HDD preflight validating OS disk lockout, NVMe block, capacity bounds, and mirror LBA.
pub(in crate::hdd::writer) fn preflight_hdd_target_geometry<D, F>(
    device: &D,
    params: HddPreflightParams<'_>,
    on_progress: &mut F,
) -> Result<HddTargetGeometry>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(Eh55FlashProgress),
{
    on_progress(Eh55FlashProgress {
        stage: Eh55FlashStage::Preflight,
        bytes_done: 0,
        bytes_total: 0,
        detail: format!("Validating safety lockouts for {}", params.device_path),
    });
    if is_unconditional_os_boot_disk(params.device_path) {
        bail!(
            "FATAL SAFETY LOCKOUT: Primary OS boot disk '{}' is unconditionally blocked!",
            params.device_path
        );
    }
    let fallback_os_disks = if params.detected_os_disks.is_none() {
        detect_host_os_disks()
    } else {
        None
    };
    let os_refs: Option<Vec<&str>> = params
        .detected_os_disks
        .or(fallback_os_disks.as_deref())
        .map(|list| list.iter().map(String::as_str).collect());
    verify_safety_lockout_with_os_disks(
        params.device_path,
        params.confirmation_token,
        os_refs.as_deref(),
    )?;

    let bus_type_id = device.bus_type();
    if bus_type_id == BUS_TYPE_NVME {
        bail!(
            "SAFETY LOCKOUT: Target '{}' reported BusType::Nvme (17); EH55 targets must be IDE/PATA/SATA/USB",
            params.device_path
        );
    }
    let (min_gb, max_gb) = params
        .expected_gb_range
        .unwrap_or(DEFAULT_EXPECTED_GB_RANGE);
    if min_gb > max_gb || min_gb < MIN_ALLOWED_GB || max_gb > MAX_ALLOWED_GB {
        bail!(
            "Invalid expected_gb_range ({min_gb}..{max_gb} GB): must fall within {MIN_ALLOWED_GB}..{MAX_ALLOWED_GB} GB"
        );
    }
    let disk_bytes = device.length_bytes()?;
    let min_bytes = u64::from(min_gb) * 1_000_000_000;
    let max_bytes = u64::from(max_gb) * 1_000_000_000;
    if disk_bytes < min_bytes || disk_bytes > max_bytes {
        bail!(
            "SAFETY ABORT: Target disk '{}' length {disk_bytes} B outside expected {min_gb}..{max_gb} GB range",
            params.device_path
        );
    }
    let mirror_main_lba = compute_eh55_mirror_main_lba(disk_bytes)?;
    Ok(HddTargetGeometry {
        disk_bytes,
        total_sectors: disk_bytes / (LBA_SECTOR_SIZE_BYTES as u64),
        bus_type_id,
        mirror_main_lba,
        mirror_offset_bytes: mirror_main_lba * (LBA_SECTOR_SIZE_BYTES as u64),
    })
}

pub(super) fn validate_eh55_primary_image(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(LBA_SECTOR_SIZE_BYTES) {
        bail!(
            "Primary EH55 image must be a non-empty multiple of {LBA_SECTOR_SIZE_BYTES} B (got {} B)",
            bytes.len()
        );
    }
    if bytes[..8] != PANASONIC_HDD_MBR_SIGNATURE {
        bail!(
            "Primary EH55 image LBA 0 lacks Panasonic 'SH' signature (got {})",
            hex::encode(&bytes[..8])
        );
    }
    if bytes[510..512] == STANDARD_MBR_BOOT_SIGNATURE {
        bail!("Primary EH55 image contains PC MBR 0x55AA signature at LBA 0[510..512]");
    }
    Ok(())
}

pub(in crate::hdd::writer) fn write_chunked_with_progress<D, F>(
    device: &mut D,
    start_offset: u64,
    data: &[u8],
    stage: Eh55FlashStage,
    label: &str,
    on_progress: &mut F,
) -> Result<()>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(Eh55FlashProgress),
{
    super::super::raw_device::write_chunked_bytes(
        device,
        start_offset,
        data,
        EH55_FLASH_CHUNK_BYTES,
        |pos, total| {
            on_progress(Eh55FlashProgress {
                stage,
                bytes_done: pos as u64,
                bytes_total: total,
                detail: format!("{label}: {pos} / {total} bytes written"),
            });
        },
    )
}

pub(in crate::hdd::writer) fn verify_region_readback_sha256<D, F>(
    device: &D,
    start_offset: u64,
    expected_len: usize,
    expected_sha256: &str,
    label: &str,
    on_progress: &mut F,
) -> Result<Vec<u8>>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(Eh55FlashProgress),
{
    super::super::raw_device::readback_sha256_verify(
        device,
        start_offset,
        expected_len,
        expected_sha256,
        label,
        EH55_FLASH_CHUNK_BYTES,
        |pos, total| {
            on_progress(Eh55FlashProgress {
                stage: Eh55FlashStage::VerifySha256,
                bytes_done: pos as u64,
                bytes_total: total,
                detail: format!("Read-back {label}: {pos} / {total} bytes"),
            });
        },
    )
}

pub(in crate::hdd::writer) fn write_mandatory_64k_cow_backup<D>(
    device: &D,
    backup_dir: &Path,
) -> Result<(PathBuf, PathBuf, String)>
where
    D: BlockFlashDevice + ?Sized,
{
    if !backup_dir.is_dir() {
        bail!(
            "Mandatory backup directory does not exist: {}",
            backup_dir.display()
        );
    }
    let half_bytes = EH55_BACKUP_SECTOR_COUNT * LBA_SECTOR_SIZE_BYTES;
    let mut backup = vec![0u8; half_bytes * 2];
    device.read_at(0, &mut backup[..half_bytes])?;
    device.read_at(LBA_MIRROR_MBR_OFFSET, &mut backup[half_bytes..])?;

    let (_, sha256) = calculate_hashes(&backup);
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let base_name = format!("eh55_pre_flash_backup_{ts_ms}_{}", std::process::id());
    let bin_name = format!("{base_name}.bin");
    let bin_path = backup_dir.join(&bin_name);
    let manifest_path = backup_dir.join(format!("{base_name}.sha256"));

    {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&bin_path)
            .with_context(|| format!("Creating CoW backup {}", bin_path.display()))?;
        f.write_all(&backup)?;
        f.flush()?;
    }
    let manifest_text = format!(
        "{sha256}  {bin_name}\n# LBA 0..63 (32 KiB) + LBA {}..{} (32 KiB)\n",
        LBA_MIRROR_MBR_SECTOR,
        LBA_MIRROR_MBR_SECTOR + EH55_BACKUP_SECTOR_COUNT as u64 - 1
    );
    fs::write(&manifest_path, manifest_text).with_context(|| {
        format!(
            "Writing backup SHA-256 manifest {}",
            manifest_path.display()
        )
    })?;

    Ok((bin_path, manifest_path, sha256))
}

pub(super) fn resolve_mirror_bytes(
    req: &Eh55HddFlashRequest,
    primary_bytes: &[u8],
) -> Result<Option<Vec<u8>>> {
    if let Some(ref p1_path) = req.mirror_p1_path {
        let bytes = fs::read(p1_path)
            .with_context(|| format!("Reading P1 MAIN mirror file {}", p1_path.display()))?;
        if bytes.is_empty() || !bytes.len().is_multiple_of(LBA_SECTOR_SIZE_BYTES) {
            bail!(
                "Mirror P1 file '{}' size ({} B) must be a non-empty multiple of {LBA_SECTOR_SIZE_BYTES} B",
                p1_path.display(),
                bytes.len()
            );
        }
        return Ok(Some(bytes));
    }
    if req.extract_p1_mirror_from_image {
        let p1_start = LBA_PARTITION_1_OFFSET as usize;
        let p1_end = p1_start + LBA_PARTITION_1_ALIGNED_SIZE;
        let Some(slice) = primary_bytes.get(p1_start..p1_end) else {
            bail!(
                "Primary image ({} B) too small to slice P1 MAIN ({} B @ 0x{:X})",
                primary_bytes.len(),
                LBA_PARTITION_1_ALIGNED_SIZE,
                LBA_PARTITION_1_OFFSET
            );
        };
        return Ok(Some(slice.to_vec()));
    }
    Ok(None)
}
