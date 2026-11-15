use panadiga_core::constants::resolve_workspace_root;
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, SdUnbrickKitRequest, prepare_sd_unbrick_kit,
    verify_sd_unbrick_media,
};
use std::fs;

#[test]
fn verify_sd_media_flags_missing_root() {
    let tmp = std::env::temp_dir().join(format!("panadiga_unbrick_empty_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let rep = verify_sd_unbrick_media(&tmp);
    assert!(!rep.digadiag_present);
    assert_eq!(rep.overall_status, "INCOMPLETE_OR_INVALID");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn optional_real_workspace_sd_kit_roundtrip() {
    let ws = resolve_workspace_root();
    let dvd = ws.join("firmware_and_images/PANA_DVD.FRM");
    let p1 =
        ws.join("firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin");
    if !dvd.is_file() || !p1.is_file() {
        return;
    }
    let tmp = std::env::temp_dir().join(format!("panadiga_unbrick_kit_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let report = prepare_sd_unbrick_kit(&SdUnbrickKitRequest {
        output_dir: tmp.clone(),
        pana_dvd_path: Some(dvd),
        prog061_path: None,
        main_path: Some(p1),
        drv_path: None,
        bgo_mode: Prog061BgoPatchMode::BypassChecksOnly,
        main_passthrough_mode: MainPassthroughMode::AvPassthrough,
        include_pana_sd: true,
        unlock_hdd_dynamic: true,
        hdd_target_gb: None,
        write_extracted_sources: false,
        allow_unsafe_auto_format: false,
        expected_main_build: Some("3410PQ".to_string()),
        allow_target_mismatch: false,
    })
    .unwrap();

    assert!(report.verification.digadiag_exact_size_ok);
    assert_eq!(
        report.verification.digadiag_bgo_mode.as_deref(),
        Some("BYPASS_CHECKS_ONLY")
    );
    assert!(report.verification.pana_sd_present);
    assert!(report.verification.pana_sd_list_sums_ok);
    assert_eq!(
        report.verification.pana_sd_prog061_mode.as_deref(),
        Some("BYPASS_CHECKS_ONLY")
    );
    assert_eq!(
        report.verification.pana_sd_main_passthrough_mode.as_deref(),
        Some("AV_PASSTHROUGH")
    );
    assert_eq!(report.verification.pana_sd_main_hdd_dynamic, Some(true));
    let _ = fs::remove_dir_all(&tmp);
}
