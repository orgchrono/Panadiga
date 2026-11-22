#![deny(unsafe_code)]

//! Unit tests for `sd48` dual-family parity and hardened recovery profile audits.

use crate::crypto::{FrmContainerMode, FrmPackModule, pack_frm};
use crate::re::bgo_unbrick::{Prog061BgoPatchMode, patch_prog061_bgo_in_place};

use super::super::super::synthetic_prog061_fixture;
use super::super::verify_sd_buffers_with_profile;
use super::dual_parity::{FACTORY_PROG061_DATE_MARKER, SD48_FAMILY_ALIAS, SD48_FAMILY_PRIMARY};
use super::{SdKitAuditProfile, simulate_sd_find_firm_entry_dual};

fn make_synthetic_prog061(uart_monitor: bool) -> Vec<u8> {
    let mut prog = synthetic_prog061_fixture();
    prog[0x32E91..0x32E91 + 8].copy_from_slice(FACTORY_PROG061_DATE_MARKER);
    if uart_monitor {
        patch_prog061_bgo_in_place(&mut prog, Prog061BgoPatchMode::UartMonitor).unwrap();
    }
    prog
}

fn make_dual_sd48(prog: Vec<u8>, model_prefix: u8, include_alias: bool) -> Vec<u8> {
    let aliases = if include_alias {
        vec![SD48_FAMILY_ALIAS.to_string()]
    } else {
        Vec::new()
    };
    let mut m_prog = FrmPackModule::new("PROG061 ", prog);
    m_prog.hardware_family = SD48_FAMILY_PRIMARY.to_string();
    m_prog.hardware_family_aliases = aliases.clone();
    m_prog.model_prefix = model_prefix;

    let mut m_main = FrmPackModule::new("MAIN3410", vec![0x5Au8; 64]);
    m_main.hardware_family = SD48_FAMILY_PRIMARY.to_string();
    m_main.hardware_family_aliases = aliases;
    m_main.model_prefix = model_prefix;

    pack_frm(&[m_prog, m_main], FrmContainerMode::Sd48, None)
        .unwrap()
        .0
}

#[test]
fn dual_family_sd_find_firm_entry_checks_both_families_prefix_and_sums() {
    let diag = make_synthetic_prog061(true);
    let good_sd = make_dual_sd48(diag.clone(), b'M', true);
    let parity_ok = simulate_sd_find_firm_entry_dual(&good_sd);
    assert!(parity_ok.is_sd48_layout);
    assert!(parity_ok.dual_parity_ok);
    assert!(parity_ok.issues.is_empty());
    assert_eq!(parity_ok.m6930lb_prog061.as_ref().unwrap().slot_index, 0);
    assert_eq!(parity_ok.m6930_prog061.as_ref().unwrap().slot_index, 2);

    let missing_alias = make_dual_sd48(diag.clone(), b'M', false);
    let parity_missing = simulate_sd_find_firm_entry_dual(&missing_alias);
    assert!(!parity_missing.dual_parity_ok);
    assert!(!parity_missing.issues.is_empty());

    let bad_prefix = make_dual_sd48(diag, b'E', true);
    let parity_bad_pfx = simulate_sd_find_firm_entry_dual(&bad_prefix);
    assert!(!parity_bad_pfx.dual_parity_ok);
    assert!(
        parity_bad_pfx
            .issues
            .iter()
            .any(|i| i.contains("expected 0x4D"))
    );
}

#[test]
fn hardened_profile_blocks_factory_unpatched_prog061_while_accepting_panamon1() {
    let factory_prog = make_synthetic_prog061(false);
    let factory_sd = make_dual_sd48(factory_prog.clone(), b'M', true);
    let rep_fail = verify_sd_buffers_with_profile(
        "/mem/factory",
        Some(Ok(&factory_prog)),
        Some(&factory_sd),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert!(rep_fail.factory_unpatched_prog061_detected);
    assert_eq!(rep_fail.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(rep_fail.checks.iter().any(|c| {
        c.id == "factory_unpatched_prog061"
            && c.severity == "fail"
            && c.title.contains("FactoryUnpatchedProg061Detected")
    }));

    let panamon_prog = make_synthetic_prog061(true);
    let mut bypass_prog = make_synthetic_prog061(false);
    patch_prog061_bgo_in_place(&mut bypass_prog, Prog061BgoPatchMode::BypassChecksOnly).unwrap();
    let panamon_sd = make_dual_sd48(bypass_prog, b'M', true);
    let rep_pass = verify_sd_buffers_with_profile(
        "/mem/panamon1",
        Some(Ok(&panamon_prog)),
        Some(&panamon_sd),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert!(!rep_pass.factory_unpatched_prog061_detected);
    assert_eq!(rep_pass.overall_status, "READY_WITH_HW_WARNINGS");
    assert!(rep_pass.dual_family_parity.as_ref().unwrap().dual_parity_ok);
}
