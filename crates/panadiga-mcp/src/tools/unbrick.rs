//! MCP handler for unified SD Unbrick Kit generation, FRM extraction, and media verification.

use anyhow::Result;
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, SdUnbrickKitRequest, extract_modules_from_frm,
    inspect_unbrick_workspace, prepare_sd_unbrick_kit, verify_sd_unbrick_media,
};
use serde_json::{Value, json};
use std::path::Path;

use crate::workspace::{optional_non_empty_str, require_non_empty_str, resolve_workspace_path};

pub fn handle_sd_unbrick_kit(args: &Value, workspace_root: &Path) -> Result<Value> {
    let action = optional_non_empty_str(args, "action")
        .unwrap_or("status")
        .to_ascii_lowercase();

    match action.as_str() {
        "status" => Ok(serde_json::to_value(inspect_unbrick_workspace(Some(
            workspace_root,
        )))?),
        "extract_frm" => {
            let frm = optional_non_empty_str(args, "pana_dvd_path")
                .unwrap_or("firmware_and_images/PANA_DVD.FRM");
            let out = require_non_empty_str(args, "output_dir")?;
            let unpack = args
                .get("unpack_payloads")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let rep = extract_modules_from_frm(
                &resolve_workspace_path(workspace_root, frm),
                &resolve_workspace_path(workspace_root, out),
                unpack,
            )?;
            Ok(serde_json::to_value(rep)?)
        }
        "verify_sd" => {
            let dir = require_non_empty_str(args, "sd_root_dir")?;
            let rep = verify_sd_unbrick_media(&resolve_workspace_path(workspace_root, dir));
            Ok(serde_json::to_value(rep)?)
        }
        "prepare_sd" => {
            let out_dir = require_non_empty_str(args, "output_dir")?;
            let bgo_mode: Prog061BgoPatchMode = optional_non_empty_str(args, "bgo_mode")
                .unwrap_or("auto_format")
                .parse()?;
            let passthrough_mode: MainPassthroughMode =
                optional_non_empty_str(args, "passthrough_mode")
                    .unwrap_or("av_passthrough")
                    .parse()?;
            let include_pana_sd = args
                .get("include_pana_sd")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let unlock_hdd_dynamic = args
                .get("unlock_hdd_dynamic")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let hdd_target_gb = args
                .get("hdd_target_gb")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let save_sources = args
                .get("write_extracted_sources")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let rep = prepare_sd_unbrick_kit(&SdUnbrickKitRequest {
                output_dir: resolve_workspace_path(workspace_root, out_dir),
                pana_dvd_path: optional_non_empty_str(args, "pana_dvd_path")
                    .map(|p| resolve_workspace_path(workspace_root, p)),
                prog061_path: optional_non_empty_str(args, "prog061_path")
                    .map(|p| resolve_workspace_path(workspace_root, p)),
                main_path: optional_non_empty_str(args, "main_path")
                    .map(|p| resolve_workspace_path(workspace_root, p)),
                drv_path: optional_non_empty_str(args, "drv_path")
                    .map(|p| resolve_workspace_path(workspace_root, p)),
                bgo_mode,
                main_passthrough_mode: passthrough_mode,
                include_pana_sd,
                unlock_hdd_dynamic,
                hdd_target_gb,
                write_extracted_sources: save_sources,
            })?;
            Ok(serde_json::to_value(rep)?)
        }
        other => Ok(json!({
            "error": format!("Unsupported panadiga_sd_unbrick_kit action '{other}': expected status, extract_frm, prepare_sd, or verify_sd")
        })),
    }
}
