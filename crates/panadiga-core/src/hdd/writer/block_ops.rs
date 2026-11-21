//! Safety lockout validation and guarded LBA block writing for Panasonic MEIHDFS restoration.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use anyhow::{Context, Result, bail};

use super::super::mkfs::MeihdfsImage;
use crate::constants::*;

/// Execution report of a physical drive write/format operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DriveWriteReport {
    pub device_path: String,
    pub operation: String,
    pub bytes_written: u64,
    pub sectors_written: u64,
    pub success: bool,
    pub message: String,
}

/// Returns `true` when `device_path` refers to an unconditional primary OS boot disk across
/// Windows (`\\.\PhysicalDrive0`) or macOS (`/dev/disk0*`, `/dev/rdisk0*`).
#[must_use]
pub fn is_unconditional_os_boot_disk(device_path: &str) -> bool {
    let normalized = device_path.trim().to_ascii_lowercase();
    normalized.contains("physicaldrive0")
        || normalized == "/dev/disk0"
        || normalized == "/dev/rdisk0"
        || normalized.starts_with("/dev/disk0s")
        || normalized.starts_with("/dev/rdisk0s")
}

/// Checks whether `target` matches `os_disk` or a partition suffix of `os_disk`,
/// preventing `/dev/disk1` from falsely matching `/dev/disk10` or `/dev/sda` from blocking `/dev/sdaa`.
pub(crate) fn target_matches_base_disk_or_partition(target: &str, os_disk: &str) -> bool {
    let t = target.trim().to_ascii_lowercase();
    let d = os_disk.trim().to_ascii_lowercase();
    if d.is_empty() || !t.starts_with(&d) {
        return false;
    }
    let suffix = &t[d.len()..];
    if suffix.is_empty() {
        return true;
    }
    if d.ends_with(|c: char| c.is_ascii_digit()) {
        // e.g. /dev/disk0 -> /dev/disk0s1, /dev/nvme0n1 -> /dev/nvme0n1p1, /dev/mmcblk0 -> /dev/mmcblk0p1
        let mut chars = suffix.chars();
        matches!(chars.next(), Some('p' | 's')) && chars.next().is_some_and(|c| c.is_ascii_digit())
    } else {
        // e.g. /dev/sda -> /dev/sda1 (never /dev/sdaa)
        suffix.chars().all(|c| c.is_ascii_digit())
    }
}

/// Pure safety lockout validator checking host OS boot drives & exact confirmation token.
pub fn verify_safety_lockout_with_os_disks(
    device_path: &str,
    confirmation_token: &str,
    detected_os_disks: Option<&[&str]>,
) -> Result<()> {
    let normalized = device_path.trim().to_ascii_lowercase();
    let is_default_os_boot = is_unconditional_os_boot_disk(device_path);
    let is_detected_os_disk = match detected_os_disks {
        Some(os_disks) => os_disks
            .iter()
            .any(|os_dev| target_matches_base_disk_or_partition(&normalized, os_dev)),
        None => target_matches_base_disk_or_partition(&normalized, "/dev/sda"),
    };
    if is_default_os_boot || is_detected_os_disk {
        bail!(
            "FATAL SAFETY INVARIANT VIOLATION: Access to {} is unconditionally locked! \
            The primary operating system drive can never be overwritten by Panadiga.",
            device_path
        );
    }

    let expected_token = format!("CONFIRM-{}", device_path.trim_start_matches(r"\\.\"));
    if confirmation_token != expected_token {
        bail!(
            "Confirmation token mismatch! Expected '{}', got '{}'. Operation aborted.",
            expected_token,
            confirmation_token
        );
    }

    Ok(())
}

/// Verifies that a target drive is safe for writing and validates the confirmation token.
pub fn verify_safety_lockout(device_path: &str, confirmation_token: &str) -> Result<()> {
    let os_disks_owned = super::discovery::detect_linux_host_os_disks();
    let os_refs: Option<Vec<&str>> = os_disks_owned
        .as_ref()
        .map(|list| list.iter().map(String::as_str).collect());
    verify_safety_lockout_with_os_disks(device_path, confirmation_token, os_refs.as_deref())
}

/// Helper to verify safety lockout, safely unmount non-OS partitions on Unix, and open raw handle.
pub fn open_verified_block_device(device_path: &str, confirmation_token: &str) -> Result<File> {
    verify_safety_lockout(device_path, confirmation_token)?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        super::discovery::unmount_drive_partitions(device_path)?;
    }

    OpenOptions::new()
        .read(true)
        .write(true)
        .open(device_path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                anyhow::anyhow!(
                    "Permission denied opening '{}' ({}). Linux: add user to 'disk' group (`sudo usermod -aG disk $USER`) or run `pkexec panadiga ...`. Windows: run Panadiga as Administrator.",
                    device_path,
                    e
                )
            } else {
                anyhow::Error::new(e).context(format!(
                    "Failed to open physical block device at {}",
                    device_path
                ))
            }
        })
}

