#![deny(unsafe_code)]

//! Zero-`unsafe` POSIX (Linux & macOS) raw block-device / image handle matching `win_impl` API.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, Result, anyhow, bail};

use crate::constants::LBA_SECTOR_SIZE_BYTES;

use super::super::block_ops::is_unconditional_os_boot_disk;
use super::super::discovery::{
    detect_linux_host_os_disks, list_physical_drives, unmount_drive_partitions,
};
use super::super::eh55_flash::{BUS_TYPE_NVME, infer_bus_type_id};
use super::{ensure_writable_sector_io, validate_sector_aligned};

/// Returns `true` when the current process runs as root (`EUID=0`) on Linux or macOS.
#[must_use]
pub fn is_current_user_admin() -> bool {
    if cfg!(target_os = "android") {
        return false;
    }
    if std::env::var("EUID").ok().is_some_and(|v| v.trim() == "0") {
        return true;
    }
    #[cfg(not(target_os = "android"))]
    {
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .filter(|out| out.status.success())
            .is_some_and(|out| String::from_utf8_lossy(&out.stdout).trim() == "0")
    }
    #[cfg(target_os = "android")]
    {
        false
    }
}

#[cfg(target_os = "linux")]
fn probe_linux_block_size(device_path: &str) -> Option<u64> {
    if let Ok(out) = Command::new("blockdev")
        .args(["--getsize64", device_path])
        .output()
        && out.status.success()
        && let Ok(bytes) = String::from_utf8_lossy(&out.stdout).trim().parse::<u64>()
        && bytes > 0
    {
        return Some(bytes);
    }
    let name = device_path.trim().rsplit('/').next()?;
    let sectors = std::fs::read_to_string(format!("/sys/block/{name}/size"))
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    (sectors > 0).then_some(sectors * LBA_SECTOR_SIZE_BYTES as u64)
}

#[cfg(target_os = "macos")]
fn probe_macos_block_size(device_path: &str) -> Option<u64> {
    let out = Command::new("diskutil")
        .args(["info", device_path])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("Disk Size:") || trimmed.starts_with("Total Size:"))
            && let Some(bytes_idx) = trimmed.find(" Bytes)")
            && let Some(open_idx) = trimmed[..bytes_idx].rfind('(')
        {
            let digits = trimmed[open_idx + 1..bytes_idx].trim();
            if let Ok(val) = digits.parse::<u64>()
                && val > 0
            {
                return Some(val);
            }
        }
    }
    None
}

/// Safe POSIX block device or raw image handle encapsulating `Mutex<File>` and OS unmounts.
#[derive(Debug)]
pub struct OwnedDiskHandle {
    writable: bool,
    path: String,
    file: Mutex<File>,
}

impl OwnedDiskHandle {
    /// Opens a Unix block device (`/dev/sdX`, `/dev/diskN`, never OS boot disks), unmounting
    /// mounted partitions automatically before opening for raw sector writes.
    pub fn open(device_path: &str, writable: bool) -> Result<Self> {
        let trimmed = device_path.trim();
        if trimmed.is_empty() {
            bail!("Device path cannot be empty");
        }
        if is_unconditional_os_boot_disk(trimmed) {
            bail!(
                "FATAL SAFETY LOCKOUT: Refusing to open primary OS disk '{}' via POSIX handle!",
                trimmed
            );
        }
        let low = trimmed.to_ascii_lowercase();
        if let Some(os_disks) = detect_linux_host_os_disks()
            && os_disks.iter().any(|d| {
                let c = d.trim().to_ascii_lowercase();
                !c.is_empty() && (low == c || low.starts_with(&c))
            })
        {
            bail!(
                "FATAL SAFETY LOCKOUT: Refusing to open detected host OS disk '{}'!",
                trimmed
            );
        }
        if writable {
            unmount_drive_partitions(trimmed)?;
        }
        let opened = OpenOptions::new()
            .read(true)
            .write(writable)
            .open(trimmed)
            .with_context(|| {
                format!("Failed opening POSIX device '{trimmed}' (writable={writable})")
            })?;
        Ok(Self {
            writable,
            path: trimmed.to_string(),
            file: Mutex::new(opened),
        })
    }

