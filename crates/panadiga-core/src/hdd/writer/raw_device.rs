#![deny(unsafe_code)]

//! Cross-platform sector-aligned block flash device abstraction, sparse mock target,
//! chunked transfer helpers, and full read-back SHA-256 verification.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, Result, bail};

use crate::constants::LBA_SECTOR_SIZE_BYTES;
use crate::nvram::calculate_hashes;

use super::win_ioctl::{OwnedDiskHandle, is_current_user_admin};

/// Windows `STORAGE_BUS_TYPE::BusTypeNvme` (`17`), unconditionally rejected for EH55 IDE/PATA & SD flashing.
pub const BUS_TYPE_NVME: u32 = 17;
const MOCK_SPARSE_PAGE_BYTES: u64 = 65_536;

/// Abstract sector-aligned block flash target implemented for real physical drives and offline file mocks.
pub trait BlockFlashDevice {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<()>;
    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()>;
    fn delete_drive_layout(&mut self) -> Result<()>;
    fn flush_and_update(&mut self) -> Result<()>;
    fn length_bytes(&self) -> Result<u64>;
    fn bus_type(&self) -> u32;
}

/// Safe cross-platform physical block device adapter delegating to [`OwnedDiskHandle`].
#[derive(Debug)]
pub struct PhysicalFlashDevice {
    handle: OwnedDiskHandle,
    cached_len_bytes: u64,
    cached_bus_type: u32,
}

/// Windows physical block device alias preserved for existing `eh55_flash` callers.
pub type WindowsPhysicalFlashDevice = PhysicalFlashDevice;
/// POSIX (Linux/macOS) physical block device alias sharing the same safe [`OwnedDiskHandle`].
pub type UnixPhysicalFlashDevice = PhysicalFlashDevice;

impl PhysicalFlashDevice {
    /// Opens `device_path` (never OS boot disks), enforcing Administrator/root privilege when `writable == true`.
    pub fn open(device_path: &str, writable: bool) -> Result<Self> {
        if writable && !is_current_user_admin() {
            bail!(
                "Administrator/root privileges required to open physical disk '{device_path}' for writing"
            );
        }
        let handle = OwnedDiskHandle::open(device_path, writable)?;
        let cached_len_bytes = handle.disk_length_bytes()?;
        let cached_bus_type = handle.query_bus_type().unwrap_or(0);
        Ok(Self {
            handle,
            cached_len_bytes,
            cached_bus_type,
        })
    }
}

/// Opens a verified physical block device target via [`PhysicalFlashDevice::open`].
pub fn open_physical_flash_device(
    device_path: &str,
    writable: bool,
) -> Result<PhysicalFlashDevice> {
    PhysicalFlashDevice::open(device_path, writable)
}

impl BlockFlashDevice for PhysicalFlashDevice {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.handle.read_at(offset, buf)
    }
    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.handle.write_at(offset, data)
    }
    fn delete_drive_layout(&mut self) -> Result<()> {
        self.handle.delete_drive_layout()
    }
    fn flush_and_update(&mut self) -> Result<()> {
        self.handle.flush()?;
        self.handle.update_properties()
    }
    fn length_bytes(&self) -> Result<u64> {
        Ok(self.cached_len_bytes)
    }
    fn bus_type(&self) -> u32 {
        self.cached_bus_type
    }
}

/// Maps human-readable OS bus strings (`nvme`, `usb`, `sata`, `ata`, `scsi`, `mmc`, `sd`) to Windows IDs.
#[must_use]
pub(crate) fn infer_bus_type_id(bus_type: &str) -> u32 {
    let low = bus_type.trim().to_ascii_lowercase();
    if low.contains("nvme") {
        BUS_TYPE_NVME
    } else if low.contains("usb") {
        7
    } else if low.contains("sata") {
        11
    } else if low.contains("ata") || low.contains("ide") {
        3
    } else if low.contains("scsi") {
        1
    } else if low.contains("raid") {
        8
    } else if low.contains("mmc") || low == "sd" {
        12
    } else {
        0
    }
}

