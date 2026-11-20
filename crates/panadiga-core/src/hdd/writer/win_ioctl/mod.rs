//! Isolated Windows Win32 raw-disk IOCTL wrapper (`CreateFileW` + `FILE_FLAG_WRITE_THROUGH` +
//! `IOCTL_STORAGE_QUERY_PROPERTY` + `IOCTL_DISK_GET_LENGTH_INFO` + `IOCTL_DISK_DELETE_DRIVE_LAYOUT`
//! + `IOCTL_DISK_UPDATE_PROPERTIES` + `FlushFileBuffers`).
//!
//! All `unsafe` Win32 FFI calls in the storage writer subsystem are confined strictly to
//! `win_impl` and `win_rw` (`#[cfg(windows)]`) behind safe `OwnedDiskHandle` (`Drop -> CloseHandle`)
//! methods, with a zero-`unsafe` `stub` module on non-Windows targets.

#[cfg(not(windows))]
mod stub;
#[cfg(windows)]
mod win_impl;
#[cfg(windows)]
mod win_rw;

#[cfg(not(windows))]
pub use stub::{OwnedDiskHandle, is_current_user_admin};
#[cfg(windows)]
pub use win_impl::{OwnedDiskHandle, is_current_user_admin};

use anyhow::{Result, bail};

use crate::constants::LBA_SECTOR_SIZE_BYTES;

pub(super) fn validate_sector_aligned(offset: u64, len: usize) -> Result<()> {
    let sec = LBA_SECTOR_SIZE_BYTES as u64;
    if !offset.is_multiple_of(sec) || !len.is_multiple_of(LBA_SECTOR_SIZE_BYTES) {
        bail!(
            "Unaligned raw disk I/O rejected (offset=0x{offset:X}, len={len}; must be {LBA_SECTOR_SIZE_BYTES}-byte aligned)"
        );
    }
    Ok(())
}

pub(super) fn ensure_writable_sector_io(
    writable: bool,
    device_path: &str,
    offset: u64,
    len: usize,
) -> Result<()> {
    if !writable {
        bail!("Cannot write to read-only handle '{device_path}'");
    }
    validate_sector_aligned(offset, len)
}

/// Probes disk length & `BusType` (`IOCTL_STORAGE_QUERY_PROPERTY` on Windows or Unix block metadata).
pub fn probe_disk_ioctl_metadata(device_path: &str) -> Result<(u64, u32)> {
    let handle = OwnedDiskHandle::open(device_path, false)?;
    let len = handle.disk_length_bytes()?;
    let bus = handle.query_bus_type().unwrap_or(0);
    Ok((len, bus))
}
