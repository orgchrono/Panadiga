#![deny(unsafe_code)]

//! Mandatory 64 KiB pre-write CoW backup, image header validation, chunked
//! writing, and full read-back SHA-256 verification helpers for EH55 flashing.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::constants::{
    LBA_MIRROR_MBR_OFFSET, LBA_MIRROR_MBR_SECTOR, LBA_PARTITION_1_ALIGNED_SIZE,
    LBA_PARTITION_1_OFFSET, LBA_SECTOR_SIZE_BYTES, PANASONIC_HDD_MBR_SIGNATURE,
    STANDARD_MBR_BOOT_SIGNATURE,
};
use crate::nvram::calculate_hashes;

use super::types::{
    BlockFlashDevice, EH55_BACKUP_SECTOR_COUNT, EH55_FLASH_CHUNK_BYTES, Eh55FlashProgress,
    Eh55FlashStage, Eh55HddFlashRequest,
};

pub(super) fn validate_eh55_primary_image(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(LBA_SECTOR_SIZE_BYTES) {
        bail!(
            "Primary EH55 image must be a non-empty multiple of {LBA_SECTOR_SIZE_BYTES} B (got {} B)",
            bytes.len()
        );
    }
    if bytes[..8] != PANASONIC_HDD_MBR_SIGNATURE {
        bail!(
            "Primary EH55 image LBA 0 lacks Panasonic 'SH' signature (got {})",
            hex::encode(&bytes[..8])
        );
    }
    if bytes[510..512] == STANDARD_MBR_BOOT_SIGNATURE {
        bail!("Primary EH55 image contains PC MBR 0x55AA signature at LBA 0[510..512]");
    }
    Ok(())
}

pub(super) fn write_chunked_with_progress<D, F>(
    device: &mut D,
    start_offset: u64,
    data: &[u8],
    stage: Eh55FlashStage,
    label: &str,
    on_progress: &mut F,
) -> Result<()>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(Eh55FlashProgress),
{
    let total = data.len() as u64;
    let mut pos = 0usize;
    while pos < data.len() {
        let end = (pos + EH55_FLASH_CHUNK_BYTES).min(data.len());
        device.write_at(start_offset + pos as u64, &data[pos..end])?;
        pos = end;
        on_progress(Eh55FlashProgress {
            stage,
            bytes_done: pos as u64,
            bytes_total: total,
            detail: format!("{label}: {pos} / {total} bytes written"),
        });
    }
    Ok(())
}

pub(super) fn verify_region_readback_sha256<D, F>(
    device: &D,
    start_offset: u64,
    expected_len: usize,
    expected_sha256: &str,
    label: &str,
    on_progress: &mut F,
) -> Result<Vec<u8>>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(Eh55FlashProgress),
{
    let mut readback = vec![0u8; expected_len];
    let mut pos = 0usize;
    let total = expected_len as u64;
    while pos < expected_len {
        let end = (pos + EH55_FLASH_CHUNK_BYTES).min(expected_len);
        device.read_at(start_offset + pos as u64, &mut readback[pos..end])?;
        pos = end;
        on_progress(Eh55FlashProgress {
            stage: Eh55FlashStage::VerifySha256,
            bytes_done: pos as u64,
            bytes_total: total,
            detail: format!("Read-back {label}: {pos} / {total} bytes"),
        });
    }
    let (_, actual_sha256) = calculate_hashes(&readback);
    if actual_sha256 != expected_sha256 {
        bail!(
            "Read-back SHA-256 mismatch on {label} at offset 0x{start_offset:X}: expected {expected_sha256}, got {actual_sha256}"
        );
    }
    Ok(readback)
}

pub(super) fn write_mandatory_64k_cow_backup<D>(
    device: &D,
    backup_dir: &Path,
) -> Result<(PathBuf, PathBuf, String)>
where
    D: BlockFlashDevice + ?Sized,
{
    if !backup_dir.is_dir() {
        bail!(
            "Mandatory backup directory does not exist: {}",
            backup_dir.display()
        );
    }
    let half_bytes = EH55_BACKUP_SECTOR_COUNT * LBA_SECTOR_SIZE_BYTES;
    let mut backup = vec![0u8; half_bytes * 2];
    device.read_at(0, &mut backup[..half_bytes])?;
    device.read_at(LBA_MIRROR_MBR_OFFSET, &mut backup[half_bytes..])?;

    let (_, sha256) = calculate_hashes(&backup);
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let base_name = format!("eh55_pre_flash_backup_{ts_ms}_{}", std::process::id());
    let bin_name = format!("{base_name}.bin");
    let bin_path = backup_dir.join(&bin_name);
    let manifest_path = backup_dir.join(format!("{base_name}.sha256"));

    {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&bin_path)
            .with_context(|| format!("Creating CoW backup {}", bin_path.display()))?;
        f.write_all(&backup)?;
        f.flush()?;
    }
    let manifest_text = format!(
        "{sha256}  {bin_name}\n# LBA 0..63 (32 KiB) + LBA {}..{} (32 KiB)\n",
        LBA_MIRROR_MBR_SECTOR,
        LBA_MIRROR_MBR_SECTOR + EH55_BACKUP_SECTOR_COUNT as u64 - 1
    );
    fs::write(&manifest_path, manifest_text).with_context(|| {
        format!(
            "Writing backup SHA-256 manifest {}",
            manifest_path.display()
        )
    })?;

    Ok((bin_path, manifest_path, sha256))
}

pub(super) fn resolve_mirror_bytes(
    req: &Eh55HddFlashRequest,
    primary_bytes: &[u8],
) -> Result<Option<Vec<u8>>> {
    if let Some(ref p1_path) = req.mirror_p1_path {
        let bytes = fs::read(p1_path)
            .with_context(|| format!("Reading P1 MAIN mirror file {}", p1_path.display()))?;
        if bytes.is_empty() || !bytes.len().is_multiple_of(LBA_SECTOR_SIZE_BYTES) {
            bail!(
                "Mirror P1 file '{}' size ({} B) must be a non-empty multiple of {LBA_SECTOR_SIZE_BYTES} B",
                p1_path.display(),
                bytes.len()
            );
        }
        return Ok(Some(bytes));
    }
    if req.extract_p1_mirror_from_image {
        let p1_start = LBA_PARTITION_1_OFFSET as usize;
        let p1_end = p1_start + LBA_PARTITION_1_ALIGNED_SIZE;
        let Some(slice) = primary_bytes.get(p1_start..p1_end) else {
            bail!(
                "Primary image ({} B) too small to slice P1 MAIN ({} B @ 0x{:X})",
                primary_bytes.len(),
                LBA_PARTITION_1_ALIGNED_SIZE,
                LBA_PARTITION_1_OFFSET
            );
        };
        return Ok(Some(slice.to_vec()));
    }
    Ok(None)
}
