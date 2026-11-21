#![deny(unsafe_code)]

//! FAT16 8.3 root directory scanner, cluster chain tracer, and `DIGADIAG.FRM`/`PANA__SD.FRM` verifier.

use anyhow::Result;
use std::io::{Read, Seek};

use crate::constants::{PANA_DIGADIAG_FRM_SIZE, PANA_DIGADIAG_RAW_VER061_SIZE};

use super::super::verify::{SdUnbrickCheck, check, verify_digadiag_buffer, verify_pana_sd_buffer};
use super::io::read_sectors;
use super::mbr_bpb::u16_le;
use super::mbr_bpb::u32_le;
use super::types::{FatBpbReport, RootDirFileReport};

const OS_JUNK_PREFIXES: [&str; 8] = [
    "SYSTEM~1", "._", ".TRASH", "TRASH", "SPOTLI", ".SPOTL", "$RECYC", "FSEVEN",
];

fn is_os_junk_entry(upper_name: &str) -> bool {
    OS_JUNK_PREFIXES
        .iter()
        .any(|pfx| upper_name.starts_with(pfx))
}

fn validate_fat16_entry_file<R: Read + Seek>(
    reader: &mut R,
    data_start_lba: u64,
    spc: u8,
    entry: &RootDirFileReport,
    check_id: &str,
    label: &str,
    checks: &mut Vec<SdUnbrickCheck>,
) -> Result<Option<Vec<u8>>> {
    if !entry.chain_complete || entry.file_size == 0 {
        checks.push(check(
            check_id,
            "fail",
            format!("Root {label} FAT16 cluster chain is broken, empty, or truncated"),
            format!("Clusters traversed: {:?}", entry.chain_clusters),
        ));
        return Ok(None);
    }
    let payload = read_fat16_file_bytes(
        reader,
        data_start_lba,
        spc,
        &entry.chain_clusters,
        entry.file_size,
    )?;
    Ok(Some(payload))
}

