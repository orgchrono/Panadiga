//! Cross-platform storage drive & mounted media facade (HDD vs SD Card separation).

use anyhow::{Result, bail};

use super::types::{PhysicalDriveInfo, StorageMediaRole, filter_drives_by_role};

/// Discovers all connected physical drives & mounted partitions across Windows, Linux, and macOS.
pub fn list_physical_drives() -> Result<Vec<PhysicalDriveInfo>> {
    #[cfg(windows)]
    {
        Ok(super::discovery_win::list_windows_drives())
    }
    #[cfg(target_os = "linux")]
    {
        Ok(super::discovery_unix::list_linux_drives())
    }
    #[cfg(target_os = "macos")]
    {
        Ok(super::discovery_unix::list_macos_drives())
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        Ok(Vec::new())
    }
}

/// Discovers physical storage drives filtered by optional media role (`HardDisk`, `SdCard`, etc.).
pub fn list_drives_by_role(role: Option<StorageMediaRole>) -> Result<Vec<PhysicalDriveInfo>> {
    Ok(filter_drives_by_role(list_physical_drives()?, role))
}

/// Dedicated Hard Disk Drive enumerator (filters out SD/MMC card readers unless role override needed).
pub fn list_hard_disk_drives() -> Result<Vec<PhysicalDriveInfo>> {
    list_drives_by_role(Some(StorageMediaRole::HardDisk))
}

/// Dedicated SD Card & removable flash reader enumerator (MMC/SD readers & removable media <= 64 GB).
pub fn list_sd_card_drives() -> Result<Vec<PhysicalDriveInfo>> {
    list_drives_by_role(Some(StorageMediaRole::SdCard))
}

/// Resolves a mounted directory (e.g. `/media/user/DISK`) to its Linux `/dev/...` partition node.
pub fn resolve_mount_point_to_block_device(mount_target: &str) -> Option<String> {
    super::discovery_unix::resolve_mount_point_to_block_device(mount_target)
}

pub(crate) fn detect_linux_host_os_disks() -> Option<Vec<String>> {
    super::discovery_unix::detect_linux_host_os_disks()
}

/// Unmounts any active partitions belonging to `device_path` before raw block operations.
pub fn unmount_drive_partitions(device_path: &str) -> Result<Vec<String>> {
    let clean = device_path.trim();
    if clean.is_empty() {
        bail!("Device path cannot be empty.");
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        return super::discovery_unix::unmount_unix_partitions(clean);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = clean;
        Ok(Vec::new())
    }
}
