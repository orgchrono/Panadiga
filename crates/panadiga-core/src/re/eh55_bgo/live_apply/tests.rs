#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use super::*;
use crate::constants::{AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};
use crate::pico::panamon1::MockPanamon1Transport;
use crate::re::eh55_bgo::repair_plan::build_bgo_repair_plan;
use crate::re::eh55_bgo::repair_plan::tests::build_synthetic_bgo_test_pair;
use crate::re::eh55_bgo::{audit_eh55_bgo_nor_8mb, eh55_bgo_sector_offset};
use crate::re::region_compat::TvStandardKind;

fn make_dirty_sa70_pair_and_plan() -> (Vec<u8>, crate::re::eh55_bgo::repair_plan::BgoRepairPlan) {
    let (mut corrupt, healthy) = build_synthetic_bgo_test_pair();
    let sa70 = eh55_bgo_sector_offset(70).unwrap();
    corrupt.copy_from_slice(&healthy);
    corrupt[sa70 + 0xE100..sa70 + 0xE200].fill(0x00);
    let plan = build_bgo_repair_plan(&corrupt, Some(&healthy)).unwrap();
    (corrupt, plan)
}

#[test]
fn live_apply_verifies_pre_sha_saves_backup_locks_and_verifies_post_sha() {
    let (corrupt, plan) = make_dirty_sa70_pair_and_plan();
    assert_eq!(plan.target_sectors, vec![70]);
    assert_eq!(plan.erased_sas, vec![70]);

    let backup_dir = std::env::temp_dir().join(format!("panadiga_bgo_live_{}", std::process::id()));
    let _ = fs::remove_dir_all(&backup_dir);

    let mut mock_deny = MockPanamon1Transport::new();
    mock_deny.map_memory(AM34_FLASH_UNCACHED_BASE, corrupt.clone());
    let mut client_deny = Panamon1Client::new(mock_deny);
    assert!(
        apply_bgo_repair_plan_live(
            &mut client_deny,
            &plan,
            &backup_dir,
            false,
            None,
            &mut |_| {}
        )
        .is_err()
    );

    let mut mock = MockPanamon1Transport::new();
    mock.bridge_active = true;
    mock.map_memory(AM34_FLASH_UNCACHED_BASE, corrupt);
    mock.boottrace_reply = "***** normal mode *****\r\nBOOTTRACE: OK\r\nPICO> ".to_string();
    let mut client = Panamon1Client::new(mock);

    let report = apply_bgo_repair_plan_live(
        &mut client,
        &plan,
        &backup_dir,
        true,
        Some(5000),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(report.pre_bgo_sha256, plan.source_bgo_sha256);
    assert_eq!(report.post_bgo_sha256, plan.expected_post_sha256);
    assert_eq!(report.erased_sectors, 1);
    assert_eq!(report.written_bytes, 256);
    assert_eq!(report.boot_normal_ok, Some(true));
    assert!(Path::new(&report.backup_path).is_file());
    assert!(!client.transport().nor_unlocked);

    let live_after = client
        .transport()
        .read_memory_bytes(AM34_FLASH_UNCACHED_BASE, AM34_FLASH_SIZE as usize)
        .unwrap();
    assert!(audit_eh55_bgo_nor_8mb(&live_after).unwrap().healthy);
    let _ = fs::remove_dir_all(&backup_dir);
}

#[test]
fn plan_live_bgo_heal_chunks_fast_writes_and_requires_confirm_token() {
    let (_corrupt, plan) = make_dirty_sa70_pair_and_plan();
    let cmd_plan = plan_live_bgo_heal_commands(&plan, Some(TvStandardKind::Ntsc)).unwrap();
    assert_eq!(cmd_plan.max_fast_write_bytes, 128);
    assert!(cmd_plan.commands.iter().any(|c| c == "BGO_INSPECT DETAIL"));
    assert!(
        cmd_plan
            .commands
            .iter()
            .any(|c| c == "FAST_READ 843F0000 00010000")
    );
    assert!(
        cmd_plan
            .commands
            .iter()
            .any(|c| c == "FAST_ERASE 843F0000 CONFIRM_UPPER_NOR")
    );
    let fast_writes: Vec<&String> = cmd_plan
        .commands
        .iter()
        .filter(|c| c.starts_with("FAST_WRITE "))
        .collect();
    assert_eq!(
        fast_writes.len(),
        2,
        "256 B write must split into two <=128 B chunks"
    );
    for fw in fast_writes {
        assert!(fw.ends_with(" CONFIRM_UPPER_NOR"));
        let hex_len = fw.split_whitespace().nth(2).unwrap().len();
        assert!(hex_len <= 256);
    }
    let tv_note = cmd_plan
        .tv_standard_note
        .expect("tv_standard_note expected");
    assert_eq!(tv_note.tv_standard, "ntsc");
    assert_eq!(tv_note.nfd_config1_pid_hex, "DYNAMIC(config1[page=9])");
    assert_eq!(tv_note.page_offset_hex, "0x00F0");
    assert_eq!(tv_note.value_byte_hex, "0x00");

    let pal_note = plan_live_bgo_heal_commands(&plan, Some(TvStandardKind::Pal))
        .unwrap()
        .tv_standard_note
        .expect("pal tv_standard_note expected");
    assert_eq!(pal_note.tv_standard, "pal");
    assert_eq!(pal_note.page_offset_hex, "0x00F0");
    assert_eq!(pal_note.value_byte_hex, "0x01");
    assert!(plan_live_bgo_heal_commands(&plan, Some(TvStandardKind::PalM)).is_err());
}

#[test]
fn heal_bgo_live_resolves_config1_page9_dynamically_from_fat_not_hardcoded_000c() {
    let (_corrupt, mut healthy) = build_synthetic_bgo_test_pair();
    // Place directory page 0x0F02 in F2_20 (SA59: 26..=28 (3) + 30..=40 (11) + 42..=58 (17) + 1 -> fid=32=0x20) slot 1.
    let sa59 = eh55_bgo_sector_offset(59).unwrap();
    healthy[sa59 + 0x200 + 4..sa59 + 0x200 + 6].copy_from_slice(&0x0F02u16.to_le_bytes());
    let dir_off = sa59 + 0x400 + 0x200;
    healthy[dir_off..dir_off + 8].copy_from_slice(b"config1\x00");
    healthy[dir_off + 8..dir_off + 12].copy_from_slice(&8812u32.to_le_bytes());
    healthy[dir_off + 12..dir_off + 14].copy_from_slice(&0x002Au16.to_le_bytes());

    // Chain 18 pages (0x002A..=0x003B -> 0xFFED) so config1[page=9] == 0x0033 (NOT 0x000C) in both FATs (fid=2 -> SA27, fid=17 -> SA44).
    for fat_sa in [27usize, 44] {
        let fbase = eh55_bgo_sector_offset(fat_sa).unwrap() + 0x400;
        for pid in 0x002Au16..=0x003B {
            let nxt = if pid == 0x003B { 0xFFED } else { pid + 1 };
            let off = fbase + usize::from(pid - 1) * 2;
            healthy[off..off + 2].copy_from_slice(&nxt.to_le_bytes());
        }
    }

    // Wire L2P tid=0x00F0 entry[2] (pid=0x0F02) inside SA29 to word 0x2002 (fid=0x20, slot=1).
    let sa29 = eh55_bgo_sector_offset(29).unwrap();
    let mut wired = false;
    for sp in 2..20usize {
        let off = sa29 + sp * 0x100;
        for s in 0..7usize {
            let soff = off + 4 + s * 0x24;
            if u16::from_le_bytes([healthy[soff], healthy[soff + 1]]) == 0x00F0 {
                healthy[soff + 4 + 2 * 2..soff + 4 + 2 * 2 + 2]
                    .copy_from_slice(&0x2002u16.to_le_bytes());
                wired = true;
            }
        }
    }
    assert!(wired, "expected synthetic SA29 P1 to host tid 0x00F0");

    let req = BgoLiveHealRequest {
        confirm_upper_nor: false,
        tv_standard: Some(TvStandardKind::Ntsc),
        dry_run: true,
    };
    let report = heal_bgo_live_with_client(
        None::<&mut Panamon1Client<MockPanamon1Transport>>,
        &healthy,
        Path::new("."),
        &req,
        &mut |_| {},
    )
    .unwrap();
    let note = report
        .command_plan
        .tv_standard_note
        .expect("tv_standard_note expected");
    assert_eq!(note.nfd_config1_pid_hex, "0x0033");
    assert_eq!(note.tv_standard, "ntsc");
    assert_eq!(note.page_offset_hex, "0x00F0");
    assert_eq!(note.value_byte_hex, "0x00");
}
