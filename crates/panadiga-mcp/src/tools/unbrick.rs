//! MCP handler for unified SD Unbrick Kit generation, FRM extraction, and media verification.

use std::{fs, path::Path};

use anyhow::{Result, bail};
use serde_json::{Value, json};

use panadiga_core::constants::DEFAULT_SD_UNBRICK_KIT_DIR;
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, SdLoaderProfile, SdUnbrickKitRequest,
    extract_modules_from_frm, inspect_unbrick_workspace,
    matshita_emu::{
        MatshitaBootScenario, MatshitaOddEmulator, build_sample_unlock_frm, parse_hex_payload,
        parse_unlock_container_mode, verify_unlock_frm_container,
    },
    prepare_sd_unbrick_kit, verify_sd_unbrick_media,
};

use crate::policy::McpSecurityPolicy;
use crate::workspace::{optional_non_empty_str, require_non_empty_str, resolve_workspace_path};

fn handle_matshita_emu_action(
    args: &Value,
    workspace_root: &Path,
    policy: &McpSecurityPolicy,
) -> Result<Value> {
    let sub = optional_non_empty_str(args, "emu_action")
        .unwrap_or("boot")
        .to_ascii_lowercase();
    match sub.as_str() {
        "boot" => {
            let scenario: MatshitaBootScenario = optional_non_empty_str(args, "scenario")
                .unwrap_or("healthy_with_disc")
                .parse()?;
            let mut emu = MatshitaOddEmulator::new(scenario);
            if let Some(rel) = optional_non_empty_str(args, "frm_path") {
                let bytes = fs::read(resolve_workspace_path(workspace_root, rel)?)?;
                let _ = emu.mount_unlock_frm("dvd", &bytes)?;
            }
            Ok(serde_json::to_value(emu.run_boot_sequence()?)?)
        }
        "inject_write" => {
            let payload_hex = optional_non_empty_str(args, "payload_hex").unwrap_or("50414E41");
            let payload = parse_hex_payload(payload_hex)?;
            let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let mode = args.get("mode").and_then(|v| v.as_u64()).unwrap_or(2) as u8;
            let use_vendor_ed = args
                .get("use_vendor_ed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let target_va = optional_non_empty_str(args, "target_va")
                .map(parse_u32_radix)
                .transpose()?;
            let mut emu = MatshitaOddEmulator::default();
            let (rec, snap) =
                emu.run_inject_write(target_va, offset, mode, use_vendor_ed, &payload)?;
            Ok(json!({ "record": rec, "snapshot": snap }))
        }
        "serial_cmd" => {
            let cmd = require_non_empty_str(args, "serial_cmd")?;
            let mut emu = MatshitaOddEmulator::default();
            if args
                .get("unlock_pass")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                let _ = emu.execute_drv_serial_command("PASS 0x4d415453 0x48495441")?;
            }
            Ok(serde_json::to_value(emu.execute_drv_serial_command(cmd)?)?)
        }
        "verify_frm" => {
            let kind = optional_non_empty_str(args, "container_kind").unwrap_or("digadiag");
            let rel = require_non_empty_str(args, "frm_path")?;
            let bytes = fs::read(resolve_workspace_path(workspace_root, rel)?)?;
            Ok(serde_json::to_value(verify_unlock_frm_container(
                kind, &bytes,
            )?)?)
        }
        "pack_unlock_frm" => {
            let kind = optional_non_empty_str(args, "container_kind").unwrap_or("sd");
            let mode = parse_unlock_container_mode(kind)?;
            let out_path_opt = optional_non_empty_str(args, "output_path")
                .map(|out_rel| resolve_workspace_path(workspace_root, out_rel))
                .transpose()?;
            if let Some(out_path) = out_path_opt.as_deref() {
                policy.require_file_write("panadiga_sd_unbrick_kit", Some(out_path))?;
            }
            let payload = match optional_non_empty_str(args, "payload_hex") {
                Some(h) => parse_hex_payload(h)?,
                None => vec![0xCBu8; 1024],
            };
            let (bytes, summary) = build_sample_unlock_frm(mode, &payload)?;
            let mut saved_path: Option<String> = None;
            if let Some(out_path) = out_path_opt.as_deref() {
                crate::workspace::write_cow_artifact(out_path, &bytes)?;
                saved_path = Some(out_path.to_string_lossy().into_owned());
            }
            Ok(json!({
                "verification": summary,
                "output_file": saved_path,
            }))
        }
        other => bail!(
            "Unsupported matshita_emu sub-action '{other}': expected boot, inject_write, serial_cmd, verify_frm, or pack_unlock_frm"
        ),
    }
}

