#![deny(unsafe_code)]

//! Unit tests for SD unbrick kit artifact builders and hardened `PANAMON1` composition.

use std::path::PathBuf;

use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::re::bgo_unbrick::{PROG061_EXPECTED_SIZE, Prog061BgoPatchMode};
use crate::re::main_passthrough::MainPassthroughMode;

use super::super::synthetic_prog061_fixture;
use super::super::verify::verify_sd_buffers;
use super::{
    SdKitBuildPreset, SdLoaderProfile, SdUnbrickKitRequest, compose_digadiag_and_prog061,
    compose_hardened_panamon1_kit, compose_patched_main_container, format_sha256_manifest,
};

#[test]
fn compose_and_verify_digadiag_and_manifest_in_memory() {
    let prog = synthetic_prog061_fixture();

    assert!(
        compose_digadiag_and_prog061(
            &prog,
            Prog061BgoPatchMode::AutoFormatBgo,
            SdLoaderProfile::Ver061RawSd48,
            false,
        )
        .is_err()
    );

    let (_patched061, diag061, sha061) = compose_digadiag_and_prog061(
        &prog,
        Prog061BgoPatchMode::BypassChecksOnly,
        SdLoaderProfile::Ver061RawSd48,
        false,
    )
    .unwrap();
    assert_eq!(diag061.len(), PROG061_EXPECTED_SIZE);
    assert_eq!(sha061.len(), 64);

    let (_patched, diag, sha) = compose_digadiag_and_prog061(
        &prog,
        Prog061BgoPatchMode::BypassChecksOnly,
        SdLoaderProfile::Ver063ExtrheadSd64,
        false,
    )
    .unwrap();
    assert_eq!(diag.len(), PANA_DIGADIAG_FRM_SIZE);
    assert_eq!(sha.len(), 64);

    let rep = verify_sd_buffers("/mem/sd", Some(Ok(&diag)), None);
    assert_eq!(rep.overall_status, "READY_WITH_HW_WARNINGS");
    assert_eq!(rep.digadiag_bgo_mode.as_deref(), Some("BYPASS_CHECKS_ONLY"));

    let fake_3060 = vec![0u8; 13_427_852];
    assert!(
        compose_patched_main_container(
            &fake_3060,
            MainPassthroughMode::AvPassthrough,
            false,
            None,
            None,
            false,
        )
        .is_err()
    );
    assert!(
        compose_patched_main_container(
            &fake_3060,
            MainPassthroughMode::FactoryOriginal,
            false,
            None,
            Some("3410PQ"),
            false,
        )
        .is_err()
    );

    let sums = format_sha256_manifest(&[(sha, "DIGADIAG.FRM")]);
    assert!(sums.ends_with("  DIGADIAG.FRM\n"));
}

#[test]
fn hardened_panamon1_preset_and_in_memory_composition_pass_dual_parity() {
    assert_eq!(
        "hardened_panamon1".parse::<SdKitBuildPreset>().unwrap(),
        SdKitBuildPreset::HardenedPanamon1
    );
    let req = SdUnbrickKitRequest::hardened_panamon1(PathBuf::from("/tmp/sd_kit"));
    assert_eq!(req.bgo_mode, Prog061BgoPatchMode::UartMonitor);
    assert_eq!(req.loader_profile, SdLoaderProfile::Ver061RawSd48);

    let prog = synthetic_prog061_fixture();

    let main_bin = vec![0xA5u8; 10_000_000];
    let artifacts = compose_hardened_panamon1_kit(
        Some(vec![0x33u8; 128]),
        &prog,
        &main_bin,
        MainPassthroughMode::FactoryOriginal,
        false,
        None,
        None,
        true,
    )
    .unwrap();
    assert_eq!(artifacts.digadiag_bytes.len(), PROG061_EXPECTED_SIZE);
    assert!(artifacts.dual_parity.dual_parity_ok);
    assert!(!artifacts.verification.factory_unpatched_prog061_detected);
    assert_eq!(artifacts.verification.pana_sd_toc_entries, 6);
    assert_eq!(
        artifacts.verification.pana_sd_prog061_mode.as_deref(),
        Some("UART_MONITOR_PANAMON1")
    );
}
