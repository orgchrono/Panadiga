#![deny(unsafe_code)]

//! Dynamic end-to-end decryption, anchor discovery, and automated bench runtime orchestrator.
//!
//! Automatically handles Panasonic `.FRM` containers (`Dvd48B`, `Dvd64B`, `Sd48B`, `Sd64B`,
//! `PANASONIC\0\0\0` 48B header + UniPhier `MainList`), standalone `EXTRHEAD` containers,
//! raw NOR dumps (`BOOT.BIN`, `Loader ver0.61`, embedded `EXTRHEAD`), and single firmware stages,
//! pairing discovered stage anchors with region-interpolated TV/electrical runtime parameters.

mod scan;

use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{
    AM34_BOOT_BIN_BASE, AM34_FLASH_BASE, AM34_MAIN_BASE_VA, PANA_FEISTEL_ROUND_KEY_32B,
    PANA_FEISTEL_SBOX_256,
};
use crate::crypto::codec::read_u32_le;
use crate::crypto::{
    DecryptedFrmTocEntry, ExtrheadCryptMode, FrmKeyring, adler32_panasonic_strided,
    decrypt_frm_toc_with_keyring, detect_and_unwrap_aes_frm, extract_drv_module,
    feistel_decrypt_buffer, parse_main_list, parse_sd_frm_toc_with_layout, try_decode_stb_slot,
    unpack_extrhead_with_key,
};
use crate::nvram::{calculate_hashes, resolve_input_path};
use crate::re::bgo_unbrick::PROG061_BASE_VA;
use crate::re::mn10300_traps::{find_extrhead_containers, identify_firmware_revision};
use crate::re::region_compat::{MmioWriteOp, TvStandardKind, build_tv_calibration_mmio_plan};
use crate::ssot::registry::{InterpolatedRegionVariant, interpolate_region_variant};
use crate::ssot::{
    HardwareProfileRegistry, ResolvedModelProfile, fingerprint_nor_or_firmware,
    is_valid_nor_flash_size,
};

pub use scan::BenchRuntimeStageDescriptor;
use scan::{build_gdb_rsp_packets, build_panamon1_bench_script, scan_stage_anchors};

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

fn looks_like_valid_main_list(raw: &[u8]) -> bool {
    if raw.len() < 20 {
        return false;
    }
    let chk = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let lsz = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]) as usize;
    if lsz < 20 || lsz > raw.len() || !(lsz - 20).is_multiple_of(8) {
        return false;
    }
    let hdr_tail = &raw[4..lsz];
    let byte_sum = hdr_tail
        .iter()
        .fold(0u32, |acc, &b| acc.wrapping_add(u32::from(b)));
    adler32_panasonic_strided(hdr_tail, 1) == chk || byte_sum == chk
}

fn infer_prog_base_va(data: &[u8]) -> u32 {
    // Check early MOV imm32 instructions (`0xFC 0xDC..0xDF <u32>`) for load VA hints.
    let probe_end = data.len().min(0x400);
    for idx in 0..probe_end.saturating_sub(6) {
        if data[idx] == 0xFC && (0xDC..=0xDF).contains(&data[idx + 1]) {
            let imm = read_u32_le(data, idx + 2);
            if (0x5080_0000..0x5084_0000).contains(&imm) {
                return 0x5080_0000;
            }
            if (0x5003_0400..0x5006_0000).contains(&imm) {
                return 0x5003_0400;
            }
        }
    }
    PROG061_BASE_VA
}

fn try_unpack_stb_zlib(stage_buf: &[u8]) -> Option<UnpackedSlotBytes> {
    let decoded = try_decode_stb_slot(stage_buf)?;
    Some(UnpackedSlotBytes {
        bytes: decoded.bytes,
        source_kind: if decoded.is_zlib {
            "FRM_FEISTEL+STB_ZLIB".to_string()
        } else {
            "FRM_FEISTEL+STB_RAW".to_string()
        },
        build_name: decoded.model,
        algo: decoded.is_zlib.then(|| "zlib".to_string()),
        dst_addr: Some(0x8000_0400),
    })
}

