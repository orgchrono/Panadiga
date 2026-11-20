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
pub use stub::{OwnedDiskHandle, is_current_user_admin, probe_disk_ioctl_metadata};
#[cfg(windows)]
pub use win_impl::{OwnedDiskHandle, is_current_user_admin, probe_disk_ioctl_metadata};
