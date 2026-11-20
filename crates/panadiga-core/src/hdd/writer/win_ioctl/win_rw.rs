//! Sector-aligned Win32 raw disk seek, read, write, and buffer flush operations.

use std::ptr::null_mut;

use anyhow::{Context, Result, bail};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_BEGIN, FlushFileBuffers, ReadFile, SetFilePointerEx, WriteFile,
};

use crate::constants::LBA_SECTOR_SIZE_BYTES;

use super::win_impl::OwnedDiskHandle;

const IO_CHUNK_BYTES: usize = 1024 * 1024;

fn validate_sector_aligned(offset: u64, len: usize) -> Result<()> {
    let sec = LBA_SECTOR_SIZE_BYTES as u64;
    if !offset.is_multiple_of(sec) || !len.is_multiple_of(LBA_SECTOR_SIZE_BYTES) {
        bail!(
            "Unaligned raw disk I/O rejected (offset=0x{offset:X}, len={len}; must be {LBA_SECTOR_SIZE_BYTES}-byte aligned)"
        );
    }
    Ok(())
}

impl OwnedDiskHandle {
    /// Flushes pending host buffers via `FlushFileBuffers`.
    pub fn flush(&mut self) -> Result<()> {
        // SAFETY: `self.handle` is an open file/disk handle.
        let ok = unsafe { FlushFileBuffers(self.handle) };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            bail!("FlushFileBuffers on '{}' failed: {err}", self.device_path);
        }
        Ok(())
    }

    fn seek_abs(&self, offset: u64) -> Result<()> {
        let off_i64 = i64::try_from(offset)
            .with_context(|| format!("Seek offset 0x{offset:X} overflows i64"))?;
        let mut new_pos: i64 = 0;
        // SAFETY: `self.handle` is valid and `new_pos` is a valid mutable stack `i64`.
        let ok = unsafe { SetFilePointerEx(self.handle, off_i64, &raw mut new_pos, FILE_BEGIN) };
        if ok == 0 || new_pos != off_i64 {
            let err = std::io::Error::last_os_error();
            bail!(
                "SetFilePointerEx(0x{offset:X}) on '{}' failed: {err}",
                self.device_path
            );
        }
        Ok(())
    }

    /// Reads sector-aligned bytes at `offset` in bounded chunks.
    pub fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<()> {
        validate_sector_aligned(offset, buf.len())?;
        self.seek_abs(offset)?;
        let mut pos = 0usize;
        while pos < buf.len() {
            let want = (buf.len() - pos).min(IO_CHUNK_BYTES);
            let mut got: u32 = 0;
            // SAFETY: `self.handle` is valid and `&mut buf[pos..pos + want]` is exclusively
            // borrowed for `want` bytes.
            let ok = unsafe {
                ReadFile(
                    self.handle,
                    buf[pos..].as_mut_ptr(),
                    want as u32,
                    &raw mut got,
                    null_mut(),
                )
            };
            if ok == 0 {
                let err = std::io::Error::last_os_error();
                bail!(
                    "ReadFile({}, offset=0x{:X}, len={want}) failed: {err}",
                    self.device_path,
                    offset + pos as u64
                );
            }
            if got == 0 {
                bail!(
                    "Unexpected EOF reading '{}' at offset 0x{:X}",
                    self.device_path,
                    offset + pos as u64
                );
            }
            pos += got as usize;
        }
        Ok(())
    }

    /// Writes sector-aligned bytes at `offset` in bounded chunks.
    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        if !self.writable {
            bail!("Cannot write to read-only handle '{}'", self.device_path);
        }
        validate_sector_aligned(offset, data.len())?;
        self.seek_abs(offset)?;
        let mut pos = 0usize;
        while pos < data.len() {
            let want = (data.len() - pos).min(IO_CHUNK_BYTES);
            let mut wrote: u32 = 0;
            // SAFETY: `self.handle` is valid and `&data[pos..pos + want]` is valid for `want` bytes.
            let ok = unsafe {
                WriteFile(
                    self.handle,
                    data[pos..].as_ptr(),
                    want as u32,
                    &raw mut wrote,
                    null_mut(),
                )
            };
            if ok == 0 || wrote as usize != want {
                let err = std::io::Error::last_os_error();
                bail!(
                    "WriteFile({}, offset=0x{:X}, len={want}, wrote={wrote}) failed: {err}",
                    self.device_path,
                    offset + pos as u64
                );
            }
            pos += wrote as usize;
        }
        Ok(())
    }
}
