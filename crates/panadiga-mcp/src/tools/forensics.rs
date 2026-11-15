//! Forensics MCP tools: HDD MEIHFS inspection, surgical NVRAM cleaning, PANA_DVD.FRM TOC, and hex dump.

use anyhow::Result;
use panadiga_core::constants::{
    DEFAULT_FRM_CONTAINER_PATH, DEFAULT_HDD_IMAGE_PATH, LBA_JOURNAL_CATALOG_OFFSET,
    LBA_MIRROR_MBR_SECTOR, LBA_PARTITION_1_OFFSET, LBA_PARTITION_2_OFFSET, LBA_SECTOR_SIZE_BYTES,
    PANASONIC_HDD_PARTITION_MAGIC, SPANSION_BOOTLOADER_SIZE_BYTES, SPANSION_FLASH_SIZE_BYTES,
    SPANSION_NVRAM_BASE_OFFSET,
};
use panadiga_core::crypto::entropy::analyze_frm_slices;
use panadiga_core::crypto::toc::analyze_toc;
use panadiga_core::crypto::{ExtrheadCryptMode, unpack_extrhead};
use panadiga_core::hdd::inspect::inspect_hdd_slice;
use panadiga_core::hdd::{
    default_capacity_patched_output_path, inspect_main_hdd_capacity, open_image_buffer,
    patch_main_hdd_capacity,
};
use panadiga_core::headers::hex_dump;
use panadiga_core::nvram::{calculate_hashes, clean_flash_nvram_slice};
use panadiga_core::re::{
    DEFAULT_NOR_STUB_LOAD_ADDR, MainPassthroughMode, PROG061_EXPECTED_SIZE, Prog061BgoPatchMode,
    build_passthrough_rsp_plan, build_readonly_nor_dumper_stub, inspect_main_passthrough_sites,
    inspect_prog061_bgo_sites, patch_main_passthrough_file,
    patch_prog061_or_build_digadiag_with_policy,
};
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::workspace::{
    optional_non_empty_str, parse_u64_hex_or_dec, paths_refer_to_same_file, require_non_empty_str,
    resolve_workspace_path,
};

pub fn handle_inspect_hdd(args: &Value, workspace_root: &Path) -> Value {
    let rel_path = optional_non_empty_str(args, "image_path").unwrap_or(DEFAULT_HDD_IMAGE_PATH);

    let img_file = resolve_workspace_path(workspace_root, rel_path);
    if !img_file.is_file() {
        return json!({ "error": format!("Image file not found: {}", img_file.display()) });
    }

    let buf = match open_image_buffer(&img_file) {
        Ok(b) => b,
        Err(e) => return json!({ "error": format!("Failed to open HDD image: {}", e) }),
    };

    let label = img_file.to_string_lossy().to_string();
    match inspect_hdd_slice(&buf, &label) {
        Ok(report) => {
            let magic_hex = hex::encode(PANASONIC_HDD_PARTITION_MAGIC);
            json!({
                "image_path": report.image_path,
                "size_bytes": report.file_size_bytes,
                "size_mb": (report.file_size_mb * 100.0).round() / 100.0,
                "mbr_tag_sh_valid": report.mbr_tables.magic_ascii == "SH",
                "disk_guid": report.mbr_tables.disk_volume_guid,
                "mirror_sector_57344_identical": report.mbr_tables.mirror_is_bit_identical,
                "mirror_lba": LBA_MIRROR_MBR_SECTOR,
                "partitions": report.partitions,
                "known_partitions": [
                    {
                        "name": "Journaling / Catalog",
                        "offset_mb": LBA_JOURNAL_CATALOG_OFFSET / (1024 * 1024),
                        "lba": LBA_JOURNAL_CATALOG_OFFSET / (LBA_SECTOR_SIZE_BYTES as u64)
                    },
                    {
                        "name": "Firmware Primary",
                        "offset_mb": LBA_PARTITION_1_OFFSET / (1024 * 1024),
                        "magic": magic_hex
                    },
                    {
                        "name": "Firmware Backup",
                        "offset_mb": LBA_PARTITION_2_OFFSET / (1024 * 1024),
                        "magic": magic_hex
                    }
                ]
            })
        }
        Err(e) => json!({ "error": format!("HDD inspection failed: {}", e) }),
    }
}

