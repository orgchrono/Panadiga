use std::fs;
use std::path::Path;

use super::patcher::PASSTHROUGH_SPECS;
use super::*;

fn build_synthetic_main_image() -> Vec<u8> {
    let mut buf = vec![0xFFu8; EXPECTED_MAIN_3410PQ_SIZE];
    for spec in &PASSTHROUGH_SPECS {
        let off = spec.file_offset().unwrap();
        buf[off..off + spec.expect.len()].copy_from_slice(spec.expect);
    }
    buf
}

#[test]
fn test_main_identity_detection_and_size_guard() {
    let mut fake_3060 = vec![0u8; 13_427_852];
    fake_3060[64..92].copy_from_slice(b"System Version : 3060NP (06)");
    fake_3060[128..136].copy_from_slice(b"DMR-EH55");
    fake_3060[140..145].copy_from_slice(b"M6930");
    let id = inspect_main_firmware_identity(&fake_3060, None);
    assert_eq!(id.build_id, "3060NP");
    assert_eq!(id.model_name.as_deref(), Some("DMR-EH55"));
    assert_eq!(id.chassis_family.as_deref(), Some("M6930"));
    assert!(!id.supports_3410pq_tbc_patch);
    assert!(inspect_main_passthrough_sites(&fake_3060).is_err());
}

#[test]
fn test_rsp_frame_formatting_and_checksums() {
    assert_eq!(format_rsp_frame("g"), "$g#67");
    let wr_cb = format_rsp_write_u32_frame(DEV_SERIAL_CALLBACK_VA, 0x5040_1F3C);
    assert_eq!(wr_cb, "$M515B2E40,4:3c1f4050#95");
    let wr_branch = format_rsp_write_mem_frame(0x574F_7B8C, &PATCH_ODD_STATE_BRANCH);
    assert!(wr_branch.starts_with("$M574F7B8C,4:ccd000cb#"));

    let plan = build_passthrough_rsp_plan(Some(0x5040_1F3C));
    assert_eq!(plan.handshake_hex, "0e0402");
    assert_eq!(plan.passthrough_frames.len(), 3);
    assert_eq!(plan.factory_restore_frames.len(), 3);
    assert_eq!(
        plan.callback_write_frame.as_deref(),
        Some("$M515B2E40,4:3c1f4050#95")
    );
}

#[test]
fn test_synthetic_inspect_patch_and_restore_round_trip() {
    let tmp = std::env::temp_dir().join(format!("panadiga_main_pass_{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();
    let in_path = tmp.join("EH55_MAIN_3410PQ_synth.bin");
    let out_path = tmp.join("EH55_MAIN_PASSTHROUGH.bin");

    let synth = build_synthetic_main_image();
    fs::write(&in_path, &synth).unwrap();

    let r_orig = inspect_main_passthrough_sites(&synth).unwrap();
    assert_eq!(r_orig.detected_mode, "FACTORY_ORIGINAL");
    assert!(r_orig.expected_size_match);

    let r_patch =
        patch_main_passthrough_file(&in_path, &out_path, MainPassthroughMode::AvPassthrough)
            .unwrap();
    assert_eq!(r_patch.detected_mode, "AV_PASSTHROUGH");
    assert_eq!(r_patch.applied_mode.as_deref(), Some("av_passthrough"));

    let mut patched = fs::read(&out_path).unwrap();
    let r_restored =
        apply_main_passthrough_in_place(&mut patched, MainPassthroughMode::FactoryOriginal)
            .unwrap();
    assert_eq!(r_restored.detected_mode, "FACTORY_ORIGINAL");
    assert_eq!(patched, synth);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_optional_real_eh55_main_3410pq_inspection() {
    let real = Path::new(r"C:\ghidra\binaries\unpacked\EH55_MAIN_3410PQ.bin");
    if !real.is_file() {
        return;
    }
    let data = fs::read(real).unwrap();
    let report = inspect_main_passthrough_sites(&data).unwrap();
    assert!(report.expected_size_match);
    assert_eq!(report.detected_mode, "FACTORY_ORIGINAL");
}
