//! Thin CLI dispatch shell for Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Workbench.

pub mod render;

use anyhow::{Context, Result, bail};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::matshita_emu::{
    MatshitaBootScenario, MatshitaOddEmulator, build_sample_unlock_frm, parse_hex_payload,
    parse_unlock_container_mode, verify_unlock_frm_container,
};
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, SdUnbrickKitRequest, build_nor_dumper_artifact,
    build_passthrough_rsp_plan, extract_modules_from_frm, inspect_main_passthrough_sites,
    inspect_prog061_container_bytes, inspect_unbrick_workspace, patch_main_passthrough_file,
    patch_prog061_or_build_digadiag_with_policy, prepare_sd_unbrick_kit,
    resolve_nor_stub_load_addr, verify_sd_unbrick_media,
};
use std::fs;

use crate::cli::args::unbrick::{MatshitaEmuAction, UnbrickActions, UnbrickArgs};
use render::{
    emit_json_if_requested, format_bgo_inspection_line, format_frm_extraction_lines,
    format_frm_unlock_verification_lines, format_matshita_boot_lines,
    format_matshita_injection_lines, format_passthrough_inspection_lines, format_rsp_plan_lines,
    format_sd_kit_lines, format_sd_verification_lines, format_unbrick_status_lines, print_lines,
};