pub fn handle_clean_nvram(args: &Value, workspace_root: &Path) -> Result<Value> {
    let input_raw = require_non_empty_str(args, "input_file")?;
    let input_path = resolve_workspace_path(workspace_root, input_raw);
    if !input_path.is_file() {
        return Ok(json!({
            "error": format!("Input flash dump not found: {}", input_path.display())
        }));
    }

    let output_raw = optional_non_empty_str(args, "output_file");
    if let Some(out_str) = output_raw {
        let out_path = resolve_workspace_path(workspace_root, out_str);
        if paths_refer_to_same_file(&input_path, &out_path) {
            return Ok(json!({
                "error": "Forensic Invariant Violation: Cannot overwrite original flash dump in-place."
            }));
        }
    }

    let data = fs::read(&input_path)?;
    if data.len() != SPANSION_FLASH_SIZE_BYTES {
        return Ok(json!({
            "error": format!(
                "Invalid flash dump size: {} bytes (expected {} / 8 MB)",
                data.len(),
                SPANSION_FLASH_SIZE_BYTES
            )
        }));
    }

    let (_, ipl_sha256) = calculate_hashes(&data[..SPANSION_BOOTLOADER_SIZE_BYTES]);
    let (_, nvram_orig_sha256) =
        calculate_hashes(&data[SPANSION_NVRAM_BASE_OFFSET..SPANSION_FLASH_SIZE_BYTES]);

    let out_label = output_raw.unwrap_or("<analysis-only>");
    let cleaned = clean_flash_nvram_slice(&data, input_raw, out_label, "<memory-backup>")?;

    let mut saved = false;
    if let Some(out_str) = output_raw {
        let out_path = resolve_workspace_path(workspace_root, out_str);
        if let Some(parent) = out_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out_path, &cleaned.cleaned_bytes)?;
        saved = true;
    }

    Ok(json!({
        "status": "success",
        "ipl_bytes_checked": SPANSION_BOOTLOADER_SIZE_BYTES,
        "ipl_sha256": ipl_sha256,
        "nvram_original_sha256": nvram_orig_sha256,
        "is_all_ff": cleaned.report.nvram_non_ff_bytes == 0,
        "sanitized_file_created": saved,
        "output_file": if saved { output_raw } else { None },
    }))
}

pub fn handle_inspect_frm(args: &Value, workspace_root: &Path) -> Result<Value> {
    let rel_path = optional_non_empty_str(args, "frm_path").unwrap_or(DEFAULT_FRM_CONTAINER_PATH);
    let frm_path = resolve_workspace_path(workspace_root, rel_path);
    if !frm_path.is_file() {
        return Ok(json!({ "error": format!("FRM file not found: {}", frm_path.display()) }));
    }

    let data = fs::read(&frm_path)?;
    let toc = analyze_toc(&data);
    let label = frm_path.to_string_lossy().to_string();
    let crypto_report = analyze_frm_slices(&data, None, &label, None, 25);

    Ok(json!({
        "file": rel_path,
        "size_bytes": data.len(),
        "toc": toc,
        "cryptanalysis": crypto_report,
    }))
}

pub fn handle_read_hex(args: &Value, workspace_root: &Path) -> Result<Value> {
    let rel_path = require_non_empty_str(args, "file_path")?;
    let path = resolve_workspace_path(workspace_root, rel_path);
    if !path.is_file() {
        return Ok(json!({ "error": format!("Binary file not found: {}", path.display()) }));
    }

    let offset = match args.get("offset") {
        Some(Value::String(s)) => parse_u64_hex_or_dec(s).map_err(|e| anyhow::anyhow!(e))?,
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
        _ => 0,
    };
    let req_len = args
        .get("length")
        .and_then(|v| v.as_u64())
        .unwrap_or(256)
        .clamp(16, 4096) as usize;

    let mut file = File::open(&path)?;
    let file_len = file.metadata()?.len();
    if offset >= file_len {
        return Ok(json!({
            "error": format!("Offset 0x{:X} exceeds file size {} bytes", offset, file_len)
        }));
    }

    file.seek(SeekFrom::Start(offset))?;
    let to_read = req_len.min((file_len - offset) as usize);
    let mut buf = vec![0u8; to_read];
    file.read_exact(&mut buf)?;

    Ok(json!({
        "file": rel_path,
        "file_size_bytes": file_len,
        "offset": format!("0x{:08X}", offset),
        "bytes_read": to_read,
        "hex_dump": hex_dump(&buf, offset as usize, to_read),
    }))
}

fn parse_u32_arg(args: &Value, key: &str, default: u32) -> Result<u32> {
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
    let load_addr = parse_u32_arg(args, "load_addr", DEFAULT_NOR_STUB_LOAD_ADDR)?;

    let stub = build_readonly_nor_dumper_stub(start_addr, total_len, block_size, load_addr)?;
    let (_, stub_sha256) = calculate_hashes(&stub);

    let mut saved_path: Option<String> = None;
    if let Some(out_raw) = optional_non_empty_str(args, "output_path") {
        let out_path = resolve_workspace_path(workspace_root, out_raw);
        panadiga_core::re::bgo_unbrick::ensure_parent_dir(&out_path)?;
        fs::write(&out_path, &stub)?;
        saved_path = Some(out_path.to_string_lossy().into_owned());
    }

    Ok(json!({
        "status": "success",
        "start_addr": format!("0x{start_addr:08X}"),
        "total_len": total_len,
        "block_size": block_size,
        "load_addr": format!("0x{load_addr:08X}"),
        "stub_size_bytes": stub.len(),
        "stub_sha256": stub_sha256,
        "stub_hex": hex::encode(&stub),
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
