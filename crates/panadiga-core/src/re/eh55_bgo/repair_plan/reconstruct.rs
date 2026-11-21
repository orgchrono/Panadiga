#![deny(unsafe_code)]

//! CoW reconstruction helpers for missing `F2` sectors, `SA41+0x14` migration, and corrupt spares.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

use super::read_le_u16;
use crate::constants::PANA_DEV_BIN_MAGIC;
use crate::re::eh55_bgo::{
    BgoSectorGeometry, Eh55BgoAuditReport, Eh55BgoSectorKind, audit_bgo_nor_with_geometry,
    parse_bgo_l2p_with_geometry,
};

fn find_f2_page_matching(
    nor: &[u8],
    geom: &BgoSectorGeometry,
    target_pid: u16,
    predicate: impl Fn(&[u8]) -> bool,
) -> Option<Vec<u8>> {
    for sa in geom.first_sa..=geom.last_sa {
        let Ok(base) = geom.sector_offset(sa) else {
            continue;
        };
        if (read_le_u16(nor, base + 4) & 0xFF00) != 0xF200 {
            continue;
        }
        for slot in 0..126usize {
            if read_le_u16(nor, base + 0x200 + slot * 4) == target_pid {
                let poff = base + 0x400 + slot * 0x200;
                let page = &nor[poff..poff + 0x200];
                if predicate(page) {
                    return Some(page.to_vec());
                }
            }
        }
    }
    None
}

fn find_superseded_page(nor: &[u8], target_pid: u16, geom: &BgoSectorGeometry) -> Option<Vec<u8>> {
    find_f2_page_matching(nor, geom, target_pid, |_| true)
}

fn find_superseded_0f0a_page(nor: &[u8], geom: &BgoSectorGeometry) -> Option<Vec<u8>> {
    find_superseded_page(nor, 0x0F0A, geom).or_else(|| {
        find_f2_page_matching(nor, geom, 0x0000, |page| {
            (0..8usize).all(|i| read_le_u16(page, i * 2) == 0x0302 + i as u16)
        })
    })
}

fn build_reconstructed_0f0a_page(nor: &[u8], geom: &BgoSectorGeometry) -> Vec<u8> {
    let Some(mut raw) = find_superseded_0f0a_page(nor, geom) else {
        return vec![0xFFu8; 512];
    };
    let mut set_fat = |pid: u16, nxt: u16| {
        if let Some(idx) = pid.checked_sub(0x0301)
            && usize::from(idx) < 256
        {
            let off = usize::from(idx) * 2;
            raw[off..off + 2].copy_from_slice(&nxt.to_le_bytes());
        }
    };
    set_fat(0x031C, 0x002A);
    let mut fs2_tail: Vec<u16> = (0x0378..0x038B).collect();
    fs2_tail.extend(0x03A3..0x03A7);
    set_fat(0x030F, fs2_tail[0]);
    for pair in fs2_tail.windows(2) {
        set_fat(pair[0], pair[1]);
    }
    if let Some(&last) = fs2_tail.last() {
        set_fat(last, 0x0042);
    }

    let mut dbg_tail = vec![
        0x0326, 0x032A, 0x032C, 0x032D, 0x032E, 0x0330, 0x0331, 0x0332, 0x0334, 0x0335, 0x0336,
        0x0337, 0x0338, 0x033B, 0x033C,
    ];
    dbg_tail.extend(0x038B..0x03A3);
    dbg_tail.push(0x03A7);
    set_fat(0x0328, dbg_tail[0]);
    for pair in dbg_tail.windows(2) {
        set_fat(pair[0], pair[1]);
    }
    if let Some(&last) = dbg_tail.last() {
        set_fat(last, 0xFFED);
    }
    raw
}

fn read_present_bgo_page(
    nor: &[u8],
    l2p: &BTreeMap<u16, u16>,
    f2_bases: &BTreeMap<u16, usize>,
    pid: u16,
) -> Option<Vec<u8>> {
    let &w = l2p.get(&pid).filter(|&&w| w != 0xFFFF)?;
    let &base = f2_bases.get(&(w >> 8))?;
    let poff = base + 0x400 + (usize::from(w & 0xFF) >> 1) * 0x200;
    nor.get(poff..poff + 0x200).map(|s| s.to_vec())
}

fn format_bgo_header_and_zero_marker(
    cow: &mut [u8],
    base: usize,
    sector_id_word: [u8; 4],
    sector_stride: usize,
) {
    cow[base..base + sector_stride].fill(0xFF);
    for i in 0..4usize {
        cow[base + 0x04 + i * 4..base + 0x08 + i * 4].copy_from_slice(&sector_id_word);
    }
    let marker = [0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF];
    cow[base + 0x14..base + 0x1C].copy_from_slice(&marker);
    cow[base + 0x5C..base + 0x64].copy_from_slice(&marker);
    cow[base + 0xFC..base + 0x100].copy_from_slice(&[0x00; 4]);
    cow[base + 0x100..base + 0x200].fill(0x00);
}

