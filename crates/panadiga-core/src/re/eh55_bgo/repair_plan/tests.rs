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
    let clean_mig = hex::decode("fefffefffeffffff").unwrap();
    healthy[sa29 + 0x14..sa29 + 0x1C].copy_from_slice(&clean_mig);
    healthy[sa29 + 0x5C..sa29 + 0x64].copy_from_slice(&clean_mig);
    let sa41 = eh55_bgo_sector_offset(41).unwrap();

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

    let write_tid_slots = |buf: &mut [u8], base: usize| {
        for (idx, (&tid, &arr)) in by_tid.iter().enumerate() {
            let off = base + (2 + idx / 7) * 0x100;
            let s = idx % 7;
            if s == 0 {
                buf[off..off + 4].copy_from_slice(&[0x00; 4]);
            }
            buf[off + 3] |= 1u8 << s;
            let soff = off + 4 + s * 0x24;
            buf[soff..soff + 2].copy_from_slice(&tid.to_le_bytes());
            buf[soff + 2..soff + 4].copy_from_slice(&0x0001u16.to_le_bytes());
            for (i, w) in arr.into_iter().enumerate() {
                buf[soff + 4 + i * 2..soff + 6 + i * 2].copy_from_slice(&w.to_le_bytes());
            }
        }
    };
    write_tid_slots(&mut healthy, sa29);

    let mut mig_bad = hex::decode("f8fff8fff8fffcffffff0400").unwrap();
    mig_bad.resize(38, 0xFF);
    let mut corrupt = healthy.clone();
    corrupt[sa29..sa29 + EH55_BGO_SECTOR_STRIDE].fill(0xFF);
    corrupt[sa29 + 0x100..sa29 + 0x200].fill(0x00);
    for i in 0..4 {
        corrupt[sa29 + 0x04 + i * 4..sa29 + 0x06 + i * 4].copy_from_slice(&0xF101u16.to_le_bytes());
    }
    for i in 0..4 {
        corrupt[sa41 + 0x04 + i * 4..sa41 + 0x06 + i * 4].copy_from_slice(&0xF101u16.to_le_bytes());
    }
    write_tid_slots(&mut corrupt, sa41);
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
    assert_eq!(plan.post_audit.p1_active_count, 1);
    assert_eq!(plan.post_audit.p1_migrating_count, 0);
    assert!(plan.post_audit.p2_missing_ids.is_empty());
    assert!(plan.post_audit.corrupt_sas.is_empty());
    assert_eq!(plan.post_audit.l2p_tag_mismatches, 0);
    assert_eq!(plan.post_audit.fat_mirror_mismatches, 0);
    assert_eq!(plan.erased_sas, vec![41, 70]);
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

#[test]
fn reconstructs_split_duplicate_f202_and_missing_f209_without_reference_or_overflow() {
    let (_orig_corrupt, mut corrupt) = build_synthetic_bgo_test_pair();
    let sa29 = eh55_bgo_sector_offset(29).unwrap();

    // Map pid 0x013F (tid=0x13, idx=15) -> slot 16 (0x0220), pid 0x01AC (tid=0x1A, idx=12) -> slot 18 (0x0224),
    // and pid 0x01C1 (tid=0x1C, idx=1) -> slot 19 (0x0226) in free subpages sp=3..5.
    for (sp_idx, tid, idx, word) in [
        (3usize, 0x0013u16, 15usize, 0x0220u16),
        (4usize, 0x001Au16, 12usize, 0x0224u16),
        (5usize, 0x001Cu16, 1usize, 0x0226u16),
    ] {
        let sp = sa29 + sp_idx * 0x100;
        corrupt[sp..sp + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        let soff = sp + 4;
        corrupt[soff..soff + 2].copy_from_slice(&tid.to_le_bytes());
        corrupt[soff + 2..soff + 4].copy_from_slice(&0x0001u16.to_le_bytes());
        corrupt[soff + 4 + idx * 2..soff + 6 + idx * 2].copy_from_slice(&word.to_le_bytes());
    }

    // Mark SA27 (fid=2, 0xF202) historical slots 1..=13 as superseded 0x0000 and place split duplicate
    // slots 16, 18, 19 in spare SA68 (leaving blank tag holes at slots 14, 15, 17); erase SA34 (0xF209).
    let sa27 = eh55_bgo_sector_offset(27).unwrap();
    for slot in 1..=13usize {
        corrupt[sa27 + 0x200 + slot * 4..sa27 + 0x200 + slot * 4 + 2]
            .copy_from_slice(&[0x00, 0x00]);
    }
    let sa68 = eh55_bgo_sector_offset(68).unwrap();
    for i in 0..4 {
        corrupt[sa68 + 0x04 + i * 4..sa68 + 0x08 + i * 4]
            .copy_from_slice(&[0x02, 0xF2, 0xFF, 0xFF]);
    }
    for (slot, pid) in [
        (16usize, 0x013Fu16),
        (18usize, 0x01ACu16),
        (19usize, 0x01C1u16),
    ] {
        corrupt[sa68 + 0x200 + slot * 4..sa68 + 0x200 + slot * 4 + 2]
            .copy_from_slice(&pid.to_le_bytes());
        corrupt[sa68 + 0x400 + slot * 0x200..sa68 + 0x400 + (slot + 1) * 0x200].fill(0xA5);
    }

    let sa34 = eh55_bgo_sector_offset(34).unwrap();
    corrupt[sa34..sa34 + EH55_BGO_SECTOR_STRIDE].fill(0xFF);
    corrupt[sa34 + 0x100..sa34 + 0x200].fill(0x00);

    let plan = build_bgo_repair_plan(&corrupt, None).unwrap();
    assert!(plan.post_audit.healthy);
    assert!(plan.post_audit.p2_missing_ids.is_empty());
    assert!(plan.post_audit.corrupt_sas.is_empty());
    assert_eq!(plan.post_audit.l2p_tag_mismatches, 0);
    assert_eq!(plan.erased_sas, vec![68]);

    let repaired = plan.repaired_nor_image.as_ref().unwrap();
    for slot in 0..=19usize {
        let tag_pid = read_le_u16(repaired, sa27 + 0x200 + slot * 4);
        assert_ne!(
            tag_pid, 0xFFFF,
            "SA27 slot {slot} must not retain 0xFFFF hole"
        );
    }
    for hole_slot in [14usize, 15, 17] {
        assert_eq!(
            read_le_u16(repaired, sa27 + 0x200 + hole_slot * 4),
            0x0000,
            "SA27 hole slot {hole_slot} must be zeroed"
        );
    }
    for (slot, pid) in [
        (16usize, 0x013Fu16),
        (18usize, 0x01ACu16),
        (19usize, 0x01C1u16),
    ] {
        assert_eq!(
            &repaired[sa27 + 0x200 + slot * 4..sa27 + 0x200 + slot * 4 + 2],
            &pid.to_le_bytes()
        );
        assert!(
            repaired[sa27 + 0x400 + slot * 0x200..sa27 + 0x400 + (slot + 1) * 0x200]
                .iter()
                .all(|&b| b == 0xA5)
        );
    }
}
