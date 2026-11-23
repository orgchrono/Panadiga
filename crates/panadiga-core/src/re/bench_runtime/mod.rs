#![deny(unsafe_code)]

//! Dynamic end-to-end decryption, anchor discovery, and automated bench runtime orchestrator.
//!
//! Automatically handles Panasonic `.FRM` containers (`Dvd48B`, `Dvd64B`, `Sd48B`, `Sd64B`,
//! `PANASONIC\0\0\0` 48B header + UniPhier `MainList`), standalone `EXTRHEAD` containers,
//! raw NOR dumps (`BOOT.BIN`, `Loader ver0.61`, embedded `EXTRHEAD`), and single firmware stages,
//! pairing discovered stage anchors with region-interpolated TV/electrical runtime parameters.

mod scan;
mod slot_unpack;
mod stage_detect;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::PANA_FEISTEL_ROUND_KEY_32B;
use crate::crypto::{
    ExtrheadCryptMode, FrmKeyring, decrypt_frm_toc_with_keyring, detect_and_unwrap_aes_frm,
    parse_sd_frm_toc_with_layout, unpack_extrhead_with_key,
};
use crate::nvram::{calculate_hashes, resolve_input_path};
use crate::re::mn10300_traps::find_extrhead_containers;
use crate::re::region_compat::MmioWriteOp;
use crate::ssot::registry::InterpolatedRegionVariant;
use crate::ssot::{HardwareProfileRegistry, fingerprint_nor_or_firmware};

pub use scan::BenchRuntimeStageDescriptor;
use scan::{build_gdb_rsp_packets, build_panamon1_bench_script, scan_stage_anchors};
use slot_unpack::{FrmStageScanCtx, algo_name, push_frm_stages, resolve_region_and_mmio};
use stage_detect::{infer_stage_kind_and_base, try_extract_embedded_lzma, write_stage_if_dir};

/// Complete dynamic decryption and automated bench runtime execution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchRuntimePlan {
    pub input_path: String,
    pub input_sha256: String,
    pub detected_container_format: String,
    pub detected_feistel_key: Option<String>,
    pub toc_layout: Option<String>,
    pub toc_entry_count: usize,
    pub resolved_model_id: Option<String>,
    pub interpolated_region: Option<InterpolatedRegionVariant>,
    pub tv_mmio_plan: Vec<MmioWriteOp>,
    pub stages: Vec<BenchRuntimeStageDescriptor>,
    pub panamon1_bench_script: Vec<String>,
    pub gdb_rsp_packets: Vec<String>,
    pub pico_p7501_commands: Vec<String>,
    pub pyghidra_command: String,
}

/// Automatically decrypts any Panasonic container/dump and builds a dynamic bench runtime plan.
pub fn build_dynamic_bench_runtime_plan(
    input_path: &Path,
    model_query: Option<&str>,
    region_override: Option<&str>,
    out_dir: Option<&Path>,
) -> Result<BenchRuntimePlan> {
    build_dynamic_bench_runtime_plan_with_keyring(
        input_path,
        model_query,
        region_override,
        out_dir,
        &FrmKeyring::builtin(),
    )
}

