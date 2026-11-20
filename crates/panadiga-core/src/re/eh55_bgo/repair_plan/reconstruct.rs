#![deny(unsafe_code)]

//! CoW reconstruction helpers for missing `F2` sectors, `SA41+0x14` migration, and corrupt spares.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

use super::read_le_u16;
use crate::constants::PANA_DEV_BIN_MAGIC;
use crate::re::eh55_bgo::{
    EH55_BGO_FIRST_SA, EH55_BGO_LAST_SA, EH55_BGO_MAX_PID, EH55_BGO_SECTOR_STRIDE,
    Eh55BgoAuditReport, Eh55BgoSectorKind, audit_eh55_bgo_nor_8mb, eh55_bgo_sector_offset,
    eval_bgo_migration_index, eval_bgo_migration_target, parse_eh55_bgo_l2p,
};

fn find_superseded_page(nor_8mb: &[u8], target_pid: u16) -> Option<Vec<u8>> {
    for sa in EH55_BGO_FIRST_SA..=EH55_BGO_LAST_SA {
        let Ok(base) = eh55_bgo_sector_offset(sa) else {
            continue;
        };
        if (read_le_u16(nor_8mb, base + 4) & 0xFF00) != 0xF200 {
            continue;
        }
        for slot in 0..126usize {
            if read_le_u16(nor_8mb, base + 0x200 + slot * 4) == target_pid {
                let poff = base + 0x400 + slot * 0x200;
                return Some(nor_8mb[poff..poff + 0x200].to_vec());
            }
        }
    }
    None
}

