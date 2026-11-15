//! Reverse-engineering & unbrick MCP tools: HDD capacity unlock, NOR dumper stub, BGO unbrick, and MAIN passthrough.

use anyhow::Result;
use panadiga_core::crypto::{ExtrheadCryptMode, unpack_extrhead};
use panadiga_core::hdd::{
    default_capacity_patched_output_path, inspect_main_hdd_capacity, patch_main_hdd_capacity,
};
use panadiga_core::nvram::calculate_hashes;
use panadiga_core::re::{
    MainPassthroughMode, PROG061_EXPECTED_SIZE, Prog061BgoPatchMode, build_nor_dumper_artifact,
    build_passthrough_rsp_plan, inspect_main_passthrough_sites, inspect_prog061_bgo_sites,
    patch_main_passthrough_file, patch_prog061_or_build_digadiag_with_policy,
    resolve_nor_stub_load_addr,
};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

use crate::workspace::{
    optional_non_empty_str, parse_u64_hex_or_dec, paths_refer_to_same_file, require_non_empty_str,
    resolve_workspace_path,
};
pub(super) fn parse_u32_arg(args: &Value, key: &str, default: u32) -> Result<u32> {
    let val = match args.get(key) {
        Some(Value::String(s)) => parse_u64_hex_or_dec(s).map_err(|e| anyhow::anyhow!(e))?,
        Some(Value::Number(n)) => n
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Invalid unsigned integer for '{key}'"))?,
        Some(Value::Null) | None => return Ok(default),
        _ => anyhow::bail!("Expected hex/decimal string or integer for '{key}'"),
    };
    u32::try_from(val).map_err(|_| anyhow::anyhow!("Value for '{key}' exceeds u32: {val}"))
}

fn dispatch_inspect_or_patch<FInspect, FDefaultOut, FPatch>(
    args: &Value,
    workspace_root: &Path,
    missing_label: &str,
    inspect_fn: FInspect,
    default_out_fn: FDefaultOut,
    patch_fn: FPatch,
) -> Result<Value>
where
    FInspect: FnOnce(&Path) -> Result<Value>,
    FDefaultOut: FnOnce(&Path) -> Result<PathBuf>,
    FPatch: FnOnce(&Path, &Path) -> Result<Value>,
{
    let in_raw = require_non_empty_str(args, "input_path")?;
    let in_path = resolve_workspace_path(workspace_root, in_raw);
    if !in_path.is_file() {
        return Ok(json!({ "error": format!("{missing_label} not found: {}", in_path.display()) }));
    }

    let action = optional_non_empty_str(args, "action").unwrap_or("inspect");
    if action.eq_ignore_ascii_case("inspect") {
        return inspect_fn(&in_path);
    }
    if !action.eq_ignore_ascii_case("patch") {
        return Ok(
            json!({ "error": format!("Unsupported action '{action}': expected 'inspect' or 'patch'") }),
        );
    }

    let out_path = match optional_non_empty_str(args, "output_path") {
        Some(out_raw) => resolve_workspace_path(workspace_root, out_raw),
        None => default_out_fn(&in_path)?,
    };
    if paths_refer_to_same_file(&in_path, &out_path) {
        return Ok(json!({
            "error": "NON-DESTRUCTIVE SAFETY RULE: output_path must be a separate copy and cannot overwrite input_path."
        }));
    }
    patch_fn(&in_path, &out_path)
}

pub fn handle_hdd_capacity(args: &Value, workspace_root: &Path) -> Result<Value> {
    let mode_str = optional_non_empty_str(args, "mode").unwrap_or("dynamic");
    let target_gb = args
        .get("target_gb")
        .and_then(|v| v.as_u64())
        .map(u32::try_from)
        .transpose()
        .map_err(|_| anyhow::anyhow!("target_gb exceeds u32 range"))?;

    dispatch_inspect_or_patch(
        args,
        workspace_root,
        "Input firmware/HDD file",
        |in_path| Ok(serde_json::to_value(inspect_main_hdd_capacity(in_path)?)?),
        |in_path| {
            Ok(default_capacity_patched_output_path(
                in_path, mode_str, target_gb,
            ))
        },
        |in_path, out_path| {
            let report = patch_main_hdd_capacity(in_path, out_path, mode_str, target_gb)?;
            Ok(serde_json::to_value(report)?)
        },
    )
}

