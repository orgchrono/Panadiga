//! Forensics MCP tools: HDD MEIHFS inspection, surgical NVRAM cleaning, PANA_DVD.FRM TOC, and hex dump.

use anyhow::Result;
use panadiga_core::constants::{
    DEFAULT_FRM_CONTAINER_PATH, DEFAULT_HDD_IMAGE_PATH, LBA_JOURNAL_CATALOG_OFFSET,
    LBA_MIRROR_MBR_SECTOR, LBA_PARTITION_1_OFFSET, LBA_PARTITION_2_OFFSET, LBA_SECTOR_SIZE_BYTES,
    PANASONIC_HDD_PARTITION_MAGIC, SPANSION_BOOTLOADER_SIZE_BYTES,
};
use panadiga_core::crypto::entropy::analyze_frm_slices;
use panadiga_core::crypto::toc::analyze_toc;
use panadiga_core::hdd::inspect::inspect_hdd_slice;
use panadiga_core::hdd::open_image_buffer;
use panadiga_core::headers::hex_dump;
use panadiga_core::nvram::{
    NvramPolicyOptions, SliceIoLabels, calculate_hashes, clean_flash_nvram_slice_with_policy,
    patch_flash_nvram_slice_with_policy,
};
use panadiga_core::re::matshita_emu::parse_hex_payload;
use panadiga_core::ssot::is_valid_nor_flash_size;
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::forensics_re::parse_u32_arg;
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
    if !is_valid_nor_flash_size(data.len() as u64) {
        return Ok(json!({
            "error": format!(
                "Invalid flash dump size: {} bytes (expected standard DIGA NOR 2/4/8/16 MiB)",
                data.len(),
            )
        }));
    }

    let action = optional_non_empty_str(args, "action").unwrap_or("clean");
    let patch_hex = optional_non_empty_str(args, "patch_hex");
    let model_id = optional_non_empty_str(args, "model_id");
    let lab_override = super::hardware::parse_lab_override_from_args(args)?;
    let out_label = output_raw.unwrap_or("<analysis-only>");
    let policy = NvramPolicyOptions {
        model_id,
        lab_override: lab_override.as_ref(),
    };
    let labels = SliceIoLabels::new(input_raw, out_label, "<memory-backup>");
    let cleaned = if action.eq_ignore_ascii_case("patch") || patch_hex.is_some() {
        let hex_str = patch_hex
            .ok_or_else(|| anyhow::anyhow!("Missing 'patch_hex' for NVRAM patch action"))?;
        let patch_bytes = parse_hex_payload(hex_str)?;
        let rel_off = usize::try_from(parse_u32_arg(args, "nvram_offset", 0)?)?;
        patch_flash_nvram_slice_with_policy(&data, rel_off, &patch_bytes, &policy, labels)?
    } else if action.eq_ignore_ascii_case("clean") {
        clean_flash_nvram_slice_with_policy(&data, &policy, labels)?
    } else {
        anyhow::bail!(
            "Unsupported action '{action}' for panadiga_clean_nvram (expected 'clean' or 'patch')"
        );
    };

    let nvram_start = cleaned.report.nvram_offset_start;
    let nvram_end = cleaned.report.nvram_offset_end;
    let (_, ipl_sha256) = calculate_hashes(&data[..nvram_start]);
    let (_, nvram_orig_sha256) = calculate_hashes(&data[nvram_start..nvram_end]);

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
        "action": if patch_hex.is_some() || action.eq_ignore_ascii_case("patch") { "patch" } else { "clean" },
        "ipl_bytes_checked": SPANSION_BOOTLOADER_SIZE_BYTES,
        "ipl_sha256": ipl_sha256,
        "nvram_original_sha256": nvram_orig_sha256,
        "clean_sha256": cleaned.report.clean_sha256,
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
