//! Handlers for `panadiga_region_compat`, `panadiga_hdd_flash_inspect`,
//! `panadiga_tbc_appliance_plan`, and `panadiga_nor_sector_audit`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use panadiga_core::hdd::capacity_patch::specs::blocks_2kb_for_gb_bounded;
use panadiga_core::hdd::writer::{
    PhysicalDriveInfo, compute_eh55_mirror_main_lba, enrich_safe_target_disks,
    inspect_safe_target_disks,
};
use panadiga_core::re::bgo_unbrick::ensure_parent_dir;
use panadiga_core::re::nor_dumper::analyze_spansion_nor_sectors;
use panadiga_core::re::region_compat::{
    LbsRegionPresetId, TvStandardKind, inspect_tv_calibration_bundle,
};
use panadiga_core::re::tbc_appliance::{TbcApplianceMode, build_tbc_appliance_plan};
use panadiga_core::re::{
    RegionCompatMode, inspect_dev_bin_bytes, inspect_main_region_compat, patch_dev_bin_bytes,
    patch_main_region_compat_in_place, retarget_frm_toc_bytes,
};
use serde_json::{Value, json};

use crate::policy::McpSecurityPolicy;
use crate::workspace::{
    optional_non_empty_str, paths_refer_to_same_file, require_non_empty_str, resolve_workspace_path,
};

fn parse_lbs_preset(raw: &str) -> Result<LbsRegionPresetId> {
    let norm = raw.trim().to_ascii_lowercase();
    let mapped = match norm.as_str() {
        "br-pal-m" => "br_pal_m_r4",
        "us-ntsc" => "us_ntsc_r1",
        "eu-pal" => "eu_pal_r2",
        _ => norm.as_str(),
    };
    mapped.parse()
}

