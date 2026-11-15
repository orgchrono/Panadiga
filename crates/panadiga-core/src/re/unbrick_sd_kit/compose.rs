//! Pure deterministic artifact builders for `DIGADIAG.FRM`, `PANA__SD.FRM`, and `SHA256SUMS`.

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, build_digadiag_frm, pack_extrhead, pack_frm,
};
use crate::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};
use crate::nvram::calculate_hashes;

use super::super::bgo_unbrick::{
    PROG061_BASE_VA, Prog061BgoPatchMode, patch_prog061_bgo_in_place_with_policy,
};
use super::super::main_passthrough::{
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, MainPassthroughMode, apply_main_passthrough_in_place,
    inspect_main_firmware_identity, inspect_main_passthrough_sites,
};
use super::super::unbrick_workbench::{load_main_binary_and_meta, load_prog061_from_bytes};
use super::verify::SdMediaVerificationReport;
use anyhow::bail;

#[derive(Debug, Clone)]
pub struct SdUnbrickKitRequest {
    pub output_dir: PathBuf,
    pub pana_dvd_path: Option<PathBuf>,
    pub prog061_path: Option<PathBuf>,
    pub main_path: Option<PathBuf>,
    pub drv_path: Option<PathBuf>,
    pub bgo_mode: Prog061BgoPatchMode,
    pub main_passthrough_mode: MainPassthroughMode,
    pub include_pana_sd: bool,
    pub unlock_hdd_dynamic: bool,
    pub hdd_target_gb: Option<u32>,
    pub write_extracted_sources: bool,
    pub allow_unsafe_auto_format: bool,
    pub expected_main_build: Option<String>,
    pub allow_target_mismatch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdUnbrickKitReport {
    pub output_dir: String,
    pub prog061_source: String,
    pub main_source: Option<String>,
    pub drv_source: Option<String>,
    pub bgo_mode_applied: String,
    pub main_passthrough_applied: Option<String>,
    pub hdd_capacity_applied: Option<String>,
    pub digadiag_path: String,
    pub digadiag_sha256: String,
    pub pana_sd_path: Option<String>,
    pub pana_sd_sha256: Option<String>,
    pub sha256sums_path: String,
    pub verification: SdMediaVerificationReport,
}

pub(crate) struct ComposedMainArtifact {
    pub container: Vec<u8>,
    pub slot_name: String,
    pub build_version_u16: u16,
    pub passthrough_applied: String,
    pub hdd_capacity_applied: Option<String>,
}

/// Pure composition of patched `PROG061` and its 128 KiB `DIGADIAG.FRM` wrapper.
pub(crate) fn compose_digadiag_and_prog061(
    prog_raw: &[u8],
    bgo_mode: Prog061BgoPatchMode,
    allow_unsafe_auto_format: bool,
) -> Result<(Vec<u8>, Vec<u8>, String)> {
    let mut prog_patched = load_prog061_from_bytes(prog_raw)?;
    patch_prog061_bgo_in_place_with_policy(&mut prog_patched, bgo_mode, allow_unsafe_auto_format)?;
    let (diag_bytes, _) = build_digadiag_frm(
        &prog_patched,
        PROG061_BASE_VA,
        "FVU ",
        "PROG061",
        4,
        PANA_DIGADIAG_FRM_SIZE,
    )?;
    let (_, sha) = calculate_hashes(&diag_bytes);
    Ok((prog_patched, diag_bytes, sha))
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
    if unlock_hdd_dynamic || hdd_target_gb.is_some() {
        let cap_mode = if unlock_hdd_dynamic {
            HddCapacityPatchMode::DynamicUnclamped
        } else {
            HddCapacityPatchMode::FixedCapacity
        };
        if apply_capacity_patch_in_place(&mut main_bin, cap_mode, hdd_target_gb).is_ok() {
            hdd_capacity_applied = Some(if unlock_hdd_dynamic {
                "dynamic".to_string()
            } else {
                format!("cap_{}gb", hdd_target_gb.unwrap_or(0))
            });
            modified = true;
        }
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

/// Pure composition of 64-byte-TOC `PANA__SD.FRM` container from constituent firmware payloads.
pub(crate) fn compose_pana_sd_container(
    drv_bytes: Option<Vec<u8>>,
    prog_patched: Vec<u8>,
    bgo_mode: Prog061BgoPatchMode,
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
    if bgo_mode == Prog061BgoPatchMode::FactoryOriginal {
        prog_mod.target_flag = 1;
    }
    sd_modules.push(prog_mod);

    let mut main_mod = FrmPackModule::new(main_slot, main_container);
    main_mod.version_u16 = main_version_u16;
    sd_modules.push(main_mod);

    let (sd_bytes, _) = pack_frm(&sd_modules, FrmContainerMode::Sd, None)?;
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

#[cfg(test)]
mod tests {
    use super::super::verify::verify_sd_buffers;
    use super::*;
    use crate::re::bgo_unbrick::{
        OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, ORIG_A2CD_HOOK,
        ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PROG061_EXPECTED_SIZE,
    };

    #[test]
    fn compose_and_verify_digadiag_and_manifest_in_memory() {
        let mut prog = vec![0xFFu8; PROG061_EXPECTED_SIZE];
        prog[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
        prog[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2].copy_from_slice(&ORIG_DEVINFO_BEQ);
        prog[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20].copy_from_slice(&ORIG_A2CD_HOOK);

        assert!(
            compose_digadiag_and_prog061(&prog, Prog061BgoPatchMode::AutoFormatBgo, false).is_err()
        );

        let (_patched, diag, sha) =
            compose_digadiag_and_prog061(&prog, Prog061BgoPatchMode::BypassChecksOnly, false)
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
}
