use panadiga_core::constants::{PANA_DIGADIAG_RAW_VER061_SIZE, resolve_workspace_root};
use panadiga_core::crypto::{FrmContainerMode, FrmPackModule, pack_frm};
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, SdLoaderProfile, SdUnbrickKitRequest,
    prepare_sd_unbrick_kit, prepare_sd_unbrick_kit_with_tier_policy, verify_sd_buffers,
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

fn build_workspace_sd_kit_with_mode(
    tag: &str,
    bgo_mode: Prog061BgoPatchMode,
    loader_profile: SdLoaderProfile,
    allow_unsafe_auto_format: bool,
    allow_unverified_profile: bool,
) -> Option<panadiga_core::re::SdUnbrickKitReport> {
    let ws = resolve_workspace_root();
    let dvd = ws.join("firmware_and_images/PANA_DVD.FRM");
    let p1 =
        ws.join("firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin");
    if !dvd.is_file() || !p1.is_file() {
        return None;
    }
    let tmp =
        std::env::temp_dir().join(format!("panadiga_unbrick_kit_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let req = SdUnbrickKitRequest {
        output_dir: tmp.clone(),
        pana_dvd_path: Some(dvd),
        prog061_path: None,
        main_path: Some(p1),
        drv_path: None,
        bgo_mode,
        main_passthrough_mode: MainPassthroughMode::AvPassthrough,
        loader_profile,
        include_pana_sd: true,
        unlock_hdd_dynamic: true,
        hdd_target_gb: None,
        write_extracted_sources: false,
        allow_unsafe_auto_format,
        expected_main_build: Some("3410PQ".to_string()),
        allow_target_mismatch: false,
        sata_bridge_reset_fix: false,
    };
    if loader_profile == SdLoaderProfile::Ver063ExtrheadSd64 && allow_unverified_profile {
        let blocked = prepare_sd_unbrick_kit(&req).unwrap_err();
        assert!(
            blocked.to_string().contains("RUNTIME TIER GATE BLOCKED"),
            "ver063 must fail closed by default: {blocked}"
        );
    }
    let result = prepare_sd_unbrick_kit_with_tier_policy(&req, allow_unverified_profile);
    let _ = fs::remove_dir_all(&tmp);
    Some(result.expect("workspace SD recovery kit roundtrip should succeed"))
}

#[test]
fn optional_real_workspace_sd_kit_roundtrip() {
    for (tag, mode, profile, allow_unverified, expected_label) in [
        (
            "bypass_061",
            Prog061BgoPatchMode::BypassChecksOnly,
            SdLoaderProfile::Ver061RawSd48,
            false,
            "BYPASS_CHECKS_ONLY",
        ),
        (
            "panamon1_061",
            Prog061BgoPatchMode::UartMonitor,
            SdLoaderProfile::Ver061RawSd48,
            false,
            "UART_MONITOR_PANAMON1",
        ),
        (
            "panamon1_063",
            Prog061BgoPatchMode::UartMonitor,
            SdLoaderProfile::Ver063ExtrheadSd64,
            true,
            "UART_MONITOR_PANAMON1",
        ),
    ] {
        let Some(report) =
            build_workspace_sd_kit_with_mode(tag, mode, profile, false, allow_unverified)
        else {
            return;
        };
        let expected_entries = match profile {
            SdLoaderProfile::Ver061RawSd48 => 6,
            SdLoaderProfile::Ver063ExtrheadSd64 => 3,
        };
        assert_eq!(report.loader_profile_applied, profile.as_str());
        assert!(report.verification.digadiag_exact_size_ok);
        assert_eq!(
            report.verification.digadiag_bgo_mode.as_deref(),
            Some(expected_label)
        );
        assert!(report.verification.pana_sd_present);
        assert_eq!(report.verification.pana_sd_toc_entries, expected_entries);
        assert!(report.verification.pana_sd_list_sums_ok);
        let expected_sd_prog_label = if profile == SdLoaderProfile::Ver061RawSd48
            && mode == Prog061BgoPatchMode::UartMonitor
        {
            "BYPASS_CHECKS_ONLY"
        } else {
            expected_label
        };
        assert_eq!(
            report.verification.pana_sd_prog061_mode.as_deref(),
            Some(expected_sd_prog_label)
        );
        assert_eq!(
            report.verification.pana_sd_main_passthrough_mode.as_deref(),
            Some("AV_PASSTHROUGH")
        );
        assert_eq!(report.verification.pana_sd_main_hdd_dynamic, Some(true));
    }
}

#[test]
fn auto_format_bgo_policy_fails_closed_without_explicit_override() {
    let ws = resolve_workspace_root();
    let dvd = ws.join("firmware_and_images/PANA_DVD.FRM");
    if !dvd.is_file() {
        return;
    }
    let tmp = std::env::temp_dir().join(format!("panadiga_unbrick_af_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let err = prepare_sd_unbrick_kit(&SdUnbrickKitRequest {
        output_dir: tmp.clone(),
        pana_dvd_path: Some(dvd),
        prog061_path: None,
        main_path: None,
        drv_path: None,
        bgo_mode: Prog061BgoPatchMode::AutoFormatBgo,
        main_passthrough_mode: MainPassthroughMode::FactoryOriginal,
        loader_profile: SdLoaderProfile::Ver061RawSd48,
        include_pana_sd: false,
        unlock_hdd_dynamic: false,
        hdd_target_gb: None,
        write_extracted_sources: false,
        allow_unsafe_auto_format: false,
        expected_main_build: None,
        allow_target_mismatch: false,
        sata_bridge_reset_fix: false,
    })
    .unwrap_err();
    let _ = fs::remove_dir_all(&tmp);
    assert!(
        err.to_string().contains("nfrom_format") || err.to_string().contains("AutoFormatBgo"),
        "unexpected error: {err}"
    );
}

#[test]
fn corrupt_pana_sd_payload_fails_fvu_checksum_verification() {
    let mut raw_diag = vec![0u8; PANA_DIGADIAG_RAW_VER061_SIZE];
    raw_diag[0..4].copy_from_slice(&[0xDC, 0x14, 0x00, 0x00]);
    let mut drv = FrmPackModule::new("DRV S211", vec![0x22u8; 64]);
    drv.hardware_family = "M6930LB".to_string();
    drv.hardware_family_aliases = vec!["M6930".to_string()];
    let (mut sd_bytes, pack_rep) = pack_frm(&[drv], FrmContainerMode::Sd48, None).unwrap();
    assert_eq!(pack_rep.modules.len(), 2);
    assert_eq!(
        pack_rep.modules[0].module_offset,
        pack_rep.modules[1].module_offset
    );

    let ok_rep = verify_sd_buffers("/mem/valid", Some(Ok(&raw_diag)), Some(&sd_bytes));
    assert_eq!(ok_rep.pana_sd_toc_entries, 2);
    assert_eq!(ok_rep.overall_status, "READY_WITH_HW_WARNINGS");

    // Corrupt a single payload byte inside shared module payload (TOC list_sums stay intact).
    let last = sd_bytes.len() - 1;
    sd_bytes[last] ^= 0x01;
    let bad_rep = verify_sd_buffers("/mem/corrupt", Some(Ok(&raw_diag)), Some(&sd_bytes));
    assert_eq!(bad_rep.overall_status, "INCOMPLETE_OR_INVALID");
    let sd_chk = bad_rep
        .checks
        .iter()
        .find(|c| c.id == "pana_sd_container")
        .unwrap();
    assert_eq!(sd_chk.severity, "fail");
    assert!(sd_chk.detail.contains("fvu_sums_ok=false"));
}
