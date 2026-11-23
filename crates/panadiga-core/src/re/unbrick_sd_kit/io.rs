//! Thin imperative filesystem shell for SD kit creation and verification.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::constants::resolve_workspace_root;
use crate::nvram::calculate_hashes;

use super::super::bgo_unbrick::ensure_parent_dir;
use super::super::unbrick_workbench::{bgo_mode_label, discover_sources, extract_modules_from_frm};
use super::compose::{
    SdLoaderProfile, SdUnbrickKitReport, SdUnbrickKitRequest,
    compose_digadiag_and_prog061_with_tier_policy, compose_pana_sd_container,
    compose_pana_sd_prog061_with_tier_policy, compose_patched_main_container,
    format_sha256_manifest,
};
use super::verify::{SdKitAuditProfile, SdMediaVerificationReport, verify_sd_buffers_with_profile};

/// Validates an SD root directory containing `DIGADIAG.FRM` and/or `PANA__SD.FRM` under `profile`.
pub fn verify_sd_unbrick_media_with_profile(
    sd_root_dir: &Path,
    profile: SdKitAuditProfile,
) -> SdMediaVerificationReport {
    let diag_path = sd_root_dir.join("DIGADIAG.FRM");
    let pana_sd_path = sd_root_dir.join("PANA__SD.FRM");

    let diag_owned: Option<std::result::Result<Vec<u8>, String>> = diag_path
        .is_file()
        .then(|| fs::read(&diag_path).map_err(|e| e.to_string()));
    let diag_ref = diag_owned
        .as_ref()
        .map(|res| res.as_deref().map_err(String::as_str));

    let pana_owned = pana_sd_path
        .is_file()
        .then(|| fs::read(&pana_sd_path).ok())
        .flatten();

    verify_sd_buffers_with_profile(
        &sd_root_dir.to_string_lossy(),
        diag_ref,
        pana_owned.as_deref(),
        profile,
    )
}

/// Validates an SD root directory containing `DIGADIAG.FRM` and/or `PANA__SD.FRM`
/// under [`SdKitAuditProfile::Standard`].
pub fn verify_sd_unbrick_media(sd_root_dir: &Path) -> SdMediaVerificationReport {
    verify_sd_unbrick_media_with_profile(sd_root_dir, SdKitAuditProfile::Standard)
}

fn find_extracted_module(
    frm_report: &Option<super::super::unbrick_workbench::FrmModuleExtractionReport>,
    prefix: &str,
) -> Option<PathBuf> {
    frm_report.as_ref().and_then(|r| {
        r.modules
            .iter()
            .find(|m| m.name.to_ascii_uppercase().starts_with(prefix))
            .map(|m| PathBuf::from(&m.decrypted_path))
    })
}

pub(crate) fn resolve_component_path(
    explicit_path: &Option<PathBuf>,
    explicit_frm: bool,
    frm_report: &Option<super::super::unbrick_workbench::FrmModuleExtractionReport>,
    module_prefix: &str,
    disc_fallback: Option<PathBuf>,
) -> Option<PathBuf> {
    explicit_path.clone().or_else(|| {
        if explicit_frm {
            find_extracted_module(frm_report, module_prefix).or(disc_fallback)
        } else {
            disc_fallback.or_else(|| find_extracted_module(frm_report, module_prefix))
        }
    })
}

