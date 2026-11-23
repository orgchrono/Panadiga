#![deny(unsafe_code)]

//! Unit tests for SD unbrick kit artifact builders and hardened `PANAMON1` composition.

use std::path::PathBuf;

use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::re::bgo_unbrick::{
    PROG061_EXPECTED_SIZE, Prog061BgoPatchMode, inspect_prog061_bgo_sites,
};
use crate::re::main_passthrough::MainPassthroughMode;

use super::super::verify::verify_sd_buffers;
use super::super::{synthetic_packed_main_fixture, synthetic_prog061_fixture};
use super::{
    SdKitBuildPreset, SdLoaderProfile, SdUnbrickKitRequest, compose_digadiag_and_prog061,
    compose_digadiag_and_prog061_with_tier_policy, compose_hardened_panamon1_kit,
    compose_patched_main_container, format_sha256_manifest,
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

    let err_t2 = compose_digadiag_and_prog061(
        &prog,
        Prog061BgoPatchMode::BypassChecksOnly,
        SdLoaderProfile::Ver063ExtrheadSd64,
        false,
    )
    .unwrap_err();
    assert!(err_t2.to_string().contains("RUNTIME TIER GATE BLOCKED"));

    let (_patched, diag, sha) = compose_digadiag_and_prog061_with_tier_policy(
        &prog,
        Prog061BgoPatchMode::BypassChecksOnly,
        SdLoaderProfile::Ver063ExtrheadSd64,
        false,
        true,
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
            false,
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
            false,
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
            None,
            true,
            true,
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
        artifacts.verification.digadiag_bgo_mode.as_deref(),
        Some("UART_MONITOR_PANAMON1")
    );
    assert_eq!(
        artifacts.verification.pana_sd_prog061_mode.as_deref(),
        Some("BYPASS_CHECKS_ONLY")
    );
}

#[test]
fn reproduces_canonical_sd_unbrick_kit_byte_for_byte_if_present() {
    let kit_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("firmware_and_images")
        .join("sd_unbrick_kit");
    let pana_sd_path = kit_dir.join("PANA__SD.FRM");
    let digadiag_path = kit_dir.join("DIGADIAG.FRM");
    if !pana_sd_path.is_file() || !digadiag_path.is_file() {
        return;
    }

    let pana_sd = std::fs::read(&pana_sd_path).unwrap();
    let canonical_digadiag = std::fs::read(&digadiag_path).unwrap();
    assert_eq!(canonical_digadiag.len(), PROG061_EXPECTED_SIZE);
    assert_eq!(pana_sd.len(), 7_563_224);

    let drv = pana_sd[0x0000_1800..0x000B_1800].to_vec();
    let prog = pana_sd[0x000B_1800..0x000F_1800].to_vec();
    let main_mod = pana_sd[0x000F_1800..0x0073_67D8].to_vec();
    let expected_geo = crate::re::bgo_unbrick::find_retarget_spec_by_loader_key("ver061")
        .expect("SSOT ver061 retarget spec")
        .target_geometry_id
        .as_str();
    let diag_insp = inspect_prog061_bgo_sites(&canonical_digadiag).unwrap();
    assert_eq!(diag_insp.detected_mode, "UART_MONITOR_PANAMON1");
    assert_eq!(diag_insp.bgo_target_geometry, expected_geo);
    let sd_prog_insp = inspect_prog061_bgo_sites(&prog).unwrap();
    assert_eq!(sd_prog_insp.detected_mode, "BYPASS_CHECKS_ONLY");
    assert_eq!(sd_prog_insp.bgo_target_geometry, expected_geo);

    let (rebuilt_sd, sd_sha) = super::builders::compose_pana_sd_container(
        Some(drv),
        prog,
        Prog061BgoPatchMode::BypassChecksOnly,
        SdLoaderProfile::Ver061RawSd48,
        main_mod,
        "MAIN3060NP",
        0x96AA,
    )
    .unwrap();
    assert_eq!(
        sd_sha,
        "2d3d62a75731f150762f269e3b650e83307da983d8d4bf222ec078b45298d9d0"
    );
    assert_eq!(rebuilt_sd, pana_sd);

    let raw_prog_path =
        std::path::Path::new(r"C:\ghidra\binaries\unpacked\eh55_frm_PROG061_BOOT.bin");
    if raw_prog_path.is_file() {
        let raw_prog = std::fs::read(raw_prog_path).unwrap();
        let (_patched, digadiag_bytes, digadiag_sha) = compose_digadiag_and_prog061(
            &raw_prog,
            Prog061BgoPatchMode::UartMonitor,
            SdLoaderProfile::Ver061RawSd48,
            false,
        )
        .unwrap();
        assert_eq!(
            digadiag_sha,
            "305cc2eebc09bbaa3bb5c5d75c0395eb16831bed4b9f80eafd57b9945e6a762d"
        );
        assert_eq!(digadiag_bytes.len(), PROG061_EXPECTED_SIZE);
        let rep = verify_sd_buffers("/mem/sd", Some(Ok(&digadiag_bytes)), None);
        assert_eq!(
            rep.digadiag_bgo_mode.as_deref(),
            Some("UART_MONITOR_PANAMON1")
        );
    }
}