fn try_extract_embedded_lzma(raw: &[u8]) -> Option<(usize, Vec<u8>)> {
    for idx in 0..raw.len().saturating_sub(32) {
        if matches!(raw[idx], 0x5D | 0x5C | 0x6D)
            && raw[idx + 1] == 0x00
            && raw[idx + 2] == 0x00
            && (raw[idx + 3] <= 0x80 || raw[idx + 3] == 0xFF)
        {
            let mut cursor = Cursor::new(&raw[idx..]);
            let mut out = Vec::new();
            if lzma_rs::lzma_decompress(&mut cursor, &mut out).is_ok() && out.len() >= 4096 {
                return Some((idx, out));
            }
        }
    }
    None
}

fn infer_stage_kind_and_base(
    stage_name: &str,
    data: &[u8],
    resolved: Option<&ResolvedModelProfile>,
    fallback_dst: Option<u32>,
) -> (&'static str, u32) {
    let up = stage_name.trim().to_ascii_uppercase();
    let train = resolved.and_then(|r| r.firmware_train.as_ref());
    if up.contains("PROG") || up.contains("DIGADIAG") {
        let inferred = infer_prog_base_va(data);
        let base = if inferred != PROG061_BASE_VA {
            inferred
        } else {
            fallback_dst
                .filter(|&a| a != 0 && a != 0x9040_0000)
                .or_else(|| {
                    train
                        .and_then(|t| t.prog_load_va)
                        .filter(|&a| a != 0 && a != 0x9040_0000)
                })
                .unwrap_or(inferred)
        };
        return ("PROG", base);
    }
    if up.starts_with("STB") {
        return ("STB", fallback_dst.unwrap_or(0x8000_0400));
    }
    if data.starts_with(b"\n root=/dev/fma") {
        return ("MAIN_LINUX_NAND", 0x9010_0000);
    }
    if up.contains("DRV") {
        let base = fallback_dst
            .filter(|&a| a != 0 && a != 0x9020_2000)
            .or_else(|| {
                train
                    .and_then(|t| t.drv_load_va)
                    .filter(|&a| a != 0 && a != 0x9020_2000)
            })
            .unwrap_or(0x4000_0000);
        return ("DRV", base);
    }
    if up.contains("MAIN") {
        let base = fallback_dst
            .filter(|&a| a != 0)
            .or_else(|| train.and_then(|t| t.main_load_va))
            .unwrap_or(AM34_MAIN_BASE_VA);
        return ("MAIN", base);
    }
    if up.contains("BOOT") || up.contains("LOADER") || up.contains("IPL") {
        return ("IPL", AM34_BOOT_BIN_BASE);
    }
    if is_valid_nor_flash_size(data.len() as u64) {
        let non_mn103 = resolved
            .and_then(|r| r.soc_core.as_ref())
            .is_some_and(|c| !c.isa.to_ascii_uppercase().contains("MN103"));
        return (
            "NOR_FULL",
            if non_mn103 {
                0x0000_0000
            } else {
                AM34_FLASH_BASE
            },
        );
    }
    let rev = identify_firmware_revision(data);
    match rev.stage.as_str() {
        "DRV" => (
            "DRV",
            train
                .and_then(|t| t.drv_load_va)
                .filter(|&a| a != 0 && a != 0x9020_2000)
                .unwrap_or(0x4000_0000),
        ),
        "BOOT_OR_PROG061" => {
            if data.len() == 0x4_0000 && !data.windows(16).any(|w| w.starts_with(b"Loader ver")) {
                ("PROG", PROG061_BASE_VA)
            } else {
                ("IPL", AM34_BOOT_BIN_BASE)
            }
        }
        _ if data.len() > 0x40_0000 => ("MAIN", AM34_MAIN_BASE_VA),
        _ => ("DRV", 0x4000_0000),
    }
}

fn sanitize_slug(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "stage".to_string()
    } else {
        trimmed.to_string()
    }
}

fn write_stage_if_dir(
    out_dir: Option<&Path>,
    idx: usize,
    stage_key: &str,
    bytes: &[u8],
) -> Result<Option<String>> {
    let Some(dir) = out_dir else {
        return Ok(None);
    };
    fs::create_dir_all(dir)
        .with_context(|| format!("Creating bench runtime output dir {}", dir.display()))?;
    let file_path = dir.join(format!("{idx:02}_{}.bin", sanitize_slug(stage_key)));
    fs::write(&file_path, bytes)
        .with_context(|| format!("Writing unpacked stage {}", file_path.display()))?;
    Ok(Some(file_path.to_string_lossy().into_owned()))
}

