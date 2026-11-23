#![deny(unsafe_code)]

//! Compact SSOT indexer for `profiles/traps/` and `profiles/forensic_atlas/` summaries.
//!
//! Enables sub-millisecond Zero-Error RAG retrieval over geometry traps, hex/MMIO catalogs,
//! bitwise masks, and container header/footer signatures without rescanning multi-MB text dumps.

use crate::guardrails::QueryGuard;
use crate::models::{Chunk, ChunkMetadata, SubsystemType};
use serde::Deserialize;
use std::{fs, path::Path};

use super::tokenizer::technical_tokenize;

#[derive(Deserialize)]
struct TrapsFile {
    #[serde(default)]
    traps: Vec<TrapItem>,
}

#[derive(Deserialize)]
struct TrapItem {
    id: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    wrong_assumption: String,
    #[serde(default)]
    verified_ground_truth: String,
    #[serde(default)]
    automated_guard: String,
}

#[derive(Deserialize)]
struct AtlasFile {
    #[serde(default)]
    headers_and_footers: Vec<AtlasHeaderFooter>,
    #[serde(default)]
    hex_constants: Vec<AtlasHexConst>,
    #[serde(default)]
    bitwise_patterns: Vec<AtlasBitwise>,
}

#[derive(Deserialize)]
struct AtlasHeaderFooter {
    artifact: String,
    kind: String,
    signature: String,
    notes: String,
}

#[derive(Deserialize)]
struct AtlasHexConst {
    hex: String,
    category: String,
    function_count: usize,
    #[serde(default)]
    sample_vas: Vec<String>,
}

#[derive(Deserialize)]
struct AtlasBitwise {
    pattern: String,
    occurrence_count: usize,
    #[serde(default)]
    sample_vas: Vec<String>,
}

fn push_train_patch_chunks(
    guard: &QueryGuard,
    train_id: &str,
    path_str: &str,
    raw: &str,
    out: &mut Vec<Chunk>,
) {
    let Ok(train) = serde_json::from_str::<panadiga_core::ssot::FirmwareTrainProfile>(raw) else {
        return;
    };
    if let Some(prog_rst) = &train.prog_sata_bridge_reset_spec {
        let text = format!(
            "[SSOT Train] {} PROG SATA bridge reset {}ms->{}ms @ 0x{:08X} callee=0x{:08X} ({})",
            train_id,
            prog_rst.factory_wait_ms,
            prog_rst.patched_wait_ms,
            prog_rst.site.va,
            prog_rst.sleep_callee_va,
            prog_rst.site.description
        );
        out.push(make_ssot_chunk(
            guard,
            format!("ssot_train::{train_id}::prog_reset"),
            "SSOT Firmware Train Specs",
            path_str,
            format!("{train_id} PROG SATA Reset Spec"),
            text,
            1,
        ));
    }
    if let Some(s2) = &train.stage2_boot_spec {
        let text = format!(
            "[SSOT Train] {} stage2_boot_spec hook=0x{:08X} payload=0x{:08X} norfrom=0x{:08X} memcpy=0x{:08X} hdd_read=0x{:08X} extrhead_uncomp=0x{:08X}",
            train_id,
            s2.base_va.wrapping_add(s2.hook_offset),
            s2.base_va.wrapping_add(s2.payload_offset),
            s2.norfrom_fvu_base_va,
            s2.symbols.memcpy_va,
            s2.symbols.hdd_read_va,
            s2.symbols.extrhead_uncomp_va,
        );
        out.push(make_ssot_chunk(
            guard,
            format!("ssot_train::{train_id}::stage2"),
            "SSOT Firmware Train Specs",
            path_str,
            format!("{train_id} Stage-2 Boot Spec"),
            text,
            2,
        ));
    }
    for (idx, b) in train.main_builds.iter().enumerate() {
        let rst = b.sata_bridge_reset_spec.as_ref().map_or_else(
            || "none".to_string(),
            |r| format!("0x{:08X}->sleep=0x{:08X}", r.site.va, r.sleep_callee_va),
        );
        let text = format!(
            "[SSOT MainBuild] {} train={} slot={} ver=0x{:04X} size={} tv={:?} chassis={:?} bgo={} detect={:?} flash={:?} reset={}",
            b.build_id,
            train_id,
            b.slot_name,
            b.frm_version_u16,
            b.unpacked_size,
            b.region_tv_standard,
            b.chassis_family,
            b.native_bgo_geometry_id,
            b.native_bgo_detect_va.map(|v| format!("0x{v:08X}")),
            b.native_bgo_flash_base_va.map(|v| format!("0x{v:08X}")),
            rst,
        );
        out.push(make_ssot_chunk(
            guard,
            format!("ssot_main::{train_id}::{}", b.build_id),
            "SSOT MAIN Build Specs",
            path_str,
            format!("{} ({})", b.slot_name, b.native_bgo_geometry_id),
            text,
            idx + 3,
        ));
    }
}

