//! Pure MBR, Protective GPT, Superfloppy, and FAT16 VBR/BPB decoders & validators.

use anyhow::Result;
use std::io::{Read, Seek};

use super::super::verify::{SdUnbrickCheck, check};
use super::io::read_sectors;
use super::types::{FatBpbReport, MbrPartitionRecord};

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

fn ascii_trimmed(bytes: &[u8]) -> String {
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

pub(crate) fn parse_bpb_from_vbr(vbr: &[u8]) -> FatBpbReport {
    let oem_name = ascii_trimmed(&vbr[3..11]);
    let bytes_per_sector = u16_le(vbr, 11);
    let sectors_per_cluster = vbr[13];
    let reserved_sectors = u16_le(vbr, 14);
    let num_fats = vbr[16];
    let root_entry_count = u16_le(vbr, 17);
    let total_sectors_16 = u32::from(u16_le(vbr, 19));
    let media_descriptor = vbr[21];
    let fat_size_16 = u32::from(u16_le(vbr, 22));
    let hidden_sectors = u32_le(vbr, 28);
    let total_sectors_32 = u32_le(vbr, 32);

    let total_sectors = if total_sectors_16 != 0 {
        total_sectors_16
    } else {
        total_sectors_32
    };
    let sectors_per_fat = if fat_size_16 != 0 {
        fat_size_16
    } else {
        u32_le(vbr, 36)
    };
    let (fat_type_label, volume_label) = if fat_size_16 != 0 {
        (ascii_trimmed(&vbr[54..62]), ascii_trimmed(&vbr[43..54]))
    } else {
        (ascii_trimmed(&vbr[82..90]), ascii_trimmed(&vbr[71..82]))
    };

    let root_dir_sectors = if bytes_per_sector != 0 {
        (u32::from(root_entry_count) * 32).div_ceil(u32::from(bytes_per_sector))
    } else {
        0
    };
    let data_start = u32::from(reserved_sectors)
        .saturating_add(u32::from(num_fats).saturating_mul(sectors_per_fat))
        .saturating_add(root_dir_sectors);
    let data_sectors = total_sectors.saturating_sub(data_start);
    let spc = u32::from(sectors_per_cluster.max(1));
    let data_clusters = if bytes_per_sector == 512 && sectors_per_cluster != 0 {
        data_sectors / spc
    } else {
        0
    };
    let computed_fat_kind = if bytes_per_sector != 512 || sectors_per_cluster == 0 {
        "INVALID"
    } else if data_clusters < 4085 {
        "FAT12"
    } else if data_clusters < 65525 {
        "FAT16"
    } else {
        "FAT32"
    }
    .to_string();

    FatBpbReport {
        oem_name,
        bytes_per_sector,
        sectors_per_cluster,
        cluster_size_bytes: u32::from(bytes_per_sector) * u32::from(sectors_per_cluster),
        reserved_sectors,
        num_fats,
        root_entry_count,
        total_sectors,
        media_descriptor,
        sectors_per_fat,
        hidden_sectors,
        fat_type_label,
        volume_label,
        data_clusters,
        computed_fat_kind,
    }
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

pub(crate) fn validate_fat16_bpb(
    part_lba: u64,
    vbr: &[u8],
    checks: &mut Vec<SdUnbrickCheck>,
) -> Option<(FatBpbReport, bool)> {
    if vbr[510] != 0x55 || vbr[511] != 0xAA {
        checks.push(check(
            "sd_vbr_sig",
            "fail",
            "Partition VBR missing 0x55AA signature",
            format!(
                "VBR at LBA {part_lba} ends in {:02X}{:02X}",
                vbr[510], vbr[511]
            ),
        ));
        return None;
    }

    let bpb = parse_bpb_from_vbr(vbr);
    let mut fat_ok = true;
    if bpb.bytes_per_sector != 512 {
        fat_ok = false;
        checks.push(check(
            "sd_bps",
            "fail",
            format!(
                "Unsupported BytesPerSector={} (expected 512)",
                bpb.bytes_per_sector
            ),
            "MN103S SD block driver reads 512-byte sectors.",
        ));
    }
    if bpb.computed_fat_kind != "FAT16" {
        fat_ok = false;
        checks.push(check(
            "sd_fat_kind",
            "fail",
            format!(
                "Filesystem is {} ({} data clusters; label='{}'), NOT FAT16!",
                bpb.computed_fat_kind, bpb.data_clusters, bpb.fat_type_label
            ),
            "FUN_4400c696 requires FAT16 (4085..=65524 clusters; rejects > 0xFFF5 clusters / FAT32).",
        ));
    } else if bpb.sectors_per_cluster > 64 {
        checks.push(check(
            "sd_cluster_size",
            "warn",
            format!(
                "Non-standard 64 KiB clusters ({} sectors/cluster) on FAT16",
                bpb.sectors_per_cluster
            ),
            "2006 embedded FAT16 drivers often cap at 32 KiB (64 sectors/cluster). Reformat with `/A:32K` if SD diag load failed[0] persists.",
        ));
    } else {
        checks.push(check(
            "sd_fat_kind",
            "pass",
            format!(
                "Valid FAT16 BPB: {} clusters, {} KB clusters, {} FATs, {} root entries",
                bpb.data_clusters,
                bpb.cluster_size_bytes / 1024,
                bpb.num_fats,
                bpb.root_entry_count
            ),
            format!(
                "OEM='{}', FSLabel='{}', HiddenSectors={}",
                bpb.oem_name, bpb.fat_type_label, bpb.hidden_sectors
            ),
        ));
    }

    if part_lba > 0 && u64::from(bpb.hidden_sectors) != part_lba {
        checks.push(check(
            "sd_hidden_sectors",
            "warn",
            format!(
                "BPB HiddenSectors ({}) != MBR lba_start ({part_lba})",
                bpb.hidden_sectors
            ),
            "Mismatch between MBR partition start and VBR HiddenSectors field.",
        ));
    }

    Some((bpb, fat_ok))
}
