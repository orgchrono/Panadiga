#![deny(unsafe_code)]

use super::*;
use crate::hdd::writer::{BUS_TYPE_NVME, FileBackedMockFlashDevice};
use crate::re::bgo_unbrick::{Prog061BgoPatchMode, patch_prog061_bgo_in_place};
use crate::re::unbrick_sd_kit::creator::plan::SDXC_ADVISORY_THRESHOLD_BYTES;
use crate::re::unbrick_sd_kit::synthetic_prog061_fixture;

fn mock_drive_info(
    path: &str,
    size_bytes: u64,
    role: StorageMediaRole,
    sys: bool,
) -> PhysicalDriveInfo {
    PhysicalDriveInfo {
        device_path: path.into(),
        display_name: "Test SD".into(),
        size_bytes,
        size_gb: size_bytes as f64 / 1e9,
        is_system_drive: sys,
        safe_to_write: !sys,
        lockout_reason: sys.then(|| "OS drive".into()),
        drive_role: role,
        bus_type: "USB".into(),
        removable: role == StorageMediaRole::SdCard,
        is_mounted: false,
        mount_points: Vec::new(),
        partitions: Vec::new(),
    }
}

#[test]
fn test_sd_creator_mock_flash_lockout_and_sdxc_advisory() {
    let dir = std::env::temp_dir().join(format!("panadiga_sd_creator_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let diag_path = dir.join("DIGADIAG.FRM");
    let mut diag = synthetic_prog061_fixture();
    patch_prog061_bgo_in_place(&mut diag, Prog061BgoPatchMode::UartMonitor).unwrap();
    fs::write(&diag_path, &diag).unwrap();

    let disk_path = dir.join("sd_mock_1gb.bin");
    let mut dev = FileBackedMockFlashDevice::create(&disk_path, 128 * 1024 * 1024, 7).unwrap();
    let req = SdCardCreatorRequest {
        device_path: Some(r"\\.\PhysicalDrive9".into()),
        confirm_token: Some("CONFIRM-PhysicalDrive9".into()),
        digadiag_path: Some(diag_path),
        cleanroom_only: false,
        ..Default::default()
    };

    let hdd_info = mock_drive_info(
        r"\\.\PhysicalDrive9",
        128 * 1024 * 1024,
        StorageMediaRole::HardDisk,
        false,
    );
    assert!(flash_sd_recovery_card(&mut dev, &hdd_info, None, &req, |_| {}).is_err());

    let sd_info = mock_drive_info(
        r"\\.\PhysicalDrive9",
        128 * 1024 * 1024,
        StorageMediaRole::SdCard,
        false,
    );
    let mut nvme_dev =
        FileBackedMockFlashDevice::create(&dir.join("nvme.bin"), 128 * 1024 * 1024, BUS_TYPE_NVME)
            .unwrap();
    assert!(flash_sd_recovery_card(&mut nvme_dev, &sd_info, None, &req, |_| {}).is_err());

    let mut stages = Vec::new();
    let rep = flash_sd_recovery_card(&mut dev, &sd_info, Some(&["/dev/sda"]), &req, |p| {
        stages.push(p.stage)
    })
    .unwrap();
    assert!(rep.readback_verified);
    assert_eq!(rep.bytes_written, 65_599 * 512);
    assert!(!rep.advisory.is_sdxc_warning);
    assert!(dev.layout_deleted());
    assert_eq!(dev.flush_count(), 1);
    assert!(stages.contains(&SdCreatorStage::InspectFat16));
    assert!(rep.emu_gate.passed);

    let cr_img = dir.join("cleanroom_32mb.img");
    let cr_req = SdCardCreatorRequest {
        output_image_path: Some(cr_img.clone()),
        ..Default::default()
    };
    let cr_rep = create_sd_recovery_image(&cr_req, &cr_img).unwrap();
    assert!(cr_rep.cleanroom_only);
    assert!(cr_rep.emu_gate.passed);
    assert!(cr_rep.readback_verified);
    assert!(cr_rep.inspection.digadiag_payload_verified);
    assert!(cr_rep.inspection.checks.iter().any(|c| {
        c.title.contains("CLEANROOM_PANAMON1_BARE_METAL")
            || c.detail.contains("CLEANROOM_PANAMON1_BARE_METAL")
    }));

    assert!(assess_sd_card_size_advisory(SDXC_ADVISORY_THRESHOLD_BYTES).is_sdxc_warning);
    let _ = fs::remove_dir_all(&dir);
}
