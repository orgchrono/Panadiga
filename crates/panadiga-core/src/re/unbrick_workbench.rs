//! Pre-MAIN DIGA Unbrick Workbench: Workspace Discovery, FRM Extraction, & Hardware Facts
//!
//! Pairs with [`super::unbrick_sd_kit`] to extract modules from `PANA_DVD.FRM` / `PANA__SD.FRM`,
//! discover existing `PROG061` / `MAIN3410PQ` / SD artifacts, and synthesize `_dev_SERIAL` RSP plans.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{
    PANA_EXTRHEAD_DEFAULT_ADLER_STEP, PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256,
    resolve_workspace_root,
};
use crate::crypto::{
    ExtrheadCryptMode, decrypt_and_parse_frm_toc, feistel_decrypt_buffer, parse_sd_frm_toc,
    unpack_extrhead,
};
use crate::hdd::{MainCapacityInspectionReport, inspect_main_buffer_capacity};
use crate::nvram::{calculate_hashes, resolve_input_path};

use super::bgo_unbrick::{
    PROG061_EXPECTED_SIZE, Prog061BgoInspection, Prog061BgoPatchMode, inspect_prog061_bgo_sites,
};
use super::main_passthrough::{
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, MainPassthroughReport, MainPassthroughRspPlan,
    build_passthrough_rsp_plan, inspect_main_passthrough_sites,
};
use super::unbrick_sd_kit::{SdMediaVerificationReport, verify_sd_unbrick_media};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractedFrmModuleReport {
    pub name: String,
    pub offset_hex: String,
    pub encrypted_size: usize,
    pub decrypted_path: String,
    pub decrypted_sha256: String,
    pub unpacked_path: Option<String>,
    pub unpacked_size: Option<usize>,
    pub dst_addr_hex: Option<String>,
    pub checksum_valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrmModuleExtractionReport {
    pub source_frm_path: String,
    pub container_format: String,
    pub output_dir: String,
    pub modules: Vec<ExtractedFrmModuleReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnbrickHardwareFact {
    pub topic: String,
    pub verdict: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbrickWorkspaceReport {
    pub workspace_root: String,
    pub discovered_pana_dvd: Option<String>,
    pub discovered_prog061: Option<String>,
    pub discovered_main_3410pq: Option<String>,
    pub discovered_drv: Option<String>,
    pub discovered_sd_dir: Option<String>,
    pub prog061_bgo: Option<Prog061BgoInspection>,
    pub main_passthrough: Option<MainPassthroughReport>,
    pub main_capacity: Option<MainCapacityInspectionReport>,
    pub rsp_plan: MainPassthroughRspPlan,
    pub sd_verification: Option<SdMediaVerificationReport>,
    pub hardware_facts: Vec<UnbrickHardwareFact>,
    pub digadiag_vs_pana_notes: Vec<String>,
}

pub(super) fn sanitize_module_slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut prev_us = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_us = false;
        } else if !prev_us && !slug.is_empty() {
            slug.push('_');
            prev_us = true;
        }
    }
    if slug.is_empty() {
        "module".to_string()
    } else {
        slug.trim_end_matches('_').to_string()
    }
}

pub(super) fn bgo_mode_label(mode: Prog061BgoPatchMode) -> &'static str {
    match mode {
        Prog061BgoPatchMode::BypassChecksOnly => "bypass",
        Prog061BgoPatchMode::AutoFormatBgo => "auto_format",
        Prog061BgoPatchMode::UartMonitor => "uart_monitor",
        Prog061BgoPatchMode::FactoryOriginal => "factory",
    }
}

fn find_first_existing(base: &Path, candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|rel| base.join(rel))
        .find(|p| p.exists())
}

pub(super) fn load_prog061_from_bytes(raw: &[u8]) -> Result<Vec<u8>> {
    if raw.len() == PROG061_EXPECTED_SIZE && !raw.starts_with(b"EXTRHEAD") {
        if inspect_prog061_bgo_sites(raw).is_ok() {
            return Ok(raw.to_vec());
        }
        let mut dec = raw.to_vec();
        feistel_decrypt_buffer(
            &mut dec,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );
        if inspect_prog061_bgo_sites(&dec).is_ok() {
            return Ok(dec);
        }
        return Ok(raw.to_vec());
    }
    if raw.len() > 0x40 {
        let mode = if raw.starts_with(b"EXTRHEAD") {
            ExtrheadCryptMode::Plain
        } else {
            ExtrheadCryptMode::Auto
        };
        return Ok(unpack_extrhead(raw, mode)?.decompressed);
    }
    bail!("Unrecognized PROG061 binary ({} bytes)", raw.len());
}

pub(super) fn load_main_binary_and_meta(raw: &[u8]) -> Result<(Vec<u8>, String, u16, u32)> {
    if raw.starts_with(b"EXTRHEAD") {
        let u = unpack_extrhead(raw, ExtrheadCryptMode::Plain)?;
        return Ok((
            u.decompressed,
            u.header.build_name,
            u.header.flag,
            u.header.adler_step,
        ));
    }
    if let Ok(u) = unpack_extrhead(raw, ExtrheadCryptMode::Main) {
        return Ok((
            u.decompressed,
            u.header.build_name,
            u.header.flag,
            u.header.adler_step,
        ));
    }
    if let Ok(u) = unpack_extrhead(raw, ExtrheadCryptMode::Auto) {
        return Ok((
            u.decompressed,
            u.header.build_name,
            u.header.flag,
            u.header.adler_step,
        ));
    }
    if raw.len() >= 10_000_000 {
        let build = if raw.len() == EXPECTED_MAIN_3410PQ_SIZE {
            "3410PQ"
        } else {
            "3060NP"
        };
        return Ok((
            raw.to_vec(),
            build.to_string(),
            1,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        ));
    }
    bail!(
        "Input does not match raw MAIN firmware or EXTRHEAD container ({} B)",
        raw.len()
    );
}

