#![deny(unsafe_code)]

//! `.FRM` / `MainList` / `DRV` slot unpackers and region-MMIO planning helpers.

use std::path::Path;
use std::str::FromStr;

use anyhow::Result;

use super::scan::{BenchRuntimeStageDescriptor, scan_stage_anchors};
use super::stage_detect::{
    infer_stage_kind_and_base, looks_like_valid_main_list, try_unpack_stb_zlib, write_stage_if_dir,
};
use crate::constants::PANA_FEISTEL_SBOX_256;
use crate::crypto::{
    DecryptedFrmTocEntry, ExtrheadCryptMode, extract_drv_module, feistel_decrypt_buffer,
    parse_main_list, unpack_extrhead_with_key,
};
use crate::nvram::calculate_hashes;
use crate::re::region_compat::{MmioWriteOp, TvStandardKind, build_tv_calibration_mmio_plan};
use crate::ssot::ResolvedModelProfile;
use crate::ssot::registry::{InterpolatedRegionVariant, interpolate_region_variant};

pub(super) struct UnpackedSlotBytes {
    pub(super) bytes: Vec<u8>,
    pub(super) source_kind: String,
    pub(super) build_name: Option<String>,
    pub(super) algo: Option<String>,
    pub(super) dst_addr: Option<u32>,
}

pub(super) fn algo_name(algo: u16) -> String {
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

pub(super) fn unpack_frm_slot_bytes(
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

pub(super) fn resolve_region_and_mmio(
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

pub(super) struct FrmStageScanCtx<'a> {
    pub(super) outer_feistel: bool,
    pub(super) rk: Option<&'a [u8; 32]>,
    pub(super) key_name: Option<&'a str>,
    pub(super) resolved: Option<&'a ResolvedModelProfile>,
    pub(super) out_dir: Option<&'a Path>,
}

pub(super) fn push_frm_stages(
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
