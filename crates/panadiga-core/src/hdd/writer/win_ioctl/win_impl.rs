//! Windows Win32 `HANDLE` lifecycle and disk metadata/layout IOCTLs for [`OwnedDiskHandle`].
//!
//! All Win32 FFI calls here carry explicit `// SAFETY:` justifications.

use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};

use anyhow::{Result, bail};
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_WRITE_THROUGH, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::{
    GET_LENGTH_INFORMATION, IOCTL_DISK_DELETE_DRIVE_LAYOUT, IOCTL_DISK_GET_LENGTH_INFO,
    IOCTL_DISK_UPDATE_PROPERTIES, IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery,
    STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY, StorageDeviceProperty,
};
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

/// Returns `true` if the current Windows process holds Administrator privileges.
#[must_use]
pub fn is_current_user_admin() -> bool {
    // SAFETY: `IsUserAnAdmin` takes no parameters and returns a Win32 `BOOL` (`0` or non-zero).
    unsafe { IsUserAnAdmin() != 0 }
}

fn encode_wide_nul(path: &str) -> Vec<u16> {
    OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// RAII Win32 `HANDLE` wrapper guaranteeing `CloseHandle` on drop.
#[derive(Debug)]
pub struct OwnedDiskHandle {
    pub(super) handle: HANDLE,
    pub(super) device_path: String,
    pub(super) writable: bool,
}

impl Drop for OwnedDiskHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() && self.handle != INVALID_HANDLE_VALUE {
            // SAFETY: `self.handle` was returned by `CreateFileW`, checked valid on open,
            // and has not been closed yet because `Drop` runs at most once.
            unsafe {
                let _ = CloseHandle(self.handle);
            }
            self.handle = INVALID_HANDLE_VALUE;
        }
    }
}

impl OwnedDiskHandle {
    /// Opens a Windows block device (`\\.\PhysicalDriveN`, never `PhysicalDrive0`) with
    /// `FILE_FLAG_WRITE_THROUGH` when `writable == true`.
    pub fn open(device_path: &str, writable: bool) -> Result<Self> {
        let trimmed = device_path.trim();
        if trimmed.is_empty() {
            bail!("Device path cannot be empty");
        }
        if trimmed.to_ascii_lowercase().contains("physicaldrive0") {
            bail!("FATAL SAFETY LOCKOUT: Refusing to open PhysicalDrive0 via Win32 IOCTL!");
        }
        let wide = encode_wide_nul(trimmed);
        let access = if writable {
            GENERIC_READ | GENERIC_WRITE
        } else {
            GENERIC_READ
        };
        let flags = if writable { FILE_FLAG_WRITE_THROUGH } else { 0 };
        // SAFETY: `wide` is a valid NUL-terminated UTF-16 buffer alive for the call duration;
        // optional security/template pointers are null per Win32 `CreateFileW` contract.
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                flags,
                null_mut(),
            )
        };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            let err = std::io::Error::last_os_error();
            bail!("CreateFileW('{trimmed}', writable={writable}) failed: {err}");
        }
        Ok(Self {
            handle,
            device_path: trimmed.to_string(),
            writable,
        })
    }

    /// Path string associated with this handle.
    #[must_use]
    pub fn device_path(&self) -> &str {
        &self.device_path
    }

    /// Queries exact raw disk byte length via `IOCTL_DISK_GET_LENGTH_INFO`.
    pub fn disk_length_bytes(&self) -> Result<u64> {
        let mut info = GET_LENGTH_INFORMATION { Length: 0 };
        let mut bytes_ret: u32 = 0;
        // SAFETY: `self.handle` is an open disk handle; `info` is valid writable `GET_LENGTH_INFORMATION`
        // of exact size `size_of::<GET_LENGTH_INFORMATION>()`, and `bytes_ret` is a valid stack `u32`.
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_DISK_GET_LENGTH_INFO,
                null(),
                0,
                (&raw mut info).cast(),
                size_of::<GET_LENGTH_INFORMATION>() as u32,
                &raw mut bytes_ret,
                null_mut(),
            )
        };
        if ok == 0 || bytes_ret as usize != size_of::<GET_LENGTH_INFORMATION>() {
            let err = std::io::Error::last_os_error();
            bail!(
                "IOCTL_DISK_GET_LENGTH_INFO on '{}' failed: {err}",
                self.device_path
            );
        }
        if info.Length <= 0 {
            bail!(
                "Disk '{}' reported non-positive length {}",
                self.device_path,
                info.Length
            );
        }
        Ok(info.Length as u64)
    }

    /// Queries `STORAGE_DEVICE_DESCRIPTOR.BusType` (`17 == BusTypeNvme`) via `IOCTL_STORAGE_QUERY_PROPERTY`.
    pub fn query_bus_type(&self) -> Result<u32> {
        let query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut out_buf = [0u8; 1024];
        let mut bytes_ret: u32 = 0;
        // SAFETY: `self.handle` is valid; `query` and `out_buf` are valid stack buffers with exact sizes.
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                (&raw const query).cast(),
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                out_buf.as_mut_ptr().cast(),
                out_buf.len() as u32,
                &raw mut bytes_ret,
                null_mut(),
            )
        };
        if ok == 0 || (bytes_ret as usize) < size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
            let err = std::io::Error::last_os_error();
            bail!(
                "IOCTL_STORAGE_QUERY_PROPERTY on '{}' failed: {err}",
                self.device_path
            );
        }
        // SAFETY: `bytes_ret >= size_of::<STORAGE_DEVICE_DESCRIPTOR>()` was verified above, and
        // `read_unaligned` handles arbitrary byte buffer alignment safely.
        let desc: STORAGE_DEVICE_DESCRIPTOR =
            unsafe { std::ptr::read_unaligned(out_buf.as_ptr().cast()) };
        Ok(desc.BusType as u32)
    }

    fn ioctl_zero(&self, code: u32, name: &str) -> Result<()> {
        let mut bytes_ret: u32 = 0;
        // SAFETY: `self.handle` is valid; zero-argument control code takes null in/out buffers.
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                code,
                null(),
                0,
                null_mut(),
                0,
                &raw mut bytes_ret,
                null_mut(),
            )
        };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            bail!(
                "{name} (0x{code:08X}) on '{}' failed: {err}",
                self.device_path
            );
        }
        Ok(())
    }

    /// Strips Windows partition layout / `0x55AA` MBR cache via `IOCTL_DISK_DELETE_DRIVE_LAYOUT`.
    pub fn delete_drive_layout(&mut self) -> Result<()> {
        if !self.writable {
            bail!("Cannot delete drive layout on read-only handle");
        }
        self.ioctl_zero(
            IOCTL_DISK_DELETE_DRIVE_LAYOUT,
            "IOCTL_DISK_DELETE_DRIVE_LAYOUT",
        )
    }

    /// Notifies Windows disk manager to refresh geometry/layout via `IOCTL_DISK_UPDATE_PROPERTIES`.
    pub fn update_properties(&mut self) -> Result<()> {
        self.ioctl_zero(IOCTL_DISK_UPDATE_PROPERTIES, "IOCTL_DISK_UPDATE_PROPERTIES")
    }
}

/// Probes disk length & `BusType` (`IOCTL_STORAGE_QUERY_PROPERTY`) for an explicit target path.
pub fn probe_disk_ioctl_metadata(device_path: &str) -> Result<(u64, u32)> {
    let handle = OwnedDiskHandle::open(device_path, false)?;
    let len = handle.disk_length_bytes()?;
    let bus = handle.query_bus_type().unwrap_or(0);
    Ok((len, bus))
}
