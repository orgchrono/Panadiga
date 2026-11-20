#![deny(unsafe_code)]

//! Deterministic offline file-backed [`BlockFlashDevice`] supporting large virtual capacities
//! via sparse 64 KiB page mapping in a temporary file.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result, bail};

use crate::constants::LBA_SECTOR_SIZE_BYTES;

use super::types::BlockFlashDevice;

const MOCK_SPARSE_PAGE_BYTES: u64 = 65_536;

struct SparseMockState {
    file: File,
    page_offsets: BTreeMap<u64, u64>,
    layout_deleted: bool,
    flush_count: usize,
}

/// Deterministic offline file-backed [`BlockFlashDevice`] supporting large virtual capacities
/// (e.g. 250 GB) via sparse 64 KiB page mapping in a real temp file without allocating 250 GB on disk.
pub struct FileBackedMockFlashDevice {
    len_bytes: u64,
    bus_type_id: u32,
    state: Mutex<SparseMockState>,
}

impl FileBackedMockFlashDevice {
    /// Creates or truncates a backing file representing a virtual disk of `len_bytes`.
    pub fn create(backing_path: &Path, len_bytes: u64, bus_type_id: u32) -> Result<Self> {
        if !len_bytes.is_multiple_of(LBA_SECTOR_SIZE_BYTES as u64) || len_bytes == 0 {
            bail!("Mock disk length {len_bytes} must be a non-zero multiple of 512 bytes");
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(backing_path)
            .with_context(|| format!("Creating mock flash file {}", backing_path.display()))?;
        Ok(Self {
            len_bytes,
            bus_type_id,
            state: Mutex::new(SparseMockState {
                file,
                page_offsets: BTreeMap::new(),
                layout_deleted: false,
                flush_count: 0,
            }),
        })
    }

    /// Returns whether `delete_drive_layout` was invoked on this mock device.
    #[must_use]
    pub fn layout_deleted(&self) -> bool {
        self.state.lock().map(|s| s.layout_deleted).unwrap_or(false)
    }

    /// Returns how many times `flush_and_update` completed.
    #[must_use]
    pub fn flush_count(&self) -> usize {
        self.state.lock().map(|s| s.flush_count).unwrap_or(0)
    }

    fn check_bounds(&self, offset: u64, len: usize) -> Result<()> {
        let sec = LBA_SECTOR_SIZE_BYTES as u64;
        if !offset.is_multiple_of(sec) || !len.is_multiple_of(LBA_SECTOR_SIZE_BYTES) {
            bail!("Unaligned mock block access: offset=0x{offset:X}, len={len}");
        }
        let end = offset
            .checked_add(len as u64)
            .context("Mock block range overflow")?;
        if end > self.len_bytes {
            bail!(
                "Mock block range 0x{offset:X}..0x{end:X} exceeds disk size 0x{:X}",
                self.len_bytes
            );
        }
        Ok(())
    }
}

impl BlockFlashDevice for FileBackedMockFlashDevice {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.check_bounds(offset, buf.len())?;
        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Mock device lock poisoned"))?;
        let mut pos = 0usize;
        while pos < buf.len() {
            let cur_off = offset + pos as u64;
            let page_idx = cur_off / MOCK_SPARSE_PAGE_BYTES;
            let in_page = cur_off % MOCK_SPARSE_PAGE_BYTES;
            let chunk = (buf.len() - pos).min((MOCK_SPARSE_PAGE_BYTES - in_page) as usize);
            if let Some(&file_page_off) = guard.page_offsets.get(&page_idx) {
                guard.file.seek(SeekFrom::Start(file_page_off + in_page))?;
                guard.file.read_exact(&mut buf[pos..pos + chunk])?;
            } else {
                buf[pos..pos + chunk].fill(0);
            }
            pos += chunk;
        }
        Ok(())
    }

    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.check_bounds(offset, data.len())?;
        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Mock device lock poisoned"))?;
        let mut pos = 0usize;
        while pos < data.len() {
            let cur_off = offset + pos as u64;
            let page_idx = cur_off / MOCK_SPARSE_PAGE_BYTES;
            let in_page = cur_off % MOCK_SPARSE_PAGE_BYTES;
            let chunk = (data.len() - pos).min((MOCK_SPARSE_PAGE_BYTES - in_page) as usize);
            let file_page_off = match guard.page_offsets.get(&page_idx).copied() {
                Some(off) => off,
                None => {
                    let end = guard.file.seek(SeekFrom::End(0))?;
                    let blank = [0u8; MOCK_SPARSE_PAGE_BYTES as usize];
                    guard.file.write_all(&blank)?;
                    guard.page_offsets.insert(page_idx, end);
                    end
                }
            };
            guard.file.seek(SeekFrom::Start(file_page_off + in_page))?;
            guard.file.write_all(&data[pos..pos + chunk])?;
            pos += chunk;
        }
        Ok(())
    }

    fn delete_drive_layout(&mut self) -> Result<()> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Mock device lock poisoned"))?;
        guard.layout_deleted = true;
        Ok(())
    }

    fn flush_and_update(&mut self) -> Result<()> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Mock device lock poisoned"))?;
        guard.file.flush()?;
        guard.flush_count += 1;
        Ok(())
    }

    fn length_bytes(&self) -> Result<u64> {
        Ok(self.len_bytes)
    }

    fn bus_type(&self) -> u32 {
        self.bus_type_id
    }
}
