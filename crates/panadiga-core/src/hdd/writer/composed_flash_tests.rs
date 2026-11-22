#![deny(unsafe_code)]

use std::fs;

use super::*;
use crate::constants::LBA_SECTOR_SIZE_BYTES;
use crate::hdd::writer::FileBackedMockFlashDevice;
use crate::re::emu_patch_suite::fixtures::build_synthetic_main_3410pq;

#[test]
fn streaming_composed_hdd_flash_writes_layout_and_patched_main_drv_without_100mb_img() {
    let dir = std::env::temp_dir().join(format!("panadiga_composed_hdd_{}", std::process::id()));
    let backup_dir = dir.join("backups");
    let _ = fs::create_dir_all(&backup_dir);
    let disk_path = dir.join("mock_250gb_sparse.img");
    let mock_size = 5 * 1024 * 1024 * 1024u64;
    let mut dev = FileBackedMockFlashDevice::create(&disk_path, mock_size, 7).unwrap();

    // 1. Clean-room layout_only mode (ZERO firmware files needed)
    let layout_req = ComposedHddFlashRequest {
        device_path: r"\\.\PhysicalDrive4".into(),
        confirmation_token: "CONFIRM-PhysicalDrive4".into(),
        backup_dir: backup_dir.clone(),
        expected_gb_range: Some((5, 260)),
        detected_os_disks: Some(vec![r"\\.\PhysicalDrive0".into()]),
        main_source_path: None,
        drv_source_path: None,
        apply_dynamic_capacity_unlock: false,
        target_capacity_gb: None,
        apply_av_passthrough: false,
        custom_guid: None,
    };
    let rep_layout = execute_composed_meihfs_hdd_flash(&mut dev, &layout_req, |_| {}).unwrap();
    assert!(rep_layout.emu_gate.passed);
    assert!(rep_layout.readback_verified);
    assert_eq!(rep_layout.p1_main_bytes_written, 0);

    // 2. Full composed mode: raw MAIN (patched & Mn103Vm verified) + raw DRV packed to EXTRHEAD
    let main_path = dir.join("MAIN_3410PQ.bin");
    let drv_path = dir.join("DRV_S211.bin");
    fs::write(&main_path, build_synthetic_main_3410pq()).unwrap();
    fs::write(&drv_path, vec![0x42u8; 4096]).unwrap();

    let comp_req = ComposedHddFlashRequest {
        main_source_path: Some(main_path),
        drv_source_path: Some(drv_path),
        apply_dynamic_capacity_unlock: true,
        apply_av_passthrough: true,
        ..layout_req
    };
    let rep = execute_composed_meihfs_hdd_flash(&mut dev, &comp_req, |_| {}).unwrap();
    assert!(rep.emu_gate.passed);
    assert!(rep.p1_main_bytes_written > 0);
    assert_eq!(rep.p1_main_bytes_written % LBA_SECTOR_SIZE_BYTES, 0);
    assert_eq!(rep.p1_main_bytes_written, rep.mirror_main_bytes_written);
    assert!(rep.p2_drv_bytes_written > 0);
    assert_eq!(rep.p1_main_sha256, rep.mirror_main_sha256);

    let _ = fs::remove_dir_all(&dir);
}