/// Compacts split or migrating `0xF101` Partition-1 sectors into one authoritative sector
/// with zero open `+0x14` migration links (`FUN_4402014f` rollback prevention).
fn compact_p1_sectors_in_cow(
    cow: &mut [u8],
    audit: &Eh55BgoAuditReport,
    geom: &BgoSectorGeometry,
    l2p: &BTreeMap<u16, u16>,
) -> Result<()> {
    let p1_sas: Vec<usize> = audit
        .sectors
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                Eh55BgoSectorKind::Partition1Active | Eh55BgoSectorKind::Partition1Migrating
            )
        })
        .map(|s| s.sa)
        .collect();
    if p1_sas.is_empty() {
        bail!("No Partition 1 (0xF101) sector found in BGO volume");
    }
    if p1_sas.len() == 1 && audit.p1_migrating_count == 0 {
        return Ok(());
    }

    let target_sa = p1_sas[0];
    let target_base = geom.sector_offset(target_sa)?;
    format_bgo_header_and_zero_marker(
        cow,
        target_base,
        [0x01, 0xF1, 0xFF, 0xFF],
        geom.sector_stride,
    );

    let max_tid = usize::from(geom.max_pid / 16);
    if 2 + (max_tid / 7) >= 256 {
        bail!("BGO Partition 1 overflow: max_tid={max_tid} exceeds 254 sub-pages");
    }
    for idx in 0..=max_tid {
        let tid = idx as u16;
        let sp = 2 + (idx / 7);
        let s = idx % 7;
        let off = target_base + sp * 0x100;
        for b_i in 0..3usize {
            cow[off + b_i] &= !(1u8 << s);
        }
        let soff = off + 4 + s * 0x24;
        cow[soff..soff + 2].copy_from_slice(&tid.to_le_bytes());
        for i in 0..16u16 {
            let pid = tid.wrapping_mul(16).wrapping_add(i);
            let w = l2p.get(&pid).copied().unwrap_or(0xFFFF);
            let w_off = soff + 4 + usize::from(i) * 2;
            cow[w_off..w_off + 2].copy_from_slice(&w.to_le_bytes());
        }
    }

    for &sa in &p1_sas[1..] {
        let base = geom.sector_offset(sa)?;
        cow[base..base + geom.sector_stride].fill(0xFF);
        cow[base + 0x100..base + 0x200].fill(0x00);
    }
    Ok(())
}