struct UnpackedSlotBytes {
    bytes: Vec<u8>,
    source_kind: String,
    build_name: Option<String>,
    algo: Option<String>,
    dst_addr: Option<u32>,
}

fn algo_name(algo: u16) -> String {
    match algo {
        0 => "zlib".to_string(),
        1 => "gzip".to_string(),
        2 => "pana_lzss".to_string(),
        3 => "lzma".to_string(),
        other => format!("algo_{other}"),
    }
}

fn try_unpack_main_list_slot(
    buf: &[u8],
    source_kind: &str,
    rk: Option<&[u8; 32]>,
) -> Option<UnpackedSlotBytes> {
    if !looks_like_valid_main_list(buf) {
        return None;
    }
    let ml = parse_main_list(buf, rk).ok()?;
    let first_hdr = ml.segments.first().map(|s| &s.header);
    Some(UnpackedSlotBytes {
        bytes: ml.data,
        source_kind: source_kind.to_string(),
        build_name: first_hdr.map(|h| h.build_name.clone()),
        algo: first_hdr.map(|h| algo_name(h.algo)),
        dst_addr: first_hdr.map(|h| h.dst_addr),
    })
}

fn unpack_frm_slot_bytes(
    raw_slot: &[u8],
    orig_size: usize,
    module_name: &str,
    outer_feistel: bool,
    rk: Option<&[u8; 32]>,
) -> UnpackedSlotBytes {
    let up = module_name.trim().to_ascii_uppercase();
    let exact_slot = &raw_slot[..orig_size.min(raw_slot.len())];
    if let Some(unpacked) = try_unpack_main_list_slot(exact_slot, "FRM_MAIN_LIST", rk) {
        return unpacked;
    }

    let mut stage_buf = raw_slot.to_vec();
    if outer_feistel && let Some(key) = rk {
        feistel_decrypt_buffer(&mut stage_buf, key, &PANA_FEISTEL_SBOX_256);
    }
    stage_buf.truncate(orig_size);

    if let Some(unpacked) = try_unpack_main_list_slot(&stage_buf, "FRM_FEISTEL_MAIN_LIST", rk) {
        return unpacked;
    }

    if !up.starts_with("PROG") || stage_buf.starts_with(b"EXTRHEAD") {
        if let Ok(u) = unpack_extrhead_with_key(&stage_buf, ExtrheadCryptMode::Auto, rk) {
            return UnpackedSlotBytes {
                bytes: u.decompressed,
                source_kind: if outer_feistel {
                    "FRM_FEISTEL+EXTRHEAD".to_string()
                } else {
                    "SD_EXTRHEAD".to_string()
                },
                build_name: Some(u.header.build_name),
                algo: Some(algo_name(u.header.algo)),
                dst_addr: Some(u.header.dst_addr),
            };
        }
        if up.starts_with("DRV")
            && let Ok(drv) = extract_drv_module(&stage_buf, rk)
        {
            return UnpackedSlotBytes {
                bytes: drv.payload,
                source_kind: "FRM_DRV_EDGE_DECRYPT".to_string(),
                build_name: drv.header.map(|h| h.model),
                algo: drv.extrhead.map(|u| algo_name(u.header.algo)),
                dst_addr: None,
            };
        }
        if let Some(stb_unpacked) = try_unpack_stb_zlib(&stage_buf) {
            return stb_unpacked;
        }
    }

    UnpackedSlotBytes {
        bytes: stage_buf,
        source_kind: if outer_feistel {
            "FRM_FEISTEL_RAW".to_string()
        } else {
            "SD_RAW".to_string()
        },
        build_name: None,
        algo: None,
        dst_addr: None,
    }
}

