#![deny(unsafe_code)]

//! Pure deterministic container builders for `DIGADIAG.FRM`, `MAIN` `EXTRHEAD`, `PANA__SD.FRM`, and `SHA256SUMS`.

use anyhow::{Result, bail};

use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, build_digadiag_frm, pack_extrhead, pack_frm,
};
use crate::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};
use crate::nvram::calculate_hashes;

use super::super::super::bgo_unbrick::{
    PROG061_BASE_VA, Prog061BgoPatchMode, find_recipe_by_loader_key,
    patch_prog061_bgo_in_place_with_policy, retarget_prog_for_loader_profile_in_place,
};
use super::super::super::main_passthrough::{
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, MainPassthroughMode, apply_main_passthrough_in_place,
    inspect_main_firmware_identity, inspect_main_passthrough_sites,
};
use super::super::super::unbrick_workbench::{load_main_binary_and_meta, load_prog061_from_bytes};
use super::types::{ComposedMainArtifact, SdLoaderProfile};

/// Pure composition of patched `PROG061` and `DIGADIAG.FRM` (256 KiB raw for `ver061`, or 128 KiB `EXTRHEAD` for `ver063`).
pub(crate) fn compose_digadiag_and_prog061(
    prog_raw: &[u8],
    bgo_mode: Prog061BgoPatchMode,
    loader_profile: SdLoaderProfile,
    allow_unsafe_auto_format: bool,
) -> Result<(Vec<u8>, Vec<u8>, String)> {
    let mut prog_patched = load_prog061_from_bytes(prog_raw)?;
    retarget_prog_for_loader_profile_in_place(&mut prog_patched, loader_profile.as_str())?;
    patch_prog061_bgo_in_place_with_policy(&mut prog_patched, bgo_mode, allow_unsafe_auto_format)?;
    let diag_bytes = match loader_profile {
        SdLoaderProfile::Ver061RawSd48 => prog_patched.clone(),
        SdLoaderProfile::Ver063ExtrheadSd64 => {
            build_digadiag_frm(
                &prog_patched,
                PROG061_BASE_VA,
                "FVU ",
                "PROG061",
                4,
                PANA_DIGADIAG_FRM_SIZE,
            )?
            .0
        }
    };
    let (_, sha) = calculate_hashes(&diag_bytes);
    Ok((prog_patched, diag_bytes, sha))
}

/// Composes the `PROG061 ` payload embedded inside `PANA__SD.FRM`.
///
/// For `ver061` (`DMR-EH55`) when serial boot `DIGADIAG.FRM` uses `UartMonitor` (`PANAMON1>`),
/// `PANA__SD.FRM`'s `PROG061 ` slot is built in `BypassChecksOnly` (FVU installer mode) so
/// normal boot with a blank HDD proceeds through `"SD version up mode."` into `MAIN3410PQ`.
pub(crate) fn compose_pana_sd_prog061(
    prog_raw: &[u8],
    requested_bgo_mode: Prog061BgoPatchMode,
    loader_profile: SdLoaderProfile,
    allow_unsafe_auto_format: bool,
) -> Result<(Vec<u8>, Prog061BgoPatchMode)> {
    let effective_mode = if loader_profile == SdLoaderProfile::Ver061RawSd48
        && requested_bgo_mode == Prog061BgoPatchMode::UartMonitor
    {
        Prog061BgoPatchMode::BypassChecksOnly
    } else {
        requested_bgo_mode
    };
    let (prog_patched, _, _) = compose_digadiag_and_prog061(
        prog_raw,
        effective_mode,
        loader_profile,
        allow_unsafe_auto_format,
    )?;
    Ok((prog_patched, effective_mode))
}