fn make_ssot_chunk(
    guard: &QueryGuard,
    id: String,
    doc_title: &str,
    path_str: &str,
    section_title: String,
    text: String,
    line_num: usize,
) -> Chunk {
    let tokens = technical_tokenize(&text, true);
    let extracted_entities = guard.process(&text).extracted_entities;
    Chunk {
        id,
        text,
        metadata: ChunkMetadata {
            doc_title: doc_title.to_string(),
            file_path: path_str.to_string(),
            section_title,
            hierarchy: vec!["Profiles".to_string(), "SSOT".to_string()],
            line_start: line_num,
            line_end: line_num,
            subsystem: SubsystemType::ReverseEngineering,
            extracted_entities,
        },
        tokens,
    }
}

/// Parses `profiles/traps/*.json` or `profiles/forensic_atlas/*.json` into compact high-signal RAG chunks.
pub(super) fn chunk_ssot_traps_or_atlas(guard: &QueryGuard, path: &Path) -> Vec<Chunk> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let path_str = path.to_string_lossy().to_string();

    if fname == "geometry_and_binary_traps.json" {
        let Ok(parsed) = serde_json::from_str::<TrapsFile>(&raw) else {
            return Vec::new();
        };
        return parsed
            .traps
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let text = format!(
                    "[SSOT Trap Catalog] {} category={} severity={} | WrongAssumption: {} | VerifiedGroundTruth: {} | Guard: {}",
                    t.id,
                    t.category,
                    t.severity,
                    t.wrong_assumption,
                    t.verified_ground_truth,
                    t.automated_guard
                );
                make_ssot_chunk(
                    guard,
                    format!("ssot_trap::{}", t.id),
                    "SSOT Geometry & Binary Traps",
                    &path_str,
                    t.id,
                    text,
                    i + 1,
                )
            })
            .collect();
    }

    if fname == "firmware_forensic_atlas.json" {
        let Ok(atlas) = serde_json::from_str::<AtlasFile>(&raw) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (idx, group) in atlas.headers_and_footers.chunks(8).enumerate() {
            let body = group
                .iter()
                .map(|h| {
                    format!(
                        "{} [{}] sig={} :: {}",
                        h.artifact, h.kind, h.signature, h.notes
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            out.push(make_ssot_chunk(
                guard,
                format!("ssot_atlas_hdr::{idx}"),
                "SSOT Forensic Atlas Headers",
                &path_str,
                format!("Headers & Footers #{idx}"),
                format!("[Forensic Atlas Headers & Footers]\n{body}"),
                idx + 1,
            ));
        }
        for (idx, group) in atlas.hex_constants.chunks(16).take(6).enumerate() {
            let body = group
                .iter()
                .map(|x| {
                    format!(
                        "{} class={} count={} VAs=[{}]",
                        x.hex,
                        x.category,
                        x.function_count,
                        x.sample_vas.join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            out.push(make_ssot_chunk(
                guard,
                format!("ssot_atlas_hex::{idx}"),
                "SSOT Forensic Atlas Hex Constants",
                &path_str,
                format!("Hex/MMIO Constants #{idx}"),
                format!("[Forensic Atlas Hex & MMIO Constants]\n{body}"),
                idx + 1,
            ));
        }
        for (idx, group) in atlas.bitwise_patterns.chunks(16).take(4).enumerate() {
            let body = group
                .iter()
                .map(|b| {
                    format!(
                        "`{}` count={} VAs=[{}]",
                        b.pattern,
                        b.occurrence_count,
                        b.sample_vas.join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            out.push(make_ssot_chunk(
                guard,
                format!("ssot_atlas_bw::{idx}"),
                "SSOT Forensic Atlas Bitwise Masks",
                &path_str,
                format!("Bitwise Masks #{idx}"),
                format!("[Forensic Atlas Bitwise Masks]\n{body}"),
                idx + 1,
            ));
        }
        return out;
    }

    if path_str.contains("firmware_trains") && fname.ends_with(".json") {
        let mut out = Vec::new();
        let train_id = fname.strip_suffix(".json").unwrap_or(fname);
        push_train_patch_chunks(guard, train_id, &path_str, &raw, &mut out);
        return out;
    }

    Vec::new()
}
