//! CLI handlers for the Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Workbench.

use anyhow::{Context, Result, bail};
use panadiga_core::constants::PANA_DIGADIAG_FRM_SIZE;
use panadiga_core::crypto::{
    ExtrheadCryptMode, build_digadiag_frm, parse_u32_radix, unpack_extrhead,
};
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, SdUnbrickKitRequest, build_passthrough_rsp_plan,
    build_readonly_nor_dumper_stub, extract_modules_from_frm, inspect_main_passthrough_sites,
    inspect_prog061_bgo_sites, inspect_unbrick_workspace, patch_main_passthrough_file,
    patch_prog061_or_build_digadiag, prepare_sd_unbrick_kit, verify_sd_unbrick_media,
};
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::cli::args::unbrick::{UnbrickActions, UnbrickArgs};

fn emit_json_if_requested<T: Serialize>(val: &T, json_path: Option<&Path>) -> Result<()> {
    if let Some(p) = json_path {
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(p, serde_json::to_string_pretty(val)?)?;
        println!("JSON report written to {}", p.display());
    }
    Ok(())
}

fn parse_bgo_mode(raw: &str) -> Result<Prog061BgoPatchMode> {
    raw.parse()
}

pub fn handle_unbrick(args: UnbrickArgs) -> Result<()> {
    match args.action {
        UnbrickActions::Status { workspace, json } => {
            let report = inspect_unbrick_workspace(workspace.as_deref());
            println!("=== PANADIGA UNBRICK WORKBENCH STATUS ===");
            println!("Workspace   : {}", report.workspace_root);
            println!(
                "PANA_DVD    : {}",
                report.discovered_pana_dvd.as_deref().unwrap_or("not found")
            );
            println!(
                "PROG061     : {}",
                report.discovered_prog061.as_deref().unwrap_or("not found")
            );
            println!(
                "MAIN 3410PQ : {}",
                report
                    .discovered_main_3410pq
                    .as_deref()
                    .unwrap_or("not found")
            );
            println!(
                "BGO Status  : {}",
                report
                    .prog061_bgo
                    .as_ref()
                    .map(|b| b.detected_mode.as_str())
                    .unwrap_or("unavailable")
            );
            println!(
                "TBC Mode    : {}",
                report
                    .main_passthrough
                    .as_ref()
                    .map(|m| m.detected_mode.as_str())
                    .unwrap_or("unavailable")
            );
            if let Some(sd) = &report.sd_verification {
                println!("SD Kit ({}) : {}", sd.sd_root_dir, sd.overall_status);
            }
            for fact in &report.hardware_facts {
                println!("[{}] {} -> {}", fact.verdict, fact.topic, fact.evidence);
            }
            emit_json_if_requested(&report, json.as_deref())
        }
        UnbrickActions::ExtractFrm {
            frm,
            outdir,
            unpack,
            json,
        } => {
            let report = extract_modules_from_frm(&frm, &outdir, unpack)?;
            println!(
                "Extracted {} modules ({}) into {}",
                report.modules.len(),
                report.container_format,
                report.output_dir
            );
            for m in &report.modules {
                println!(
                    "  - {:<10} {} ({} B) -> {}",
                    m.name, m.offset_hex, m.encrypted_size, m.decrypted_path
                );
            }
            emit_json_if_requested(&report, json.as_deref())
        }
        UnbrickActions::Bgo {
            input,
            output,
            mode,
            raw_prog061,
            json,
        } => {
            let parsed_mode = parse_bgo_mode(&mode)?;
            if let Some(out_path) = output {
                let rep =
                    patch_prog061_or_build_digadiag(&input, &out_path, parsed_mode, !raw_prog061)?;
                println!(
                    "Patched BGO ({}) -> {} ({} B, wrapped={})",
                    rep.mode, rep.output_path, rep.output_size, rep.digadiag_wrapped
                );
                emit_json_if_requested(&rep, json.as_deref())
            } else {
                let raw =
                    fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
                let payload = if raw.starts_with(b"EXTRHEAD") {
                    unpack_extrhead(&raw, ExtrheadCryptMode::Plain)?.decompressed
                } else {
                    raw
                };
                let rep = inspect_prog061_bgo_sites(&payload)?;
                println!(
                    "PROG061 BGO Inspection: mode={} ({})",
                    rep.detected_mode, rep.raw_sha256
                );
                emit_json_if_requested(&rep, json.as_deref())
            }
        }
        UnbrickActions::Passthrough {
            input,
            output,
            mode,
            rsp_plan,
            callback_va,
            json,
        } => {
            if rsp_plan {
                let cb = match callback_va {
                    Some(raw) => Some(parse_u32_radix(&raw)?),
                    None => None,
                };
                let plan = build_passthrough_rsp_plan(cb);
                println!("_dev_SERIAL Handshake Hex: {}", plan.handshake_hex);
                for f in &plan.passthrough_frames {
                    println!("  RSP $M: {f}");
                }
                for s in &plan.pico_cli_shortcuts {
                    println!("  Pico  : {s}");
                }
                return emit_json_if_requested(&plan, json.as_deref());
            }

            let in_path =
                input.context("--input <MAIN_3410PQ> is required unless --rsp-plan is set")?;
            let parsed_mode: MainPassthroughMode = mode.parse()?;
            if let Some(out_path) = output {
                let rep = patch_main_passthrough_file(&in_path, &out_path, parsed_mode)?;
                println!(
                    "Patched MAIN TBC ({}) -> {}",
                    rep.detected_mode,
                    rep.output_path.as_deref().unwrap_or("")
                );
                emit_json_if_requested(&rep, json.as_deref())
            } else {
                let buf =
                    fs::read(&in_path).with_context(|| format!("Reading {}", in_path.display()))?;
                let rep = inspect_main_passthrough_sites(&buf)?;
                println!(
                    "MAIN Passthrough Status: {} (size={} B, match={})",
                    rep.detected_mode, rep.file_size, rep.expected_size_match
                );
                for s in &rep.sites {
                    println!(
                        "  * {} @ {} [{}]: {}",
                        s.name, s.va_hex, s.state, s.rsp_passthrough_frame
                    );
                }
                emit_json_if_requested(&rep, json.as_deref())
            }
        }
        UnbrickActions::PrepareSd {
            outdir,
            frm,
            prog061,
            main,
            drv,
            bgo_mode,
            passthrough_mode,
            diag_only,
            no_hdd_dynamic,
            target_gb,
            save_sources,
            json,
        } => {
            let report = prepare_sd_unbrick_kit(&SdUnbrickKitRequest {
                output_dir: outdir,
                pana_dvd_path: frm,
                prog061_path: prog061,
                main_path: main,
                drv_path: drv,
                bgo_mode: parse_bgo_mode(&bgo_mode)?,
                main_passthrough_mode: passthrough_mode.parse()?,
                include_pana_sd: !diag_only,
                unlock_hdd_dynamic: !no_hdd_dynamic && target_gb.is_none(),
                hdd_target_gb: target_gb,
                write_extracted_sources: save_sources,
            })?;
            println!(
                "SD Unbrick Kit Ready ({}) in {}",
                report.verification.overall_status, report.output_dir
            );
            println!(
                "  DIGADIAG.FRM : {} [{}]",
                report.digadiag_path,
                &report.digadiag_sha256[..16]
            );
            if let Some(ref sd) = report.pana_sd_path {
                println!("  PANA__SD.FRM : {sd}");
            }
            emit_json_if_requested(&report, json.as_deref())
        }
        UnbrickActions::VerifySd { dir, json } => {
            let report = verify_sd_unbrick_media(&dir);
            println!(
                "SD Verification ({}): {}",
                report.sd_root_dir, report.overall_status
            );
            for chk in &report.checks {
                println!(
                    "  [{}] {}: {}",
                    chk.severity.to_uppercase(),
                    chk.title,
                    chk.detail
                );
            }
            emit_json_if_requested(&report, json.as_deref())?;
            if report.overall_status == "INCOMPLETE_OR_INVALID" {
                bail!("SD media validation failed; inspect missing or corrupt artifacts above");
            }
            Ok(())
        }
        UnbrickActions::NorStub {
            start,
            len,
            block_size,
            load_addr,
            out,
            wrap_digadiag,
        } => {
            let start_u32 = parse_u32_radix(&start)?;
            let len_u32 = parse_u32_radix(&len)?;
            let load_u32 = parse_u32_radix(&load_addr)?;
            let stub = build_readonly_nor_dumper_stub(start_u32, len_u32, block_size, load_u32)?;
            let payload = if wrap_digadiag {
                let (frm, _) = build_digadiag_frm(
                    &stub,
                    load_u32,
                    "FVU ",
                    "NOR_DMP",
                    4,
                    PANA_DIGADIAG_FRM_SIZE,
                )?;
                frm
            } else {
                stub
            };
            if let Some(p) = out {
                if let Some(parent) = p.parent()
                    && !parent.as_os_str().is_empty()
                {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&p, &payload)?;
                println!(
                    "Wrote MN103S NOR dumper ({} B, wrapped={}, load=0x{:08X}) -> {}",
                    payload.len(),
                    wrap_digadiag,
                    load_u32,
                    p.display()
                );
            } else {
                println!(
                    "Assembled MN103S NOR dumper ({} B, start=0x{:08X}, len=0x{:X}, load=0x{:08X})",
                    payload.len(),
                    start_u32,
                    len_u32,
                    load_u32
                );
            }
            Ok(())
        }
    }
}
