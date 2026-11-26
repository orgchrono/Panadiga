use panadiga_core::constants::{PANA_DIGADIAG_RAW_VER061_SIZE, resolve_workspace_root};
use panadiga_core::crypto::{FrmContainerMode, FrmPackModule, pack_frm};
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, SdLoaderProfile, SdUnbrickKitRequest,
    prepare_sd_unbrick_kit, prepare_sd_unbrick_kit_with_tier_policy, verify_sd_buffers,
    verify_sd_unbrick_media,
};
use std::fs;
use std::path::PathBuf;

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
        sata_hdd_autoboot: false,
        main_bgo_retarget: false,
        nor_autoboot_capsule: false,
        m6930lb_localization: false,
        localization_tv_profile: None,
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
        sata_hdd_autoboot: false,
        main_bgo_retarget: false,
        nor_autoboot_capsule: false,
        m6930lb_localization: false,
        localization_tv_profile: None,
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

fn resolve_autoboot_workspace_sources() -> Option<(PathBuf, PathBuf, PathBuf)> {
    let ws = resolve_workspace_root();
    let dvd = ws.join("firmware_and_images/PANA_DVD.FRM");
    let main_unpacked =
        ws.join("firmware_and_images/extracted_partitions/frm_unpacked/main3060np_unpacked.bin");
    let prog_unpacked =
        ws.join("firmware_and_images/extracted_partitions/frm_unpacked/prog061_unpacked.bin");
    (dvd.is_file() && main_unpacked.is_file() && prog_unpacked.is_file()).then_some((
        dvd,
        main_unpacked,
        prog_unpacked,
    ))
}