struct SparseMockState {
    file: File,
    page_offsets: BTreeMap<u64, u64>,
    layout_deleted: bool,
    flush_count: usize,
}

/// Deterministic offline file-backed [`BlockFlashDevice`] supporting large virtual capacities via sparse 64 KiB pages.
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

    fn lock_state(&self) -> Result<MutexGuard<'_, SparseMockState>> {
        self.state
            .lock()
            .map_err(|_| anyhow::anyhow!("Mock device lock poisoned"))
    }

    #[must_use]
    pub fn layout_deleted(&self) -> bool {
        self.state.lock().map(|s| s.layout_deleted).unwrap_or(false)
    }

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
        let mut guard = self.lock_state()?;
        let mut pos = 0usize;
        while pos < buf.len() {
            let cur_off = offset + pos as u64;
            let (page_idx, in_page) = (
                cur_off / MOCK_SPARSE_PAGE_BYTES,
                cur_off % MOCK_SPARSE_PAGE_BYTES,
            );
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
        let mut guard = self.lock_state()?;
        let mut pos = 0usize;
        while pos < data.len() {
            let cur_off = offset + pos as u64;
            let (page_idx, in_page) = (
                cur_off / MOCK_SPARSE_PAGE_BYTES,
                cur_off % MOCK_SPARSE_PAGE_BYTES,
            );
            let chunk = (data.len() - pos).min((MOCK_SPARSE_PAGE_BYTES - in_page) as usize);
            let file_page_off = match guard.page_offsets.get(&page_idx).copied() {
                Some(off) => off,
                None => {
                    let end = guard.file.seek(SeekFrom::End(0))?;
                    guard
                        .file
                        .write_all(&[0u8; MOCK_SPARSE_PAGE_BYTES as usize])?;
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
        self.lock_state()?.layout_deleted = true;
        Ok(())
    }

    fn flush_and_update(&mut self) -> Result<()> {
        let mut guard = self.lock_state()?;
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

/// Writes `data` onto `device` starting at `start_offset` in `chunk_bytes` steps, invoking `on_chunk(done, total)`.
pub fn write_chunked_bytes<D: BlockFlashDevice + ?Sized, F: FnMut(usize, u64)>(
    device: &mut D,
    start_offset: u64,
    data: &[u8],
    chunk_bytes: usize,
    mut on_chunk: F,
) -> Result<()> {
    let (step, total, mut pos) = (
        chunk_bytes.max(LBA_SECTOR_SIZE_BYTES),
        data.len() as u64,
        0usize,
    );
    while pos < data.len() {
        let end = (pos + step).min(data.len());
        device.write_at(start_offset + pos as u64, &data[pos..end])?;
        pos = end;
        on_chunk(pos, total);
    }
    Ok(())
}

/// Reads back `expected_len` bytes from `device` starting at `start_offset` and verifies SHA-256 against `expected_sha256`.
pub fn readback_sha256_verify<D: BlockFlashDevice + ?Sized, F: FnMut(usize, u64)>(
    device: &D,
    start_offset: u64,
    expected_len: usize,
    expected_sha256: &str,
    label: &str,
    chunk_bytes: usize,
    mut on_chunk: F,
) -> Result<Vec<u8>> {
    let (step, total, mut pos) = (
        chunk_bytes.max(LBA_SECTOR_SIZE_BYTES),
        expected_len as u64,
        0usize,
    );
    let mut readback = vec![0u8; expected_len];
    while pos < expected_len {
        let end = (pos + step).min(expected_len);
        device.read_at(start_offset + pos as u64, &mut readback[pos..end])?;
        pos = end;
        on_chunk(pos, total);
    }
    let (_, actual_sha256) = calculate_hashes(&readback);
    if actual_sha256 != expected_sha256 {
        bail!(
            "Read-back SHA-256 mismatch on {label} at offset 0x{start_offset:X}: expected {expected_sha256}, got {actual_sha256}"
        );
    }
    Ok(readback)
}