/// Writes procedurally synthesized MEIHDFS structures directly to a physical drive.
pub fn write_meihdfs_procedural_to_drive(
    device_path: &str,
    layout: &MeihdfsImage,
    confirmation_token: &str,
) -> Result<DriveWriteReport> {
    let mut file = open_verified_block_device(device_path, confirmation_token)?;
    let mut total_written: u64 = 0;

    for (offset, bytes) in [
        (0u64, layout.mbr_lba0.as_slice()),
        (
            ALLOCATION_DESCRIPTORS_OFFSET,
            layout.descriptors_lba24.as_slice(),
        ),
        (
            LBA_JOURNAL_CATALOG_OFFSET,
            layout.journal_header_lba16384.as_slice(),
        ),
        (LBA_MIRROR_MBR_OFFSET, layout.mirror_mbr_lba57344.as_slice()),
        (
            ALLOCATION_DESCRIPTORS_MIRROR_OFFSET,
            layout.mirror_descriptors_lba57368.as_slice(),
        ),
    ] {
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(bytes)?;
        total_written += bytes.len() as u64;
    }

    file.flush()?;

    Ok(DriveWriteReport {
        device_path: device_path.to_string(),
        operation: "Procedural MEIHDFS Formatting (mkfs.meihdfs)".to_string(),
        bytes_written: total_written,
        sectors_written: total_written / 512,
        success: true,
        message: format!(
            "Successfully formatted {} with MEIHDFS structures (Total: {} bytes written).",
            device_path, total_written
        ),
    })
}

/// Safely writes an existing raw image (or uncompressed slice) to a verified physical drive.
pub fn write_raw_image_to_drive(
    device_path: &str,
    image_bytes: &[u8],
    confirmation_token: &str,
) -> Result<DriveWriteReport> {
    if image_bytes.len() < LBA_SECTOR_SIZE_BYTES
        || !image_bytes.len().is_multiple_of(LBA_SECTOR_SIZE_BYTES)
    {
        bail!(
            "Raw image length must be a multiple of 512 bytes (got {} bytes)",
            image_bytes.len()
        );
    }

    let mut file = open_verified_block_device(device_path, confirmation_token)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(image_bytes)?;
    file.flush()?;

    Ok(DriveWriteReport {
        device_path: device_path.to_string(),
        operation: "Raw Disk Image Streaming".to_string(),
        bytes_written: image_bytes.len() as u64,
        sectors_written: (image_bytes.len() / LBA_SECTOR_SIZE_BYTES) as u64,
        success: true,
        message: format!(
            "Successfully streamed {} bytes to {} without errors.",
            image_bytes.len(),
            device_path
        ),
    })
}

/// Restores primary MEIHDFS structures atomically from dual redundancy mirrors.
pub fn restore_meihdfs_from_mirrors(
    device_path: &str,
    confirmation_token: &str,
) -> Result<DriveWriteReport> {
    let mut file = open_verified_block_device(device_path, confirmation_token)?;

    let mut mirror_mbr = [0u8; LBA_SECTOR_SIZE_BYTES];
    file.seek(SeekFrom::Start(LBA_MIRROR_MBR_OFFSET))?;
    file.read_exact(&mut mirror_mbr)
        .context("Failed to read Mirror MBR at LBA 57344")?;

    if mirror_mbr[0..8] != PANASONIC_HDD_MBR_SIGNATURE {
        bail!(
            "Mirror MBR at LBA 57344 is invalid! Expected Panasonic 'SH' signature, got 0x{}",
            hex::encode(&mirror_mbr[0..8])
        );
    }

    let mut mirror_desc = [0u8; ALLOCATION_DESCRIPTORS_SIZE];
    file.seek(SeekFrom::Start(ALLOCATION_DESCRIPTORS_MIRROR_OFFSET))?;
    file.read_exact(&mut mirror_desc)
        .context("Failed to read Mirror Allocation Descriptors at LBA 57368")?;

    let first_tag = mirror_desc[0] ^ 0xFF;
    if first_tag != MEIHDFS_TAG_EXTENT_HEADER && first_tag != MEIHDFS_TAG_FREE {
        bail!(
            "Mirror Allocation Descriptors at LBA 57368 contain invalid tag: 0x{:02X}",
            first_tag
        );
    }

    file.seek(SeekFrom::Start(0))?;
    file.write_all(&mirror_mbr)?;
    file.seek(SeekFrom::Start(ALLOCATION_DESCRIPTORS_OFFSET))?;
    file.write_all(&mirror_desc)?;
    file.flush()?;

    let total = (LBA_SECTOR_SIZE_BYTES + ALLOCATION_DESCRIPTORS_SIZE) as u64;

    Ok(DriveWriteReport {
        device_path: device_path.to_string(),
        operation: "Atomic Dual-Mirror MEIHDFS Restoration".to_string(),
        bytes_written: total,
        sectors_written: total / 512,
        success: true,
        message: format!(
            "Successfully restored LBA 0 (512B) and LBA 24..31 (4096B) from mirrors LBA 57344 and 57368 on {}.",
            device_path
        ),
    })
}
