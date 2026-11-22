#![deny(unsafe_code)]

//! Physical NOR intake discovery & draft SSOT emission for `panadiga profiles discover-nor`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use panadiga_core::crypto::parse_extrhead_header;
use panadiga_core::re::audit_bgo_nor_auto;
use panadiga_core::re::inspect_nor_dump_integrity;
use panadiga_core::re::mn10300_traps::find_extrhead_containers;
use panadiga_core::re::nor_dumper::analyze_spansion_nor_sectors;
use panadiga_core::ssot::{
    ClaimEvidence, EvidenceSource, EvidenceTier, ModelClaims, ModelProfileManifest,
    NorLayoutProfile, NorRegion, NorRegionKind, NorRegionPolicy, RegionSignature,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(in super::super) struct DiscoveredExtrheadEntry {
    pub offset: usize,
    pub offset_hex: String,
    pub tag: Option<String>,
    pub dst_addr_hex: Option<String>,
    pub dst_len: Option<u32>,
    pub src_len: Option<u32>,
    pub build_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(in super::super) struct NorDiscoveryReport {
    pub input_path: String,
    pub model_id: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub loader_banner: Option<String>,
    pub loader_version: Option<String>,
    pub loader_date: Option<String>,
    pub integrity_ok: bool,
    pub integrity_warnings: Vec<String>,
    pub extrhead_entries: Vec<DiscoveredExtrheadEntry>,
    pub bgo_sector_count_64k: usize,
    pub bgo_start_hex: Option<String>,
    pub bgo_end_hex: Option<String>,
    pub bgo_ifodev2_model: Option<String>,
    pub bgo_healthy: Option<bool>,
    pub eemul_1284_offsets_hex: Vec<String>,
    pub eemul_f1_offsets_hex: Vec<String>,
    pub spansion_8mb_blank_sectors: Option<usize>,
    pub regions: Vec<NorRegion>,
    pub emitted_files: Vec<String>,
}

fn slugify_id(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_us = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_us = false;
        } else if !prev_us && !out.is_empty() {
            out.push('_');
            prev_us = true;
        }
    }
    let trimmed = out.trim_end_matches('_');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Scans a raw NOR image using `panadiga_core` analyzers and optionally writes draft JSONs.
pub(in super::super) fn discover_nor_dump(
    input: &Path,
    model_id: &str,
    out_dir: Option<&Path>,
) -> Result<NorDiscoveryReport> {
    let nor = fs::read(input).with_context(|| format!("Reading NOR binary {}", input.display()))?;
    let integrity = inspect_nor_dump_integrity(&nor);
    let sha256 = integrity.sha256.clone();
    let ipl_len = nor.len().min(0x4_0000);
    let loader_banner = integrity.loader_banner.clone();
    let loader_version = integrity.loader_version.clone();
    let loader_date = integrity.loader_date.clone();

    let extr_matches = find_extrhead_containers(&nor, 0x8400_0000);
    let extrhead_entries: Vec<DiscoveredExtrheadEntry> = extr_matches
        .iter()
        .map(|m| {
            let parsed = nor
                .get(m.offset..m.offset + 0x40)
                .and_then(|hdr| parse_extrhead_header(hdr).ok());
            DiscoveredExtrheadEntry {
                offset: m.offset,
                offset_hex: format!("0x{:06X}", m.offset),
                tag: parsed.as_ref().map(|h| h.tag.trim().to_string()),
                dst_addr_hex: parsed.as_ref().map(|h| format!("0x{:08X}", h.dst_addr)),
                dst_len: parsed.as_ref().map(|h| h.dst_len),
                src_len: parsed.as_ref().map(|h| h.src_len),
                build_name: parsed.as_ref().map(|h| h.build_name.clone()),
            }
        })
        .collect();

    let mut bgo_sectors = Vec::new();
    let mut eemul_1284 = Vec::new();
    let mut eemul_f1 = Vec::new();
    for off in (0..nor.len()).step_by(0x1_0000) {
        if off + 8 > nor.len() {
            break;
        }
        let tag_u16 = u16::from_le_bytes([nor[off], nor[off + 1]]);
        if tag_u16 == 0xF101 || (0xF201..=0xF2FF).contains(&tag_u16) {
            bgo_sectors.push(off);
        }
        if nor[off + 4..off + 8] == *b"1284" || nor[off..off + 4] == [0x00, 0x00, 0xF0, 0x0F] {
            eemul_1284.push(format!("0x{off:06X}"));
        }
        if nor[off + 5] == 0xF1 && off >= ipl_len {
            eemul_f1.push(format!("0x{off:06X}"));
        }
    }

    let bgo_audit = audit_bgo_nor_auto(&nor).ok();
    let spansion_blank = if nor.len() == 8 * 1024 * 1024 {
        analyze_spansion_nor_sectors(&nor)
            .ok()
            .map(|r| r.blank_sectors)
    } else {
        None
    };

    let bgo_start_off = bgo_sectors.first().copied();
    let bgo_end_off = bgo_sectors
        .last()
        .map(|&o| (o + 0xFFFF).min(nor.len().saturating_sub(1)));
    let mut regions = Vec::new();
    if ipl_len >= 0x10000 {
        regions.push(NorRegion {
            kind: NorRegionKind::Ipl,
            start_offset: 0,
            end_offset: (ipl_len - 1) as u32,
            policy: NorRegionPolicy::ReadOnlyPermanent,
            tag: Some(
                loader_banner
                    .clone()
                    .unwrap_or_else(|| "Bootloader IPL".to_string()),
            ),
            signature: None,
        });
    }
    for m in &extr_matches {
        if m.offset >= ipl_len {
            regions.push(NorRegion {
                kind: NorRegionKind::Extrhead,
                start_offset: m.offset as u32,
                end_offset: ((m.offset + 0xFFFF).min(nor.len().saturating_sub(1))) as u32,
                policy: NorRegionPolicy::GatedUpperNor,
                tag: Some("EXTRHEAD".to_string()),
                signature: None,
            });
        }
    }
    if let (Some(s), Some(e)) = (bgo_start_off, bgo_end_off) {
        regions.push(NorRegion {
            kind: NorRegionKind::Bgo,
            start_offset: s as u32,
            end_offset: e as u32,
            policy: NorRegionPolicy::GatedUpperNor,
            tag: Some("BGO FTL".to_string()),
            signature: Some(RegionSignature {
                marker_byte_offset: 0,
                marker_value: 0x01,
            }),
        });
    }
    if nor.len() >= 0x1_0000 {
        let nvram_start = (nor.len() - 0x1_0000) as u32;
        regions.push(NorRegion {
            kind: NorRegionKind::Eemul,
            start_offset: nvram_start,
            end_offset: (nor.len() - 1) as u32,
            policy: NorRegionPolicy::NvramSanitize,
            tag: Some("EEMUL NVRAM (top 64 KiB)".to_string()),
            signature: (!eemul_f1.is_empty()).then_some(RegionSignature {
                marker_byte_offset: 5,
                marker_value: 0xF1,
            }),
        });
    }

    let slug = slugify_id(model_id);
    let mut emitted = Vec::new();
    if let Some(dir) = out_dir {
        fs::create_dir_all(dir)?;
        let dump_evidence = ClaimEvidence {
            tier: if integrity.ok {
                EvidenceTier::PhysicalDumpVerified
            } else {
                EvidenceTier::CommunityUnverified
            },
            sources: vec![EvidenceSource {
                kind: "physical_dump".to_string(),
                reference: input.to_string_lossy().into_owned(),
                sha256: Some(sha256.clone()),
                notes: Some("Auto-discovered via panadiga profiles discover-nor".to_string()),
            }],
            notes: (!integrity.issues.is_empty()).then(|| integrity.issues.join("; ")),
        };
        let layout_id = format!("{slug}_nor_layout_draft");
        let layout = NorLayoutProfile {
            id: layout_id.clone(),
            flash_ref: "unknown_flash".to_string(),
            train_ref: "unknown_train".to_string(),
            regions: regions.clone(),
            evidence: dump_evidence.clone(),
        };
        let layout_path = dir.join(format!("{layout_id}.json"));
        fs::write(&layout_path, serde_json::to_string_pretty(&layout)?)?;
        emitted.push(layout_path.to_string_lossy().into_owned());

        let pending = ClaimEvidence::unverified("Pending verification via service manual or FRM");
        let model = ModelProfileManifest {
            model_id: model_id.to_string(),
            family_label: model_id.to_string(),
            series: "Discovered DIGA".to_string(),
            region_suffix: None,
            tv_standard: None,
            hw_code: bgo_audit.as_ref().and_then(|a| a.ifodev2_model.clone()),
            board_ref: None,
            train_ref: None,
            nor_layout_ref: Some(layout_id),
            exact_aliases: Vec::new(),
            fixture_ref: Some(format!("{slug}.fixture.stub.json")),
            claims: ModelClaims {
                board: pending.clone(),
                flash_chip: dump_evidence.clone(),
                nor_layout: dump_evidence,
                train: pending.clone(),
                eeprom: pending,
            },
        };
        let model_path = dir.join(format!("{slug}.draft.json"));
        fs::write(&model_path, serde_json::to_string_pretty(&model)?)?;
        emitted.push(model_path.to_string_lossy().into_owned());
    }

    Ok(NorDiscoveryReport {
        input_path: input.to_string_lossy().into_owned(),
        model_id: model_id.to_string(),
        size_bytes: nor.len(),
        sha256,
        loader_banner,
        loader_version,
        loader_date,
        integrity_ok: integrity.ok,
        integrity_warnings: integrity.issues,
        extrhead_entries,
        bgo_sector_count_64k: bgo_sectors.len(),
        bgo_start_hex: bgo_start_off.map(|v| format!("0x{v:06X}")),
        bgo_end_hex: bgo_end_off.map(|v| format!("0x{v:06X}")),
        bgo_ifodev2_model: bgo_audit.as_ref().and_then(|a| a.ifodev2_model.clone()),
        bgo_healthy: bgo_audit.as_ref().map(|a| a.healthy),
        eemul_1284_offsets_hex: eemul_1284,
        eemul_f1_offsets_hex: eemul_f1,
        spansion_8mb_blank_sectors: spansion_blank,
        regions,
        emitted_files: emitted,
    })
}
