#![deny(unsafe_code)]

use std::cell::RefCell;
use std::path::Path;

use super::burn_os::{build_imapi2_burn_script, build_linux_burn_args, build_macos_burn_args};
use super::*;

#[derive(Default)]
struct MockOpticalBackend {
    burned: RefCell<Vec<String>>,
}

impl OpticalBurnBackend for MockOpticalBackend {
    fn enumerate_burners(&self) -> Vec<OpticalBurnerDriveInfo> {
        vec![OpticalBurnerDriveInfo {
            device_id: "/dev/sr0".into(),
            display_name: "HL-DT-ST DVDRAM GH24NSC0".into(),
            os_backend: "mock".into(),
            supported_write_speeds_kbps: vec![1385, 5540],
            min_write_speed_kbps: 1385,
            min_write_speed_label: "1385 KB/s (1x DVD)".into(),
            safe_optical_target: true,
            expected_confirm_token: expected_optical_confirm_token("/dev/sr0"),
        }]
    }

    fn build_burn_command_preview(&self, burner_id: &str, iso_path: &Path) -> Result<String> {
        Ok(format!(
            "mock_burn --speed=1 --dev={burner_id} {}",
            iso_path.display()
        ))
    }

    fn burn_iso_lowest_speed(&self, burner_id: &str, iso_path: &Path) -> Result<String> {
        self.burned
            .borrow_mut()
            .push(format!("{burner_id}:{}", iso_path.display()));
        Ok("MOCK_BURN_1X_OK".into())
    }
}

#[test]
fn dvd_creator_synthesizes_iso9660_level1_enforces_emu_gate_and_lockout() {
    let dir = std::env::temp_dir().join(format!("panadiga_dvd_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let out_iso = dir.join("eh55_recovery.iso");

    let req = DvdCreatorRequest {
        output_iso_path: Some(out_iso.clone()),
        ..Default::default()
    };
    let rep = create_dvd_recovery_iso(&req, &out_iso).unwrap();
    assert!(rep.cleanroom_only);
    assert!(rep.emu_gate.passed);
    assert!(rep.inspection.valid_pvd);
    assert!(rep.inspection.valid_terminator);
    assert_eq!(rep.inspection.logical_block_size, 2048);
    assert_eq!(rep.inspection.overall_status, "PASS");
    assert_eq!(rep.write_speed_policy, "1x_or_drive_minimum_dao");
    assert_eq!(rep.files[0].name_8_3, "DIGADIAG.FRM");

    let mock = MockOpticalBackend::default();
    let bad_req = DvdCreatorRequest {
        burner_id: Some("/dev/sr0".into()),
        confirm_token: Some("WRONG".into()),
        dry_run: true,
        ..Default::default()
    };
    assert!(create_dvd_recovery_disc_with_backend(&bad_req, &mock, |_| {}).is_err());

    let ok_req = DvdCreatorRequest {
        burner_id: Some("/dev/sr0".into()),
        confirm_token: Some("CONFIRM-sr0".into()),
        dry_run: false,
        ..Default::default()
    };
    let burn_rep = create_dvd_recovery_disc_with_backend(&ok_req, &mock, |_| {}).unwrap();
    assert!(burn_rep.emu_gate.passed);
    assert_eq!(mock.burned.borrow().len(), 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cross_platform_1x_burner_builders_include_min_speed_and_dao_flags() {
    let iso = Path::new("recovery_eh55.iso");
    let win_script = build_imapi2_burn_script("DRIVE_ID_1", iso);
    assert!(win_script.contains("ForceMediaToBeClosed = $true"));
    assert!(win_script.contains("SetWriteSpeed"));
    assert!(win_script.contains("SupportedWriteSpeeds | Sort-Object"));

    let xor_args = build_linux_burn_args("xorriso", "/dev/sr0", iso);
    assert!(xor_args.contains(&"speed=1".to_string()));
    assert!(xor_args.contains(&"-dao".to_string()));

    let grow_args = build_linux_burn_args("growisofs", "/dev/sr0", iso);
    assert!(grow_args.contains(&"-speed=1".to_string()));

    let mac_args = build_macos_burn_args("IOBDServices/1", iso);
    assert!(mac_args.contains(&"-speed".to_string()));
    assert!(mac_args.contains(&"1".to_string()));
    assert!(mac_args.contains(&"-verifyburn".to_string()));
}