#[test]
fn ver061_hdd_autoboot_fails_closed_without_main_bgo_retarget_and_passes_with_8mb_main() {
    use panadiga_core::constants::CHK_ID_MAIN_BGO_GEOMETRY;
    use panadiga_core::crypto::{ExtrheadCryptMode, pack_extrhead};
    use panadiga_core::re::bgo_unbrick::Prog061BgoPatchMode::BypassChecksOnly;
    use panadiga_core::re::main_bgo_retarget::{MAIN_BGO_STATE_8MB, MAIN_BGO_STATE_FACTORY_4MB};
    use panadiga_core::re::sata_hdd_autoboot::apply_prog061_hdd_autoboot_in_place;

    let tmp_guard =
        std::env::temp_dir().join(format!("panadiga_autoboot_guard_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_guard);
    let mut req = SdUnbrickKitRequest::hardened_panamon1(tmp_guard.clone());
    req.sata_bridge_reset_fix = true;
    req.sata_hdd_autoboot = true;
    let err = prepare_sd_unbrick_kit(&req).unwrap_err();
    assert!(
        err.to_string().contains("--main-bgo-retarget"),
        "expected fail-closed --main-bgo-retarget guard, got: {err}"
    );
    let _ = fs::remove_dir_all(&tmp_guard);

    let Some((dvd, main_unpacked, prog_unpacked)) = resolve_autoboot_workspace_sources() else {
        return;
    };

    let tmp_pass =
        std::env::temp_dir().join(format!("panadiga_autoboot_8mb_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_pass);
    let mut pass_req = SdUnbrickKitRequest::hardened_panamon1(tmp_pass.clone());
    pass_req.pana_dvd_path = Some(dvd);
    pass_req.prog061_path = Some(prog_unpacked.clone());
    pass_req.main_path = Some(main_unpacked.clone());
    pass_req.sata_bridge_reset_fix = true;
    pass_req.sata_hdd_autoboot = true;
    pass_req.main_bgo_retarget = true;
    let ok_rep = prepare_sd_unbrick_kit(&pass_req).unwrap();
    assert!(ok_rep.main_bgo_retarget_applied);
    assert_eq!(
        ok_rep.verification.pana_sd_main_bgo_geometry.as_deref(),
        Some(MAIN_BGO_STATE_8MB)
    );
    assert!(
        ok_rep
            .verification
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_MAIN_BGO_GEOMETRY && c.severity == "pass")
    );
    let _ = fs::remove_dir_all(&tmp_pass);

    // Construct an in-memory ver061 kit pairing SATA HDD autoboot PROG061 with factory 4 MiB MAIN3060NP:
    // verify_sd_buffers must fail closed with CHK_ID_MAIN_BGO_GEOMETRY == "fail".
    let mut prog_autoboot = fs::read(prog_unpacked).unwrap();
    let bgo_done = panadiga_core::re::bgo_unbrick::patch_prog061_bgo_in_place(
        &mut prog_autoboot,
        BypassChecksOnly,
    );
    bgo_done.unwrap();
    apply_prog061_hdd_autoboot_in_place(&mut prog_autoboot, true).unwrap();
    let main_4mb_ctr = pack_extrhead(
        &fs::read(main_unpacked).unwrap(),
        "MAIN",
        panadiga_core::constants::AM34_MAIN_BASE_VA,
        "3060NP",
        ExtrheadCryptMode::Main,
        16,
        1,
    )
    .unwrap();
    let mut m_prog = FrmPackModule::new("PROG061 ", prog_autoboot);
    m_prog.hardware_family = "M6930LB".to_string();
    m_prog.hardware_family_aliases = vec!["M6930".to_string()];
    m_prog.model_prefix = b'M';
    let mut m_main = FrmPackModule::new("MAIN3060NP", main_4mb_ctr);
    m_main.hardware_family = "M6930LB".to_string();
    m_main.hardware_family_aliases = vec!["M6930".to_string()];
    m_main.model_prefix = b'M';
    let (bad_sd, _) = pack_frm(&[m_prog, m_main], FrmContainerMode::Sd48, None).unwrap();
    let bad_rep = verify_sd_buffers("/mem/autoboot_4mb_fail", None, Some(&bad_sd));
    assert_eq!(bad_rep.overall_status, "INCOMPLETE_OR_INVALID");
    assert_eq!(
        bad_rep.pana_sd_main_bgo_geometry.as_deref(),
        Some(MAIN_BGO_STATE_FACTORY_4MB)
    );
    assert!(
        bad_rep
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_MAIN_BGO_GEOMETRY && c.severity == "fail")
    );
}

#[test]
fn nor_autoboot_capsule_sd_kit_fails_closed_and_roundtrips_naut_payload() {
    use panadiga_core::constants::{
        CHK_ID_MAIN_BGO_GEOMETRY, CHK_ID_SATA_HDD_AUTOBOOT, PANA_DIGADIAG_RAW_VER061_SIZE,
    };
    use panadiga_core::crypto::parse_sd_frm_toc_with_layout;
    use panadiga_core::re::{
        NOR_AUTOBOOT_CAPSULE_MAGIC_LE, NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET,
        inspect_prog061_hdd_autoboot_site, unpack_nor_autoboot_capsule,
    };

    let tmp_guard =
        std::env::temp_dir().join(format!("panadiga_naut_guard_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_guard);
    let mut guard_req = SdUnbrickKitRequest::hardened_panamon1(tmp_guard.clone());
    guard_req.nor_autoboot_capsule = true;
    let err = prepare_sd_unbrick_kit(&guard_req).unwrap_err();
    let _ = fs::remove_dir_all(&tmp_guard);
    assert!(
        err.to_string().contains("--sata-hdd-autoboot"),
        "expected fail-closed --sata-hdd-autoboot guard, got: {err}"
    );

    let Some((dvd, main_unpacked, prog_unpacked)) = resolve_autoboot_workspace_sources() else {
        return;
    };

    let tmp_ok = std::env::temp_dir().join(format!("panadiga_naut_ok_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_ok);
    let mut ok_req = SdUnbrickKitRequest::hardened_panamon1(tmp_ok.clone());
    ok_req.pana_dvd_path = Some(dvd);
    ok_req.prog061_path = Some(prog_unpacked);
    ok_req.main_path = Some(main_unpacked);
    ok_req.sata_bridge_reset_fix = true;
    ok_req.sata_hdd_autoboot = true;
    ok_req.main_bgo_retarget = true;
    ok_req.nor_autoboot_capsule = true;
    let rep = prepare_sd_unbrick_kit(&ok_req).unwrap();
    assert!(rep.nor_autoboot_capsule_applied);
    assert_eq!(rep.verification.overall_status, "READY_WITH_HW_WARNINGS");
    for chk_id in [CHK_ID_MAIN_BGO_GEOMETRY, CHK_ID_SATA_HDD_AUTOBOOT] {
        assert!(
            rep.verification
                .checks
                .iter()
                .any(|c| c.id == chk_id && c.severity == "pass"),
            "Missing pass check {chk_id}: {:?}",
            rep.verification.checks
        );
    }

    let sd_bytes = fs::read(rep.pana_sd_path.as_deref().unwrap()).unwrap();
    let (_, toc) = parse_sd_frm_toc_with_layout(&sd_bytes);
    let prog_entry = toc
        .iter()
        .find(|e| e.module_name.trim() == "PROG061")
        .expect("PROG061 TOC slot");
    let off = prog_entry.module_offset as usize;
    let sz = prog_entry.module_size as usize;
    assert_eq!(sz, PANA_DIGADIAG_RAW_VER061_SIZE);
    let slot = &sd_bytes[off..off + sz];
    assert_eq!(
        &slot[NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET..NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET + 4],
        &NOR_AUTOBOOT_CAPSULE_MAGIC_LE.to_le_bytes()
    );
    assert!(slot[0x2_0000..].iter().all(|&b| b == 0xFF));
    let inner = unpack_nor_autoboot_capsule(slot).expect("valid zlib self-unpacker");
    assert_eq!(
        inspect_prog061_hdd_autoboot_site(&inner).unwrap().state,
        "SATA_HDD_AUTOBOOT"
    );
    let _ = fs::remove_dir_all(&tmp_ok);
}
