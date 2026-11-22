#![deny(unsafe_code)]

//! Thin CLI dispatch shell for Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Workbench.

pub mod bgo_tbc;
mod nor_matshita;
mod region_compat;
pub mod render;

use anyhow::{Context, Result, bail};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, build_passthrough_rsp_plan, extract_modules_from_frm,
    inspect_main_passthrough_sites, inspect_prog061_container_bytes, inspect_sd_target,
    inspect_unbrick_workspace, patch_main_passthrough_file,
    patch_prog061_or_build_digadiag_with_policy,
};
use std::fs;

use crate::cli::args::unbrick::{UnbrickActions, UnbrickArgs};
use render::{
    emit_json_if_requested, format_bgo_inspection_line, format_frm_extraction_lines,
    format_passthrough_inspection_lines, format_rsp_plan_lines, format_sd_disk_inspection_lines,
    format_unbrick_status_lines, print_lines,
};

pub fn handle_unbrick(args: UnbrickArgs) -> Result<()> {
    match args.action {
        UnbrickActions::Status { workspace, json } => {
            let report = inspect_unbrick_workspace(workspace.as_deref());
            print_lines(&format_unbrick_status_lines(&report));
            emit_json_if_requested(&report, json.as_deref())
        }
        UnbrickActions::ExtractFrm(frm_args) => {
            let report =
                extract_modules_from_frm(&frm_args.frm, &frm_args.outdir, frm_args.unpack)?;
            print_lines(&format_frm_extraction_lines(&report));
            emit_json_if_requested(&report, frm_args.json.as_deref())
        }
        UnbrickActions::Bgo(bgo) => {
            let parsed_mode: Prog061BgoPatchMode = bgo.mode.parse()?;
            if let Some(out_path) = bgo.output {
                let rep = patch_prog061_or_build_digadiag_with_policy(
                    &bgo.input,
                    &out_path,
                    parsed_mode,
                    !bgo.raw_prog061,
                    bgo.allow_unsafe_bgo_format,
                )?;
                println!(
                    "Patched BGO ({}) -> {} ({} B, wrapped={})",
                    rep.mode, rep.output_path, rep.output_size, rep.digadiag_wrapped
                );
                emit_json_if_requested(&rep, bgo.json.as_deref())
            } else {
                let raw = fs::read(&bgo.input)
                    .with_context(|| format!("Reading {}", bgo.input.display()))?;
                let rep = inspect_prog061_container_bytes(&raw)?;
                println!("{}", format_bgo_inspection_line(&rep));
                emit_json_if_requested(&rep, bgo.json.as_deref())
            }
        }
        UnbrickActions::Passthrough(pt) => {
            if pt.rsp_plan {
                let cb = pt.callback_va.as_deref().map(parse_u32_radix).transpose()?;
                let plan = build_passthrough_rsp_plan(cb);
                print_lines(&format_rsp_plan_lines(&plan));
                return emit_json_if_requested(&plan, pt.json.as_deref());
            }

            let in_path = pt
                .input
                .context("--input <MAIN_3410PQ> is required unless --rsp-plan is set")?;
            let parsed_mode: MainPassthroughMode = pt.mode.parse()?;
            if let Some(out_path) = pt.output {
                let rep = patch_main_passthrough_file(&in_path, &out_path, parsed_mode)?;
                println!(
                    "Patched MAIN TBC ({}) -> {}",
                    rep.detected_mode,
                    rep.output_path.as_deref().unwrap_or("")
                );
                emit_json_if_requested(&rep, pt.json.as_deref())
            } else {
                let buf =
                    fs::read(&in_path).with_context(|| format!("Reading {}", in_path.display()))?;
                let rep = inspect_main_passthrough_sites(&buf)?;
                print_lines(&format_passthrough_inspection_lines(&rep));
                emit_json_if_requested(&rep, pt.json.as_deref())
            }
        }
        UnbrickActions::PrepareSd(sd) => bgo_tbc::handle_prepare_sd(sd),
        UnbrickActions::RebuildFromDump(args) => bgo_tbc::handle_rebuild_from_dump(args),
        UnbrickActions::VerifySd(args) => bgo_tbc::handle_verify_sd(args),
        UnbrickActions::TbcPlan(args) => bgo_tbc::handle_tbc_plan(args),
        UnbrickActions::BgoAudit(args) => bgo_tbc::handle_bgo_audit(args),
        UnbrickActions::BgoRepairPlan(args) => bgo_tbc::handle_bgo_repair_plan(args),
        UnbrickActions::BgoLiveApply(args) => bgo_tbc::handle_bgo_live_apply(args),
        UnbrickActions::InspectSd { target, json } => {
            let report = inspect_sd_target(&target)?;
            print_lines(&format_sd_disk_inspection_lines(&report));
            emit_json_if_requested(&report, json.as_deref())?;
            if report.overall_status == "FAIL" {
                bail!("SD raw MBR/BPB/FAT16 inspection failed; review [FAIL] checks above");
            }
            Ok(())
        }
        UnbrickActions::NorStub(stub) => nor_matshita::handle_nor_stub(
            &stub.start,
            &stub.len,
            stub.block_size,
            stub.load_addr.as_deref(),
            stub.out,
            stub.wrap_digadiag,
            stub.json.as_deref(),
        ),
        UnbrickActions::DecodeNorDump(dec) => {
            nor_matshita::handle_decode_nor_dump(&dec.input, dec.out, dec.json.as_deref())
        }
        UnbrickActions::MatshitaEmu { action } => nor_matshita::handle_matshita_emu(action),
        UnbrickActions::RegionCompat { action } => region_compat::handle_region_compat(action),
    }
}
