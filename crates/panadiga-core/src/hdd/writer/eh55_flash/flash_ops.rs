#![deny(unsafe_code)]

//! Orchestrates fail-closed Panasonic DMR-EH55 HDD flashing over any [`BlockFlashDevice`].

use std::fs;

use anyhow::{Context, Result, bail};

use crate::constants::LBA_SECTOR_SIZE_BYTES;
use crate::nvram::calculate_hashes;

use super::backup_verify::{
    HddPreflightParams, preflight_hdd_target_geometry, resolve_mirror_bytes,
    validate_eh55_primary_image, verify_region_readback_sha256, write_chunked_with_progress,
    write_mandatory_64k_cow_backup,
};
use super::types::{
    BlockFlashDevice, EH55_BACKUP_SECTOR_COUNT, Eh55FlashProgress, Eh55FlashStage,
    Eh55HddFlashReport, Eh55HddFlashRequest,
};

/// Executes the fail-closed EH55 HDD flash workflow over any [`BlockFlashDevice`]:
/// safety lockout (`PhysicalDrive0`, OS disks, `BusTypeNvme`), capacity bounds,
/// mandatory 64 KiB CoW backup, `delete_drive_layout`, chunked primary + high-LBA `MAIN` mirror
/// write, `flush_and_update`, and full read-back SHA-256 verification.
pub fn execute_eh55_hdd_flash<D, F>(
    device: &mut D,
    req: &Eh55HddFlashRequest,
    mut on_progress: F,
) -> Result<Eh55HddFlashReport>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(Eh55FlashProgress),
{
    let geo = preflight_hdd_target_geometry(
        device,
        HddPreflightParams {
            device_path: &req.device_path,
            confirmation_token: &req.confirmation_token,
            expected_gb_range: req.expected_gb_range,
            detected_os_disks: req.detected_os_disks.as_deref(),
        },
        &mut on_progress,
    )?;
    let disk_bytes = geo.disk_bytes;
    let bus_type_id = geo.bus_type_id;
    let mirror_main_lba = geo.mirror_main_lba;
    let mirror_offset_bytes = geo.mirror_offset_bytes;

    let primary_bytes = fs::read(&req.image_path)
        .with_context(|| format!("Reading primary image {}", req.image_path.display()))?;
    validate_eh55_primary_image(&primary_bytes)?;
    if primary_bytes.len() as u64 > mirror_offset_bytes {
        bail!(
            "Primary image ({} B) overlaps mirror_main_lba {} (offset 0x{mirror_offset_bytes:X})",
            primary_bytes.len(),
            mirror_main_lba
        );
    }
    let (_, primary_sha256) = calculate_hashes(&primary_bytes);

    let mirror_bytes_opt = resolve_mirror_bytes(req, &primary_bytes)?;
    let mirror_sha256 = if let Some(ref m) = mirror_bytes_opt {
        let end = mirror_offset_bytes
            .checked_add(m.len() as u64)
            .context("Mirror range overflow")?;
        if end > disk_bytes {
            bail!(
                "Mirror P1 payload ({} B @ LBA {mirror_main_lba}) exceeds disk size {disk_bytes} B",
                m.len()
            );
        }
        Some(calculate_hashes(m).1)
    } else {
        None
    };

    on_progress(Eh55FlashProgress {
        stage: Eh55FlashStage::Backup64k,
        bytes_done: 0,
        bytes_total: (EH55_BACKUP_SECTOR_COUNT * LBA_SECTOR_SIZE_BYTES * 2) as u64,
        detail: "Saving mandatory 64 KiB pre-write CoW backup (LBA 0..63 + 57344..57407)"
            .to_string(),
    });
    let (backup_bin_path, backup_manifest_path, backup_64k_sha256) =
        write_mandatory_64k_cow_backup(device, &req.backup_dir)?;

    on_progress(Eh55FlashProgress {
        stage: Eh55FlashStage::WipeLayout,
        bytes_done: 0,
        bytes_total: 0,
        detail: "Clearing OS partition layout prior to raw MEIHFS write".to_string(),
    });
    device.delete_drive_layout()?;

    write_chunked_with_progress(
        device,
        0,
        &primary_bytes,
        Eh55FlashStage::Write100m,
        "Primary EH55 image",
        &mut on_progress,
    )?;

    let mirror_bytes_written = if let Some(ref m) = mirror_bytes_opt {
        write_chunked_with_progress(
            device,
            mirror_offset_bytes,
            m,
            Eh55FlashStage::WriteMirror,
            "Mirror P1 MAIN",
            &mut on_progress,
        )?;
        m.len()
    } else {
        0
    };

    device.flush_and_update()?;

    let rb_primary = verify_region_readback_sha256(
        device,
        0,
        primary_bytes.len(),
        &primary_sha256,
        "Primary EH55 image",
        &mut on_progress,
    )?;
    validate_eh55_primary_image(&rb_primary)?;

    if let (Some(m), Some(expected_m_sha)) = (&mirror_bytes_opt, &mirror_sha256) {
        let _ = verify_region_readback_sha256(
            device,
            mirror_offset_bytes,
            m.len(),
            expected_m_sha,
            "Mirror P1 MAIN",
            &mut on_progress,
        )?;
    }

    on_progress(Eh55FlashProgress {
        stage: Eh55FlashStage::Done,
        bytes_done: primary_bytes.len() as u64 + mirror_bytes_written as u64,
        bytes_total: primary_bytes.len() as u64 + mirror_bytes_written as u64,
        detail: "EH55 HDD flash and full read-back SHA-256 verification completed".to_string(),
    });

    Ok(Eh55HddFlashReport {
        device_path: req.device_path.clone(),
        disk_bytes,
        bus_type_id,
        mirror_main_lba,
        backup_bin_path: backup_bin_path.to_string_lossy().into_owned(),
        backup_manifest_path: backup_manifest_path.to_string_lossy().into_owned(),
        backup_64k_sha256,
        primary_bytes_written: primary_bytes.len(),
        primary_sha256,
        mirror_bytes_written,
        mirror_sha256,
        readback_verified: true,
    })
}