fn build_reconstructed_0f0a_page(nor_8mb: &[u8]) -> Vec<u8> {
    let Some(mut raw) = find_superseded_page(nor_8mb, 0x0F0A) else {
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
    nor_8mb: &[u8],
    l2p: &BTreeMap<u16, u16>,
    f2_bases: &BTreeMap<u16, usize>,
    pid: u16,
) -> Option<Vec<u8>> {
    let &w = l2p.get(&pid).filter(|&&w| w != 0xFFFF)?;
    let &base = f2_bases.get(&(w >> 8))?;
    let poff = base + 0x400 + (usize::from(w & 0xFF) >> 1) * 0x200;
    nor_8mb.get(poff..poff + 0x200).map(|s| s.to_vec())
}

fn repair_p1_migration_in_cow(cow: &mut [u8], audit: &Eh55BgoAuditReport) -> Result<()> {
    let active_p1: Vec<usize> = audit
        .sectors
        .iter()
        .filter(|s| s.kind == Eh55BgoSectorKind::Partition1Active)
        .map(|s| s.sa)
        .collect();
    if active_p1.len() <= 1 {
        return Ok(());
    }
    let target_sa = *active_p1
        .iter()
        .find(|&&sa| {
            eh55_bgo_sector_offset(sa)
                .is_ok_and(|b| (0..4).all(|i| read_le_u16(cow, b + 0x14 + i * 2) == 0xFFFF))
        })
        .unwrap_or(&active_p1[0]);
    let target_word = (target_sa - 25) as u16;

    for sa in active_p1 {
        if sa == target_sa {
            continue;
        }
        let mig_off = eh55_bgo_sector_offset(sa)? + 0x14;
        let orig = cow[mig_off..mig_off + 38].to_vec();
        let Some(k) = (0..15usize).find(|&k| {
            let t_word = read_le_u16(&orig, 8 + k * 2);
            let tail_clean = ((k + 1)..15).all(|u| read_le_u16(&orig, 8 + u * 2) == 0xFFFF);
            let w3_ok = ((read_le_u16(&orig, 6) & !((1u16 << k) - 1)) & (1u16 << k)) != 0;
            tail_clean && (t_word == target_word || t_word == 0xFFFF) && w3_ok
        }) else {
            bail!("Cannot find 1->0 compatible migration bitmap slot in SA{sa}");
        };

        for w_i in 0..3usize {
            let next = read_le_u16(cow, mig_off + w_i * 2) & !((1u16 << (k + 1)) - 1);
            cow[mig_off + w_i * 2..mig_off + w_i * 2 + 2].copy_from_slice(&next.to_le_bytes());
        }
        let w3_next = read_le_u16(cow, mig_off + 6) & !((1u16 << k) - 1);
        cow[mig_off + 6..mig_off + 8].copy_from_slice(&w3_next.to_le_bytes());
        let t_off = mig_off + 8 + k * 2;
        let t_next = read_le_u16(cow, t_off) & target_word;
        cow[t_off..t_off + 2].copy_from_slice(&t_next.to_le_bytes());

        if eval_bgo_migration_index(cow, mig_off) != Some(k)
            || eval_bgo_migration_target(cow, mig_off) != target_word
        {
            bail!("Failed post-patch verification of SA{sa} migration target");
        }
    }
    Ok(())
}

pub(super) fn reconstruct_corrupt_bgo_in_cow(
    corrupt_nor: &[u8],
) -> Result<(Vec<u8>, Vec<u16>, Vec<usize>)> {
    let pre_audit = audit_eh55_bgo_nor_8mb(corrupt_nor)?;
    let l2p = parse_eh55_bgo_l2p(corrupt_nor)?;
    let mut cow = corrupt_nor.to_vec();
    if pre_audit.healthy {
        return Ok((cow, Vec::new(), Vec::new()));
    }

    let mut clean_spares = pre_audit.spare_sas.clone();
    for &sa in &pre_audit.corrupt_sas {
        let base = eh55_bgo_sector_offset(sa)?;
        cow[base..base + EH55_BGO_SECTOR_STRIDE].fill(0xFF);
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
    for pid in 0x0001..=EH55_BGO_MAX_PID {
        if let Some(&w) = l2p.get(&pid).filter(|&&w| w != 0xFFFF)
            && let Some(map) = f2_slots.get_mut(&(w >> 8))
        {
            let slot = usize::from(w & 0xFF) >> 1;
            if slot >= 126 || map.insert(slot, pid).is_some() {
                bail!("Invalid/duplicate slot {slot} for F2 ID 0x{:02X}", w >> 8);
            }
        }
    }

    let recon_0f0a = build_reconstructed_0f0a_page(corrupt_nor);
    for (&fid, &sa) in missing_f2_ids.iter().zip(assigned_spares.iter()) {
        let base = eh55_bgo_sector_offset(sa)?;
        let id_word = [fid as u8, 0xF2, 0xFF, 0xFF];
        for i in 0..4 {
            cow[base + 0x04 + i * 4..base + 0x08 + i * 4].copy_from_slice(&id_word);
        }
        let marker = [0xFE, 0xFF, 0xFE, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF];
        cow[base + 0x14..base + 0x1C].copy_from_slice(&marker);
        cow[base + 0x5C..base + 0x64].copy_from_slice(&marker);
        cow[base + 0xFC..base + 0x100].copy_from_slice(&[0x00; 4]);

        let slots = &f2_slots[&fid];
        if !slots.contains_key(&0) {
            cow[base + 0x200..base + 0x202].copy_from_slice(&[0x00, 0x00]);
        }
        for (&slot, &pid) in slots {
            let tag_off = base + 0x200 + slot * 4;
            cow[tag_off..tag_off + 2].copy_from_slice(&pid.to_le_bytes());
            let poff = base + 0x400 + slot * 0x200;
            if pid == 0x0055 {
                let fill = find_superseded_page(corrupt_nor, 0x0055);
                if let Some(sup) = fill {
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
                if let Some(sup) = find_superseded_page(corrupt_nor, 0x0001) {
                    cow[poff..poff + 512].copy_from_slice(&sup);
                } else {
                    cow[poff..poff + 4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
                    cow[poff + 4..poff + 12].copy_from_slice(b"M6930LB\x00");
                }
            } else if let Some(sup) = find_superseded_page(corrupt_nor, pid) {
                cow[poff..poff + 512].copy_from_slice(&sup);
            }
        }
    }

    repair_p1_migration_in_cow(&mut cow, &pre_audit)?;
    Ok((cow, missing_f2_ids, assigned_spares))
}