fn resolve_region_and_mmio(
    resolved: Option<&ResolvedModelProfile>,
    region_override: Option<&str>,
) -> (Option<InterpolatedRegionVariant>, Vec<MmioWriteOp>) {
    let mut region = region_override.map(|tok| {
        let m_id = resolved.map_or("GENERIC", |r| r.manifest.model_id.as_str());
        let ser = resolved.map_or("", |r| r.manifest.series.as_str());
        let hw = resolved.and_then(|r| r.manifest.hw_code.as_deref());
        interpolate_region_variant(tok, m_id, ser, hw)
    });
    if region.is_none()
        && let Some(r) = resolved
    {
        region = r
            .active_region
            .clone()
            .or_else(|| r.regional_variants.first().cloned());
    }
    let tv_kind = region
        .as_ref()
        .map_or(TvStandardKind::PalM, |v| match v.tv_standard.as_str() {
            "PAL-M" => TvStandardKind::PalM,
            s if s.contains("SECAM") => TvStandardKind::Secam,
            s if s.starts_with("PAL") => TvStandardKind::Pal,
            other => TvStandardKind::from_str(other).unwrap_or(TvStandardKind::Ntsc),
        });
    (region, build_tv_calibration_mmio_plan(tv_kind))
}

struct FrmStageScanCtx<'a> {
    outer_feistel: bool,
    rk: Option<&'a [u8; 32]>,
    key_name: Option<&'a str>,
    resolved: Option<&'a ResolvedModelProfile>,
    out_dir: Option<&'a Path>,
}

