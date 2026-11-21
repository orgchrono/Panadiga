#![deny(unsafe_code)]

//! Pure structural verifier for DIGA NOR BGO FTL volumes (default: EH55 `SA26..=SA70`).

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use crate::constants::{AM34_FLASH_UNCACHED_BASE, PANA_DEV_BIN_MAGIC};
use crate::crypto::codec::extract_c_ascii;

use super::audit_helpers::{
    BgoSectorGeometry, eval_bgo_migration_index, eval_bgo_migration_target,
    parse_bgo_l2p_with_geometry, read_bgo_page, read_le_u16,
};
use super::{Eh55BgoAuditReport, Eh55BgoSectorKind, Eh55BgoSectorReport};

/// Runs a non-destructive structural audit on an arbitrary discovered BGO sector geometry.
pub fn audit_bgo_nor_with_geometry(
    nor: &[u8],
    geom: &BgoSectorGeometry,
) -> Result<Eh55BgoAuditReport> {
    let l2p = parse_bgo_l2p_with_geometry(nor, geom)?;

    let sector_count = geom.last_sa.saturating_sub(geom.first_sa) + 1;
    let mut sectors = Vec::with_capacity(sector_count);
    let mut p1_active_count = 0usize;
    let mut p1_migrating_count = 0usize;
    let mut f2_bases: BTreeMap<u16, usize> = BTreeMap::new();
    let mut p2_present_set: BTreeSet<u16> = BTreeSet::new();
    let mut spare_sas = Vec::new();
    let mut corrupt_sas = Vec::new();
    let max_f2_word = 0xF200u16.saturating_add(geom.expected_p2_count as u16);

    for sa in geom.first_sa..=geom.last_sa {
        let base = geom.sector_offset(sa)?;
        let phys_va = AM34_FLASH_UNCACHED_BASE + base as u32;
        let w0 = read_le_u16(nor, base + 4);
        let consistent = (0..4).all(|i| read_le_u16(nor, base + 4 + i * 4) == w0);
        let zero_marker_ok = nor[base + 0x100..base + 0x200].iter().all(|&b| b == 0);
        let migration_idx = eval_bgo_migration_index(nor, base + 0x14);
        let migration_target = eval_bgo_migration_target(nor, base + 0x14);

        let kind = if !consistent || !zero_marker_ok {
            Eh55BgoSectorKind::Corrupt
        } else if w0 == 0xFFFF {
            let hdr_clean = nor[base..base + 0x100].iter().all(|&b| b == 0xFF);
            let data_clean = nor[base + 0x200..base + geom.sector_stride]
                .iter()
                .all(|&b| b == 0xFF);
            if hdr_clean && data_clean {
                spare_sas.push(sa);
                Eh55BgoSectorKind::Spare
            } else {
                Eh55BgoSectorKind::Corrupt
            }
        } else if w0 == 0xF101 {
            if migration_target == 0xFFFF {
                p1_active_count += 1;
                Eh55BgoSectorKind::Partition1Active
            } else {
                p1_migrating_count += 1;
                Eh55BgoSectorKind::Partition1Migrating
            }
        } else if (0xF201..=max_f2_word).contains(&w0) && migration_target == 0xFFFF {
            let fid = w0 & 0xFF;
            if p2_present_set.insert(fid) {
                f2_bases.insert(fid, base);
                Eh55BgoSectorKind::Partition2
            } else {
                Eh55BgoSectorKind::Corrupt
            }
        } else {
            Eh55BgoSectorKind::Corrupt
        };

        if kind == Eh55BgoSectorKind::Corrupt {
            corrupt_sas.push(sa);
        }

        sectors.push(Eh55BgoSectorReport {
            sa,
            flash_offset: base,
            phys_va,
            sector_id: w0,
            kind,
            migration_idx,
            migration_target,
            zero_marker_ok,
        });
    }

    let p2_present_ids: Vec<u16> = p2_present_set.iter().copied().collect();
    let p2_missing_ids: Vec<u16> = (1u16..=(geom.expected_p2_count as u16))
        .filter(|id| !p2_present_set.contains(id))
        .collect();

    let mut active_l2p_pages = 0usize;
    let mut l2p_tag_mismatches = 0usize;
    for pid in 0x0001..=geom.max_pid {
        if let Some(&w) = l2p.get(&pid)
            && w != 0xFFFF
        {
            active_l2p_pages += 1;
            if read_bgo_page(nor, &l2p, &f2_bases, pid).is_none() {
                l2p_tag_mismatches += 1;
            }
        }
    }

    let mut fat_mirror_mismatches = 0usize;
    let mut fat_pri = Vec::with_capacity(15 * 0x200);
    let mut fat_pri_complete = true;
    for idx in 0..15u16 {
        let p_pri = read_bgo_page(nor, &l2p, &f2_bases, 0x0F07 + idx);
        let p_bak = read_bgo_page(nor, &l2p, &f2_bases, 0x0F16 + idx);
        if let Some(a) = p_pri {
            fat_pri.extend_from_slice(a);
        } else {
            fat_pri_complete = false;
        }
        match (p_pri, p_bak) {
            (Some(a), Some(b)) if a == b => {}
            _ => fat_mirror_mismatches += 1,
        }
    }

    let mut nfrom_file_count = 0usize;
    for dpid in [0x0F02u16, 0x0F03, 0x0F04] {
        if let Some(dp) = read_bgo_page(nor, &l2p, &f2_bases, dpid) {
            for idx in 0..16usize {
                let ent = &dp[idx * 16..(idx + 1) * 16];
                if ent[0] != 0 && ent[0] != 0xFF {
                    nfrom_file_count += 1;
                    let byte_size = u32::from_le_bytes([ent[8], ent[9], ent[10], ent[11]]);
                    let start_pid = read_le_u16(ent, 12);
                    if byte_size > 0 && start_pid != 0xFFFF {
                        let expected_pages = (byte_size as usize).div_ceil(0x200);
                        let mut cur = start_pid;
                        let mut ok = fat_pri_complete;
                        for _ in 0..expected_pages {
                            let fat_off = usize::from(cur.saturating_sub(1)) * 2;
                            if !ok
                                || cur == 0
                                || cur == 0xFFFF
                                || cur == 0xFFED
                                || fat_off + 2 > fat_pri.len()
                            {
                                ok = false;
                                break;
                            }
                            cur = read_le_u16(&fat_pri, fat_off);
                        }
                        if !ok || cur != 0xFFED {
                            fat_mirror_mismatches += 1;
                        }
                    }
                }
            }
        }
    }

    let ifodev2_model = read_bgo_page(nor, &l2p, &f2_bases, 0x0001).and_then(|page| {
        (page.len() >= 12 && page[0..4] == PANA_DEV_BIN_MAGIC)
            .then(|| extract_c_ascii(&page[0x04..0x0C]))
    });

    let healthy = corrupt_sas.is_empty()
        && p1_active_count == 1
        && p1_migrating_count == 0
        && p2_missing_ids.is_empty()
        && active_l2p_pages > 0
        && l2p_tag_mismatches == 0
        && fat_mirror_mismatches == 0
        && ifodev2_model.is_some();

    Ok(Eh55BgoAuditReport {
        healthy,
        total_sectors: sectors.len(),
        p1_active_count,
        p1_migrating_count,
        p2_present_ids,
        p2_missing_ids,
        spare_sas,
        corrupt_sas,
        active_l2p_pages,
        l2p_tag_mismatches,
        fat_mirror_mismatches,
        nfrom_file_count,
        ifodev2_model,
        sectors,
    })
}

/// Runs a complete non-destructive structural audit on the EH55 `SA26..=SA70` BGO region inside an 8 MiB NOR dump.
pub fn audit_eh55_bgo_nor_8mb(nor_8mb: &[u8]) -> Result<Eh55BgoAuditReport> {
    audit_bgo_nor_with_geometry(nor_8mb, &BgoSectorGeometry::eh55_default())
}
