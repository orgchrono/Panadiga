#![deny(unsafe_code)]

//! Offline unit tests for `eh55_flash` using [`FileBackedMockFlashDevice`].

use std::fs;
use std::path::Path;

use crate::constants::{LBA_MIRROR_MBR_OFFSET, PANASONIC_HDD_MBR_FULL_SIGNATURE};
use crate::nvram::calculate_hashes;

use super::super::types::{PhysicalDriveInfo, StorageMediaRole};
use super::backup_verify::verify_region_readback_sha256;
use super::{
    BUS_TYPE_NVME, BlockFlashDevice, Eh55FlashStage, Eh55HddFlashRequest,
    FileBackedMockFlashDevice, compute_eh55_mirror_main_lba, enrich_safe_target_disks,
    execute_eh55_hdd_flash,
};

const TEST_250GB_BYTES: u64 = 250_059_350_016; // 488,397,168 * 512

fn build_test_eh55_primary(len_bytes: usize) -> Vec<u8> {
    let mut img = vec![0x3Cu8; len_bytes];
    img[..512].fill(0);
    img[..16].copy_from_slice(&PANASONIC_HDD_MBR_FULL_SIGNATURE);
    img
}

fn make_req(
    dir: &Path,
    device_path: &str,
    token: &str,
    os_disks: Option<Vec<String>>,
) -> Eh55HddFlashRequest {
    Eh55HddFlashRequest {
        device_path: device_path.to_string(),
        confirmation_token: token.to_string(),
        image_path: dir.join("eh55_primary.img"),
        mirror_p1_path: Some(dir.join("eh55_p1_mirror.bin")),
        extract_p1_mirror_from_image: false,
        backup_dir: dir.join("backups"),
        expected_gb_range: None,
        detected_os_disks: os_disks,
    }
}

fn sample_drive(
    device_path: &str,
    display_name: &str,
    size_bytes: u64,
    is_system_drive: bool,
    bus_type: &str,
) -> PhysicalDriveInfo {
    PhysicalDriveInfo {
        device_path: device_path.to_string(),
        display_name: display_name.to_string(),
        size_bytes,
        size_gb: size_bytes as f64 / 1_000_000_000.0,
        is_system_drive,
        safe_to_write: !is_system_drive,
        lockout_reason: is_system_drive.then(|| "System OS".to_string()),
        drive_role: if is_system_drive {
            StorageMediaRole::SystemOs
        } else {
            StorageMediaRole::HardDisk
        },
        bus_type: bus_type.to_string(),
        removable: false,
        is_mounted: is_system_drive,
        mount_points: if is_system_drive {
            vec!["C:\\".to_string()]
        } else {
            Vec::new()
        },
        partitions: Vec::new(),
    }
}

