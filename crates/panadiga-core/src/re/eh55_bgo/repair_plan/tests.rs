#![deny(unsafe_code)]

use std::collections::BTreeMap;

use super::*;
use crate::constants::{AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE, PANA_DEV_BIN_MAGIC};
use crate::re::eh55_bgo::{EH55_BGO_LAST_SA, audit_eh55_bgo_nor_8mb};

pub(in crate::re::eh55_bgo) fn build_synthetic_bgo_test_pair() -> (Vec<u8>, Vec<u8>) {
    let mut healthy = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    for sa in EH55_BGO_FIRST_SA..=EH55_BGO_LAST_SA {
        let base = eh55_bgo_sector_offset(sa).unwrap();
        healthy[base + 0x100..base + 0x200].fill(0x00);
    }

    let sa29 = eh55_bgo_sector_offset(29).unwrap();
    for i in 0..4 {
        healthy[sa29 + 0x04 + i * 4..sa29 + 0x06 + i * 4].copy_from_slice(&0xF101u16.to_le_bytes());
    }
    let sa41 = eh55_bgo_sector_offset(41).unwrap();
    for i in 0..4 {
        healthy[sa41 + 0x04 + i * 4..sa41 + 0x06 + i * 4].copy_from_slice(&0xF101u16.to_le_bytes());
    }

    let f2_sas: Vec<usize> = (26..=28).chain(30..=40).chain(42..=67).collect();
    assert_eq!(f2_sas.len(), 40);
    let mut pid_to_word: BTreeMap<u16, u16> = BTreeMap::new();
    for (idx, &sa) in f2_sas.iter().enumerate() {
        let fid = (idx + 1) as u16;
        let base = eh55_bgo_sector_offset(sa).unwrap();
        let id_bytes = [fid as u8, 0xF2, 0xFF, 0xFF];
        for i in 0..4 {
            healthy[base + 0x04 + i * 4..base + 0x08 + i * 4].copy_from_slice(&id_bytes);
        }
        healthy[base + 0xFC..base + 0x100].copy_from_slice(&[0; 4]);
        let pid = if fid == 1 {
            0x0001
        } else if fid <= 16 {
            0x0F07 + (fid - 2)
        } else if fid <= 31 {
            0x0F16 + (fid - 17)
        } else {
            0x0010 + fid
        };
        healthy[base + 0x200..base + 0x202].copy_from_slice(&pid.to_le_bytes());
        let poff = base + 0x400;
        if pid == 0x0001 {
            healthy[poff..poff + 4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
            healthy[poff + 4..poff + 12].copy_from_slice(b"M6930LB\x00");
        } else if (0x0F07..=0x0F24).contains(&pid) {
            healthy[poff..poff + 4].copy_from_slice(&[0xED, 0xFF, 0xFF, 0xFF]);
        }
        pid_to_word.insert(pid, fid << 8);
    }

    let mut by_tid: BTreeMap<u16, [u16; 16]> = BTreeMap::new();
    for (pid, w) in pid_to_word {
        let entry = by_tid.entry(pid / 16).or_insert([0xFFFF; 16]);
        entry[usize::from(pid % 16)] = w;
    }
    for (idx, (tid, arr)) in by_tid.into_iter().enumerate() {
        let off = sa41 + (2 + idx / 7) * 0x100;
        let s = idx % 7;
        if s == 0 {
            healthy[off..off + 4].copy_from_slice(&[0x00; 4]);
        }
        healthy[off + 3] |= 1u8 << s;
        let soff = off + 4 + s * 0x24;
        healthy[soff..soff + 2].copy_from_slice(&tid.to_le_bytes());
        healthy[soff + 2..soff + 4].copy_from_slice(&0x0001u16.to_le_bytes());
        for (i, w) in arr.into_iter().enumerate() {
            healthy[soff + 4 + i * 2..soff + 6 + i * 2].copy_from_slice(&w.to_le_bytes());
        }
    }

    let mut mig_bad = hex::decode("f8fff8fff8fffcffffff0400").unwrap();
    mig_bad.resize(38, 0xFF);
    let mut mig_ok = mig_bad.clone();
    mig_ok[0] = 0xF0;
    mig_ok[2] = 0xF0;
    mig_ok[4] = 0xF0;
    mig_ok[6] = 0xF8;
    mig_ok[14] = 0x04;
    mig_ok[15] = 0x00;
    healthy[sa41 + 0x14..sa41 + 0x14 + 38].copy_from_slice(&mig_ok);

    let mut corrupt = healthy.clone();
    corrupt[sa41 + 0x14..sa41 + 0x14 + 38].copy_from_slice(&mig_bad);
    for sa in std::iter::once(35).chain(56..=67) {
        let base = eh55_bgo_sector_offset(sa).unwrap();
        corrupt[base..base + EH55_BGO_SECTOR_STRIDE].fill(0xFF);
        corrupt[base + 0x100..base + 0x200].fill(0x00);
    }
    let sa70 = eh55_bgo_sector_offset(70).unwrap();
    corrupt[sa70 + 0xE100..sa70 + 0xE200].fill(0x00);

    (corrupt, healthy)
}

#[test]
fn builds_and_cow_verifies_bgo_repair_plan_offline() {
    let (corrupt, healthy) = build_synthetic_bgo_test_pair();
    assert!(audit_eh55_bgo_nor_8mb(&healthy).unwrap().healthy);
    assert!(!audit_eh55_bgo_nor_8mb(&corrupt).unwrap().healthy);

    let plan = build_bgo_repair_plan(&corrupt, None).unwrap();
    assert!(plan.post_audit.healthy);
    assert!(plan.post_audit.p2_missing_ids.is_empty());
    assert!(plan.post_audit.corrupt_sas.is_empty());
    assert_eq!(plan.post_audit.l2p_tag_mismatches, 0);
    assert_eq!(plan.post_audit.fat_mirror_mismatches, 0);
    assert_eq!(plan.erased_sas, vec![70]);
    assert!(!plan.ops.is_empty());
}

#[test]
fn rejects_non_monotonic_write_and_out_of_bgo_bounds_e05() {
    let nor = vec![0x00u8; AM34_FLASH_SIZE as usize];
    let bad_bounds = vec![BgoRepairOp::WriteBytes {
        sa: 0,
        phys_va: 0x8400_0100,
        data: vec![0xAA],
        requires_erase_first: false,
    }];
    let err_bounds = validate_and_simulate_bgo_plan(&nor, &bad_bounds)
        .unwrap_err()
        .to_string();
    assert!(err_bounds.contains("E05"), "err={err_bounds}");

    let bad_flip = vec![BgoRepairOp::WriteBytes {
        sa: 42,
        phys_va: AM34_FLASH_UNCACHED_BASE + eh55_bgo_sector_offset(42).unwrap() as u32,
        data: vec![0xFF],
        requires_erase_first: false,
    }];
    let err_flip = validate_and_simulate_bgo_plan(&nor, &bad_flip)
        .unwrap_err()
        .to_string();
    assert!(err_flip.contains("0->1"), "err={err_flip}");
}
