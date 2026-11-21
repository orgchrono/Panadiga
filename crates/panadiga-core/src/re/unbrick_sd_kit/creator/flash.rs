#![deny(unsafe_code)]

//! Strict lockout-enforced physical SD writer (`StorageMediaRole::SdCard`) and `.img` creator.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::super::fat16_builder::SynthesizedSdImage;
use super::super::inspect_sd::{SdDiskInspectionReport, inspect_sd_disk_bytes};
use super::plan::{
    SdCardCreatorPlan, SdCardCreatorRequest, assess_sd_card_size_advisory, plan_sd_recovery_card,
};
use super::report::{
    SdCardAdvisory, SdCardCreatorReport, SdCreatorProgress, SdCreatorStage,
    build_layperson_sd_steps,
};
use crate::hdd::writer::{
    BUS_TYPE_NVME, BlockFlashDevice, PhysicalDriveInfo, StorageMediaRole, list_physical_drives,
    open_physical_flash_device, readback_sha256_verify, verify_safety_lockout,
    verify_safety_lockout_with_os_disks, write_chunked_bytes,
};
use crate::nvram::calculate_hashes;

const SD_FLASH_CHUNK_BYTES: usize = 512 * 1024;

#[allow(clippy::too_many_arguments)]
fn assemble_sd_creator_report(
    target: String,
    target_kind: &str,
    plan: SdCardCreatorPlan,
    synth: SynthesizedSdImage,
    advisory: SdCardAdvisory,
    readback_verified: bool,
    inspection: SdDiskInspectionReport,
) -> SdCardCreatorReport {
    let next_steps =
        build_layperson_sd_steps(&plan.model, plan.volume_mb, &advisory, plan.has_pana_sd);
    SdCardCreatorReport {
        target,
        target_kind: target_kind.to_string(),
        model: plan.model,
        volume_mb: plan.volume_mb,
        cleanroom_only: plan.cleanroom_only,
        bytes_written: synth.image_bytes.len(),
        image_sha256: synth.image_sha256,
        advisory,
        files: synth.files,
        emu_gate: plan.emu_gate,
        readback_verified,
        inspection,
        next_steps,
    }
}

/// Creates and self-audits a standalone raw `.img` file without touching any physical media.
pub fn create_sd_recovery_image(
    req: &SdCardCreatorRequest,
    out_path: &Path,
) -> Result<SdCardCreatorReport> {
    let (plan, synth) = plan_sd_recovery_card(req)?;
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Creating output directory {}", parent.display()))?;
    }
    fs::write(out_path, &synth.image_bytes)
        .with_context(|| format!("Writing SD recovery image {}", out_path.display()))?;
    let readback = fs::read(out_path)
        .with_context(|| format!("Verifying SD recovery image {}", out_path.display()))?;
    let (_, actual_sha) = calculate_hashes(&readback);
    if actual_sha != synth.image_sha256 {
        bail!(
            "Written SD image SHA-256 mismatch at {}: expected {}, got {}",
            out_path.display(),
            synth.image_sha256,
            actual_sha
        );
    }
    let inspection = inspect_sd_disk_bytes(&out_path.display().to_string(), &readback)?;
    let advisory = assess_sd_card_size_advisory(0);
    let readback_ok = inspection.overall_status == "PASS";
    Ok(assemble_sd_creator_report(
        out_path.display().to_string(),
        "image_file",
        plan,
        synth,
        advisory,
        readback_ok,
        inspection,
    ))
}