fn push_frm_stages(
    raw: &[u8],
    entries: &[DecryptedFrmTocEntry],
    ctx: &FrmStageScanCtx<'_>,
    stages: &mut Vec<BenchRuntimeStageDescriptor>,
) -> Result<()> {
    for (idx, entry) in entries.iter().enumerate() {
        let start = entry.module_offset as usize;
        let orig_size = entry.module_size as usize;
        let end = start.saturating_add(orig_size);
        if end > raw.len() || orig_size == 0 {
            continue;
        }
        let slice_end = if ctx.outer_feistel {
            ((end + 7) & !7).min(raw.len())
        } else {
            end
        };
        let unpacked = unpack_frm_slot_bytes(
            &raw[start..slice_end],
            orig_size,
            &entry.module_name,
            ctx.outer_feistel,
            ctx.rk,
        );
        let stage_key = entry.module_name.trim().to_string();
        let (kind, base_va) =
            infer_stage_kind_and_base(&stage_key, &unpacked.bytes, ctx.resolved, unpacked.dst_addr);
        let (_, sha) = calculate_hashes(&unpacked.bytes);
        let written = write_stage_if_dir(ctx.out_dir, idx, &stage_key, &unpacked.bytes)?;
        stages.push(scan_stage_anchors(
            stage_key,
            kind.to_string(),
            base_va,
            &unpacked.bytes,
            sha,
            unpacked.source_kind,
            ctx.key_name.map(str::to_string),
            unpacked.build_name,
            unpacked.algo,
            written,
        ));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::AM34_SC0_BASE;
    use crate::crypto::{FrmContainerMode, FrmPackModule, pack_extrhead, pack_frm};
    use crate::re::bgo_unbrick::{
        OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ,
        ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ,
    };

    #[test]
    fn bench_runtime_decrypts_frm_discovers_anchors_and_interpolates_region() {
        let dir =
            std::env::temp_dir().join(format!("panadiga_bench_rt_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut prog = vec![0xFFu8; 0x4_0000];
        prog[..0x1000].fill(0xCB);
        prog[OFF_BGO_INIT_CALL..OFF_BGO_INIT_CALL + ORIG_A2BC_BGO_CALL.len()]
            .copy_from_slice(&ORIG_A2BC_BGO_CALL);
        prog[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + ORIG_A2CD_HOOK.len()]
            .copy_from_slice(&ORIG_A2CD_HOOK);
        prog[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + ORIG_SUM_BEQ.len()].copy_from_slice(&ORIG_SUM_BEQ);
        prog[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + ORIG_DEVINFO_BEQ.len()]
            .copy_from_slice(&ORIG_DEVINFO_BEQ);
        // Place BGO geometry marker (0x84130000 / 0x84400000) & SC0 MMIO ref
        prog[0x337A4 + 32..0x337A4 + 36].copy_from_slice(&0x10000u32.to_le_bytes());
        prog[0x337A4 + 44..0x337A4 + 48].copy_from_slice(&0x8413_0000u32.to_le_bytes());
        prog[0x337A4 + 48..0x337A4 + 52].copy_from_slice(&0x8440_0000u32.to_le_bytes());
        prog[0x337A4 + 52..0x337A4 + 56].copy_from_slice(&0x10000u32.to_le_bytes());
        prog[0x1200..0x1204].copy_from_slice(&AM34_SC0_BASE.to_le_bytes());

        let drv_payload = b"DRV SLOGON\0\0\0\0\0\0test".to_vec();
        let drv_ext = pack_extrhead(
            &drv_payload,
            "DRV ",
            0x4000_0000,
            "DRV_S211",
            ExtrheadCryptMode::Plain,
            16,
            1,
        )
        .unwrap();

        let (frm, _) = pack_frm(
            &[
                FrmPackModule::new("DRV S211", drv_ext),
                FrmPackModule::new("PROG061 ", prog),
            ],
            FrmContainerMode::Dvd,
            None,
        )
        .unwrap();
        let frm_path = dir.join("TEST_PANA.FRM");
        fs::write(&frm_path, &frm).unwrap();

        let out_dir = dir.join("unpacked");
        let plan =
            build_dynamic_bench_runtime_plan(&frm_path, Some("DMR-EH55LB-S"), None, Some(&out_dir))
                .expect("bench runtime plan succeeds");

        assert_eq!(plan.detected_container_format, "FRM_FEISTEL");
        assert_eq!(plan.detected_feistel_key.as_deref(), Some("old_df72"));
        assert_eq!(plan.resolved_model_id.as_deref(), Some("DMR-EH55"));
        let reg = plan.interpolated_region.expect("interpolated LB region");
        assert_eq!(reg.tv_standard, "PAL-M");
        assert_eq!(reg.aiof_vmodesel_hex, "0x0002");
        assert_eq!(plan.stages.len(), 2);
        let prog_stage = plan.stages.iter().find(|s| s.stage_kind == "PROG").unwrap();
        assert_eq!(
            prog_stage.bgo_partition_base_hex.as_deref(),
            Some("0x84130000")
        );
        assert_eq!(
            prog_stage.panamon1_slack_va_hex.as_deref(),
            Some("0x50439494")
        );
        assert!(out_dir.join("bench_runtime_manifest.json").is_file());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stb_unaligned_feistel_tail_and_cached_va_normalization() {
        use crate::crypto::feistel_encrypt_buffer;
        use flate2::{Compression, write::ZlibEncoder};
        use std::io::Write;

        let payload = b"    c    ST20_OS20_MOCK_FIRMWARE_PAYLOAD_WITH_TAIL_ALIGNMENT_TEST";
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(payload).unwrap();
        let zbytes = enc.finish().unwrap();

        let mut hdr = vec![0xFFu8; 256];
        let hdr_str = format!(
            "CRC:1234\0size:{}\0Type:M4\0AppVer:1.03\0Model: M4EN\0zlib:{}\0",
            zbytes.len(),
            payload.len()
        );
        hdr[..hdr_str.len()].copy_from_slice(hdr_str.as_bytes());
        let mut slot = [hdr, zbytes].concat();
        while slot.len() % 8 != 4 {
            slot.push(0x41);
        }
        let orig_size = slot.len();
        let pad_len = (orig_size + 7) & !7;
        slot.resize(pad_len, 0xFF);
        feistel_encrypt_buffer(
            &mut slot,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );

        let unpacked = unpack_frm_slot_bytes(
            &slot,
            orig_size,
            "STB 103",
            true,
            Some(&PANA_FEISTEL_ROUND_KEY_32B),
        );
        assert_eq!(unpacked.source_kind, "FRM_FEISTEL+STB_ZLIB");
        assert_eq!(unpacked.algo.as_deref(), Some("zlib"));
        assert!(unpacked.bytes.starts_with(payload));

        assert_eq!(
            infer_stage_kind_and_base("DIGADIAG", &[0u8; 16], None, Some(0x9040_0000)),
            ("PROG", PROG061_BASE_VA)
        );
        assert_eq!(
            infer_stage_kind_and_base("DRV S514", &[0u8; 16], None, Some(0x9020_2000)),
            ("DRV", 0x4000_0000)
        );
    }
}
