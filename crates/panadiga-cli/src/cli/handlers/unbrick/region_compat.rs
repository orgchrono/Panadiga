//! CLI handler for deterministic Region & Model Compatibility patching (`region-compat`).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use panadiga_core::re::{
    RegionCompatMode, inspect_dev_bin_bytes, inspect_main_region_compat, patch_dev_bin_bytes,
    patch_main_region_compat_in_place, retarget_frm_toc_bytes,
};

use super::super::json_io::write_cli_output_bytes;
use super::render::{emit_json_if_requested, print_lines};
use crate::cli::args::unbrick::RegionCompatAction;

fn write_and_report<T: serde::Serialize>(
    out_path: &Path,
    bytes: &[u8],
    report: &T,
    summary_lines: &[String],
    json: Option<&Path>,
) -> Result<()> {
    write_cli_output_bytes(out_path, bytes)?;
    print_lines(summary_lines);
    emit_json_if_requested(report, json)
}

pub fn handle_region_compat(action: RegionCompatAction) -> Result<()> {
    match action {
        RegionCompatAction::InspectMain { input, json } => {
            let bytes = fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
            let rep = inspect_main_region_compat(&bytes)?;
            println!(
                "MAIN Region Compat [{}]: state={} @ {} ({})",
                rep.build_profile, rep.state, rep.va_hex, rep.sha256
            );
            emit_json_if_requested(&rep, json.as_deref())
        }
        RegionCompatAction::PatchMain {
            input,
            output,
            mode,
            json,
        } => {
            let mut bytes =
                fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
            let parsed_mode: RegionCompatMode = mode.parse()?;
            let rep = patch_main_region_compat_in_place(&mut bytes, parsed_mode)?;
            let lines = vec![format!(
                "Patched MAIN [{}] -> {} ({} @ {}) -> {}",
                rep.build_profile,
                rep.state,
                rep.va_hex,
                rep.sha256,
                output.display()
            )];
            write_and_report(&output, &bytes, &rep, &lines, json.as_deref())
        }
        RegionCompatAction::InspectDev { input, json } => {
            let bytes = fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
            let rep = inspect_dev_bin_bytes(&bytes)?;
            println!(
                "DEV.BIN: model='{}' oem='{}' sig='{}' md5={} sha256={}",
                rep.model, rep.oem, rep.signature, rep.md5, rep.sha256
            );
            emit_json_if_requested(&rep, json.as_deref())
        }
        RegionCompatAction::PatchDev {
            input,
            output,
            model,
            oem,
            json,
        } => {
            let bytes = fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
            let (out, rep) = patch_dev_bin_bytes(&bytes, &model, oem)?;
            let lines = vec![format!(
                "Patched DEV.BIN -> model='{}' oem='{}' md5={} -> {}",
                rep.model,
                rep.oem,
                rep.md5,
                output.display()
            )];
            write_and_report(&output, &out, &rep, &lines, json.as_deref())
        }
        RegionCompatAction::RetargetFrm {
            input,
            output,
            hw,
            model_group,
            oem,
            target_flag,
            json,
        } => {
            let bytes = fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
            let (out, rep) = retarget_frm_toc_bytes(
                &bytes,
                hw.as_deref(),
                model_group.as_deref(),
                oem,
                target_flag,
            )?;
            let mut lines = vec![format!(
                "Retargeted .FRM [{}]: updated {} active entries (sha256={}) -> {}",
                rep.layout,
                rep.updated_entries,
                rep.sha256,
                output.display()
            )];
            for e in &rep.entries {
                if !e.module_name.trim().is_empty() && e.module_size > 0 {
                    lines.push(format!(
                        "  [{}] {} | hw={} | model={} | flag={} | sum_ok={}",
                        e.index,
                        e.module_name,
                        e.hardware_family,
                        e.model_group,
                        e.target_flag,
                        e.list_sum_valid
                    ));
                }
            }
            write_and_report(&output, &out, &rep, &lines, json.as_deref())
        }
    }
}