    fn lock_file(&self) -> Result<MutexGuard<'_, File>> {
        self.file
            .lock()
            .map_err(|_| anyhow!("POSIX disk handle mutex poisoned"))
    }

    /// Path string associated with this handle.
    #[must_use]
    pub fn device_path(&self) -> &str {
        &self.path
    }

    /// Resolves block-device / image byte length via `SEEK_END` or OS block introspection.
    pub fn disk_length_bytes(&self) -> Result<u64> {
        {
            let mut guard = self.lock_file()?;
            if let Ok(end_len) = guard.seek(SeekFrom::End(0))
                && end_len > 0
            {
                return Ok(end_len);
            }
        }

        #[cfg(target_os = "linux")]
        if let Some(bytes) = probe_linux_block_size(&self.path) {
            return Ok(bytes);
        }
        #[cfg(target_os = "macos")]
        if let Some(bytes) = probe_macos_block_size(&self.path) {
            return Ok(bytes);
        }
        if let Ok(drives) = list_physical_drives()
            && let Some(found) = drives.into_iter().find(|d| d.device_path == self.path)
            && found.size_bytes > 0
        {
            return Ok(found.size_bytes);
        }
        bail!("Block device '{}' reported non-positive length", self.path)
    }

    /// Infers Win32-equivalent `STORAGE_BUS_TYPE` ID (`17 == BusTypeNvme`) from Unix discovery.
    pub fn query_bus_type(&self) -> Result<u32> {
        if self.path.to_ascii_lowercase().contains("nvme") {
            return Ok(BUS_TYPE_NVME);
        }
        if let Ok(drives) = list_physical_drives()
            && let Some(found) = drives.into_iter().find(|d| d.device_path == self.path)
        {
            return Ok(infer_bus_type_id(&found.bus_type));
        }
        Ok(0)
    }

    /// Strips any existing MBR/GPT partition signature on sector 0 before raw MEIHFS writes.
    pub fn delete_drive_layout(&mut self) -> Result<()> {
        if !self.writable {
            bail!("Cannot delete drive layout on read-only handle");
        }
        let _ = unmount_drive_partitions(&self.path);
        self.write_at(0, &[0u8; LBA_SECTOR_SIZE_BYTES])?;
        self.flush()
    }

    /// Flushes host buffers and requests kernel partition-table re-read on Linux block nodes.
    pub fn update_properties(&mut self) -> Result<()> {
        self.flush()?;
        #[cfg(target_os = "linux")]
        if self.path.starts_with("/dev/") {
            let _ = Command::new("blockdev")
                .args(["--rereadpt", &self.path])
                .status();
        }
        Ok(())
    }

    /// Flushes all pending POSIX file buffers and metadata to disk.
    pub fn flush(&mut self) -> Result<()> {
        let mut guard = self.lock_file()?;
        guard.flush()?;
        guard
            .sync_all()
            .with_context(|| format!("sync_all() on '{}' failed", self.path))
    }

    /// Reads sector-aligned bytes at `offset`.
    pub fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<()> {
        validate_sector_aligned(offset, buf.len())?;
        let mut guard = self.lock_file()?;
        guard.seek(SeekFrom::Start(offset))?;
        guard.read_exact(buf).with_context(|| {
            format!(
                "POSIX read_exact({}, offset=0x{offset:X}, len={}) failed",
                self.path,
                buf.len()
            )
        })
    }

    /// Writes sector-aligned bytes at `offset`.
    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        ensure_writable_sector_io(self.writable, &self.path, offset, data.len())?;
        let mut guard = self.lock_file()?;
        guard.seek(SeekFrom::Start(offset))?;
        guard.write_all(data).with_context(|| {
            format!(
                "POSIX write_all({}, offset=0x{offset:X}, len={}) failed",
                self.path,
                data.len()
            )
        })
    }
}
