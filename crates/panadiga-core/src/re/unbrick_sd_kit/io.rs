//! Thin imperative filesystem shell for SD kit creation and verification.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::constants::resolve_workspace_root;
use crate::nvram::calculate_hashes;

use super::super::bgo_unbrick::ensure_parent_dir;
use super::super::unbrick_workbench::{bgo_mode_label, discover_sources, extract_modules_from_frm};
use super::compose::{
    SdUnbrickKitReport, SdUnbrickKitRequest, compose_digadiag_and_prog061,
    compose_pana_sd_container, compose_patched_main_container, format_sha256_manifest,
};
use super::verify::{SdMediaVerificationReport, verify_sd_buffers};

/// Validates an SD root directory containing `DIGADIAG.FRM` and/or `PANA__SD.FRM`.
pub fn verify_sd_unbrick_media(sd_root_dir: &Path) -> SdMediaVerificationReport {
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

    verify_sd_buffers(
        &sd_root_dir.to_string_lossy(),
        diag_ref,
        pana_owned.as_deref(),
    )
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

/// Builds a complete verified SD Unbrick Kit (`DIGADIAG.FRM`, optional `PANA__SD.FRM`, & `SHA256SUMS.txt`).
pub fn prepare_sd_unbrick_kit(req: &SdUnbrickKitRequest) -> Result<SdUnbrickKitReport> {
    let ws_root = resolve_workspace_root();
    let (disc_frm, disc_prog, disc_main, disc_drv, _) = discover_sources(&ws_root);
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
        compose_digadiag_and_prog061(&prog_raw, req.bgo_mode)?;

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

    if req.include_pana_sd {
        let drv_path = req
            .drv_path
            .clone()
            .or(disc_drv)
            .or_else(|| find_extracted_module(&frm_report, "DRV"));
        let main_path = req
            .main_path
            .clone()
            .or(disc_main)
            .or_else(|| find_extracted_module(&frm_report, "MAIN"))
            .context("No MAIN firmware or PANA_DVD.FRM source available for PANA__SD.FRM")?;

        main_source_label = Some(main_path.to_string_lossy().into_owned());
        let raw_main = fs::read(&main_path)
            .with_context(|| format!("Reading MAIN source {}", main_path.display()))?;
        let composed_main = compose_patched_main_container(
            &raw_main,
            req.main_passthrough_mode,
            req.unlock_hdd_dynamic,
            req.hdd_target_gb,
        )?;
        main_passthrough_applied = Some(composed_main.passthrough_applied);
        hdd_capacity_applied = composed_main.hdd_capacity_applied;

        let drv_bytes = match drv_path {
            Some(dp) => {
                drv_source_label = Some(dp.to_string_lossy().into_owned());
                Some(fs::read(&dp)?)
            }
            None => None,
        };

        let (sd_bytes, sd_sha) = compose_pana_sd_container(
            drv_bytes,
            prog_patched,
            composed_main.container,
            composed_main.slot_name,
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
        main_passthrough_applied,
        hdd_capacity_applied,
        digadiag_path: digadiag_path.to_string_lossy().into_owned(),
        digadiag_sha256,
        pana_sd_path,
        pana_sd_sha256,
        sha256sums_path: sums_path.to_string_lossy().into_owned(),
        verification,
    })
}
