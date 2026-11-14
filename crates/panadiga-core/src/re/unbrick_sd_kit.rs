//! SD Recovery Kit Generator (`DIGADIAG.FRM` + `PANA__SD.FRM`) and Media Validator

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::constants::{
    PANA_DIGADIAG_DEFAULT_LOAD_ADDR, PANA_DIGADIAG_FRM_SIZE, PANA_SD_TOC_ENTRY_SIZE,
    resolve_workspace_root,
};
use crate::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, build_digadiag_frm, pack_extrhead,
    pack_frm, parse_sd_frm_toc, unpack_extrhead,
};
use crate::hdd::{
    HddCapacityPatchMode, apply_capacity_patch_in_place, inspect_main_buffer_capacity,
};
use crate::nvram::calculate_hashes;

use super::bgo_unbrick::{
    PROG061_BASE_VA, Prog061BgoPatchMode, ensure_parent_dir, inspect_prog061_bgo_sites,
    patch_prog061_bgo_in_place,
};
use super::main_passthrough::{
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, MainPassthroughMode, apply_main_passthrough_in_place,
    inspect_main_passthrough_sites,
};
use super::unbrick_workbench::{
    bgo_mode_label, discover_sources, extract_modules_from_frm, load_main_binary_and_meta,
    load_prog061_from_bytes,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdUnbrickCheck {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdMediaVerificationReport {
    pub sd_root_dir: String,
    pub overall_status: String,
    pub digadiag_present: bool,
    pub digadiag_exact_size_ok: bool,
    pub digadiag_dst_addr_hex: Option<String>,
    pub digadiag_bgo_mode: Option<String>,
    pub pana_sd_present: bool,
    pub pana_sd_toc_entries: usize,
    pub pana_sd_list_sums_ok: bool,
    pub pana_sd_prog061_mode: Option<String>,
    pub pana_sd_main_build: Option<String>,
    pub pana_sd_main_passthrough_mode: Option<String>,
    pub pana_sd_main_hdd_dynamic: Option<bool>,
    pub checks: Vec<SdUnbrickCheck>,
}

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

fn verify_digadiag_file(
    diag_path: &Path,
    checks: &mut Vec<SdUnbrickCheck>,
) -> (bool, Option<String>, Option<String>) {
    if !diag_path.is_file() {
        checks.push(SdUnbrickCheck {
            id: "digadiag_container".to_string(),
            severity: "fail".to_string(),
            title: "DIGADIAG.FRM missing".to_string(),
            detail: "Required at SD root for BOOT.BIN Service/Diag SDDIAG boot".to_string(),
        });
        return (false, None, None);
    }

    let bytes = match fs::read(diag_path) {
        Ok(b) => b,
        Err(e) => {
            checks.push(SdUnbrickCheck {
                id: "digadiag_container".to_string(),
                severity: "fail".to_string(),
                title: "DIGADIAG.FRM read error".to_string(),
                detail: e.to_string(),
            });
            return (false, None, None);
        }
    };

    let exact_sz = bytes.len() == PANA_DIGADIAG_FRM_SIZE;
    if !bytes.starts_with(b"EXTRHEAD") {
        checks.push(SdUnbrickCheck {
            id: "digadiag_container".to_string(),
            severity: "fail".to_string(),
            title: "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)".to_string(),
            detail:
                "DIGADIAG.FRM must start with plaintext 'EXTRHEAD' (BOOT.BIN SDDIAG requirement)"
                    .to_string(),
        });
        return (exact_sz, None, None);
    }

    match unpack_extrhead(&bytes, ExtrheadCryptMode::Plain) {
        Ok(u) => {
            let dst_hex = Some(format!("0x{:08X}", u.header.dst_addr));
            let bgo_mode = inspect_prog061_bgo_sites(&u.decompressed)
                .ok()
                .map(|r| r.detected_mode);
            let ok = exact_sz
                && u.checksum_valid
                && u.footer_present
                && u.header.dst_addr == PANA_DIGADIAG_DEFAULT_LOAD_ADDR;
            checks.push(SdUnbrickCheck {
                id: "digadiag_container".to_string(),
                severity: if ok { "pass" } else { "fail" }.to_string(),
                title: "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)".to_string(),
                detail: format!(
                    "size={} B (exact_128k={}), tag='{}', dst=0x{:08X}, adler_ok={}, footer={}, bgo_mode={}",
                    bytes.len(),
                    exact_sz,
                    u.header.tag,
                    u.header.dst_addr,
                    u.checksum_valid,
                    u.footer_present,
                    bgo_mode.as_deref().unwrap_or("CUSTOM_PAYLOAD"),
                ),
            });
            (exact_sz, dst_hex, bgo_mode)
        }
        Err(e) => {
            checks.push(SdUnbrickCheck {
                id: "digadiag_container".to_string(),
                severity: "fail".to_string(),
                title: "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)".to_string(),
                detail: format!("Failed to unpack plaintext EXTRHEAD: {e}"),
            });
            (exact_sz, None, None)
        }
    }
}

/// Validates an SD root directory containing `DIGADIAG.FRM` and/or `PANA__SD.FRM`.
pub fn verify_sd_unbrick_media(sd_root_dir: &Path) -> SdMediaVerificationReport {
    let mut checks = Vec::new();
    let diag_path = sd_root_dir.join("DIGADIAG.FRM");
    let pana_sd_path = sd_root_dir.join("PANA__SD.FRM");

    let digadiag_present = diag_path.is_file();
    let (digadiag_exact_size_ok, digadiag_dst_addr_hex, digadiag_bgo_mode) =
        verify_digadiag_file(&diag_path, &mut checks);

    let mut pana_sd_toc_entries = 0usize;
    let mut pana_sd_list_sums_ok = false;
    let mut pana_sd_prog061_mode = None;
    let mut pana_sd_main_build = None;
    let mut pana_sd_main_passthrough_mode = None;
    let mut pana_sd_main_hdd_dynamic = None;
    let pana_sd_present = pana_sd_path.is_file();

    if pana_sd_present {
        if let Ok(sd_bytes) = fs::read(&pana_sd_path) {
            let toc = parse_sd_frm_toc(&sd_bytes);
            pana_sd_toc_entries = toc.len();
            pana_sd_list_sums_ok = !toc.is_empty()
                && toc.iter().all(|e| {
                    e.list_sum_valid
                        && (e.module_offset as usize).saturating_add(e.module_size as usize)
                            <= sd_bytes.len()
                });

            for e in &toc {
                let mod_slice =
                    &sd_bytes[e.module_offset as usize..(e.module_offset + e.module_size) as usize];
                let name_up = e.module_name.trim().to_ascii_uppercase();
                if name_up.starts_with("PROG") {
                    pana_sd_prog061_mode = inspect_prog061_bgo_sites(mod_slice)
                        .ok()
                        .map(|b| b.detected_mode);
                } else if name_up.starts_with("MAIN")
                    && let Ok(u) = unpack_extrhead(mod_slice, ExtrheadCryptMode::Main)
                {
                    pana_sd_main_build = Some(if u.header.build_name.is_empty() {
                        e.module_name.trim().to_string()
                    } else {
                        u.header.build_name.clone()
                    });
                    pana_sd_main_passthrough_mode = inspect_main_passthrough_sites(&u.decompressed)
                        .ok()
                        .map(|p| p.detected_mode);
                    pana_sd_main_hdd_dynamic = inspect_main_buffer_capacity(
                        &u.decompressed,
                        MAIN_BASE_VA,
                        "EXTRHEAD_CONTAINER",
                        Some(u.header.build_name.clone()),
                        "PANA__SD.FRM:MAIN",
                    )
                    .ok()
                    .map(|cap| cap.is_dynamic_unclamped);
                }
            }

            checks.push(SdUnbrickCheck {
                id: "pana_sd_container".to_string(),
                severity: if pana_sd_list_sums_ok { "pass" } else { "fail" }.to_string(),
                title: "PANA__SD.FRM (64B Plaintext SD TOC)".to_string(),
                detail: format!(
                    "entries={}, list_sums_ok={}, entry_stride={}B, prog061={}, main_build={}, tbc_mode={}",
                    pana_sd_toc_entries,
                    pana_sd_list_sums_ok,
                    PANA_SD_TOC_ENTRY_SIZE,
                    pana_sd_prog061_mode.as_deref().unwrap_or("n/a"),
                    pana_sd_main_build.as_deref().unwrap_or("n/a"),
                    pana_sd_main_passthrough_mode.as_deref().unwrap_or("factory_or_non_3410pq"),
                ),
            });
        }
    } else {
        checks.push(SdUnbrickCheck {
            id: "pana_sd_container".to_string(),
            severity: "warn".to_string(),
            title: "PANA__SD.FRM absent (DIGADIAG-only SD)".to_string(),
            detail: "Valid if you only want DIGADIAG.FRM (PROG061 BGO/UART monitor) to run from SD without reflashing MAIN/DRV".to_string(),
        });
    }

    checks.push(SdUnbrickCheck {
        id: "sd_hw_91006".to_string(),
        severity: "warn".to_string(),
        title: "BOOT.BIN SD-91006 Hardware Gate (1-bit -> 4-bit ACMD6)".to_string(),
        detail: "Requires legacy SDSC card (<= 2 GiB, FAT16/FAT12) AND intact SD DAT1..DAT3 traces (P6801/P6802 <-> P53501). BOOT.BIN has no UART RX console so Pico cannot force 1-bit mode over P7501.".to_string(),
    });

    let overall_status = if checks.iter().any(|c| c.severity == "fail") {
        "INCOMPLETE_OR_INVALID"
    } else if checks.iter().any(|c| c.severity == "warn") {
        "READY_WITH_HW_WARNINGS"
    } else {
        "READY"
    }
    .to_string();

    SdMediaVerificationReport {
        sd_root_dir: sd_root_dir.to_string_lossy().into_owned(),
        overall_status,
        digadiag_present,
        digadiag_exact_size_ok,
        digadiag_dst_addr_hex,
        digadiag_bgo_mode,
        pana_sd_present,
        pana_sd_toc_entries,
        pana_sd_list_sums_ok,
        pana_sd_prog061_mode,
        pana_sd_main_build,
        pana_sd_main_passthrough_mode,
        pana_sd_main_hdd_dynamic,
        checks,
    }
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
        .or_else(|| {
            frm_report.as_ref().and_then(|r| {
                r.modules
                    .iter()
                    .find(|m| m.name.to_ascii_uppercase().starts_with("PROG"))
                    .map(|m| PathBuf::from(&m.decrypted_path))
            })
        })
        .or(disc_prog)
        .context("No PROG061 or PANA_DVD.FRM source available to build DIGADIAG.FRM")?;

    let prog_raw = fs::read(&prog_src_path)
        .with_context(|| format!("Reading PROG061 source {}", prog_src_path.display()))?;
    let mut prog_patched = load_prog061_from_bytes(&prog_raw)?;
    patch_prog061_bgo_in_place(&mut prog_patched, req.bgo_mode)?;

    fs::create_dir_all(&req.output_dir)?;
    let digadiag_path = req.output_dir.join("DIGADIAG.FRM");
    let (diag_bytes, _) = build_digadiag_frm(
        &prog_patched,
        PROG061_BASE_VA,
        "FVU ",
        "PROG061",
        4,
        PANA_DIGADIAG_FRM_SIZE,
    )?;
    fs::write(&digadiag_path, &diag_bytes)?;
    let (_, digadiag_sha256) = calculate_hashes(&diag_bytes);

    let mut manifest_hashes = vec![format!("{digadiag_sha256}  DIGADIAG.FRM")];
    if req.write_extracted_sources {
        let raw_out = req.output_dir.join("PROG061_PATCHED.bin");
        fs::write(&raw_out, &prog_patched)?;
        manifest_hashes.push(format!(
            "{}  PROG061_PATCHED.bin",
            calculate_hashes(&prog_patched).1
        ));
    }

    let mut pana_sd_path = None;
    let mut pana_sd_sha256 = None;
    let mut main_source_label = None;
    let mut drv_source_label = None;
    let mut main_passthrough_applied = None;
    let mut hdd_capacity_applied = None;

    if req.include_pana_sd {
        let drv_path = req.drv_path.clone().or(disc_drv).or_else(|| {
            frm_report.as_ref().and_then(|r| {
                r.modules
                    .iter()
                    .find(|m| m.name.to_ascii_uppercase().starts_with("DRV"))
                    .map(|m| PathBuf::from(&m.decrypted_path))
            })
        });
        let main_path = req
            .main_path
            .clone()
            .or(disc_main)
            .or_else(|| {
                frm_report.as_ref().and_then(|r| {
                    r.modules
                        .iter()
                        .find(|m| m.name.to_ascii_uppercase().starts_with("MAIN"))
                        .map(|m| PathBuf::from(&m.decrypted_path))
                })
            })
            .context("No MAIN firmware or PANA_DVD.FRM source available for PANA__SD.FRM")?;

        main_source_label = Some(main_path.to_string_lossy().into_owned());
        let raw_main = fs::read(&main_path)
            .with_context(|| format!("Reading MAIN source {}", main_path.display()))?;
        let (mut main_bin, build_name, flag, adler_step) = load_main_binary_and_meta(&raw_main)?;
        let mut modified_main = false;

        if main_bin.len() == EXPECTED_MAIN_3410PQ_SIZE
            && inspect_main_passthrough_sites(&main_bin).is_ok()
        {
            apply_main_passthrough_in_place(&mut main_bin, req.main_passthrough_mode)?;
            main_passthrough_applied = Some(req.main_passthrough_mode.as_str().to_string());
            modified_main = true;
        } else {
            main_passthrough_applied = Some("skipped_non_3410pq_build".to_string());
        }

        if req.unlock_hdd_dynamic || req.hdd_target_gb.is_some() {
            let cap_mode = if req.unlock_hdd_dynamic {
                HddCapacityPatchMode::DynamicUnclamped
            } else {
                HddCapacityPatchMode::FixedCapacity
            };
            if apply_capacity_patch_in_place(&mut main_bin, cap_mode, req.hdd_target_gb).is_ok() {
                hdd_capacity_applied = Some(if req.unlock_hdd_dynamic {
                    "dynamic".to_string()
                } else {
                    format!("cap_{}gb", req.hdd_target_gb.unwrap_or(0))
                });
                modified_main = true;
            }
        }

        let main_container = if modified_main || !raw_main.starts_with(&[0x51, 0x87, 0x13, 0x8D]) {
            pack_extrhead(
                &main_bin,
                "MAIN",
                MAIN_BASE_VA,
                if build_name.is_empty() {
                    "3410PQ"
                } else {
                    &build_name
                },
                ExtrheadCryptMode::Main,
                adler_step.max(16),
                flag.max(1),
            )?
        } else {
            raw_main
        };

        let mut sd_modules = Vec::new();
        if let Some(dp) = drv_path {
            drv_source_label = Some(dp.to_string_lossy().into_owned());
            let mut m = FrmPackModule::new("DRV S211", fs::read(&dp)?);
            m.version_u16 = 0x0211;
            sd_modules.push(m);
        }
        let mut prog_mod = FrmPackModule::new("PROG061 ", prog_patched);
        prog_mod.version_u16 = 0x0061;
        sd_modules.push(prog_mod);

        let main_slot = if build_name.contains("3060") {
            "MAIN3060NP"
        } else {
            "MAIN3410PQ"
        };
        let mut main_mod = FrmPackModule::new(main_slot, main_container);
        main_mod.version_u16 = 0x3410;
        sd_modules.push(main_mod);

        let (sd_bytes, _) = pack_frm(&sd_modules, FrmContainerMode::Sd, None)?;
        let out_sd = req.output_dir.join("PANA__SD.FRM");
        fs::write(&out_sd, &sd_bytes)?;
        let (_, sd_sha) = calculate_hashes(&sd_bytes);
        manifest_hashes.push(format!("{sd_sha}  PANA__SD.FRM"));
        pana_sd_path = Some(out_sd.to_string_lossy().into_owned());
        pana_sd_sha256 = Some(sd_sha);
    }

    if !req.write_extracted_sources && extracted_tmp.exists() {
        let _ = fs::remove_dir_all(&extracted_tmp);
    }

    let sums_path = req.output_dir.join("SHA256SUMS.txt");
    ensure_parent_dir(&sums_path)?;
    fs::write(&sums_path, manifest_hashes.join("\n") + "\n")?;
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
