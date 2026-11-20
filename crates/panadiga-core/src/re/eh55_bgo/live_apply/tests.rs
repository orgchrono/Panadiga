#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use super::*;
use crate::constants::{AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};
use crate::pico::panamon1::MockPanamon1Transport;
use crate::re::eh55_bgo::repair_plan::build_bgo_repair_plan;
use crate::re::eh55_bgo::repair_plan::tests::build_synthetic_bgo_test_pair;
use crate::re::eh55_bgo::{audit_eh55_bgo_nor_8mb, eh55_bgo_sector_offset};

#[test]
fn live_apply_verifies_pre_sha_saves_backup_locks_and_verifies_post_sha() {
    let (mut corrupt, healthy) = build_synthetic_bgo_test_pair();
    let sa70 = eh55_bgo_sector_offset(70).unwrap();
    corrupt.copy_from_slice(&healthy);
    corrupt[sa70 + 0xE100..sa70 + 0xE200].fill(0x00);

    let plan = build_bgo_repair_plan(&corrupt, Some(&healthy)).unwrap();
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
    mock.boottrace_reply = "***** normal mode *****\r\nBOOTTRACE: OK\r\nPANADIGA> ".to_string();
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