pub fn handle_sd_unbrick_kit(args: &Value, workspace_root: &Path) -> Result<Value> {
    handle_sd_unbrick_kit_with_policy(args, workspace_root, &McpSecurityPolicy::read_only())
}

pub fn handle_sd_unbrick_kit_with_policy(
    args: &Value,
    workspace_root: &Path,
    policy: &McpSecurityPolicy,
) -> Result<Value> {
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
            let frm_path = resolve_workspace_path(workspace_root, frm)?;
            let out_path = resolve_workspace_path(workspace_root, out)?;
            policy.require_file_write("panadiga_sd_unbrick_kit", Some(&out_path))?;
            let unpack = args
                .get("unpack_payloads")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let rep = extract_modules_from_frm(&frm_path, &out_path, unpack)?;
            Ok(serde_json::to_value(rep)?)
        }
        "verify_sd" => {
            let dir =
                optional_non_empty_str(args, "sd_root_dir").unwrap_or(DEFAULT_SD_UNBRICK_KIT_DIR);
            let dir_path = resolve_workspace_path(workspace_root, dir)?;
            let rep = verify_sd_unbrick_media(&dir_path);
            Ok(serde_json::to_value(rep)?)
        }
        "prepare_sd" => {
            let out_dir =
                optional_non_empty_str(args, "output_dir").unwrap_or(DEFAULT_SD_UNBRICK_KIT_DIR);
            let resolved_out_dir = resolve_workspace_path(workspace_root, out_dir)?;
            policy.require_file_write("panadiga_sd_unbrick_kit", Some(&resolved_out_dir))?;
            let loader_profile: SdLoaderProfile = optional_non_empty_str(args, "loader_profile")
                .unwrap_or("ver061")
                .parse()?;
            let bgo_mode: Prog061BgoPatchMode = optional_non_empty_str(args, "bgo_mode")
                .unwrap_or("bypass")
                .parse()?;
            let allow_unsafe_auto_format = args
                .get("allow_unsafe_auto_format")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
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
            let expected_main_build =
                optional_non_empty_str(args, "expected_main_build").map(str::to_string);
            let allow_target_mismatch = args
                .get("allow_target_mismatch")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let sata_bridge_reset_fix = args
                .get("sata_bridge_reset_fix")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let sata_hdd_autoboot = args
                .get("sata_hdd_autoboot")
                .or_else(|| args.get("sata_bridge_stage2_boot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let main_bgo_retarget = args
                .get("main_bgo_retarget")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let nor_autoboot_capsule = args
                .get("nor_autoboot_capsule")
                .or_else(|| args.get("portao4_self_unpack"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let rep = prepare_sd_unbrick_kit(&SdUnbrickKitRequest {
                output_dir: resolved_out_dir,
                pana_dvd_path: optional_non_empty_str(args, "pana_dvd_path")
                    .map(|p| resolve_workspace_path(workspace_root, p))
                    .transpose()?,
                prog061_path: optional_non_empty_str(args, "prog061_path")
                    .map(|p| resolve_workspace_path(workspace_root, p))
                    .transpose()?,
                main_path: optional_non_empty_str(args, "main_path")
                    .map(|p| resolve_workspace_path(workspace_root, p))
                    .transpose()?,
                drv_path: optional_non_empty_str(args, "drv_path")
                    .map(|p| resolve_workspace_path(workspace_root, p))
                    .transpose()?,
                bgo_mode,
                allow_unsafe_auto_format,
                main_passthrough_mode: passthrough_mode,
                loader_profile,
                include_pana_sd,
                unlock_hdd_dynamic,
                hdd_target_gb,
                write_extracted_sources: save_sources,
                expected_main_build,
                allow_target_mismatch,
                sata_bridge_reset_fix,
                sata_hdd_autoboot,
                main_bgo_retarget,
                nor_autoboot_capsule,
            })?;
            Ok(serde_json::to_value(rep)?)
        }
        "matshita_emu" => handle_matshita_emu_action(args, workspace_root, policy),
        other => Ok(json!({
            "error": format!("Unsupported panadiga_sd_unbrick_kit action '{other}': expected status, extract_frm, prepare_sd, verify_sd, or matshita_emu")
        })),
    }
}
