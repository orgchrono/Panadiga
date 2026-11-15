//! Sector-aligned 512-byte I/O orchestration for raw `.img` files, logical drives, and `PhysicalDriveN`.

use anyhow::{Context, Result, bail};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use super::fat16::inspect_fat16_root_and_digadiag;
use super::mbr_bpb::{SECTOR_SIZE, resolve_partition_layout, validate_fat16_bpb};
use super::types::SdDiskInspectionReport;

pub(crate) fn read_sectors<R: Read + Seek>(r: &mut R, lba: u64, count: usize) -> Result<Vec<u8>> {
    let offset = lba
        .checked_mul(SECTOR_SIZE as u64)
        .context("LBA overflow")?;
    r.seek(SeekFrom::Start(offset))
        .with_context(|| format!("Seek to LBA {lba} (byte 0x{offset:X}) failed"))?;
    let mut buf = vec![0u8; count * SECTOR_SIZE];
    r.read_exact(&mut buf)
        .with_context(|| format!("Read {count} sector(s) at LBA {lba} failed"))?;
    Ok(buf)
}

/// Pure sector-based analyzer for an SD card, raw `.img`, or logical FAT volume.
pub fn inspect_sd_reader<R: Read + Seek>(
    target_label: &str,
    reader: &mut R,
    target_is_logical_volume: bool,
) -> Result<SdDiskInspectionReport> {
    let sec0 = read_sectors(reader, 0, 1)?;
    let mut checks = Vec::new();
    let res = resolve_partition_layout(reader, &sec0, target_is_logical_volume, &mut checks)?;

    let mut bpb_opt = None;
    let mut digadiag_entry = None;
    let mut pana_sd_entry = None;
    let mut root_files_sample = Vec::new();
    let mut digadiag_payload_verified = false;

    if let (Some(part_lba), Some(vbr)) = (res.active_partition_lba, res.vbr.as_deref())
        && let Some((bpb, fat_ok)) = validate_fat16_bpb(part_lba, vbr, &mut checks)
    {
        if fat_ok && bpb.root_entry_count > 0 && bpb.sectors_per_fat > 0 {
            let root = inspect_fat16_root_and_digadiag(reader, part_lba, &bpb, &mut checks)?;
            digadiag_entry = root.digadiag_entry;
            pana_sd_entry = root.pana_sd_entry;
            root_files_sample = root.root_files_sample;
            digadiag_payload_verified = root.digadiag_payload_verified;
        }
        bpb_opt = Some(bpb);
    }

    let overall_status = if checks.iter().any(|c| c.severity == "fail") {
        "FAIL"
    } else if checks.iter().any(|c| c.severity == "warn") {
        "WARN"
    } else {
        "PASS"
    }
    .to_string();

    Ok(SdDiskInspectionReport {
        target: target_label.to_string(),
        partition_scheme: res.partition_scheme,
        mbr_partitions: res.mbr_partitions,
        active_partition_lba: res.active_partition_lba,
        bpb: bpb_opt,
        digadiag_entry,
        pana_sd_entry,
        root_files_sample,
        digadiag_payload_verified,
        overall_status,
        checks,
    })
}

/// Inspect raw in-memory disk image bytes directly (deterministic unit test entrypoint).
pub fn inspect_sd_disk_bytes(label: &str, bytes: &[u8]) -> Result<SdDiskInspectionReport> {
    let mut cursor = Cursor::new(bytes);
    inspect_sd_reader(label, &mut cursor, false)
}

fn normalize_sd_target(target: &str) -> (String, bool) {
    let trimmed = target.trim();
    if trimmed.starts_with(r"\\.\") {
        let lower = trimmed.to_ascii_lowercase();
        let is_logical = lower.len() == 6 && lower.ends_with(':');
        return (trimmed.to_string(), is_logical);
    }
    let clean = trimmed.trim_end_matches(['\\', '/']);
    if clean.len() == 2 && clean.as_bytes()[1] == b':' && clean.as_bytes()[0].is_ascii_alphabetic()
    {
        return (format!(r"\\.\{}", clean.to_ascii_uppercase()), true);
    }
    if trimmed.to_ascii_lowercase().starts_with("physicaldrive") {
        return (format!(r"\\.\{trimmed}"), false);
    }
    (trimmed.to_string(), false)
}

/// Open an SD raw `.img` file, logical drive (`E:` / `\\.\E:`), or `\\.\PhysicalDriveN` read-only and audit MBR/BPB/FAT16/8.3.
pub fn inspect_sd_target(target: &str) -> Result<SdDiskInspectionReport> {
    let (open_path, is_logical) = normalize_sd_target(target);
    let path = Path::new(&open_path);
    let mut file = File::open(path).with_context(|| {
        format!(
            "Failed to open SD target '{}' read-only (on Windows, \\\\.\\PhysicalDriveN may require Administrator privileges)",
            path.display()
        )
    })?;
    if !is_logical
        && !open_path.starts_with(r"\\.\")
        && file.metadata().map(|m| m.is_dir()).unwrap_or(false)
    {
        bail!(
            "'{}' is a directory; use `unbrick verify-sd` for folders or pass a drive letter (`E:`), `\\\\.\\PhysicalDriveN`, or raw `.img` file.",
            target
        );
    }
    inspect_sd_reader(target, &mut file, is_logical)
}
