#![deny(unsafe_code)]

//! Payload preparation and preflight parameter adapters for composed MEIHFS flashing.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::constants::{LBA_SECTOR_SIZE_BYTES, PANA_EXTRHEAD_DEFAULT_ADLER_STEP};
use crate::crypto::{
    ExtrheadCryptMode, pack_extrhead, parse_sd_frm_toc_with_layout, unpack_dvd_frm_to_modules,
    unpack_extrhead,
};
use crate::hdd::capacity_patch::{HddCapacityPatchMode, apply_capacity_patch_in_place};
use crate::re::main_passthrough::{
    MAIN_BASE_VA, MainPassthroughMode, apply_main_passthrough_in_place,
    inspect_main_firmware_identity,
};
use crate::re::prewrite_gate::{PreWriteTarget, require_prewrite_gate};

use super::super::eh55_flash::backup_verify::HddPreflightParams;
use super::ComposedHddFlashRequest;

fn pad_to_sector_size(mut bytes: Vec<u8>) -> Vec<u8> {
    let rem = bytes.len() % LBA_SECTOR_SIZE_BYTES;
    if rem != 0 {
        bytes.resize(bytes.len() + (LBA_SECTOR_SIZE_BYTES - rem), 0xFF);
    }
    bytes
}

fn extract_module_from_frm_bytes(raw: &[u8], prefix: &str) -> Result<Option<Vec<u8>>> {
    if let Ok(mods) = unpack_dvd_frm_to_modules(raw)
        && let Some(m) = mods
            .into_iter()
            .find(|m| m.name.to_ascii_uppercase().starts_with(prefix))
    {
        return Ok(Some(m.payload));
    }
    let (layout, entries) = parse_sd_frm_toc_with_layout(raw);
    if layout.is_some()
        && let Some(e) = entries
            .into_iter()
            .find(|e| e.module_name.to_ascii_uppercase().starts_with(prefix))
    {
        let start = e.module_offset as usize;
        let end = start.saturating_add(e.module_size as usize);
        if end <= raw.len() {
            return Ok(Some(raw[start..end].to_vec()));
        }
    }
    Ok(None)
}

pub(super) fn prepare_main_extrhead(path: &Path, req: &ComposedHddFlashRequest) -> Result<Vec<u8>> {
    let raw = fs::read(path).with_context(|| format!("Reading MAIN source {}", path.display()))?;
    let payload = if raw.starts_with(b"EXTRHEAD") {
        raw
    } else if let Some(from_frm) = extract_module_from_frm_bytes(&raw, "MAIN")? {
        from_frm
    } else {
        raw
    };

    let needs_patch = req.apply_av_passthrough
        || req.apply_dynamic_capacity_unlock
        || req.target_capacity_gb.is_some();

    let packed = if payload.starts_with(b"EXTRHEAD") && !needs_patch {
        payload
    } else {
        let mut raw_main = if payload.starts_with(b"EXTRHEAD") {
            unpack_extrhead(&payload, ExtrheadCryptMode::Main)?.decompressed
        } else {
            payload
        };
        let ident = inspect_main_firmware_identity(&raw_main, None);
        if req.apply_av_passthrough {
            apply_main_passthrough_in_place(&mut raw_main, MainPassthroughMode::AvPassthrough)?;
        }
        if req.apply_dynamic_capacity_unlock {
            apply_capacity_patch_in_place(
                &mut raw_main,
                HddCapacityPatchMode::DynamicUnclamped,
                None,
            )?;
        } else if let Some(gb) = req.target_capacity_gb {
            apply_capacity_patch_in_place(
                &mut raw_main,
                HddCapacityPatchMode::FixedCapacity,
                Some(gb),
            )?;
        }
        let _ = require_prewrite_gate(PreWriteTarget::PatchedMain {
            raw_main: &raw_main,
            expect_passthrough: req.apply_av_passthrough && ident.supports_3410pq_tbc_patch,
            expect_capacity_unlock: req.apply_dynamic_capacity_unlock,
        })?;
        pack_extrhead(
            &raw_main,
            "FVU ",
            MAIN_BASE_VA,
            &ident.build_id,
            ExtrheadCryptMode::Main,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
            1,
        )?
    };
    Ok(pad_to_sector_size(packed))
}

pub(super) fn prepare_drv_extrhead(path: &Path) -> Result<Vec<u8>> {
    let raw = fs::read(path).with_context(|| format!("Reading DRV source {}", path.display()))?;
    let payload = if raw.starts_with(b"EXTRHEAD") {
        raw
    } else if let Some(from_frm) = extract_module_from_frm_bytes(&raw, "DRV")? {
        from_frm
    } else {
        pack_extrhead(
            &raw,
            "DRV ",
            crate::constants::AM34_DRV_S211_BASE,
            "S211",
            ExtrheadCryptMode::Drv,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
            1,
        )?
    };
    Ok(pad_to_sector_size(payload))
}

pub(super) fn preflight_params(req: &ComposedHddFlashRequest) -> HddPreflightParams<'_> {
    HddPreflightParams {
        device_path: &req.device_path,
        confirmation_token: &req.confirmation_token,
        expected_gb_range: req.expected_gb_range,
        detected_os_disks: req.detected_os_disks.as_deref(),
    }
}