/// Pure composition of TBC-passthrough + optional HDD-capacity-unlocked `MAIN` container.
pub(crate) fn compose_patched_main_container(
    raw_main: &[u8],
    passthrough_mode: MainPassthroughMode,
    unlock_hdd_dynamic: bool,
    hdd_target_gb: Option<u32>,
    expected_main_build: Option<&str>,
    allow_target_mismatch: bool,
) -> Result<ComposedMainArtifact> {
    let (mut main_bin, build_name, flag, adler_step) = load_main_binary_and_meta(raw_main)?;
    let identity = inspect_main_firmware_identity(&main_bin, Some(&build_name));
    if let Some(expected) = expected_main_build
        && !allow_target_mismatch
        && !identity.build_id.eq_ignore_ascii_case(expected.trim())
    {
        bail!(
            "MAIN build mismatch: expected '{}', detected '{}' ({} B, model={:?}, chassis={:?}); pass --allow-target-mismatch to override",
            expected.trim(),
            identity.build_id,
            identity.size_bytes,
            identity.model_name,
            identity.chassis_family
        );
    }
    let mut modified = false;

    let passthrough_applied = if main_bin.len() == EXPECTED_MAIN_3410PQ_SIZE
        && identity.supports_3410pq_tbc_patch
        && inspect_main_passthrough_sites(&main_bin).is_ok()
    {
        apply_main_passthrough_in_place(&mut main_bin, passthrough_mode)?;
        if passthrough_mode == MainPassthroughMode::AvPassthrough {
            modified = true;
        }
        passthrough_mode.as_str().to_string()
    } else if passthrough_mode == MainPassthroughMode::AvPassthrough && !allow_target_mismatch {
        bail!(
            "AV TBC Passthrough sites (0x5771ABFB/0x574F7B8C/0x574F7C5C) apply ONLY to MAIN 3410PQ ({} B), detected '{}' ({} B); pass --passthrough-mode factory_original or --allow-target-mismatch",
            EXPECTED_MAIN_3410PQ_SIZE,
            identity.build_id,
            identity.size_bytes
        );
    } else {
        "skipped_non_3410pq_build".to_string()
    };

    let mut hdd_capacity_applied = None;
    if let Some(gb) = hdd_target_gb {
        if apply_capacity_patch_in_place(
            &mut main_bin,
            HddCapacityPatchMode::FixedCapacity,
            Some(gb),
        )
        .is_ok()
        {
            hdd_capacity_applied = Some(format!("cap_{gb}gb"));
            modified = true;
        }
    } else if unlock_hdd_dynamic
        && apply_capacity_patch_in_place(
            &mut main_bin,
            HddCapacityPatchMode::DynamicUnclamped,
            None,
        )
        .is_ok()
    {
        hdd_capacity_applied = Some("dynamic".to_string());
        modified = true;
    }

    let resolved_build = if identity.build_id != "UNKNOWN" {
        identity.build_id
    } else if !build_name.is_empty() {
        build_name
    } else {
        "3410PQ".to_string()
    };

    let container = if modified || !raw_main.starts_with(&[0x51, 0x87, 0x13, 0x8D]) {
        pack_extrhead(
            &main_bin,
            "MAIN",
            MAIN_BASE_VA,
            &resolved_build,
            ExtrheadCryptMode::Main,
            adler_step.max(16),
            flag.max(1),
        )?
    } else {
        raw_main.to_vec()
    };

    let slot_name = format!("MAIN{resolved_build}").chars().take(10).collect();
    let build_version_u16 = if resolved_build == "3060NP" {
        0x96AA
    } else {
        0x3410
    };

    Ok(ComposedMainArtifact {
        container,
        slot_name,
        build_version_u16,
        passthrough_applied,
        hdd_capacity_applied,
    })
}

/// Pure composition of `PANA__SD.FRM` (`sd48` for `ver061`, `sd64` for `ver063`) from constituent firmware payloads.
pub(crate) fn compose_pana_sd_container(
    drv_bytes: Option<Vec<u8>>,
    prog_patched: Vec<u8>,
    bgo_mode: Prog061BgoPatchMode,
    loader_profile: SdLoaderProfile,
    main_container: Vec<u8>,
    main_slot: &str,
    main_version_u16: u16,
) -> Result<(Vec<u8>, String)> {
    let mut sd_modules = Vec::new();
    if let Some(drv) = drv_bytes {
        let mut m = FrmPackModule::new("DRV S211", drv);
        m.version_u16 = 0x975E;
        sd_modules.push(m);
    }
    let mut prog_mod = FrmPackModule::new("PROG061 ", prog_patched);
    prog_mod.version_u16 = 0x1202;
    if bgo_mode == Prog061BgoPatchMode::FactoryOriginal
        && loader_profile != SdLoaderProfile::Ver061RawSd48
    {
        prog_mod.target_flag = 1;
    }
    sd_modules.push(prog_mod);

    let mut main_mod = FrmPackModule::new(main_slot, main_container);
    main_mod.version_u16 = main_version_u16;
    sd_modules.push(main_mod);

    if loader_profile == SdLoaderProfile::Ver061RawSd48
        && let Some(recipe) = find_recipe_by_loader_key(loader_profile.as_str())
        && let Some((primary_family, aliases)) = recipe.family_aliases.split_first()
    {
        for m in &mut sd_modules {
            m.hardware_family = primary_family.clone();
            m.hardware_family_aliases = aliases.to_vec();
            m.model_prefix = recipe.model_prefix_byte;
        }
    }

    let container_mode = match loader_profile {
        SdLoaderProfile::Ver061RawSd48 => FrmContainerMode::Sd48,
        SdLoaderProfile::Ver063ExtrheadSd64 => FrmContainerMode::Sd,
    };
    let (sd_bytes, _) = pack_frm(&sd_modules, container_mode, None)?;
    let (_, sd_sha) = calculate_hashes(&sd_bytes);
    Ok((sd_bytes, sd_sha))
}

/// Pure deterministic `SHA256SUMS.txt` text formatter.
pub(crate) fn format_sha256_manifest(entries: &[(String, &str)]) -> String {
    let mut out = String::new();
    for (sha, name) in entries {
        out.push_str(sha);
        out.push_str("  ");
        out.push_str(name);
        out.push('\n');
    }
    out
}