pub(crate) struct RootInspectionResult {
    pub digadiag_entry: Option<RootDirFileReport>,
    pub pana_sd_entry: Option<RootDirFileReport>,
    pub root_files_sample: Vec<String>,
    pub digadiag_payload_verified: bool,
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
        if !(2..0xFFF0).contains(&next) {
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

pub(crate) fn inspect_fat16_root_and_digadiag<R: Read + Seek>(
    reader: &mut R,
    part_lba: u64,
    bpb: &FatBpbReport,
    checks: &mut Vec<SdUnbrickCheck>,
) -> Result<RootInspectionResult> {
    let mut digadiag_entry = None;
    let mut pana_sd_entry = None;
    let mut root_files_sample = Vec::new();
    let mut digadiag_payload_verified = false;

    let fat_start_lba = part_lba + u64::from(bpb.reserved_sectors);
    let fat_sectors_to_read = (bpb.sectors_per_fat as usize).min(256);
    let fat_bytes = read_sectors(reader, fat_start_lba, fat_sectors_to_read)?;
    let root_start_lba = fat_start_lba + u64::from(bpb.num_fats) * u64::from(bpb.sectors_per_fat);
    let root_dir_sectors = (u32::from(bpb.root_entry_count) * 32).div_ceil(512) as usize;
    let root_bytes = read_sectors(reader, root_start_lba, root_dir_sectors)?;
    let data_start_lba = root_start_lba + root_dir_sectors as u64;

    let mut lfn_count = 0usize;
    let mut junk_entries: Vec<String> = Vec::new();
    let mut prev_was_lfn = false;
    for entry in root_bytes.as_chunks::<32>().0 {
        if entry[0] == 0x00 {
            break;
        }
        if entry[0] == 0xE5 {
            prev_was_lfn = false;
            continue;
        }
        let attr = entry[11];
        if attr == 0x0F {
            lfn_count += 1;
            prev_was_lfn = true;
            continue;
        }
        if (attr & 0x08) != 0 {
            prev_was_lfn = false;
            continue;
        }
        let name = format_8_3_name(&entry[0..11]);
        let upper = name.to_ascii_uppercase();
        if is_os_junk_entry(&upper) || prev_was_lfn {
            junk_entries.push(name.clone());
        }
        if root_files_sample.len() < 16 {
            root_files_sample.push(name.clone());
        }
        let first_cluster = u16_le(entry, 26);
        let file_size = u32_le(entry, 28);
        if upper == "DIGADIAG.FRM" || upper == "PANA__SD.FRM" {
            let (chain, contig, complete) =
                trace_fat16_chain(&fat_bytes, first_cluster, file_size, bpb.cluster_size_bytes);
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

    if lfn_count > 0 || !junk_entries.is_empty() {
        checks.push(check(
            "sd_root_lfn_or_junk",
            "warn",
            format!(
                "Root has {lfn_count} LFN (0x0F) entries and {} junk item(s)",
                junk_entries.len()
            ),
            format!(
                "Non-clean 8.3 entries: [{}]. AM34 IPL SDDIAG prefers clean 8.3 root.",
                junk_entries.join(", ")
            ),
        ));
    }

    if let Some(ref d) = digadiag_entry {
        let valid_size = d.file_size == PANA_DIGADIAG_FRM_SIZE as u32
            || d.file_size == PANA_DIGADIAG_RAW_VER061_SIZE as u32;
        if !valid_size {
            checks.push(check(
                "sd_digadiag_entry", "fail",
                format!("Root DIGADIAG.FRM wrong size {} B (expected {} or {})", d.file_size, PANA_DIGADIAG_RAW_VER061_SIZE, PANA_DIGADIAG_FRM_SIZE),
                "Loader ver0.61 requires 262144 B (0x40000) raw; Loader ver0.63 requires 131072 B (0x20000) plaintext EXTRHEAD.",
            ));
        } else if let Some(payload) = validate_fat16_entry_file(
            reader,
            data_start_lba,
            bpb.sectors_per_cluster,
            d,
            "sd_digadiag_entry",
            "DIGADIAG.FRM",
            checks,
        )? {
            let diag = verify_digadiag_buffer(Some(Ok(&payload)));
            digadiag_payload_verified = diag.exact_size_ok && diag.check.severity == "pass";
            if !digadiag_payload_verified {
                checks.push(diag.check);
            } else {
                checks.push(check(
                    "sd_digadiag_entry", "pass",
                    format!("Root 8.3 DIGADIAG.FRM OK: {} B @ cluster {} (dst={}, mode={}, contig={})", d.file_size, d.first_cluster, diag.dst_addr_hex.as_deref().unwrap_or("?"), diag.bgo_mode.as_deref().unwrap_or("?"), d.chain_contiguous),
                    "Exact 8.3 root directory entry, FAT16 cluster chain, and DIGADIAG payload verified.",
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

    if let Some(ref p) = pana_sd_entry
        && let Some(payload) = validate_fat16_entry_file(
            reader,
            data_start_lba,
            bpb.sectors_per_cluster,
            p,
            "sd_pana_sd_entry",
            "PANA__SD.FRM",
            checks,
        )?
    {
        let pana = verify_pana_sd_buffer(Some(&payload));
        let sev = match pana.check.severity.as_str() {
            "fail" => "fail",
            "warn" => "warn",
            _ => "pass",
        };
        checks.push(check(
            "sd_pana_sd_entry",
            sev,
            format!(
                "Root 8.3 PANA__SD.FRM {}: {} B @ cluster {} ({} clusters, contig={})",
                sev.to_ascii_uppercase(),
                p.file_size,
                p.first_cluster,
                p.chain_clusters.len(),
                p.chain_contiguous
            ),
            pana.check.detail,
        ));
    }

    Ok(RootInspectionResult {
        digadiag_entry,
        pana_sd_entry,
        root_files_sample,
        digadiag_payload_verified,
    })
}