pub fn handle_unbrick(args: UnbrickArgs) -> Result<()> {
    match args.action {
        UnbrickActions::Status { workspace, json } => {
            let report = inspect_unbrick_workspace(workspace.as_deref());
            print_lines(&format_unbrick_status_lines(&report));
            emit_json_if_requested(&report, json.as_deref())
        }
        UnbrickActions::ExtractFrm {
            frm,
            outdir,
            unpack,
            json,
        } => {
            let report = extract_modules_from_frm(&frm, &outdir, unpack)?;
            print_lines(&format_frm_extraction_lines(&report));
            emit_json_if_requested(&report, json.as_deref())
        }
        UnbrickActions::Bgo {
            input,
            output,
            mode,
            raw_prog061,
            allow_unsafe_bgo_format,
            json,
        } => {
            let parsed_mode: Prog061BgoPatchMode = mode.parse()?;
            if let Some(out_path) = output {
                let rep = patch_prog061_or_build_digadiag_with_policy(
                    &input,
                    &out_path,
                    parsed_mode,
                    !raw_prog061,
                    allow_unsafe_bgo_format,
                )?;
                println!(
                    "Patched BGO ({}) -> {} ({} B, wrapped={})",
                    rep.mode, rep.output_path, rep.output_size, rep.digadiag_wrapped
                );
                emit_json_if_requested(&rep, json.as_deref())
            } else {
                let raw =
                    fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
                let rep = inspect_prog061_container_bytes(&raw)?;
                println!("{}", format_bgo_inspection_line(&rep));
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
                let cb = callback_va.as_deref().map(parse_u32_radix).transpose()?;
                let plan = build_passthrough_rsp_plan(cb);
                print_lines(&format_rsp_plan_lines(&plan));
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
                print_lines(&format_passthrough_inspection_lines(&rep));
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
            allow_unsafe_bgo_format,
            expected_main_build,
            allow_target_mismatch,
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
                bgo_mode: bgo_mode.parse()?,
                main_passthrough_mode: passthrough_mode.parse()?,
                include_pana_sd: !diag_only,
                unlock_hdd_dynamic: !no_hdd_dynamic && target_gb.is_none(),
                hdd_target_gb: target_gb,
                write_extracted_sources: save_sources,
                allow_unsafe_auto_format: allow_unsafe_bgo_format,
                expected_main_build,
                allow_target_mismatch,
            })?;
            print_lines(&format_sd_kit_lines(&report));
            emit_json_if_requested(&report, json.as_deref())
        }
        UnbrickActions::VerifySd { dir, json } => {
            let report = verify_sd_unbrick_media(&dir);
            print_lines(&format_sd_verification_lines(&report));
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
            let explicit_load = load_addr.as_deref().map(parse_u32_radix).transpose()?;
            let load_u32 = resolve_nor_stub_load_addr(explicit_load, wrap_digadiag);
            let (_, payload) =
                build_nor_dumper_artifact(start_u32, len_u32, block_size, load_u32, wrap_digadiag)?;
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
        UnbrickActions::MatshitaEmu { action } => match action {
            MatshitaEmuAction::Boot {
                scenario,
                frm,
                json,
            } => {
                let parsed: MatshitaBootScenario = scenario.parse()?;
                let mut emu = MatshitaOddEmulator::new(parsed);
                if let Some(frm_path) = frm {
                    let bytes = fs::read(&frm_path)
                        .with_context(|| format!("Reading {}", frm_path.display()))?;
                    let _ = emu.mount_unlock_frm("dvd", &bytes)?;
                }
                let report = emu.run_boot_sequence()?;
                print_lines(&format_matshita_boot_lines(&report));
                emit_json_if_requested(&report, json.as_deref())
            }
            MatshitaEmuAction::InjectWrite {
                offset,
                mode,
                vendor_ed,
                va,
                payload_hex,
                payload_file,
                json,
            } => {
                let payload = match (payload_hex, payload_file) {
                    (Some(h), None) => parse_hex_payload(&h)?,
                    (None, Some(p)) => {
                        fs::read(&p).with_context(|| format!("Reading {}", p.display()))?
                    }
                    (Some(_), Some(_)) => {
                        bail!("Pass either --payload-hex or --payload-file, not both");
                    }
                    (None, None) => b"PANADIGA_INJECT".to_vec(),
                };
                let target_va = va.as_deref().map(parse_u32_radix).transpose()?;
                let mut emu = MatshitaOddEmulator::default();
                let (rec, snap) =
                    emu.run_inject_write(target_va, offset, mode, vendor_ed, &payload)?;
                print_lines(&format_matshita_injection_lines(&rec, &snap));
                emit_json_if_requested(
                    &serde_json::json!({ "record": rec, "snapshot": snap }),
                    json.as_deref(),
                )
            }
            MatshitaEmuAction::SerialCmd {
                unlock_pass,
                cmd,
                json,
            } => {
                let mut emu = MatshitaOddEmulator::default();
                if unlock_pass {
                    let _ = emu.execute_drv_serial_command("PASS 0x4d415453 0x48495441")?;
                }
                let resp = emu.execute_drv_serial_command(&cmd)?;
                println!("DRV CLI [{}]: {}", resp.status, resp.command);
                print_lines(&resp.output_lines);
                emit_json_if_requested(&resp, json.as_deref())
            }
            MatshitaEmuAction::VerifyFrm { kind, input, json } => {
                let bytes =
                    fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
                let summary = verify_unlock_frm_container(&kind, &bytes)?;
                print_lines(&format_frm_unlock_verification_lines(&summary));
                emit_json_if_requested(&summary, json.as_deref())
            }
            MatshitaEmuAction::PackUnlockFrm {
                kind,
                payload_file,
                out,
                json,
            } => {
                let mode = parse_unlock_container_mode(&kind)?;
                let prog_payload = match payload_file {
                    Some(p) => fs::read(&p).with_context(|| format!("Reading {}", p.display()))?,
                    None => vec![0xCBu8; 1024],
                };
                let (bytes, summary) = build_sample_unlock_frm(mode, &prog_payload)?;
                if let Some(parent) = out.parent()
                    && !parent.as_os_str().is_empty()
                {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&out, &bytes)?;
                println!(
                    "Wrote unlock {} -> {}",
                    summary.container_kind,
                    out.display()
                );
                print_lines(&format_frm_unlock_verification_lines(&summary));
                emit_json_if_requested(&summary, json.as_deref())
            }
        },
    }
}
