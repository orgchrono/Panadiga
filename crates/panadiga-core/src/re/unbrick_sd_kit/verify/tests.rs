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

fn pack_ver061_pair_sd48(prog_bytes: Vec<u8>, main_ctr: Vec<u8>) -> Vec<u8> {
    use crate::crypto::{FrmContainerMode, FrmPackModule, pack_frm};
    let mut m_prog = FrmPackModule::new("PROG061 ", prog_bytes);
    m_prog.hardware_family = "M6930LB".to_string();
    m_prog.hardware_family_aliases = vec!["M6930".to_string()];
    m_prog.model_prefix = b'M';

    let mut m_main = FrmPackModule::new("MAIN3060NP", main_ctr);
    m_main.hardware_family = "M6930LB".to_string();
    m_main.hardware_family_aliases = vec!["M6930".to_string()];
    m_main.model_prefix = b'M';

    pack_frm(&[m_prog, m_main], FrmContainerMode::Sd48, None)
        .unwrap()
        .0
}

fn make_synthetic_main_250gb_ctr() -> Vec<u8> {
    super::super::synthetic_packed_main_fixture("TEST3410PQ", 0x3410, Some(250))
}

#[test]
fn hardening_gates_catch_bgo_geometry_mismatch_fvu_panamon_trap_and_report_rtsd() {
    use crate::re::bgo_unbrick::{
        BASE_FACTORY_BGO_GEOMETRY_ID, Prog061BgoPatchMode, patch_prog061_bgo_in_place,
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
    let (mut eh55_mon_prog, expected_geo) = super::super::synthetic_eh55_prog061_fixture();
    patch_prog061_bgo_in_place(&mut eh55_mon_prog, Prog061BgoPatchMode::UartMonitor).unwrap();

    let main_ctr = make_synthetic_main_250gb_ctr();
    let sd_bytes = pack_ver061_pair_sd48(eh55_mon_prog.clone(), main_ctr);

    let rep_trap_std = verify_sd_buffers(
        "/mem/trap_250gb_std",
        Some(Ok(&eh55_mon_prog)),
        Some(&sd_bytes),
    );
    assert_eq!(rep_trap_std.overall_status, "READY_WITH_HW_WARNINGS");
    assert!(
        rep_trap_std
            .checks
            .iter()
            .any(|c| c.id == "fvu_panamon_trap" && c.severity == "warn")
    );

    let rep_trap = verify_sd_buffers_with_profile(
        "/mem/trap_250gb",
        Some(Ok(&eh55_mon_prog)),
        Some(&sd_bytes),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert_eq!(rep_trap.overall_status, "INCOMPLETE_OR_INVALID");
    assert_eq!(
        rep_trap.digadiag_bgo_geometry.as_deref(),
        Some(expected_geo.as_str())
    );
    assert_eq!(
        rep_trap.pana_sd_prog061_bgo_geometry.as_deref(),
        Some(expected_geo.as_str())
    );
    assert_eq!(
        rep_trap.pana_sd_main_hdd_capacity_label.as_deref(),
        Some("250GB")
    );
    assert!(
        rep_trap
            .checks
            .iter()
            .any(|c| c.id == "fvu_panamon_trap" && c.severity == "fail")
    );
    assert!(
        rep_trap
            .checks
            .iter()
            .any(|c| c.id == "bgo_allowlist_invariant" && c.severity == "pass")
    );
    assert!(
        rep_trap
            .checks
            .iter()
            .any(|c| c.id == "rtsd_lba48_gate" && c.severity == "pass")
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

#[test]
fn sata_bridge_reset_fix_audit_enforces_symmetry_and_catches_corruption() {
    use crate::constants::CHK_ID_SATA_BRIDGE_RESET_FIX;
    use crate::re::bgo_unbrick::{Prog061BgoPatchMode, patch_prog061_bgo_in_place};
    use crate::re::sata_bridge_fix::{
        OFF_PROG061_SATA_BRIDGE_RESET_WAIT, SATA_BRIDGE_STATE_FACTORY_2MS,
        SATA_BRIDGE_STATE_WAIT_1000MS, SataBridgeResetFixMode,
        apply_prog061_sata_bridge_reset_fix_in_place,
    };

    let (mut eh55_mon_prog, _) = super::super::synthetic_eh55_prog061_fixture();

    let mut prog_sata = eh55_mon_prog.clone();
    patch_prog061_bgo_in_place(&mut prog_sata, Prog061BgoPatchMode::BypassChecksOnly).unwrap();
    apply_prog061_sata_bridge_reset_fix_in_place(
        &mut prog_sata,
        SataBridgeResetFixMode::Wait1000Ms,
    )
    .unwrap();
    patch_prog061_bgo_in_place(&mut eh55_mon_prog, Prog061BgoPatchMode::UartMonitor).unwrap();

    let main_ctr = make_synthetic_main_250gb_ctr();
    let sd_partial = pack_ver061_pair_sd48(prog_sata.clone(), main_ctr.clone());
    let rep_partial_hard = verify_sd_buffers_with_profile(
        "/mem/sata_partial_hard",
        Some(Ok(&eh55_mon_prog)),
        Some(&sd_partial),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert_eq!(rep_partial_hard.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(
        rep_partial_hard
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_SATA_BRIDGE_RESET_FIX && c.severity == "fail")
    );

    let mut prog_corrupt = prog_sata;
    prog_corrupt[OFF_PROG061_SATA_BRIDGE_RESET_WAIT] ^= 0x01;
    let sd_corrupt = pack_ver061_pair_sd48(prog_corrupt, main_ctr);
    let rep_corrupt = verify_sd_buffers(
        "/mem/sata_corrupt",
        Some(Ok(&eh55_mon_prog)),
        Some(&sd_corrupt),
    );
    assert_eq!(rep_corrupt.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(
        rep_corrupt
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_SATA_BRIDGE_RESET_FIX && c.severity == "fail")
    );

    // Direct symmetric-pass verification for PROG061 + MAIN 1000 ms pair and factory 2 ms silence.
    let mut checks = Vec::new();
    push_sata_bridge_fix_check(
        &mut checks,
        true,
        Some("3060NP"),
        Some(SATA_BRIDGE_STATE_WAIT_1000MS),
        Some(SATA_BRIDGE_STATE_WAIT_1000MS),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].id, CHK_ID_SATA_BRIDGE_RESET_FIX);
    assert_eq!(checks[0].severity, "pass");

    checks.clear();
    push_sata_bridge_fix_check(
        &mut checks,
        true,
        Some("3410PQ"),
        Some(SATA_BRIDGE_STATE_FACTORY_2MS),
        Some(SATA_BRIDGE_STATE_FACTORY_2MS),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert!(checks.is_empty());
}

#[test]
fn stage2_boot_audit_enforces_reset_fix_pairing_and_catches_corruption() {
    use crate::constants::CHK_ID_SATA_BRIDGE_STAGE2_BOOT;
    use crate::re::bgo_unbrick::{Prog061BgoPatchMode, patch_prog061_bgo_in_place};
    use crate::re::sata_bridge_fix::{
        SATA_BRIDGE_STATE_FACTORY_2MS, SATA_BRIDGE_STATE_WAIT_1000MS, SataBridgeResetFixMode,
        apply_prog061_sata_bridge_reset_fix_in_place,
    };
    use crate::re::sata_bridge_stage2::{
        OFF_PROG061_STAGE2_HOOK_SITE, PROG061_STAGE2_STATE_ENABLED, PROG061_STAGE2_STATE_MISMATCH,
        apply_prog061_stage2_boot_in_place,
    };

    let mut checks = Vec::new();
    push_stage2_boot_check(
        &mut checks,
        Some(PROG061_STAGE2_STATE_ENABLED),
        Some(SATA_BRIDGE_STATE_WAIT_1000MS),
        Some(SATA_BRIDGE_STATE_WAIT_1000MS),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].id, CHK_ID_SATA_BRIDGE_STAGE2_BOOT);
    assert_eq!(checks[0].severity, "pass");

    checks.clear();
    push_stage2_boot_check(
        &mut checks,
        Some(PROG061_STAGE2_STATE_ENABLED),
        Some(SATA_BRIDGE_STATE_FACTORY_2MS),
        Some(SATA_BRIDGE_STATE_WAIT_1000MS),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert_eq!(checks[0].severity, "fail");

    checks.clear();
    push_stage2_boot_check(
        &mut checks,
        Some(PROG061_STAGE2_STATE_MISMATCH),
        Some(SATA_BRIDGE_STATE_WAIT_1000MS),
        Some(SATA_BRIDGE_STATE_WAIT_1000MS),
        SdKitAuditProfile::Standard,
    );
    assert_eq!(checks[0].severity, "fail");

    let (mut mon_prog, _) = super::super::synthetic_eh55_prog061_fixture();
    let mut sd_prog = mon_prog.clone();
    patch_prog061_bgo_in_place(&mut mon_prog, Prog061BgoPatchMode::UartMonitor).unwrap();
    patch_prog061_bgo_in_place(&mut sd_prog, Prog061BgoPatchMode::BypassChecksOnly).unwrap();
    apply_prog061_sata_bridge_reset_fix_in_place(&mut sd_prog, SataBridgeResetFixMode::Wait1000Ms)
        .unwrap();
    apply_prog061_stage2_boot_in_place(&mut sd_prog, true).unwrap();
    sd_prog[OFF_PROG061_STAGE2_HOOK_SITE] ^= 0x01;
    let sd_bad = pack_ver061_pair_sd48(sd_prog, make_synthetic_main_250gb_ctr());
    let rep_bad = verify_sd_buffers("/mem/stage2_corrupt", Some(Ok(&mon_prog)), Some(&sd_bad));
    assert_eq!(rep_bad.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(
        rep_bad
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_SATA_BRIDGE_STAGE2_BOOT && c.severity == "fail")
    );
}

#[test]
fn main_bgo_geometry_audit_enforces_ver061_stage2_hardening() {
    use crate::constants::CHK_ID_MAIN_BGO_GEOMETRY;
    use crate::re::main_bgo_retarget::{
        MAIN_BGO_STATE_8MB, MAIN_BGO_STATE_FACTORY_4MB, MAIN_BGO_STATE_MISMATCH,
    };
    use crate::re::sata_bridge_stage2::PROG061_STAGE2_STATE_ENABLED;

    let cases = [
        (
            Some(MAIN_BGO_STATE_8MB),
            Some(SdLoaderProfile::Ver061RawSd48),
            Some(PROG061_STAGE2_STATE_ENABLED),
            Some("pass"),
        ),
        (
            Some(MAIN_BGO_STATE_FACTORY_4MB),
            Some(SdLoaderProfile::Ver061RawSd48),
            Some(PROG061_STAGE2_STATE_ENABLED),
            Some("fail"),
        ),
        (
            Some(MAIN_BGO_STATE_FACTORY_4MB),
            Some(SdLoaderProfile::Ver061RawSd48),
            None,
            Some("warn"),
        ),
        (
            Some(MAIN_BGO_STATE_FACTORY_4MB),
            Some(SdLoaderProfile::Ver063ExtrheadSd64),
            None,
            Some("pass"),
        ),
        (
            Some(MAIN_BGO_STATE_MISMATCH),
            Some(SdLoaderProfile::Ver061RawSd48),
            None,
            Some("fail"),
        ),
        (None, Some(SdLoaderProfile::Ver061RawSd48), None, None),
    ];

    for (geo, profile, stage2, expected_sev) in cases {
        let mut checks = Vec::new();
        push_main_bgo_geometry_check(&mut checks, Some("3060NP"), geo, profile, stage2);
        assert_eq!(checks.len(), usize::from(expected_sev.is_some()));
        if let Some(sev) = expected_sev {
            assert_eq!(checks[0].id, CHK_ID_MAIN_BGO_GEOMETRY);
            assert_eq!(checks[0].severity, sev);
        }
    }
}
