//! Raw SD Card / Image Forensic Auditor (MBR / GPT / Superfloppy / FAT16 BPB / 8.3 SFN).
//!
//! Uses sector-aligned (512-byte) I/O so the exact same analyzer works seamlessly on:
//! - In-memory `Cursor<Vec<u8>>` images (deterministic unit tests),
//! - Raw `.img` / `.bin` disk images,
//! - Windows logical volumes (`E:` / `\\.\E:`), and
//! - Physical disks (`\\.\PhysicalDriveN`, non-destructive read-only).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use crate::constants::PANA_DIGADIAG_FRM_SIZE;

use super::verify::{SdUnbrickCheck, check, verify_digadiag_buffer};

const SECTOR_SIZE: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MbrPartitionRecord {
    pub slot: u8,
    pub boot_flag: u8,
    pub partition_type: u8,
    pub type_name: String,
    pub lba_start: u32,
    pub sector_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FatBpbReport {
    pub oem_name: String,
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub cluster_size_bytes: u32,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entry_count: u16,
    pub total_sectors: u32,
    pub media_descriptor: u8,
    pub sectors_per_fat: u32,
    pub hidden_sectors: u32,
    pub fat_type_label: String,
    pub volume_label: String,
    pub data_clusters: u32,
    pub computed_fat_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootDirFileReport {
    pub name_8_3: String,
    pub attr: u8,
    pub hidden: bool,
    pub system: bool,
    pub read_only: bool,
    pub first_cluster: u16,
    pub file_size: u32,
    pub has_preceding_lfn: bool,
    pub chain_clusters: Vec<u16>,
    pub chain_contiguous: bool,
    pub chain_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdDiskInspectionReport {
    pub target: String,
    pub partition_scheme: String,
    pub mbr_partitions: Vec<MbrPartitionRecord>,
    pub active_partition_lba: Option<u64>,
    pub bpb: Option<FatBpbReport>,
    pub digadiag_entry: Option<RootDirFileReport>,
    pub pana_sd_entry: Option<RootDirFileReport>,
    pub root_files_sample: Vec<String>,
    pub digadiag_payload_verified: bool,
    pub overall_status: String,
    pub checks: Vec<SdUnbrickCheck>,
}

fn u16_le(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

fn u32_le(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

fn ascii_trimmed(slice: &[u8]) -> String {
    let raw = String::from_utf8_lossy(slice);
    raw.trim_end_matches([' ', '\0']).to_string()
}

fn partition_type_label(ptype: u8) -> &'static str {
    match ptype {
        0x00 => "Empty",
        0x01 => "FAT12",
        0x04 => "FAT16 (<32MB)",
        0x05 => "Extended CHS",
        0x06 => "FAT16B (>=32MB)",
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

fn is_fat16_part_type(ptype: u8) -> bool {
    matches!(ptype, 0x04 | 0x06 | 0x0E)
}

fn looks_like_fat_vbr(sec: &[u8]) -> bool {
    if sec.len() < SECTOR_SIZE || sec[510] != 0x55 || sec[511] != 0xAA {
        return false;
    }
    let jump_ok = sec[0] == 0xEB && sec[2] == 0x90 || sec[0] == 0xE9;
    let bps = u16_le(sec, 11);
    let spc = sec[13];
    let fats = sec[16];
    jump_ok && bps == 512 && spc.is_power_of_two() && (1..=2).contains(&fats)
}

fn read_sectors<R: Read + Seek>(r: &mut R, lba: u64, count: usize) -> Result<Vec<u8>> {
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

fn parse_mbr_entries(sec0: &[u8]) -> Vec<MbrPartitionRecord> {
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

fn parse_bpb_from_vbr(vbr: &[u8]) -> FatBpbReport {
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

fn format_8_3_name(raw11: &[u8]) -> String {
    let stem = String::from_utf8_lossy(&raw11[0..8])
        .trim_end_matches(' ')
        .to_string();
    let ext = String::from_utf8_lossy(&raw11[8..11])
        .trim_end_matches(' ')
        .to_string();
    if ext.is_empty() {
        stem
    } else {
        format!("{stem}.{ext}")
    }
}

fn trace_fat16_chain(
    fat_bytes: &[u8],
    start_cluster: u16,
    file_size: u32,
    cluster_size: u32,
) -> (Vec<u16>, bool, bool) {
    if start_cluster < 2 || cluster_size == 0 || file_size == 0 {
        return (Vec::new(), true, file_size == 0);
    }
    let needed = file_size.div_ceil(cluster_size) as usize;
    let max_steps = needed.saturating_add(8).min(4096);
    let mut chain = Vec::new();
    let mut cur = start_cluster;
    let mut complete = false;

    for _ in 0..max_steps {
        let off = usize::from(cur) * 2;
        if off + 2 > fat_bytes.len() || chain.contains(&cur) {
            break;
        }
        chain.push(cur);
        let next = u16_le(fat_bytes, off);
        if next >= 0xFFF8 {
            complete = chain.len() >= needed;
            break;
        }
        if next < 2 || next >= 0xFFF0 {
            break;
        }
        cur = next;
    }
    let contiguous = chain.windows(2).all(|w| w[1] == w[0] + 1);
    (chain, contiguous, complete)
}

fn read_fat16_file_bytes<R: Read + Seek>(
    reader: &mut R,
    data_start_lba: u64,
    spc: u8,
    chain: &[u16],
    file_size: u32,
) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(file_size as usize);
    let remaining_total = file_size as usize;
    for &cluster in chain {
        if out.len() >= remaining_total {
            break;
        }
        let lba = data_start_lba + u64::from(cluster - 2) * u64::from(spc);
        let sec = read_sectors(reader, lba, usize::from(spc))?;
        let take = (remaining_total - out.len()).min(sec.len());
        out.extend_from_slice(&sec[..take]);
    }
    Ok(out)
}

/// Pure sector-based analyzer for an SD card, raw `.img`, or logical FAT volume.
pub fn inspect_sd_reader<R: Read + Seek>(
    target_label: &str,
    reader: &mut R,
    target_is_logical_volume: bool,
) -> Result<SdDiskInspectionReport> {
    let sec0 = read_sectors(reader, 0, 1)?;
    let mut checks = Vec::new();
    let mut mbr_partitions = Vec::new();
    let mut partition_scheme = "UNKNOWN".to_string();
    let mut active_partition_lba: Option<u64> = None;
    let mut vbr_opt: Option<Vec<u8>> = None;

    if target_is_logical_volume && looks_like_fat_vbr(&sec0) {
        partition_scheme = "LOGICAL_VOLUME".to_string();
        active_partition_lba = Some(0);
        vbr_opt = Some(sec0.clone());
        checks.push(check(
            "sd_partition_table",
            "warn",
            "Inspected mounted drive letter (MBR at LBA 0 not visible via drive letter)",
            "Reading \\\\.\\X: starts at the partition VBR. Pass \\\\.\\PhysicalDriveN (or raw disk image) to verify that LBA 0 has an MBR partition table (type 0x04/0x06/0x0E) rather than GPT or superfloppy.",
        ));
    } else if sec0[510] == 0x55 && sec0[511] == 0xAA {
        let entries = parse_mbr_entries(&sec0);
        if looks_like_fat_vbr(&sec0) && entries.iter().all(|e| e.lba_start == 0) {
            partition_scheme = "SUPERFLOPPY_NO_MBR".to_string();
            active_partition_lba = Some(0);
            vbr_opt = Some(sec0.clone());
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
            if is_fat16_part_type(primary.partition_type) {
                active_partition_lba = Some(u64::from(primary.lba_start));
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
                active_partition_lba = Some(u64::from(primary.lba_start));
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
                vbr_opt = Some(read_sectors(reader, lba, 1)?);
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

    let mut bpb_opt = None;
    let mut digadiag_entry = None;
    let mut pana_sd_entry = None;
    let mut root_files_sample = Vec::new();
    let mut digadiag_payload_verified = false;

    if let (Some(part_lba), Some(vbr)) = (active_partition_lba, vbr_opt.as_deref()) {
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
        } else {
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
            } else {
                if bpb.sectors_per_cluster > 64 {
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

            if fat_ok && bpb.root_entry_count > 0 && bpb.sectors_per_fat > 0 {
                let fat_start_lba = part_lba + u64::from(bpb.reserved_sectors);
                let fat_sectors_to_read = (bpb.sectors_per_fat as usize).min(256);
                let fat_bytes = read_sectors(reader, fat_start_lba, fat_sectors_to_read)?;
                let root_start_lba =
                    fat_start_lba + u64::from(bpb.num_fats) * u64::from(bpb.sectors_per_fat);
                let root_dir_sectors =
                    (u32::from(bpb.root_entry_count) * 32).div_ceil(512) as usize;
                let root_bytes = read_sectors(reader, root_start_lba, root_dir_sectors)?;
                let data_start_lba = root_start_lba + root_dir_sectors as u64;

                let mut prev_was_lfn = false;
                for entry in root_bytes.chunks_exact(32) {
                    if entry[0] == 0x00 {
                        break;
                    }
                    if entry[0] == 0xE5 {
                        prev_was_lfn = false;
                        continue;
                    }
                    let attr = entry[11];
                    if attr == 0x0F {
                        prev_was_lfn = true;
                        continue;
                    }
                    if (attr & 0x08) != 0 {
                        prev_was_lfn = false;
                        continue;
                    }
                    let name = format_8_3_name(&entry[0..11]);
                    if root_files_sample.len() < 16 {
                        root_files_sample.push(name.clone());
                    }
                    let first_cluster = u16_le(entry, 26);
                    let file_size = u32_le(entry, 28);
                    let upper = name.to_ascii_uppercase();
                    if upper == "DIGADIAG.FRM" || upper == "PANA__SD.FRM" {
                        let (chain, contig, complete) = trace_fat16_chain(
                            &fat_bytes,
                            first_cluster,
                            file_size,
                            bpb.cluster_size_bytes,
                        );
                        let rep = RootDirFileReport {
                            name_8_3: name.clone(),
                            attr,
                            hidden: (attr & 0x02) != 0,
                            system: (attr & 0x04) != 0,
                            read_only: (attr & 0x01) != 0,
                            first_cluster,
                            file_size,
                            has_preceding_lfn: prev_was_lfn,
                            chain_clusters: chain,
                            chain_contiguous: contig,
                            chain_complete: complete,
                        };
                        if upper == "DIGADIAG.FRM" {
                            digadiag_entry = Some(rep);
                        } else {
                            pana_sd_entry = Some(rep);
                        }
                    }
                    prev_was_lfn = false;
                }

                if let Some(ref d) = digadiag_entry {
                    if d.file_size != PANA_DIGADIAG_FRM_SIZE as u32 {
                        checks.push(check(
                            "sd_digadiag_entry",
                            "fail",
                            format!(
                                "Root DIGADIAG.FRM has wrong size {} B (expected {})",
                                d.file_size, PANA_DIGADIAG_FRM_SIZE
                            ),
                            "FUN_4400a97c requires exact 131072 B (0x20000) plaintext EXTRHEAD container.",
                        ));
                    } else if !d.chain_complete {
                        checks.push(check(
                            "sd_digadiag_entry",
                            "fail",
                            "Root DIGADIAG.FRM FAT16 cluster chain is broken or truncated",
                            format!("Clusters traversed: {:?}", d.chain_clusters),
                        ));
                    } else {
                        let payload = read_fat16_file_bytes(
                            reader,
                            data_start_lba,
                            bpb.sectors_per_cluster,
                            &d.chain_clusters,
                            d.file_size,
                        )?;
                        let (_, exact_ok, dst_hex, bgo_mode, payload_chk) =
                            verify_digadiag_buffer(Some(Ok(&payload)));
                        digadiag_payload_verified = exact_ok && payload_chk.severity == "pass";
                        if !digadiag_payload_verified {
                            checks.push(payload_chk);
                        } else {
                            checks.push(check(
                                "sd_digadiag_entry",
                                "pass",
                                format!(
                                    "Root 8.3 DIGADIAG.FRM OK: {} B @ cluster {} (dst={}, mode={}, contig={})",
                                    d.file_size,
                                    d.first_cluster,
                                    dst_hex.as_deref().unwrap_or("?"),
                                    bgo_mode.as_deref().unwrap_or("?"),
                                    d.chain_contiguous
                                ),
                                "Exact 8.3 root directory entry, FAT16 cluster chain, and EXTRHEAD container verified.",
                            ));
                        }
                    }
                } else {
                    let hint = if root_files_sample.is_empty() {
                        "Root directory is empty.".to_string()
                    } else {
                        format!(
                            "Root files found: [{}]. Ensure file is literally named 'DIGADIAG.FRM' in root.",
                            root_files_sample.join(", ")
                        )
                    };
                    checks.push(check(
                        "sd_digadiag_entry",
                        "fail",
                        "DIGADIAG.FRM missing from FAT16 root directory",
                        hint,
                    ));
                }
            }
            bpb_opt = Some(bpb);
        }
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
        partition_scheme,
        mbr_partitions,
        active_partition_lba,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PANA_DIGADIAG_DEFAULT_LOAD_ADDR;
    use crate::crypto::build_digadiag_frm;

    fn make_synthetic_mbr_fat16_image(
        part_type: u8,
        spc: u8,
        total_sectors: u32,
        include_digadiag: bool,
        superfloppy: bool,
    ) -> Vec<u8> {
        let part_start: usize = if superfloppy { 0 } else { 63 };
        let vol_sectors = total_sectors - part_start as u32;
        let reserved: u16 = 1;
        let num_fats: u8 = 2;
        let root_entries: u16 = 512;
        let fat_sectors: u16 = 128;
        let mut img = vec![0u8; (total_sectors as usize) * SECTOR_SIZE];

        if !superfloppy {
            img[510] = 0x55;
            img[511] = 0xAA;
            img[0x1BE + 4] = part_type;
            img[0x1BE + 8..0x1BE + 12].copy_from_slice(&(part_start as u32).to_le_bytes());
            img[0x1BE + 12..0x1BE + 16].copy_from_slice(&vol_sectors.to_le_bytes());
        }

        let vbr_off = part_start * SECTOR_SIZE;
        img[vbr_off..vbr_off + 3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
        img[vbr_off + 3..vbr_off + 11].copy_from_slice(b"PANADIGA");
        img[vbr_off + 11..vbr_off + 13].copy_from_slice(&512u16.to_le_bytes());
        img[vbr_off + 13] = spc;
        img[vbr_off + 14..vbr_off + 16].copy_from_slice(&reserved.to_le_bytes());
        img[vbr_off + 16] = num_fats;
        img[vbr_off + 17..vbr_off + 19].copy_from_slice(&root_entries.to_le_bytes());
        img[vbr_off + 21] = 0xF8;
        img[vbr_off + 22..vbr_off + 24].copy_from_slice(&fat_sectors.to_le_bytes());
        img[vbr_off + 28..vbr_off + 32].copy_from_slice(&(part_start as u32).to_le_bytes());
        img[vbr_off + 32..vbr_off + 36].copy_from_slice(&vol_sectors.to_le_bytes());
        img[vbr_off + 43..vbr_off + 54].copy_from_slice(b"EH55SD     ");
        img[vbr_off + 54..vbr_off + 62].copy_from_slice(b"FAT16   ");
        img[vbr_off + 510] = 0x55;
        img[vbr_off + 511] = 0xAA;

        let fat1_off = (part_start + usize::from(reserved)) * SECTOR_SIZE;
        let root_off =
            (part_start + usize::from(reserved) + usize::from(num_fats) * usize::from(fat_sectors))
                * SECTOR_SIZE;
        let data_start_lba = part_start
            + usize::from(reserved)
            + usize::from(num_fats) * usize::from(fat_sectors)
            + 32;

        if include_digadiag {
            // Minimal valid compressed PROG061-like payload wrapped into 128 KiB plaintext DIGADIAG.FRM.
            let prog = vec![0xCB; 0x4000];
            let (diag, _) = build_digadiag_frm(
                &prog,
                PANA_DIGADIAG_DEFAULT_LOAD_ADDR,
                "FVU ",
                "TESTSD",
                4,
                PANA_DIGADIAG_FRM_SIZE,
            )
            .unwrap();

            let cluster_bytes = usize::from(spc) * SECTOR_SIZE;
            let clusters_needed = diag.len().div_ceil(cluster_bytes);
            for i in 0..clusters_needed {
                let clus = (2 + i) as u16;
                let next = if i + 1 == clusters_needed {
                    0xFFFFu16
                } else {
                    clus + 1
                };
                let f_off = fat1_off + usize::from(clus) * 2;
                img[f_off..f_off + 2].copy_from_slice(&next.to_le_bytes());
            }
            img[root_off..root_off + 11].copy_from_slice(b"DIGADIAGFRM");
            img[root_off + 11] = 0x20;
            img[root_off + 26..root_off + 28].copy_from_slice(&2u16.to_le_bytes());
            img[root_off + 28..root_off + 32].copy_from_slice(&(diag.len() as u32).to_le_bytes());

            let dst_off = data_start_lba * SECTOR_SIZE;
            img[dst_off..dst_off + diag.len()].copy_from_slice(&diag);
        }

        img
    }

    #[test]
    fn valid_mbr_fat16_with_digadiag_passes_full_audit() {
        // ~32 MB image, spc=4 -> ~16,000 clusters (valid FAT16)
        let img = make_synthetic_mbr_fat16_image(0x06, 4, 65_536, true, false);
        let rep = inspect_sd_disk_bytes("mem:ok.img", &img).unwrap();
        assert_eq!(rep.partition_scheme, "MBR");
        assert_eq!(rep.overall_status, "PASS", "checks: {:?}", rep.checks);
        assert!(rep.digadiag_payload_verified);
        assert_eq!(rep.bpb.as_ref().unwrap().computed_fat_kind, "FAT16");
    }

    #[test]
    fn detects_superfloppy_and_gpt_failures() {
        let sf = make_synthetic_mbr_fat16_image(0x06, 4, 65_536, false, true);
        let rep_sf = inspect_sd_disk_bytes("mem:sf.img", &sf).unwrap();
        assert_eq!(rep_sf.partition_scheme, "SUPERFLOPPY_NO_MBR");
        assert_eq!(rep_sf.overall_status, "FAIL");

        let mut gpt = vec![0u8; 4096];
        gpt[510] = 0x55;
        gpt[511] = 0xAA;
        gpt[0x1BE + 4] = 0xEE;
        gpt[0x1BE + 8..0x1BE + 12].copy_from_slice(&1u32.to_le_bytes());
        gpt[0x1BE + 12..0x1BE + 16].copy_from_slice(&1000u32.to_le_bytes());
        let rep_gpt = inspect_sd_disk_bytes("mem:gpt.img", &gpt).unwrap();
        assert_eq!(rep_gpt.partition_scheme, "GPT");
        assert_eq!(rep_gpt.overall_status, "FAIL");
    }
}
