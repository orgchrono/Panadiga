#![deny(unsafe_code)]

//! Hardened `Loader ver0.61` (`PANAMON1` + dual-family `M6930LB`/`M6930` `sd48`) SD kit composer.

use anyhow::{Result, bail};

use super::super::super::bgo_unbrick::Prog061BgoPatchMode;
use super::super::super::main_passthrough::MainPassthroughMode;
use super::super::verify::{
    SdKitAuditProfile, simulate_sd_find_firm_entry_dual, verify_sd_buffers_with_profile,
};
use super::builders::{
    compose_digadiag_and_prog061_with_tier_policy, compose_pana_sd_container,
    compose_pana_sd_prog061_with_tier_policy, compose_patched_main_container,
};
use super::types::{HardenedPanamon1KitArtifacts, SdLoaderProfile};

/// Pure composition & hardened verification of a `ver0.61` `PANAMON1` SD kit
/// (`262,144 B` raw `DIGADIAG.FRM` + dual-family `M6930LB`/`M6930` `sd48` `PANA__SD.FRM`).
#[allow(clippy::too_many_arguments)]
pub fn compose_hardened_panamon1_kit(
    drv_bytes: Option<Vec<u8>>,
    prog061_raw: &[u8],
    main_raw: &[u8],
    main_passthrough_mode: MainPassthroughMode,
    unlock_hdd_dynamic: bool,
    hdd_target_gb: Option<u32>,
    expected_main_build: Option<&str>,
    allow_target_mismatch: bool,
) -> Result<HardenedPanamon1KitArtifacts> {
    let (_prog_patched, digadiag_bytes, digadiag_sha256) =
        compose_digadiag_and_prog061_with_tier_policy(
            prog061_raw,
            Prog061BgoPatchMode::UartMonitor,
            SdLoaderProfile::Ver061RawSd48,
            false,
            false,
        )?;
    let (pana_sd_prog, pana_sd_bgo_mode) = compose_pana_sd_prog061_with_tier_policy(
        prog061_raw,
        Prog061BgoPatchMode::UartMonitor,
        SdLoaderProfile::Ver061RawSd48,
        false,
        false,
        false,
    )?;
    let composed_main = compose_patched_main_container(
        main_raw,
        main_passthrough_mode,
        unlock_hdd_dynamic,
        hdd_target_gb,
        expected_main_build,
        allow_target_mismatch,
        false,
    )?;
    let (pana_sd_bytes, pana_sd_sha256) = compose_pana_sd_container(
        drv_bytes,
        pana_sd_prog,
        pana_sd_bgo_mode,
        SdLoaderProfile::Ver061RawSd48,
        composed_main.container,
        &composed_main.slot_name,
        composed_main.build_version_u16,
    )?;

    let dual_parity = simulate_sd_find_firm_entry_dual(&pana_sd_bytes);
    let verification = verify_sd_buffers_with_profile(
        "mem://hardened_panamon1",
        Some(Ok(&digadiag_bytes)),
        Some(&pana_sd_bytes),
        SdKitAuditProfile::HardenedRecovery,
    );
    if !dual_parity.dual_parity_ok || verification.overall_status == "INCOMPLETE_OR_INVALID" {
        bail!(
            "Hardened PANAMON1 SD kit failed verification: parity_ok={}, status={}, issues={:?}",
            dual_parity.dual_parity_ok,
            verification.overall_status,
            dual_parity.issues
        );
    }

    Ok(HardenedPanamon1KitArtifacts {
        digadiag_bytes,
        digadiag_sha256,
        pana_sd_bytes,
        pana_sd_sha256,
        dual_parity,
        verification,
    })
}