/// Reconstructs corrupt/missing F2 sectors inside a CoW buffer using discovered BGO sector geometry.
pub fn reconstruct_corrupt_bgo_in_cow_with_geometry(
    corrupt_nor: &[u8],
    geom: &BgoSectorGeometry,
    default_ifodev2: Option<&[u8; 8]>,
) -> Result<(Vec<u8>, Vec<u16>, Vec<usize>)> {
    let pre_audit = audit_bgo_nor_with_geometry(corrupt_nor, geom)?;
    let l2p = parse_bgo_l2p_with_geometry(corrupt_nor, geom)?;
    let mut cow = corrupt_nor.to_vec();
    if pre_audit.healthy {
        return Ok((cow, Vec::new(), Vec::new()));
    }

    let mut clean_spares = pre_audit.spare_sas.clone();
    for &sa in &pre_audit.corrupt_sas {
        let base = geom.sector_offset(sa)?;
        cow[base..base + geom.sector_stride].fill(0xFF);
        cow[base + 0x100..base + 0x200].fill(0x00);
        if !clean_spares.contains(&sa) {
            clean_spares.push(sa);
        }
    }
    clean_spares.sort_unstable();
    if clean_spares.len() < pre_audit.p2_missing_ids.len() {
        bail!(
            "Insufficient BGO spare sectors ({} available < {} missing F2 IDs)",
            clean_spares.len(),
            pre_audit.p2_missing_ids.len()
        );
    }

    let mut f2_bases = BTreeMap::new();
    for s in &pre_audit.sectors {
        if s.kind == Eh55BgoSectorKind::Partition2 {
            f2_bases.insert(s.sector_id & 0xFF, s.flash_offset);
        }
    }
    let missing_f2_ids = pre_audit.p2_missing_ids.clone();
    let assigned_spares = clean_spares[..missing_f2_ids.len()].to_vec();
    let mut f2_slots: BTreeMap<u16, BTreeMap<usize, u16>> = missing_f2_ids
        .iter()
        .map(|&f| (f, BTreeMap::new()))
        .collect();
    for pid in 0x0001..=geom.max_pid {
        if let Some(&w) = l2p.get(&pid).filter(|&&w| w != 0xFFFF)
            && let Some(map) = f2_slots.get_mut(&(w >> 8))
        {
            let slot = usize::from(w & 0xFF) >> 1;
            if slot >= 126 || map.insert(slot, pid).is_some() {
                bail!("Invalid/duplicate slot {slot} for F2 ID 0x{:02X}", w >> 8);
            }
        }
    }

    let recon_0f0a = build_reconstructed_0f0a_page(corrupt_nor, geom);
    let ifodev2_bytes = default_ifodev2.unwrap_or(b"M6930LB\x00");
    for (&fid, &sa) in missing_f2_ids.iter().zip(assigned_spares.iter()) {
        let base = geom.sector_offset(sa)?;
        format_bgo_header_and_zero_marker(
            &mut cow,
            base,
            [fid as u8, 0xF2, 0xFF, 0xFF],
            geom.sector_stride,
        );

        let slots = &f2_slots[&fid];
        if !slots.contains_key(&0) {
            cow[base + 0x200..base + 0x202].copy_from_slice(&[0x00, 0x00]);
        }
        for (&slot, &pid) in slots {
            let tag_off = base + 0x200 + slot * 4;
            cow[tag_off..tag_off + 2].copy_from_slice(&pid.to_le_bytes());
            let poff = base + 0x400 + slot * 0x200;
            if pid == 0x0055 {
                if let Some(sup) = find_superseded_page(corrupt_nor, 0x0055, geom) {
                    cow[poff..poff + 512].copy_from_slice(&sup);
                } else {
                    cow[poff..poff + 2].copy_from_slice(&[0x01, 0x01]);
                }
            } else if (0x0F07..=0x0F24).contains(&pid) {
                let sib = if pid <= 0x0F15 { pid + 15 } else { pid - 15 };
                if let Some(m) = read_present_bgo_page(corrupt_nor, &l2p, &f2_bases, sib) {
                    cow[poff..poff + 512].copy_from_slice(&m);
                } else if pid == 0x0F0A || pid == 0x0F19 {
                    cow[poff..poff + 512].copy_from_slice(&recon_0f0a);
                }
            } else if pid == 0x0001 {
                if let Some(sup) = find_superseded_page(corrupt_nor, 0x0001, geom) {
                    cow[poff..poff + 512].copy_from_slice(&sup);
                } else {
                    cow[poff..poff + 4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
                    cow[poff + 4..poff + 12].copy_from_slice(ifodev2_bytes);
                }
            } else if let Some(sup) = find_superseded_page(corrupt_nor, pid, geom) {
                cow[poff..poff + 512].copy_from_slice(&sup);
            }
        }
    }

    compact_p1_sectors_in_cow(&mut cow, &pre_audit, geom, &l2p)?;
    Ok((cow, missing_f2_ids, assigned_spares))
}

pub(super) fn reconstruct_corrupt_bgo_in_cow(
    corrupt_nor: &[u8],
) -> Result<(Vec<u8>, Vec<u16>, Vec<usize>)> {
    reconstruct_corrupt_bgo_in_cow_with_geometry(
        corrupt_nor,
        &BgoSectorGeometry::eh55_default(),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_bgo_geometry_reconstruct_target_word_and_ifodev2() {
        let geom = BgoSectorGeometry {
            first_sa: 10,
            last_sa: 14,
            base_offset: 0x04_0000,
            sector_stride: 0x1_0000,
            expected_flash_bytes: 0x10_0000,
            expected_p2_count: 1,
            max_pid: 0x0F24,
        };
        let mut nor = vec![0xFFu8; geom.expected_flash_bytes];
        let p1 = geom.sector_offset(12).unwrap();
        for i in 0..4 {
            nor[p1 + 4 + i * 4..p1 + 6 + i * 4].copy_from_slice(&0xF101u16.to_le_bytes());
        }
        nor[p1 + 0x100..p1 + 0x200].fill(0x00);
        let sp = p1 + 2 * 0x100;
        nor[sp..sp + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]); // slot 0 status=8
        let soff = sp + 4;
        nor[soff..soff + 4].copy_from_slice(&[0x00, 0x00, 0xFF, 0xFF]); // tid=0, t2=0xFFFF
        nor[soff + 6..soff + 8].copy_from_slice(&0x0102u16.to_le_bytes()); // pid=1 -> fid=1, slot=1

        let spare = geom.sector_offset(10).unwrap();
        nor[spare + 0x100..spare + 0x200].fill(0x00); // make spare clean except zero marker -> corrupt so reclaimed!
        let (cow, missing, assigned) =
            reconstruct_corrupt_bgo_in_cow_with_geometry(&nor, &geom, Some(b"M7920EU\x00"))
                .unwrap();
        assert_eq!(missing, vec![1]);
        assert_eq!(assigned, vec![10]);
        let f2_base = geom.sector_offset(10).unwrap();
        assert_eq!(
            &cow[f2_base + 0x400 + 0x200 + 4..f2_base + 0x400 + 0x200 + 12],
            b"M7920EU\x00"
        );
    }
}
