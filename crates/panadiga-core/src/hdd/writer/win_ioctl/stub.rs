#![deny(unsafe_code)]

//! Zero-`unsafe` non-Windows stub preserving cross-platform compilation of `win_ioctl`.

use anyhow::{Result, anyhow};

const UNSUPPORTED_MSG: &str = "Windows raw disk IOCTL is only supported on Windows targets";

/// Returns `false` on non-Windows targets for Win32 admin checks.
#[must_use]
pub fn is_current_user_admin() -> bool {
    false
}

/// Non-Windows stub preserving cross-platform compilation without `unsafe`.
#[derive(Debug)]
pub struct OwnedDiskHandle;

impl OwnedDiskHandle {
    pub fn open(_device_path: &str, _writable: bool) -> Result<Self> {
        Err(anyhow!(UNSUPPORTED_MSG))
    }

    #[must_use]
    pub fn device_path(&self) -> &str {
        ""
    }

    pub fn disk_length_bytes(&self) -> Result<u64> {
        Err(anyhow!(UNSUPPORTED_MSG))
    }

    pub fn query_bus_type(&self) -> Result<u32> {
        Err(anyhow!(UNSUPPORTED_MSG))
    }

    pub fn delete_drive_layout(&mut self) -> Result<()> {
        Err(anyhow!(UNSUPPORTED_MSG))
    }

    pub fn update_properties(&mut self) -> Result<()> {
        Err(anyhow!(UNSUPPORTED_MSG))
    }

    pub fn flush(&mut self) -> Result<()> {
        Err(anyhow!(UNSUPPORTED_MSG))
    }

    pub fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> Result<()> {
        Err(anyhow!(UNSUPPORTED_MSG))
    }

    pub fn write_at(&mut self, _offset: u64, _data: &[u8]) -> Result<()> {
        Err(anyhow!(UNSUPPORTED_MSG))
    }
}

pub fn probe_disk_ioctl_metadata(_device_path: &str) -> Result<(u64, u32)> {
    Err(anyhow!(UNSUPPORTED_MSG))
}
