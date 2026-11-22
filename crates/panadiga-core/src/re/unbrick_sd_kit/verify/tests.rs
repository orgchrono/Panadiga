use super::*;

#[test]
fn missing_and_non_extrhead_buffers_fail_cleanly() {
    let rep_missing = verify_sd_buffers("/mem/sd", None, None);
    assert_eq!(rep_missing.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(!rep_missing.digadiag_present);

    let bad = vec![0xAAu8; 64];
    let rep_bad = verify_sd_buffers("/mem/sd", Some(Ok(&bad)), None);
    assert_eq!(rep_bad.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(rep_bad.digadiag_present);
    assert!(!rep_bad.digadiag_exact_size_ok);
}

#[test]
fn ver061_48b_sd_toc_and_raw_digadiag_pass_and_detect_mismatch() {
    let mut sd48 = vec![0u8; 0x1880];
    sd48[0..8].copy_from_slice(b"DRV S211");
    sd48[0x0C..0x10].copy_from_slice(&0x1800u32.to_le_bytes());
    sd48[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
    let sum: u16 = sd48[..0x2E]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
    sd48[0x2E..0x30].copy_from_slice(&sum.to_le_bytes());

    let mut raw_diag = vec![0u8; crate::constants::PANA_DIGADIAG_RAW_VER061_SIZE];
    raw_diag[0..4].copy_from_slice(&[0xDC, 0x14, 0x00, 0x00]);
    let rep = verify_sd_buffers("/mem/sd48", Some(Ok(&raw_diag)), Some(&sd48));
    assert!(rep.digadiag_present);
    assert!(rep.digadiag_exact_size_ok);
    assert!(rep.pana_sd_present);
    assert!(rep.pana_sd_list_sums_ok);
    assert_eq!(rep.overall_status, "READY_WITH_HW_WARNINGS");
    let sd_chk = rep
        .checks
        .iter()
        .find(|c| c.id == "pana_sd_container")
        .unwrap();
    assert_eq!(sd_chk.severity, "pass");
    assert!(sd_chk.title.contains("48B Plaintext SD TOC"));
    assert!(sd_chk.detail.contains("entry_stride=48B"));

    // Pairing 256 KiB raw ver0.61 DIGADIAG.FRM with 64B sd64 PANA__SD.FRM must fail consistency.
    let mut sd64 = vec![0u8; 0x2080];
    sd64[0..8].copy_from_slice(b"DRV S211");
    sd64[0x0C..0x10].copy_from_slice(&0x2000u32.to_le_bytes());
    sd64[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
    let sum64: u16 = sd64[..0x3E]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
    sd64[0x3E..0x40].copy_from_slice(&sum64.to_le_bytes());
    let rep_mismatch = verify_sd_buffers("/mem/mismatch", Some(Ok(&raw_diag)), Some(&sd64));
    assert_eq!(rep_mismatch.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(
        rep_mismatch
            .checks
            .iter()
            .any(|c| c.id == "loader_profile_mismatch" && c.severity == "fail")
    );
}

#[test]
fn hardening_gates_catch_bgo_geometry_mismatch_fvu_panamon_trap_and_report_rtsd() {
    use crate::constants::PANA_EXTRHEAD_DEFAULT_ADLER_STEP;
    use crate::crypto::{
        ExtrheadCryptMode, FrmContainerMode, FrmPackModule, pack_extrhead, pack_frm,
    };
    use crate::hdd::capacity_patch::codec::test_helpers::build_synthetic_hdd_cmd_main;
    use crate::hdd::capacity_patch::specs::MAIN_DEFAULT_BASE_VA;
    use crate::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};
    use crate::re::bgo_unbrick::{
        BASE_FACTORY_BGO_GEOMETRY_ID, ORIG_SUM_BEQ, Prog061BgoPatchMode,
        find_retarget_spec_by_loader_key, patch_prog061_bgo_in_place,
    };

    // 1. Real-marker PROG061 left in EH50 4 MiB geometry fails HardenedRecovery (`bgo_geometry_mismatch`).
    let mut eh50_marker_prog = super::super::synthetic_prog061_fixture();
    eh50_marker_prog[0x03_6E2C..0x03_6E30].copy_from_slice(&0x8000_0000u32.to_le_bytes());
    eh50_marker_prog[0x03_6E2C + 0x0C..0x03_6E2C + 0x10].copy_from_slice(&0x1Cu32.to_le_bytes());
    eh50_marker_prog[0x03_6E2C + 0x30..0x03_6E2C + 0x34]
        .copy_from_slice(&0x8420_0000u32.to_le_bytes());
    patch_prog061_bgo_in_place(&mut eh50_marker_prog, Prog061BgoPatchMode::UartMonitor).unwrap();

    let rep_geo_fail = verify_sd_buffers_with_profile(
        "/mem/eh50_in_ver061",
        Some(Ok(&eh50_marker_prog)),
        None,
        SdKitAuditProfile::HardenedRecovery,
    );
    assert_eq!(rep_geo_fail.overall_status, "INCOMPLETE_OR_INVALID");
    assert_eq!(
        rep_geo_fail.digadiag_bgo_geometry.as_deref(),
        Some(BASE_FACTORY_BGO_GEOMETRY_ID)
    );
    assert!(
        rep_geo_fail
            .checks
            .iter()
            .any(|c| c.id == "bgo_geometry_mismatch" && c.severity == "fail")
    );

    // 2. PANA__SD with UartMonitor PROG061 triggers `fvu_panamon_trap` warn, and 250 GB MAIN emits `rtsd_target_consistency`.
    let mut eh55_mon_prog = super::super::synthetic_prog061_fixture();
    eh55_mon_prog[0x03_6E2C..0x03_6E30].copy_from_slice(&0x8000_0000u32.to_le_bytes());
    eh55_mon_prog[0x03_6E2C + 0x0C..0x03_6E2C + 0x10].copy_from_slice(&0x28u32.to_le_bytes());
    eh55_mon_prog[0x03_6E2C + 0x30..0x03_6E2C + 0x34]
        .copy_from_slice(&0x8413_0000u32.to_le_bytes());
    let ver061_spec = find_retarget_spec_by_loader_key("ver061").unwrap();
    let eh55_sum_off = ver061_spec.bgo_sum_beq_offset as usize;
    eh55_mon_prog[eh55_sum_off..eh55_sum_off + 2].copy_from_slice(&ORIG_SUM_BEQ);
    patch_prog061_bgo_in_place(&mut eh55_mon_prog, Prog061BgoPatchMode::UartMonitor).unwrap();

    let mut synth_main = build_synthetic_hdd_cmd_main();
    apply_capacity_patch_in_place(
        &mut synth_main,
        HddCapacityPatchMode::FixedCapacity,
        Some(250),
    )
    .unwrap();
    let main_ctr = pack_extrhead(
        &synth_main,
        "MAIN",
        MAIN_DEFAULT_BASE_VA,
        "TEST3410PQ",
        ExtrheadCryptMode::Main,
        PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        0x3410,
    )
    .unwrap();

    let mut m_prog = FrmPackModule::new("PROG061 ", eh55_mon_prog.clone());
    m_prog.hardware_family = "M6930LB".to_string();
    m_prog.hardware_family_aliases = vec!["M6930".to_string()];
    m_prog.model_prefix = b'M';

    let mut m_main = FrmPackModule::new("MAIN3410", main_ctr);
    m_main.hardware_family = "M6930LB".to_string();
    m_main.hardware_family_aliases = vec!["M6930".to_string()];
    m_main.model_prefix = b'M';

    let (sd_bytes, _) = pack_frm(&[m_prog, m_main], FrmContainerMode::Sd48, None).unwrap();

    let rep_trap = verify_sd_buffers_with_profile(
        "/mem/trap_250gb",
        Some(Ok(&eh55_mon_prog)),
        Some(&sd_bytes),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert_eq!(rep_trap.overall_status, "READY_WITH_HW_WARNINGS");
    assert_eq!(
        rep_trap.digadiag_bgo_geometry.as_deref(),
        Some(ver061_spec.target_geometry_id.as_str())
    );
    assert_eq!(
        rep_trap.pana_sd_prog061_bgo_geometry.as_deref(),
        Some(ver061_spec.target_geometry_id.as_str())
    );
    assert_eq!(
        rep_trap.pana_sd_main_hdd_capacity_label.as_deref(),
        Some("250GB")
    );
    assert!(
        rep_trap
            .checks
            .iter()
            .any(|c| c.id == "fvu_panamon_trap" && c.severity == "warn")
    );
    assert!(
        rep_trap
            .checks
            .iter()
            .any(|c| c.id == "rtsd_target_consistency"
                && c.severity == "pass"
                && c.title.contains("250GB"))
    );
}