/// Writes the deterministic FAT16B (`0x06`) partition onto any [`BlockFlashDevice`] enforcing
/// strict `StorageMediaRole::SdCard` lockout, `CONFIRM-<device>` validation, and readback SHA-256.
pub fn flash_sd_recovery_card<D, F>(
    device: &mut D,
    drive_info: &PhysicalDriveInfo,
    host_os_disks: Option<&[&str]>,
    req: &SdCardCreatorRequest,
    mut on_progress: F,
) -> Result<SdCardCreatorReport>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(SdCreatorProgress),
{
    if drive_info.drive_role != StorageMediaRole::SdCard {
        bail!(
            "FATAL SAFETY LOCKOUT: '{}' has role {:?}; SD Creator allows ONLY StorageMediaRole::SdCard",
            drive_info.device_path,
            drive_info.drive_role
        );
    }
    if drive_info.is_system_drive || !drive_info.safe_to_write {
        bail!(
            "FATAL SAFETY LOCKOUT: '{}' is marked unsafe/system: {:?}",
            drive_info.device_path,
            drive_info.lockout_reason
        );
    }
    if device.bus_type() == BUS_TYPE_NVME {
        bail!(
            "FATAL SAFETY LOCKOUT: '{}' reports NVMe bus type (ID 17); refused for SD card flashing",
            drive_info.device_path
        );
    }
    let token = req.confirm_token.as_deref().unwrap_or("");
    match host_os_disks {
        Some(os_disks) => {
            verify_safety_lockout_with_os_disks(&drive_info.device_path, token, Some(os_disks))?
        }
        None => verify_safety_lockout(&drive_info.device_path, token)?,
    }
    on_progress(SdCreatorProgress {
        stage: SdCreatorStage::LockoutVerify,
        bytes_done: 0,
        bytes_total: 0,
        detail: format!(
            "Verified SdCard lockout and confirmation token for {}",
            drive_info.device_path
        ),
    });

    let (plan, synth) = plan_sd_recovery_card(req)?;
    on_progress(SdCreatorProgress {
        stage: SdCreatorStage::Synthesize,
        bytes_done: synth.image_bytes.len() as u64,
        bytes_total: synth.image_bytes.len() as u64,
        detail: format!(
            "Synthesized {} MiB FAT16B image ({} B)",
            plan.volume_mb,
            synth.image_bytes.len()
        ),
    });

    let dev_bytes = device.length_bytes()?;
    if dev_bytes < synth.image_bytes.len() as u64 {
        bail!(
            "Target SD device '{}' capacity ({} B) is smaller than {} B image",
            drive_info.device_path,
            dev_bytes,
            synth.image_bytes.len()
        );
    }
    let advisory = assess_sd_card_size_advisory(dev_bytes);

    device.delete_drive_layout()?;
    on_progress(SdCreatorProgress {
        stage: SdCreatorStage::DeleteLayout,
        bytes_done: 0,
        bytes_total: 0,
        detail: "Cleared existing SD partition table layout".to_string(),
    });

    write_chunked_bytes(
        device,
        0,
        &synth.image_bytes,
        SD_FLASH_CHUNK_BYTES,
        |done, total| {
            on_progress(SdCreatorProgress {
                stage: SdCreatorStage::WritePartition,
                bytes_done: done as u64,
                bytes_total: total,
                detail: format!("Flashing FAT16B SD layout: {done} / {total} bytes"),
            });
        },
    )?;
    device.flush_and_update()?;

    let readback = readback_sha256_verify(
        device,
        0,
        synth.image_bytes.len(),
        &synth.image_sha256,
        &drive_info.device_path,
        SD_FLASH_CHUNK_BYTES,
        |done, total| {
            on_progress(SdCreatorProgress {
                stage: SdCreatorStage::ReadbackVerify,
                bytes_done: done as u64,
                bytes_total: total,
                detail: format!("Verifying SD read-back SHA-256: {done} / {total} bytes"),
            });
        },
    )?;
    let inspection = inspect_sd_disk_bytes(&drive_info.device_path, &readback)?;
    if inspection.overall_status != "PASS" {
        bail!(
            "Post-flash readback inspection failed ({}): {:?}",
            inspection.overall_status,
            inspection.checks
        );
    }
    on_progress(SdCreatorProgress {
        stage: SdCreatorStage::InspectFat16,
        bytes_done: synth.image_bytes.len() as u64,
        bytes_total: synth.image_bytes.len() as u64,
        detail: "Post-write FAT16B + DIGADIAG.FRM read-back audit PASSED".to_string(),
    });

    Ok(assemble_sd_creator_report(
        drive_info.device_path.clone(),
        "physical_sd",
        plan,
        synth,
        advisory,
        true,
        inspection,
    ))
}

/// Discovers host drives, validates `StorageMediaRole::SdCard` lockout, opens raw handle, and flashes.
pub fn create_sd_recovery_card_on_device<F>(
    req: &SdCardCreatorRequest,
    on_progress: F,
) -> Result<SdCardCreatorReport>
where
    F: FnMut(SdCreatorProgress),
{
    let target = req
        .device_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .context("Missing target physical SD device_path")?;
    let drives = list_physical_drives()?;
    let drive_info = drives
        .iter()
        .find(|d| d.device_path.eq_ignore_ascii_case(target))
        .with_context(|| format!("Physical SD target '{target}' not found in host drive list"))?;
    if drive_info.drive_role != StorageMediaRole::SdCard {
        bail!(
            "FATAL SAFETY LOCKOUT: '{}' has role {:?}; only StorageMediaRole::SdCard may be flashed",
            drive_info.device_path,
            drive_info.drive_role
        );
    }
    let mut device = open_physical_flash_device(&drive_info.device_path, true)?;
    flash_sd_recovery_card(&mut device, drive_info, None, req, on_progress)
}

#[cfg(test)]
#[path = "flash_tests.rs"]
mod tests;