fn synthetic_packed_main(build_name: &str, version_u16: u16) -> Vec<u8> {
    synthetic_packed_main_fixture(build_name, version_u16, None)
}

#[test]
fn compose_patched_main_gives_hdd_target_gb_precedence_over_dynamic() {
    use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};
    use crate::hdd::capacity_patch::inspect_main_buffer_capacity;
    use crate::hdd::capacity_patch::specs::MAIN_DEFAULT_BASE_VA;

    let packed = synthetic_packed_main("TEST3410PQ", 0x3410);

    let composed = compose_patched_main_container(
        &packed,
        MainPassthroughMode::FactoryOriginal,
        true,
        Some(250),
        None,
        true,
        false,
        false,
    )
    .unwrap();
    assert_eq!(composed.hdd_capacity_applied.as_deref(), Some("cap_250gb"));

    let unpacked = unpack_extrhead(&composed.container, ExtrheadCryptMode::Main).unwrap();
    let cap = inspect_main_buffer_capacity(
        &unpacked.decompressed,
        MAIN_DEFAULT_BASE_VA,
        "EXTRHEAD_CONTAINER",
        Some(unpacked.header.build_name),
        "test",
    )
    .unwrap();
    assert!(!cap.is_dynamic_unclamped);
    assert_eq!(cap.slot_fallback_blocks_2kb, 0x0746_6528);
    assert!((cap.effective_default_limit_gb - 250.0).abs() < 1.0);
}

#[test]
fn compose_patched_main_accepts_3060np_and_3410pq_and_explicit_frm_precedence() {
    use crate::re::unbrick_workbench::{ExtractedFrmModuleReport, FrmModuleExtractionReport};

    let req = SdUnbrickKitRequest::hardened_panamon1(PathBuf::from("/tmp/sd_flex"));
    assert_eq!(req.expected_main_build, None);

    let packed_3060 = synthetic_packed_main("3060NP", 0x3060);
    let out_3060 = compose_patched_main_container(
        &packed_3060,
        MainPassthroughMode::FactoryOriginal,
        true,
        Some(250),
        req.expected_main_build.as_deref(),
        false,
        false,
        false,
    )
    .unwrap();
    assert_eq!(out_3060.build_version_u16, 0x96AA);
    assert_eq!(out_3060.slot_name, "MAIN3060NP");
    assert_eq!(out_3060.passthrough_applied, "skipped_non_3410pq_build");
    assert_eq!(out_3060.hdd_capacity_applied.as_deref(), Some("cap_250gb"));

    let packed_3410 = synthetic_packed_main("TEST3410PQ", 0x3410);
    let out_3410 = compose_patched_main_container(
        &packed_3410,
        MainPassthroughMode::FactoryOriginal,
        true,
        None,
        req.expected_main_build.as_deref(),
        false,
        false,
        false,
    )
    .unwrap();
    assert_eq!(out_3410.build_version_u16, 0x3410);
    assert_eq!(out_3410.hdd_capacity_applied.as_deref(), Some("dynamic"));

    let frm_rep = Some(FrmModuleExtractionReport {
        source_frm_path: "custom/PANA_DVD.FRM".to_string(),
        container_format: "feistel64_ecb".to_string(),
        output_dir: "tmp".to_string(),
        modules: vec![ExtractedFrmModuleReport {
            name: "MAIN3060NP".to_string(),
            offset_hex: "0x0F1800".to_string(),
            encrypted_size: 123,
            decrypted_path: "tmp/main3060np_dec.bin".to_string(),
            decrypted_sha256: "00".repeat(32),
            unpacked_path: None,
            unpacked_size: None,
            dst_addr_hex: None,
            checksum_valid: None,
        }],
    });
    let disc_fallback = Some(PathBuf::from("workspace/hdd_part1_3410pq.bin"));
    let explicit_override = Some(PathBuf::from("explicit/MAIN_CUSTOM.bin"));

    assert_eq!(
        super::super::io::resolve_component_path(
            &None,
            true,
            &frm_rep,
            "MAIN",
            disc_fallback.clone()
        ),
        Some(PathBuf::from("tmp/main3060np_dec.bin"))
    );
    assert_eq!(
        super::super::io::resolve_component_path(
            &None,
            false,
            &frm_rep,
            "MAIN",
            disc_fallback.clone()
        ),
        disc_fallback
    );
    assert_eq!(
        super::super::io::resolve_component_path(&explicit_override, true, &frm_rep, "MAIN", None),
        explicit_override
    );

    let out_200_stock = compose_patched_main_container(
        &packed_3060,
        MainPassthroughMode::FactoryOriginal,
        false,
        Some(200),
        None,
        false,
        false,
        false,
    )
    .unwrap();
    assert_eq!(
        out_200_stock.hdd_capacity_applied.as_deref(),
        Some("factory_200gb")
    );
    assert_eq!(out_200_stock.container, packed_3060);
}