fn slice_frm_slot(buf: &[u8], off: u32, sz: u32, encrypted_frm: bool) -> Result<Vec<u8>> {
    let start = off as usize;
    let end = start.saturating_add(sz as usize);
    if end > buf.len() || sz == 0 {
        bail!(
            "FRM module range 0x{start:X}..0x{end:X} exceeds container ({} B)",
            buf.len()
        );
    }
    let mut slice = buf[start..end].to_vec();
    if encrypted_frm {
        feistel_decrypt_buffer(
            &mut slice,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );
    }
    Ok(slice)
}

/// Extracts all modules (`DRV S211`, `PROG061`, `MAIN*`) from `PANA_DVD.FRM` or `PANA__SD.FRM`.
pub fn extract_modules_from_frm(
    input_path: &Path,
    output_dir: &Path,
    unpack_extrhead_payloads: bool,
) -> Result<FrmModuleExtractionReport> {
    let resolved = resolve_input_path(input_path)?;
    let raw = fs::read(&resolved).with_context(|| format!("Reading {}", resolved.display()))?;

    let enc_entries = decrypt_and_parse_frm_toc(&raw);
    let enc_ok = !enc_entries.is_empty()
        && enc_entries.iter().all(|e| {
            e.list_sum_valid
                && (e.module_offset as usize).saturating_add(e.module_size as usize) <= raw.len()
        });
    let (container_format, encrypted_frm, entries) = if enc_ok {
        ("PANA_DVD_48B_FEISTEL".to_string(), true, enc_entries)
    } else {
        let sd_entries = parse_sd_frm_toc(&raw);
        if sd_entries.is_empty() {
            bail!(
                "No valid Feistel 48B or SD 64B TOC entries found in {}",
                resolved.display()
            );
        }
        ("PANA_SD_64B_PLAIN".to_string(), false, sd_entries)
    };

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

fn hardware_facts_catalog() -> Vec<UnbrickHardwareFact> {
    vec![
        UnbrickHardwareFact {
            topic: "DIGADIAG.FRM vs PANA__SD.FRM / PANA_DVD.FRM".to_string(),
            verdict: "DISTINCT_STAGE15_VS_STAGE2_CONTAINERS".to_string(),
            evidence: "DIGADIAG.FRM is a fixed 128 KiB (0x20000) single-payload plaintext EXTRHEAD ('FVU ', VA 0x50400000, adler_step=4) consumed by BOOT.BIN SDDIAG. PANA_DVD.FRM uses 48B Feistel TOC + full Feistel payload (optical disc). PANA__SD.FRM uses plaintext 64B TOC entries pointing to PROG/DRV/MAIN containers for SD Update.".to_string(),
        },
        UnbrickHardwareFact {
            topic: "Forcing SD 1-bit Mode via Pico P7501 (-91006 Bypass)".to_string(),
            verdict: "NOT_POSSIBLE_OVER_P7501_UART".to_string(),
            evidence: "BOOT.BIN executes from NOR (0x44000000/0x80000000), passes 1st GetSD_Status @ 0x44027826 in 1-bit mode, then switches to 4-bit via ACMD6 @ 0x44027F79 before 2nd GetSD_Status @ 0x44027F93 (-91006). uart0_getc @ 0x4400479F has no active caller and P7501 lacks OCD clock (DCSR=0xFFFFFFFF). Fix via P6801/P6802 DAT1..3 + <=2GB SDSC FAT16 or TSOP-48 NOR patch.".to_string(),
        },
        UnbrickHardwareFact {
            topic: "Standalone Digital PCB TBC Operation Outside Drive".to_string(),
            verdict: "ODD_HDD_REMOVABLE_MAIN_PCB_REQUIRED".to_string(),
            evidence: "Patching MAIN passthrough sites (0x5771ABFB, 0x574F7B8C, 0x574F7C5C) lets TBC (E-E) run without ODD/HDD checks (or with CF/SSD via IDE2SAT2). However, Digital PCB P59001 still requires Main PCB P7402 (schematics na_p49_blk1, vol2_p125) for IC3001 video clamps/75-ohm drivers, IC7501 Timer MCU TBUS handshake, and 8+ power rails.".to_string(),
        },
    ]
}

pub(super) fn discover_sources(
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
        digadiag_vs_pana_notes: vec![
            "DIGADIAG.FRM -> BOOT.BIN (NOR 0x44000000) loads 128 KiB plaintext EXTRHEAD 'FVU ' to 0x50400000 and jumps directly (zero BGO/NVRAM check).".to_string(),
            "PANA__SD.FRM -> PROG061 reads 64B plaintext TOC slots to reflash DRV/PROG/MAIN when SD Update runs.".to_string(),
            "PANA_DVD.FRM -> Optical update container with 48B Feistel TOC + full Feistel module layer over DRV S211 / PROG061 / MAIN3060NP.".to_string(),
        ],
    }
}