fn parse_mock_drive(item: &Value) -> Result<PhysicalDriveInfo> {
    let mut obj = item
        .as_object()
        .cloned()
        .context("mock_disks entries must be JSON objects")?;
    let sz = obj.get("size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
    let is_sys = obj
        .get("is_system_drive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    obj.entry("display_name")
        .or_insert_with(|| json!("Simulated Disk"));
    obj.entry("size_gb")
        .or_insert_with(|| json!(sz as f64 / 1_000_000_000.0));
    obj.entry("safe_to_write").or_insert_with(|| json!(!is_sys));
    obj.entry("lockout_reason").or_insert(Value::Null);
    serde_json::from_value(Value::Object(obj)).context("Invalid mock_disks entry")
}

pub(super) fn resolve_cow_paths(
    args: &Value,
    workspace_root: &Path,
    policy: &McpSecurityPolicy,
) -> Result<(PathBuf, PathBuf)> {
    let in_path =
        resolve_workspace_path(workspace_root, require_non_empty_str(args, "input_path")?)?;
    let out_path =
        resolve_workspace_path(workspace_root, require_non_empty_str(args, "output_path")?)?;
    policy.require_file_write("panadiga_region_compat", Some(&out_path))?;
    if !in_path.is_file() {
        bail!("Input file not found: {}", in_path.display());
    }
    if paths_refer_to_same_file(&in_path, &out_path) {
        bail!("CoW safety violation: output_path must differ from input_path");
    }
    Ok((in_path, out_path))
}

fn optional_single_char(args: &Value, key: &str) -> Result<Option<char>> {
    let Some(s) = optional_non_empty_str(args, key) else {
        return Ok(None);
    };
    let mut chars = s.chars();
    let ch = chars.next().context("Empty character string")?;
    if chars.next().is_some() {
        bail!("Expected single ASCII character for '{key}', got '{s}'");
    }
    Ok(Some(ch))
}

fn read_opt_file(args: &Value, key: &str, ws: &Path) -> Result<Option<Vec<u8>>> {
    optional_non_empty_str(args, key)
        .map(|p| {
            let abs = resolve_workspace_path(ws, p)?;
            fs::read(&abs).with_context(|| format!("Reading {key}: {}", abs.display()))
        })
        .transpose()
}

pub fn handle_region_compat(args: &Value, workspace_root: &Path) -> Result<Value> {
    handle_region_compat_with_policy(args, workspace_root, &McpSecurityPolicy::read_only())
}

pub fn handle_region_compat_with_policy(
    args: &Value,
    workspace_root: &Path,
    policy: &McpSecurityPolicy,
) -> Result<Value> {
    let action = optional_non_empty_str(args, "action")
        .unwrap_or("inspect_tv_bundle")
        .to_ascii_lowercase();
    match action.as_str() {
        "inspect_main" => {
            let in_path =
                resolve_workspace_path(workspace_root, require_non_empty_str(args, "input_path")?)?;
            let bytes = fs::read(&in_path)?;
            Ok(serde_json::to_value(inspect_main_region_compat(&bytes)?)?)
        }
        "patch_main" => {
            let (in_path, out_path) = resolve_cow_paths(args, workspace_root, policy)?;
            let mut bytes = fs::read(&in_path)?;
            let mode: RegionCompatMode = optional_non_empty_str(args, "mode")
                .unwrap_or("bypass")
                .parse()?;
            let rep = patch_main_region_compat_in_place(&mut bytes, mode)?;
            ensure_parent_dir(&out_path)?;
            fs::write(&out_path, &bytes)?;
            Ok(json!({ "cow_output": out_path.display().to_string(), "report": rep }))
        }
        "inspect_dev" => {
            let in_path =
                resolve_workspace_path(workspace_root, require_non_empty_str(args, "input_path")?)?;
            let bytes = fs::read(&in_path)?;
            Ok(serde_json::to_value(inspect_dev_bin_bytes(&bytes)?)?)
        }
        "patch_dev" => {
            let (in_path, out_path) = resolve_cow_paths(args, workspace_root, policy)?;
            let model = require_non_empty_str(args, "model")?;
            let oem = optional_single_char(args, "oem")?;
            let bytes = fs::read(&in_path)?;
            let (out, rep) = patch_dev_bin_bytes(&bytes, model, oem)?;
            ensure_parent_dir(&out_path)?;
            fs::write(&out_path, &out)?;
            Ok(json!({
                "cow_output": out_path.display().to_string(),
                "metadata_and_dev_oem_only": true,
                "report": rep
            }))
        }
        "retarget_frm" => {
            let (in_path, out_path) = resolve_cow_paths(args, workspace_root, policy)?;
            let oem = optional_single_char(args, "oem")?;
            let flag = args
                .get("target_flag")
                .and_then(|v| v.as_u64())
                .map(u8::try_from)
                .transpose()
                .map_err(|_| anyhow::anyhow!("target_flag exceeds u8"))?;
            let bytes = fs::read(&in_path)?;
            let (out, rep) = retarget_frm_toc_bytes(
                &bytes,
                optional_non_empty_str(args, "hw"),
                optional_non_empty_str(args, "model_group"),
                oem,
                flag,
            )?;
            ensure_parent_dir(&out_path)?;
            fs::write(&out_path, &out)?;
            Ok(json!({ "cow_output": out_path.display().to_string(), "report": rep }))
        }
        "inspect_tv_bundle" => {
            let preset =
                parse_lbs_preset(optional_non_empty_str(args, "preset").unwrap_or("br-pal-m"))?;
            let tv_std = optional_non_empty_str(args, "tv_standard")
                .map(str::parse::<TvStandardKind>)
                .transpose()?;
            let dev_b = read_opt_file(args, "dev_bin_path", workspace_root)?;
            let main_b = read_opt_file(args, "main_path", workspace_root)?;
            let frm_b = read_opt_file(args, "frm_path", workspace_root)?;
            let nv_b = read_opt_file(args, "nvram_path", workspace_root)?;
            let rep = inspect_tv_calibration_bundle(
                preset,
                tv_std,
                dev_b.as_deref(),
                main_b.as_deref(),
                frm_b.as_deref(),
                nv_b.as_deref(),
            )?;
            Ok(serde_json::to_value(rep)?)
        }
        other => bail!("Unsupported region_compat action '{other}'"),
    }
}

pub fn handle_hdd_flash_inspect(args: &Value) -> Result<Value> {
    let target_gb = args
        .get("target_gb")
        .and_then(|v| v.as_u64())
        .map(u32::try_from)
        .transpose()
        .map_err(|_| anyhow::anyhow!("target_gb exceeds u32"))?
        .unwrap_or(250);
    let allow_ext = args
        .get("allow_extended_4tb")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let blocks_2kb = blocks_2kb_for_gb_bounded(target_gb, allow_ext)?;

    let disks = if let Some(arr) = args.get("mock_disks").and_then(|v| v.as_array()) {
        let raw: Vec<PhysicalDriveInfo> = arr
            .iter()
            .map(parse_mock_drive)
            .collect::<Result<Vec<_>>>()?;
        enrich_safe_target_disks(&raw)
    } else {
        inspect_safe_target_disks()
    };

    let explicit_bytes = args.get("disk_bytes").and_then(|v| v.as_u64());
    let mirror_main_lba = explicit_bytes
        .map(compute_eh55_mirror_main_lba)
        .transpose()?;
    let safe_count = disks.iter().filter(|d| d.safe_to_flash).count();
    let blocked_count = disks.len() - safe_count;

    Ok(json!({
        "read_only": true,
        "dry_run_only": true,
        "target_gb": target_gb,
        "allow_extended_4tb": allow_ext,
        "blocks_2kb": blocks_2kb,
        "blocks_2kb_hex": format!("0x{blocks_2kb:08X}"),
        "mirror_main_lba": mirror_main_lba,
        "safe_target_count": safe_count,
        "blocked_target_count": blocked_count,
        "disks": disks,
    }))
}

pub fn handle_tbc_appliance_plan(args: &Value) -> Result<Value> {
    let mode: TbcApplianceMode = optional_non_empty_str(args, "mode")
        .unwrap_or("stage15_digadiag_tbc")
        .parse()?;
    let std_kind: TvStandardKind = optional_non_empty_str(args, "tv_standard")
        .unwrap_or("ntsc")
        .parse()?;
    let exp = args
        .get("experimental_ram_chainload")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let plan = build_tbc_appliance_plan(mode, std_kind, exp)?;
    Ok(serde_json::to_value(plan)?)
}

pub fn handle_nor_sector_audit(args: &Value, workspace_root: &Path) -> Result<Value> {
    let raw_path = optional_non_empty_str(args, "nor_path")
        .or_else(|| optional_non_empty_str(args, "input_path"))
        .context("Missing required 'nor_path' argument")?;
    let nor_path = resolve_workspace_path(workspace_root, raw_path)?;
    let nor = fs::read(&nor_path).with_context(|| format!("Reading {}", nor_path.display()))?;
    let rep = analyze_spansion_nor_sectors(&nor)?;
    Ok(serde_json::to_value(rep)?)
}