fn init_bench_dir(
    tag: &str,
    primary_len: usize,
    mirror_len: usize,
) -> (std::path::PathBuf, Vec<u8>) {
    let dir = std::env::temp_dir().join(format!("panadiga_eh55_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("backups")).unwrap();
    let primary = build_test_eh55_primary(primary_len);
    fs::write(dir.join("eh55_primary.img"), &primary).unwrap();
    if mirror_len > 0 {
        fs::write(dir.join("eh55_p1_mirror.bin"), vec![0x7Du8; mirror_len]).unwrap();
    }
    (dir, primary)
}

#[test]
fn compute_mirror_main_lba_matches_250gb_field_geometry_and_guards_underflow() {
    assert_eq!(
        compute_eh55_mirror_main_lba(TEST_250GB_BYTES).unwrap(),
        488_266_096
    );
    assert!(compute_eh55_mirror_main_lba(1024 * 512).is_err());
}

#[test]
fn lockout_rejects_physicaldrive0_os_disk_and_nvme_before_mutating_mock_disk() {
    let (dir, _) = init_bench_dir("lockout", 1024 * 1024, 64 * 1024);

    let mock_path = dir.join("mock_disk.bin");
    let mut dev0 = FileBackedMockFlashDevice::create(&mock_path, TEST_250GB_BYTES, 3).unwrap();
    let req0 = make_req(&dir, r"\\.\PhysicalDrive0", "CONFIRM-PhysicalDrive0", None);
    assert!(execute_eh55_hdd_flash(&mut dev0, &req0, |_| {}).is_err());
    assert!(!dev0.layout_deleted());

    let req_os = make_req(
        &dir,
        r"\\.\PhysicalDrive2",
        "CONFIRM-PhysicalDrive2",
        Some(vec![r"\\.\PhysicalDrive2".to_string()]),
    );
    assert!(execute_eh55_hdd_flash(&mut dev0, &req_os, |_| {}).is_err());
    assert!(!dev0.layout_deleted());

    let mut nvme_dev =
        FileBackedMockFlashDevice::create(&mock_path, TEST_250GB_BYTES, BUS_TYPE_NVME).unwrap();
    let req_nvme = make_req(
        &dir,
        r"\\.\PhysicalDrive1",
        "CONFIRM-PhysicalDrive1",
        Some(vec![]),
    );
    let err = execute_eh55_hdd_flash(&mut nvme_dev, &req_nvme, |_| {})
        .unwrap_err()
        .to_string();
    assert!(err.contains("BusType::Nvme (17)"));
    assert!(!nvme_dev.layout_deleted());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn enrich_safe_target_disks_marks_nvme_and_os_blocked() {
    let sample = vec![
        sample_drive(
            r"\\.\PhysicalDrive0",
            "OS NVMe",
            500_000_000_000,
            true,
            "NVMe",
        ),
        sample_drive(
            r"\\.\PhysicalDrive1",
            "External NVMe",
            TEST_250GB_BYTES,
            false,
            "NVMe",
        ),
        sample_drive(
            r"\\.\PhysicalDrive2",
            "Hitachi 250GB USB",
            TEST_250GB_BYTES,
            false,
            "USB",
        ),
    ];
    let enriched = enrich_safe_target_disks(&sample);
    assert!(enriched[0].is_os_blocked && !enriched[0].safe_to_flash);
    assert!(enriched[1].is_nvme_blocked && !enriched[1].safe_to_flash);
    assert!(enriched[2].safe_to_flash);
    assert_eq!(enriched[2].mirror_main_lba, Some(488_266_096));
}

#[test]
fn full_mock_flash_writes_64k_backup_mirror_and_detects_single_bit_tamper() {
    let (dir, primary) = init_bench_dir("flash", 2 * 1024 * 1024, 128 * 1024);

    let mock_path = dir.join("mock_250gb.bin");
    let mut dev = FileBackedMockFlashDevice::create(&mock_path, TEST_250GB_BYTES, 7).unwrap();
    let mut pre_lba0 = vec![0x11u8; 64 * 512];
    pre_lba0[510] = 0x55;
    pre_lba0[511] = 0xAA;
    let pre_mirror = vec![0x22u8; 64 * 512];
    dev.write_at(0, &pre_lba0).unwrap();
    dev.write_at(LBA_MIRROR_MBR_OFFSET, &pre_mirror).unwrap();
    let mut expected_backup = pre_lba0;
    expected_backup.extend_from_slice(&pre_mirror);
    let expected_backup_sha = calculate_hashes(&expected_backup).1;

    let req = make_req(
        &dir,
        r"\\.\PhysicalDrive1",
        "CONFIRM-PhysicalDrive1",
        Some(vec![]),
    );
    let mut stages = Vec::new();
    let rep = execute_eh55_hdd_flash(&mut dev, &req, |p| stages.push(p.stage)).unwrap();

    assert!(dev.layout_deleted());
    assert_eq!(dev.flush_count(), 1);
    assert!(rep.readback_verified);
    assert_eq!(rep.mirror_main_lba, 488_266_096);
    assert_eq!(rep.backup_64k_sha256, expected_backup_sha);
    assert_eq!(fs::read(&rep.backup_bin_path).unwrap().len(), 65_536);
    assert!(
        fs::read_to_string(&rep.backup_manifest_path)
            .unwrap()
            .contains(&expected_backup_sha)
    );
    for expected_stage in [
        Eh55FlashStage::Backup64k,
        Eh55FlashStage::WipeLayout,
        Eh55FlashStage::Write100m,
        Eh55FlashStage::WriteMirror,
        Eh55FlashStage::VerifySha256,
    ] {
        assert!(stages.contains(&expected_stage));
    }
    assert_eq!(stages.last().copied(), Some(Eh55FlashStage::Done));

    let mut sector = [0u8; 512];
    dev.read_at(1024, &mut sector).unwrap();
    sector[17] ^= 0x01;
    dev.write_at(1024, &sector).unwrap();
    let tamper_err = verify_region_readback_sha256(
        &dev,
        0,
        primary.len(),
        &rep.primary_sha256,
        "Primary EH55 image",
        &mut |_| {},
    )
    .unwrap_err()
    .to_string();
    assert!(tamper_err.contains("SHA-256 mismatch"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn capacity_bounds_support_5gb_to_2tb_and_reject_mismatched_drive_sizes() {
    let (dir, _) = init_bench_dir("5gb", 512 * 1024, 0);
    let sv0511d_bytes: u64 = 9_965_088 * 512;
    let mut dev_5gb =
        FileBackedMockFlashDevice::create(&dir.join("mock_5gb.bin"), sv0511d_bytes, 3).unwrap();

    let mut req = make_req(
        &dir,
        r"\\.\PhysicalDrive1",
        "CONFIRM-PhysicalDrive1",
        Some(vec![]),
    );
    req.mirror_p1_path = None;
    assert!(execute_eh55_hdd_flash(&mut dev_5gb, &req, |_| {}).is_err());
    assert!(!dev_5gb.layout_deleted());

    req.expected_gb_range = Some((5, 2200));
    let rep = execute_eh55_hdd_flash(&mut dev_5gb, &req, |_| {}).unwrap();
    assert_eq!(rep.mirror_main_lba, 9_834_016);
    assert!(rep.readback_verified);

    let _ = fs::remove_dir_all(&dir);
}
