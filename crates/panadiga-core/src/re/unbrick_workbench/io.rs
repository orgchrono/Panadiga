//! Thin imperative workspace discovery and `.FRM` filesystem extraction shell.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::constants::resolve_workspace_root;
use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};
use crate::hdd::inspect_main_buffer_capacity;
use crate::nvram::{calculate_hashes, resolve_input_path};

use super::super::bgo_unbrick::inspect_prog061_bgo_sites;
use super::super::main_passthrough::{
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, build_passthrough_rsp_plan,
    inspect_main_passthrough_sites,
};
use super::super::unbrick_sd_kit::verify_sd_unbrick_media;
use super::catalog::{
    ExtractedFrmModuleReport, FrmModuleExtractionReport, UnbrickWorkspaceReport,
    digadiag_vs_pana_notes, hardware_facts_catalog, sanitize_module_slug,
};
use super::codec::{
    detect_and_parse_frm_toc, load_main_binary_and_meta, load_prog061_from_bytes, slice_frm_slot,
};

fn find_first_existing(base: &Path, candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|rel| base.join(rel))
        .find(|p| p.exists())
}

pub(crate) fn discover_sources(
    root: &Path,
) -> (
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
) {
    let pana_dvd = find_first_existing(root, &["firmware_and_images/PANA_DVD.FRM", "PANA_DVD.FRM"]);
    let prog061 = find_first_existing(
        root,
        &[
            "firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin",
            "firmware_and_images/extracted_partitions/frm_unpacked/prog061_raw.bin",
        ],
    );
    let main_3410 = find_first_existing(
        root,
        &[
            "firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin",
            "firmware_and_images/extracted_partitions/frm_unpacked/main3060np_dec.bin",
        ],
    );
    let drv = find_first_existing(
        root,
        &["firmware_and_images/extracted_partitions/frm_unpacked/drv_s211_dec.bin"],
    );
    let sd_dir = [root.join("sd_card_eh55"), root.join("unbrick_sd_kit")]
        .into_iter()
        .find(|d| d.join("DIGADIAG.FRM").is_file() || d.join("PANA__SD.FRM").is_file());
    (pana_dvd, prog061, main_3410, drv, sd_dir)
}

/// Extracts all modules (`DRV S211`, `PROG061`, `MAIN*`) from `PANA_DVD.FRM` or `PANA__SD.FRM`.
pub fn extract_modules_from_frm(
    input_path: &Path,
    output_dir: &Path,
    unpack_extrhead_payloads: bool,
) -> Result<FrmModuleExtractionReport> {
    let resolved = resolve_input_path(input_path)?;
    let raw = fs::read(&resolved).with_context(|| format!("Reading {}", resolved.display()))?;
    let (container_format, encrypted_frm, entries) =
        detect_and_parse_frm_toc(&raw, &resolved.display().to_string())?;

    fs::create_dir_all(output_dir)
        .with_context(|| format!("Creating extraction dir {}", output_dir.display()))?;

    let mut modules = Vec::with_capacity(entries.len());
    for entry in entries {
        let slug = sanitize_module_slug(&entry.module_name);
        let dec = slice_frm_slot(&raw, entry.module_offset, entry.module_size, encrypted_frm)?;
        let dec_path = output_dir.join(format!("{slug}_dec.bin"));
        fs::write(&dec_path, &dec).with_context(|| format!("Writing {}", dec_path.display()))?;
        let (_, dec_sha) = calculate_hashes(&dec);

        let mut unpacked_path = None;
        let mut unpacked_size = None;
        let mut dst_addr_hex = None;
        let mut checksum_valid = None;

        if unpack_extrhead_payloads && !slug.starts_with("prog") {
            let mode = if slug.starts_with("main") {
                ExtrheadCryptMode::Main
            } else if slug.starts_with("drv") {
                ExtrheadCryptMode::Drv
            } else {
                ExtrheadCryptMode::Auto
            };
            if let Ok(u) = unpack_extrhead(&dec, mode) {
                let up_path = output_dir.join(format!("{slug}_unpacked.bin"));
                fs::write(&up_path, &u.decompressed)
                    .with_context(|| format!("Writing {}", up_path.display()))?;
                unpacked_path = Some(up_path.to_string_lossy().into_owned());
                unpacked_size = Some(u.decompressed.len());
                dst_addr_hex = Some(format!("0x{:08X}", u.header.dst_addr));
                checksum_valid = Some(u.checksum_valid);
            }
        }

        modules.push(ExtractedFrmModuleReport {
            name: entry.module_name.trim().to_string(),
            offset_hex: format!("0x{:08X}", entry.module_offset),
            encrypted_size: entry.module_size as usize,
            decrypted_path: dec_path.to_string_lossy().into_owned(),
            decrypted_sha256: dec_sha,
            unpacked_path,
            unpacked_size,
            dst_addr_hex,
            checksum_valid,
        });
    }

    Ok(FrmModuleExtractionReport {
        source_frm_path: resolved.to_string_lossy().into_owned(),
        container_format,
        output_dir: output_dir.to_string_lossy().into_owned(),
        modules,
    })
}

/// Inspects workspace sources, BGO/MAIN/capacity patch state, `_dev_SERIAL` RSP plan, and SD readiness.
pub fn inspect_unbrick_workspace(workspace_override: Option<&Path>) -> UnbrickWorkspaceReport {
    let root = workspace_override
        .map(PathBuf::from)
        .unwrap_or_else(resolve_workspace_root);
    let (pana_dvd, prog061, main_src, drv, sd_dir) = discover_sources(&root);

    let prog061_bgo = prog061
        .as_ref()
        .and_then(|p| fs::read(p).ok())
        .and_then(|raw| load_prog061_from_bytes(&raw).ok())
        .and_then(|bin| inspect_prog061_bgo_sites(&bin).ok());

    let (main_passthrough, main_capacity) = main_src
        .as_ref()
        .and_then(|p| fs::read(p).ok())
        .and_then(|raw| load_main_binary_and_meta(&raw).ok())
        .map(|(bin, build, _, _)| {
            let pass = if bin.len() == EXPECTED_MAIN_3410PQ_SIZE {
                inspect_main_passthrough_sites(&bin).ok()
            } else {
                None
            };
            let cap = inspect_main_buffer_capacity(
                &bin,
                MAIN_BASE_VA,
                "EXTRHEAD_CONTAINER",
                Some(build),
                "MAIN",
            )
            .ok();
            (pass, cap)
        })
        .unwrap_or((None, None));

    let sd_verification = sd_dir.as_deref().map(verify_sd_unbrick_media);

    UnbrickWorkspaceReport {
        workspace_root: root.to_string_lossy().into_owned(),
        discovered_pana_dvd: pana_dvd.map(|p| p.to_string_lossy().into_owned()),
        discovered_prog061: prog061.map(|p| p.to_string_lossy().into_owned()),
        discovered_main_3410pq: main_src.map(|p| p.to_string_lossy().into_owned()),
        discovered_drv: drv.map(|p| p.to_string_lossy().into_owned()),
        discovered_sd_dir: sd_dir.map(|p| p.to_string_lossy().into_owned()),
        prog061_bgo,
        main_passthrough,
        main_capacity,
        rsp_plan: build_passthrough_rsp_plan(Some(0x5040_1F3C)),
        sd_verification,
        hardware_facts: hardware_facts_catalog(),
        digadiag_vs_pana_notes: digadiag_vs_pana_notes(),
    }
}