/// Automatically decrypts any Panasonic container/dump (including `PANAEUSB.FRM` AES pairs & `keys.ukf`).
pub fn build_dynamic_bench_runtime_plan_with_keyring(
    input_path: &Path,
    model_query: Option<&str>,
    region_override: Option<&str>,
    out_dir: Option<&Path>,
    keyring: &FrmKeyring,
) -> Result<BenchRuntimePlan> {
    let resolved_path = resolve_input_path(input_path)?;
    let raw =
        fs::read(&resolved_path).with_context(|| format!("Reading {}", resolved_path.display()))?;
    if raw.is_empty() {
        bail!("Input firmware file {} is empty", resolved_path.display());
    }
    let (_, input_sha256) = calculate_hashes(&raw);
    let reg = HardwareProfileRegistry::builtin();

    let aes_unwrapped = detect_and_unwrap_aes_frm(&raw, keyring)?;
    let (kname_opt, rk_buf, toc_layout, enc_entries) = if aes_unwrapped.is_some() {
        (
            None,
            [0u8; 32],
            crate::crypto::TocLayout::Dvd48B,
            Vec::new(),
        )
    } else {
        decrypt_frm_toc_with_keyring(&raw, keyring)
    };
    let enc_valid = kname_opt.is_some()
        && !enc_entries.is_empty()
        && enc_entries.iter().all(|e| {
            (e.module_offset as usize)
                .checked_add(e.module_size as usize)
                .is_some_and(|end| end <= raw.len())
        });

    let fp_model = if model_query.is_none() {
        fingerprint_nor_or_firmware(&raw).detected_model
    } else {
        None
    };
    let effective_model = model_query.map(str::to_string).or(fp_model);
    let resolved_profile = effective_model
        .as_deref()
        .and_then(|q| reg.resolve_model(q).ok());

    let mut stages = Vec::new();
    let (container_format, detected_key, toc_label, toc_count) =
        if let Some(unwrapped) = aes_unwrapped {
            let mut total_entries = 0usize;
            let mut first_layout = None;
            for (_, sub_bytes) in &unwrapped.sub_containers {
                let (sub_k, sub_rk, sub_layout, sub_entries) =
                    decrypt_frm_toc_with_keyring(sub_bytes, keyring);
                first_layout.get_or_insert_with(|| format!("{sub_layout:?}"));
                total_entries += sub_entries.len();
                let ctx = FrmStageScanCtx {
                    outer_feistel: sub_k.is_some(),
                    rk: Some(&sub_rk),
                    key_name: sub_k.as_deref(),
                    resolved: resolved_profile.as_ref(),
                    out_dir,
                };
                push_frm_stages(sub_bytes, &sub_entries, &ctx, &mut stages)?;
            }
            (
                "FRM_BLURAY_AES".to_string(),
                Some(unwrapped.key_pair.name),
                first_layout,
                total_entries,
            )
        } else if enc_valid {
            let layout_str = format!("{toc_layout:?}");
            let ctx = FrmStageScanCtx {
                outer_feistel: true,
                rk: Some(&rk_buf),
                key_name: kname_opt.as_deref(),
                resolved: resolved_profile.as_ref(),
                out_dir,
            };
            push_frm_stages(&raw, &enc_entries, &ctx, &mut stages)?;
            (
                "FRM_FEISTEL".to_string(),
                kname_opt,
                Some(layout_str),
                enc_entries.len(),
            )
        } else {
            let (sd_layout, sd_entries) = parse_sd_frm_toc_with_layout(&raw);
            let sd_valid = !sd_entries.is_empty()
                && sd_entries.iter().all(|e| {
                    (e.module_offset as usize)
                        .checked_add(e.module_size as usize)
                        .is_some_and(|end| end <= raw.len())
                });
            if sd_valid {
                let layout_str = sd_layout.map(|l| format!("{l:?}"));
                let ctx = FrmStageScanCtx {
                    outer_feistel: false,
                    rk: Some(&PANA_FEISTEL_ROUND_KEY_32B),
                    key_name: None,
                    resolved: resolved_profile.as_ref(),
                    out_dir,
                };
                push_frm_stages(&raw, &sd_entries, &ctx, &mut stages)?;
                (
                    "FRM_SD_PLAIN".to_string(),
                    None,
                    layout_str,
                    sd_entries.len(),
                )
            } else if let Ok(ext) = unpack_extrhead_with_key(&raw, ExtrheadCryptMode::Auto, None) {
                let key_lbl = if raw.starts_with(b"EXTRHEAD") {
                    None
                } else {
                    Some("auto_extrhead".to_string())
                };
                let stage_key = resolved_path.file_name().map_or_else(
                    || ext.header.tag.clone(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let (kind, base_va) = infer_stage_kind_and_base(
                    &stage_key,
                    &ext.decompressed,
                    resolved_profile.as_ref(),
                    Some(ext.header.dst_addr),
                );
                let (_, sha) = calculate_hashes(&ext.decompressed);
                let written = write_stage_if_dir(out_dir, 0, &stage_key, &ext.decompressed)?;
                stages.push(scan_stage_anchors(
                    stage_key,
                    kind.to_string(),
                    base_va,
                    &ext.decompressed,
                    sha,
                    "EXTRHEAD_STANDALONE".to_string(),
                    key_lbl.clone(),
                    Some(ext.header.build_name),
                    Some(algo_name(ext.header.algo)),
                    written,
                ));
                ("EXTRHEAD_STANDALONE".to_string(), key_lbl, None, 1)
            } else {
                let file_key = resolved_path.file_name().map_or_else(
                    || "raw_stage".to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let (kind, base_va) =
                    infer_stage_kind_and_base(&file_key, &raw, resolved_profile.as_ref(), None);
                let written = write_stage_if_dir(out_dir, 0, &file_key, &raw)?;
                stages.push(scan_stage_anchors(
                    file_key,
                    kind.to_string(),
                    base_va,
                    &raw,
                    input_sha256.clone(),
                    "RAW_BINARY".to_string(),
                    None,
                    None,
                    None,
                    written,
                ));
                let extr_hits = find_extrhead_containers(&raw, base_va);
                if extr_hits.is_empty()
                    && kind == "NOR_FULL"
                    && let Some((off, lzma_dec)) = try_extract_embedded_lzma(&raw)
                {
                    let sub_key = format!("embedded_lzma_0x{off:06x}");
                    let (_, sha) = calculate_hashes(&lzma_dec);
                    let sub_written = write_stage_if_dir(out_dir, 1, &sub_key, &lzma_dec)?;
                    stages.push(scan_stage_anchors(
                        sub_key,
                        "VXWORKS_BAL".to_string(),
                        0x4000_0000,
                        &lzma_dec,
                        sha,
                        "EMBEDDED_LZMA".to_string(),
                        None,
                        None,
                        Some("lzma".to_string()),
                        sub_written,
                    ));
                }
                for (idx, m) in extr_hits.into_iter().enumerate() {
                    if let Ok(ext) =
                        unpack_extrhead_with_key(&raw[m.offset..], ExtrheadCryptMode::Plain, None)
                    {
                        let sub_key = format!("embedded_{}_{idx}", ext.header.tag.trim());
                        let (sub_kind, sub_base) = infer_stage_kind_and_base(
                            &sub_key,
                            &ext.decompressed,
                            resolved_profile.as_ref(),
                            Some(ext.header.dst_addr),
                        );
                        let (_, sha) = calculate_hashes(&ext.decompressed);
                        let sub_written =
                            write_stage_if_dir(out_dir, idx + 1, &sub_key, &ext.decompressed)?;
                        stages.push(scan_stage_anchors(
                            sub_key,
                            sub_kind.to_string(),
                            sub_base,
                            &ext.decompressed,
                            sha,
                            "EMBEDDED_EXTRHEAD".to_string(),
                            None,
                            Some(ext.header.build_name),
                            Some(algo_name(ext.header.algo)),
                            sub_written,
                        ));
                    }
                }
                (
                    if kind == "NOR_FULL" {
                        "RAW_NOR_DUMP".to_string()
                    } else {
                        "RAW_STAGE".to_string()
                    },
                    None,
                    None,
                    stages.len(),
                )
            }
        };

    let (interpolated_region, tv_mmio_plan) =
        resolve_region_and_mmio(resolved_profile.as_ref(), region_override);
    let bgo_base = stages
        .iter()
        .find_map(|s| s.bgo_partition_base_hex.as_deref())
        .unwrap_or("0x84130000");
    let model_id_str = resolved_profile
        .as_ref()
        .map_or_else(|| "DMR-EH55".to_string(), |r| r.manifest.model_id.clone());

    let mut pico_p7501_commands = vec![
        "PING".to_string(),
        "P7501_CHECK".to_string(),
        format!("TARGET_PROFILE {model_id_str}"),
        "DIAGWATCH".to_string(),
    ];
    if let Some(reg_v) = &interpolated_region {
        pico_p7501_commands.push(format!(
            "# REGION {} | TV={} | AIOF_VMODESEL={}",
            reg_v.region_code, reg_v.tv_standard, reg_v.aiof_vmodesel_hex
        ));
    }

    let mut pyghidra_cmd = format!(
        "python tools/ghidra/ghidra_zero_undef_pipeline.py all --frm \"{}\" --model {}",
        resolved_path.display(),
        model_id_str
    );
    if let Some(reg_v) = &interpolated_region {
        pyghidra_cmd.push_str(&format!(" --region {}", reg_v.region_code));
    }

    let plan = BenchRuntimePlan {
        input_path: resolved_path.to_string_lossy().into_owned(),
        input_sha256,
        detected_container_format: container_format,
        detected_feistel_key: detected_key,
        toc_layout: toc_label,
        toc_entry_count: toc_count,
        resolved_model_id: resolved_profile
            .as_ref()
            .map(|r| r.manifest.model_id.clone()),
        interpolated_region,
        panamon1_bench_script: build_panamon1_bench_script(bgo_base, &tv_mmio_plan),
        gdb_rsp_packets: build_gdb_rsp_packets(&tv_mmio_plan),
        pico_p7501_commands,
        pyghidra_command: pyghidra_cmd,
        tv_mmio_plan,
        stages,
    };

    if let Some(dir) = out_dir {
        let manifest_path = dir.join("bench_runtime_manifest.json");
        let pretty = serde_json::to_string_pretty(&plan)?;
        fs::write(&manifest_path, pretty)
            .with_context(|| format!("Writing {}", manifest_path.display()))?;
    }

    Ok(plan)
}