/// Builds a complete verified SD Unbrick Kit (`DIGADIAG.FRM`, optional `PANA__SD.FRM`, & `SHA256SUMS.txt`),
/// enforcing the R1 runtime `< T3` tier-gate policy unless `allow_unverified_profile == true`.
pub fn prepare_sd_unbrick_kit_with_tier_policy(
    req: &SdUnbrickKitRequest,
    allow_unverified_profile: bool,
) -> Result<SdUnbrickKitReport> {
    if req.sata_bridge_reset_fix && !req.include_pana_sd {
        bail!(
            "--sata-bridge-reset-fix targets PANA__SD.FRM (PROG061 FVU + MAIN3060NP) and cannot be combined with --diag-only"
        );
    }
    if req.main_bgo_retarget && !req.include_pana_sd {
        bail!(
            "--main-bgo-retarget targets MAIN3060NP inside PANA__SD.FRM and cannot be combined with --diag-only"
        );
    }
    if req.sata_bridge_stage2_boot
        && (!req.include_pana_sd
            || !req.sata_bridge_reset_fix
            || req.loader_profile != SdLoaderProfile::Ver061RawSd48)
    {
        bail!(
            "--sata-bridge-stage2-boot requires PANA__SD.FRM (--include_pana_sd), --sata-bridge-reset-fix, and --loader-profile ver061 (fail-closed)"
        );
    }
    if req.sata_bridge_stage2_boot
        && req.loader_profile == SdLoaderProfile::Ver061RawSd48
        && !req.main_bgo_retarget
    {
        bail!(
            "Refusing ver061 Stage-2 SATA bridge kit without --main-bgo-retarget: stock 4 MiB MAIN3060NP fails BGO_Init() on 8 MiB EH55 NOR (src/nfd_api.c:228)"
        );
    }
    let ws_root = resolve_workspace_root();
    let sources = discover_sources(&ws_root);
    let disc_frm = sources.pana_dvd;
    let disc_prog = sources.prog061;
    let disc_main = sources.main_3410;
    let disc_drv = sources.drv;
    let explicit_frm = req.pana_dvd_path.is_some();
    let frm_path = req.pana_dvd_path.clone().or(disc_frm);

    let extracted_tmp = req.output_dir.join("_extracted_frm");
    let need_extract = req.prog061_path.is_none()
        || (req.include_pana_sd && (req.drv_path.is_none() || req.main_path.is_none()));
    let frm_report = match (frm_path, need_extract) {
        (Some(p), true) => Some(extract_modules_from_frm(&p, &extracted_tmp, false)?),
        _ => None,
    };

    let prog_src_path = req
        .prog061_path
        .clone()
        .or_else(|| find_extracted_module(&frm_report, "PROG"))
        .or(disc_prog)
        .context("No PROG061 or PANA_DVD.FRM source available to build DIGADIAG.FRM")?;

    let prog_raw = fs::read(&prog_src_path)
        .with_context(|| format!("Reading PROG061 source {}", prog_src_path.display()))?;
    let (prog_patched, diag_bytes, digadiag_sha256) =
        compose_digadiag_and_prog061_with_tier_policy(
            &prog_raw,
            req.bgo_mode,
            req.loader_profile,
            req.allow_unsafe_auto_format,
            allow_unverified_profile,
        )?;

    fs::create_dir_all(&req.output_dir)?;
    let digadiag_path = req.output_dir.join("DIGADIAG.FRM");
    fs::write(&digadiag_path, &diag_bytes)?;

    let mut manifest_entries: Vec<(String, &str)> = vec![(digadiag_sha256.clone(), "DIGADIAG.FRM")];
    if req.write_extracted_sources {
        let raw_out = req.output_dir.join("PROG061_PATCHED.bin");
        fs::write(&raw_out, &prog_patched)?;
        manifest_entries.push((calculate_hashes(&prog_patched).1, "PROG061_PATCHED.bin"));
    }

    let mut pana_sd_path = None;
    let mut pana_sd_sha256 = None;
    let mut main_source_label = None;
    let mut drv_source_label = None;
    let mut main_passthrough_applied = None;
    let mut hdd_capacity_applied = None;
    let mut sata_bridge_reset_fix_applied = false;
    let mut sata_bridge_stage2_boot_applied = false;
    let mut main_bgo_retarget_applied = false;

    if req.include_pana_sd {
        let drv_path =
            resolve_component_path(&req.drv_path, explicit_frm, &frm_report, "DRV", disc_drv);
        let main_path =
            resolve_component_path(&req.main_path, explicit_frm, &frm_report, "MAIN", disc_main)
                .context("No MAIN firmware or PANA_DVD.FRM source available for PANA__SD.FRM")?;

        main_source_label = Some(main_path.to_string_lossy().into_owned());
        let raw_main = fs::read(&main_path)
            .with_context(|| format!("Reading MAIN source {}", main_path.display()))?;
        let composed_main = compose_patched_main_container(
            &raw_main,
            req.main_passthrough_mode,
            req.unlock_hdd_dynamic,
            req.hdd_target_gb,
            req.expected_main_build.as_deref(),
            req.allow_target_mismatch,
            req.sata_bridge_reset_fix,
            req.main_bgo_retarget,
        )?;
        main_passthrough_applied = Some(composed_main.passthrough_applied);
        hdd_capacity_applied = composed_main.hdd_capacity_applied;
        sata_bridge_reset_fix_applied = composed_main.sata_bridge_reset_fix_applied;
        main_bgo_retarget_applied = composed_main.main_bgo_retarget_applied;

        let drv_bytes = match drv_path {
            Some(dp) => {
                drv_source_label = Some(dp.to_string_lossy().into_owned());
                Some(fs::read(&dp)?)
            }
            None => None,
        };

        let (pana_sd_prog, pana_sd_bgo_mode) = compose_pana_sd_prog061_with_tier_policy(
            &prog_raw,
            req.bgo_mode,
            req.loader_profile,
            req.allow_unsafe_auto_format,
            allow_unverified_profile,
            req.sata_bridge_reset_fix,
            req.sata_bridge_stage2_boot,
        )?;
        sata_bridge_stage2_boot_applied = req.sata_bridge_stage2_boot;
        let (sd_bytes, sd_sha) = compose_pana_sd_container(
            drv_bytes,
            pana_sd_prog,
            pana_sd_bgo_mode,
            req.loader_profile,
            composed_main.container,
            &composed_main.slot_name,
            composed_main.build_version_u16,
        )?;
        let out_sd = req.output_dir.join("PANA__SD.FRM");
        fs::write(&out_sd, &sd_bytes)?;
        manifest_entries.push((sd_sha.clone(), "PANA__SD.FRM"));
        pana_sd_path = Some(out_sd.to_string_lossy().into_owned());
        pana_sd_sha256 = Some(sd_sha);
    }

    if !req.write_extracted_sources && extracted_tmp.exists() {
        let _ = fs::remove_dir_all(&extracted_tmp);
    }

    let sums_path = req.output_dir.join("SHA256SUMS.txt");
    ensure_parent_dir(&sums_path)?;
    fs::write(&sums_path, format_sha256_manifest(&manifest_entries))?;
    let verification = verify_sd_unbrick_media(&req.output_dir);

    Ok(SdUnbrickKitReport {
        output_dir: req.output_dir.to_string_lossy().into_owned(),
        prog061_source: prog_src_path.to_string_lossy().into_owned(),
        main_source: main_source_label,
        drv_source: drv_source_label,
        bgo_mode_applied: bgo_mode_label(req.bgo_mode).to_string(),
        loader_profile_applied: req.loader_profile.as_str().to_string(),
        main_passthrough_applied,
        hdd_capacity_applied,
        sata_bridge_reset_fix_applied,
        sata_bridge_stage2_boot_applied,
        main_bgo_retarget_applied,
        digadiag_path: digadiag_path.to_string_lossy().into_owned(),
        digadiag_sha256,
        pana_sd_path,
        pana_sd_sha256,
        sha256sums_path: sums_path.to_string_lossy().into_owned(),
        verification,
    })
}

/// Fail-closed wrapper around [`prepare_sd_unbrick_kit_with_tier_policy`] (`allow_unverified_profile = false`).
pub fn prepare_sd_unbrick_kit(req: &SdUnbrickKitRequest) -> Result<SdUnbrickKitReport> {
    prepare_sd_unbrick_kit_with_tier_policy(req, false)
}
