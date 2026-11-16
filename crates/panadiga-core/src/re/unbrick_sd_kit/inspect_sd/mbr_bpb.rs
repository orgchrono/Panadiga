//! Pure MBR, Protective GPT, and Superfloppy partition table decoders & validators.

use anyhow::Result;
use std::io::{Read, Seek};

use super::super::verify::{SdUnbrickCheck, check};
pub(crate) use super::bpb::validate_fat16_bpb;
use super::io::read_sectors;
use super::types::MbrPartitionRecord;

pub(crate) const SECTOR_SIZE: usize = 512;

pub(crate) struct PartitionResolution {
    pub partition_scheme: String,
    pub mbr_partitions: Vec<MbrPartitionRecord>,
    pub active_partition_lba: Option<u64>,
    pub vbr: Option<Vec<u8>>,
}

pub(crate) fn u16_le(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

pub(crate) fn u32_le(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

pub(crate) fn ascii_trimmed(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches([' ', '\0'])
        .to_string()
}

pub(crate) fn partition_type_label(ptype: u8) -> &'static str {
    match ptype {
        0x00 => "Empty",
        0x01 => "FAT12",
        0x04 => "FAT16 <32MB",
        0x05 => "Extended",
        0x06 => "FAT16B",
        0x07 => "NTFS/exFAT",
        0x0B => "FAT32 CHS",
        0x0C => "FAT32 LBA",
        0x0E => "FAT16 LBA",
        0x0F => "Extended LBA",
        0xEE => "GPT Protective MBR",
        0xEF => "EFI System",
        _ => "Other",
    }
}

pub(crate) fn is_fat16_part_type(ptype: u8) -> bool {
    matches!(ptype, 0x04 | 0x06 | 0x0E)
}

pub(crate) fn looks_like_fat_vbr(sec: &[u8]) -> bool {
    if sec.len() < SECTOR_SIZE || sec[510] != 0x55 || sec[511] != 0xAA {
        return false;
    }
    let jump_ok = (sec[0] == 0xEB && sec[2] == 0x90) || sec[0] == 0xE9;
    let bps = u16_le(sec, 11);
    let spc = sec[13];
    let fats = sec[16];
    jump_ok && bps == 512 && spc.is_power_of_two() && (1..=2).contains(&fats)
}

pub(crate) fn parse_mbr_entries(sec0: &[u8]) -> Vec<MbrPartitionRecord> {
    let mut out = Vec::new();
    for slot in 0..4u8 {
        let off = 0x1BE + (usize::from(slot) * 16);
        let part_type = sec0[off + 4];
        let lba_start = u32_le(sec0, off + 8);
        let sector_count = u32_le(sec0, off + 12);
        if part_type != 0 || lba_start != 0 || sector_count != 0 {
            out.push(MbrPartitionRecord {
                slot,
                boot_flag: sec0[off],
                partition_type: part_type,
                type_name: partition_type_label(part_type).to_string(),
                lba_start,
                sector_count,
            });
        }
    }
    out
}

pub(crate) fn resolve_partition_layout<R: Read + Seek>(
    reader: &mut R,
    sec0: &[u8],
    target_is_logical_volume: bool,
    checks: &mut Vec<SdUnbrickCheck>,
) -> Result<PartitionResolution> {
    let mut mbr_partitions = Vec::new();
    let mut partition_scheme = "UNKNOWN".to_string();
    let mut active_partition_lba: Option<u64> = None;
    let mut vbr: Option<Vec<u8>> = None;

    if target_is_logical_volume && looks_like_fat_vbr(sec0) {
        partition_scheme = "LOGICAL_VOLUME".to_string();
        active_partition_lba = Some(0);
        vbr = Some(sec0.to_vec());
        checks.push(check(
            "sd_partition_table",
            "warn",
            "Inspected mounted drive letter (MBR at LBA 0 not visible via drive letter)",
            "Reading \\\\.\\X: starts at the partition VBR. Pass \\\\.\\PhysicalDriveN (or raw disk image) to verify that LBA 0 has an MBR partition table (type 0x04/0x06/0x0E) rather than GPT or superfloppy.",
        ));
    } else if sec0[510] == 0x55 && sec0[511] == 0xAA {
        let entries = parse_mbr_entries(sec0);
        if looks_like_fat_vbr(sec0) && entries.iter().all(|e| e.lba_start == 0) {
            partition_scheme = "SUPERFLOPPY_NO_MBR".to_string();
            active_partition_lba = Some(0);
            vbr = Some(sec0.to_vec());
            checks.push(check(
                "sd_partition_table",
                "fail",
                "Superfloppy layout detected at LBA 0 (NO MBR partition table!)",
                "Sector 0 contains a FAT VBR directly without an MBR partition table. Panasonic DMR-EH55 /BOOT.BIN embedded FAT mounters (FUN_4400c696) commonly require a valid MBR partition entry (0x04/0x06/0x0E) — clean and convert to MBR with diskpart (`clean` -> `convert mbr` -> `create partition primary`).",
            ));
        } else if entries.iter().any(|e| e.partition_type == 0xEE) {
            partition_scheme = "GPT".to_string();
            mbr_partitions = entries;
            checks.push(check(
                "sd_partition_table",
                "fail",
                "GPT Protective MBR (0xEE) detected",
                "Card uses GPT instead of legacy MBR. /BOOT.BIN cannot mount GPT — run `diskpart` -> `clean` -> `convert mbr` -> `create partition primary` -> `format fs=fat quick`.",
            ));
        } else if entries.is_empty() {
            partition_scheme = "EMPTY_MBR".to_string();
            checks.push(check(
                "sd_partition_table",
                "fail",
                "MBR has 0x55AA signature but all 4 partition entries are empty",
                "Create a single primary MBR FAT16 partition before formatting.",
            ));
        } else {
            partition_scheme = "MBR".to_string();
            mbr_partitions = entries;
            let primary = &mbr_partitions[0];
            if mbr_partitions.len() > 1 {
                checks.push(check(
                    "sd_multi_partition",
                    "warn",
                    format!("{} MBR partitions present", mbr_partitions.len()),
                    "Embedded SD bootloaders only scan partition slot 0; keep only a single FAT16 primary partition.",
                ));
            }
            active_partition_lba = Some(u64::from(primary.lba_start));
            if is_fat16_part_type(primary.partition_type) {
                checks.push(check(
                    "sd_partition_table",
                    "pass",
                    format!(
                        "Valid MBR Partition #0: type=0x{:02X} ({}) start LBA={} sectors={}",
                        primary.partition_type,
                        primary.type_name,
                        primary.lba_start,
                        primary.sector_count
                    ),
                    "MBR layout compatible with Panasonic embedded FAT16 loader.",
                ));
            } else {
                checks.push(check(
                    "sd_partition_table",
                    "fail",
                    format!(
                        "Unsupported MBR partition type 0x{:02X} ({})",
                        primary.partition_type, primary.type_name
                    ),
                    "Expected FAT16 MBR type 0x04, 0x06, or 0x0E; FAT32 (0x0B/0x0C), exFAT/NTFS (0x07), and FAT12 (0x01) typically fail FUN_4400c696.",
                ));
            }
            if let Some(lba) = active_partition_lba {
                vbr = Some(read_sectors(reader, lba, 1)?);
            }
        }
    } else {
        checks.push(check(
            "sd_partition_table",
            "fail",
            format!(
                "Invalid LBA 0 boot signature (0x{:02X}{:02X} != 0x55AA)",
                sec0[510], sec0[511]
            ),
            "Sector 0 is neither a valid MBR nor a FAT VBR.",
        ));
    }

    Ok(PartitionResolution {
        partition_scheme,
        mbr_partitions,
        active_partition_lba,
        vbr,
    })
}