pub fn handle_nor_dumper_stub(args: &Value, workspace_root: &Path) -> Result<Value> {
    let start_addr = parse_u32_arg(args, "start_addr", 0x8400_0000)?;
    let total_len = parse_u32_arg(args, "total_len", 0x0080_0000)?;
    let block_size = parse_u32_arg(args, "block_size", 4096)?;
    let wrap_digadiag = args
        .get("wrap_digadiag")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let explicit_load = match args.get("load_addr") {
        Some(Value::Null) | None => None,
        Some(_) => Some(parse_u32_arg(args, "load_addr", 0)?),
    };
    let load_addr = resolve_nor_stub_load_addr(explicit_load, wrap_digadiag);

    let (raw_stub_size_bytes, payload) =
        build_nor_dumper_artifact(start_addr, total_len, block_size, load_addr, wrap_digadiag)?;
    let (_, stub_sha256) = calculate_hashes(&payload);

    let mut saved_path: Option<String> = None;
    if let Some(out_raw) = optional_non_empty_str(args, "output_path") {
        let out_path = resolve_workspace_path(workspace_root, out_raw);
        panadiga_core::re::bgo_unbrick::ensure_parent_dir(&out_path)?;
        fs::write(&out_path, &payload)?;
        saved_path = Some(out_path.to_string_lossy().into_owned());
    }

    Ok(json!({
        "status": "success",
        "start_addr": format!("0x{start_addr:08X}"),
        "total_len": total_len,
        "block_size": block_size,
        "load_addr": format!("0x{load_addr:08X}"),
        "wrap_digadiag": wrap_digadiag,
        "raw_stub_size_bytes": raw_stub_size_bytes,
        "stub_size_bytes": payload.len(),
        "stub_sha256": stub_sha256,
        "stub_hex": hex::encode(&payload),
        "output_file": saved_path,
    }))
}

pub fn handle_bgo_unbrick(args: &Value, workspace_root: &Path) -> Result<Value> {
    let mode_str = optional_non_empty_str(args, "mode").unwrap_or("bypass");
    let allow_unsafe_bgo_format = args
        .get("allow_unsafe_bgo_format")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let wrap = args
        .get("wrap_as_digadiag")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    dispatch_inspect_or_patch(
        args,
        workspace_root,
        "PROG061/DIGADIAG file",
        |in_path| {
            let raw = fs::read(in_path)?;
            let payload = if raw.starts_with(b"EXTRHEAD") || raw.len() != PROG061_EXPECTED_SIZE {
                unpack_extrhead(&raw, ExtrheadCryptMode::Auto)?.decompressed
            } else {
                raw
            };
            Ok(serde_json::to_value(inspect_prog061_bgo_sites(&payload)?)?)
        },
        |in_path| {
            let parent = in_path.parent().unwrap_or_else(|| Path::new("."));
            Ok(if wrap {
                parent.join("DIGADIAG_BGO_UNBRICK.FRM")
            } else {
                parent.join("PROG061_BGO_PATCHED.bin")
            })
        },
        |in_path, out_path| {
            let mode: Prog061BgoPatchMode = mode_str.parse()?;
            let report = patch_prog061_or_build_digadiag_with_policy(
                in_path,
                out_path,
                mode,
                wrap,
                allow_unsafe_bgo_format,
            )?;
            Ok(serde_json::to_value(report)?)
        },
    )
}

pub fn handle_main_passthrough(args: &Value, workspace_root: &Path) -> Result<Value> {
    let action = optional_non_empty_str(args, "action").unwrap_or("inspect");
    if action.eq_ignore_ascii_case("rsp_plan") {
        let cb_va = if args.get("callback_target_va").is_some_and(|v| !v.is_null()) {
            Some(parse_u32_arg(args, "callback_target_va", 0)?)
        } else {
            None
        };
        return Ok(serde_json::to_value(build_passthrough_rsp_plan(cb_va))?);
    }

    let mode_str = optional_non_empty_str(args, "mode").unwrap_or("av_passthrough");
    let mode = MainPassthroughMode::from_str_mode(mode_str)?;

    dispatch_inspect_or_patch(
        args,
        workspace_root,
        "Unpacked EH55_MAIN binary",
        |in_path| {
            let data = fs::read(in_path)?;
            Ok(serde_json::to_value(inspect_main_passthrough_sites(
                &data,
            )?)?)
        },
        |in_path| {
            let parent = in_path.parent().unwrap_or_else(|| Path::new("."));
            Ok(parent.join(format!("EH55_MAIN_{}.bin", mode.as_str())))
        },
        |in_path, out_path| {
            let report = patch_main_passthrough_file(in_path, out_path, mode)?;
            Ok(serde_json::to_value(report)?)
        },
    )
}