#[test]
fn sata_bridge_reset_fix_patches_pana_sd_prog_without_touching_digadiag() {
    use crate::re::sata_bridge_fix::{
        SATA_BRIDGE_STATE_FACTORY_2MS, SATA_BRIDGE_STATE_WAIT_1000MS,
        inspect_prog061_sata_bridge_reset_site,
    };

    let prog = synthetic_prog061_fixture();
    let (_p, diag_bytes, _) = compose_digadiag_and_prog061(
        &prog,
        Prog061BgoPatchMode::UartMonitor,
        SdLoaderProfile::Ver061RawSd48,
        false,
    )
    .unwrap();
    assert_eq!(
        inspect_prog061_sata_bridge_reset_site(&diag_bytes)
            .unwrap()
            .state,
        SATA_BRIDGE_STATE_FACTORY_2MS
    );

    let (sd_prog, mode) = super::compose_pana_sd_prog061_with_tier_policy(
        &prog,
        Prog061BgoPatchMode::UartMonitor,
        SdLoaderProfile::Ver061RawSd48,
        false,
        false,
        true,
        false,
    )
    .unwrap();
    assert_eq!(mode, Prog061BgoPatchMode::BypassChecksOnly);
    assert_eq!(
        inspect_prog061_sata_bridge_reset_site(&sd_prog)
            .unwrap()
            .state,
        SATA_BRIDGE_STATE_WAIT_1000MS
    );

    // Stage-2 fail-closed without reset fix, and patches only PROG061 (leaving DIGADIAG untouched) when enabled.
    assert!(
        super::compose_pana_sd_prog061_with_tier_policy(
            &prog,
            Prog061BgoPatchMode::UartMonitor,
            SdLoaderProfile::Ver061RawSd48,
            false,
            false,
            false,
            true,
        )
        .is_err()
    );
    let (sd_stage2_prog, mode2) = super::compose_pana_sd_prog061_with_tier_policy(
        &prog,
        Prog061BgoPatchMode::UartMonitor,
        SdLoaderProfile::Ver061RawSd48,
        false,
        false,
        true,
        true,
    )
    .unwrap();
    assert_eq!(mode2, Prog061BgoPatchMode::BypassChecksOnly);
    assert_eq!(
        crate::re::sata_bridge_stage2::inspect_prog061_stage2_boot_site(&sd_stage2_prog)
            .unwrap()
            .state,
        crate::re::sata_bridge_stage2::PROG061_STAGE2_STATE_ENABLED
    );
    assert_eq!(
        crate::re::sata_bridge_stage2::inspect_prog061_stage2_boot_site(&diag_bytes)
            .unwrap()
            .state,
        crate::re::sata_bridge_stage2::PROG061_STAGE2_STATE_FACTORY
    );

    // Non-3060NP container fails closed when sata_bridge_reset_fix = true or main_bgo_retarget = true.
    let packed = synthetic_packed_main("3060NP", 0x96AA);
    assert!(
        compose_patched_main_container(
            &packed,
            MainPassthroughMode::FactoryOriginal,
            false,
            None,
            None,
            true,
            true,
            false,
        )
        .is_err()
    );
    assert!(
        compose_patched_main_container(
            &packed,
            MainPassthroughMode::FactoryOriginal,
            false,
            None,
            None,
            true,
            false,
            true,
        )
        .is_err()
    );
}
