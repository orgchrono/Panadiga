#![deny(unsafe_code)]

//! Streaming MEIHFS HDD Flasher by Composition (`ComposedHddFlashRequest`).
//!
//! Synthesizes MEIHFS metadata (`mkfs`), extracts/patches optional `MAIN`/`DRV` firmware
//! into sector-aligned `EXTRHEAD` containers in memory (without needing any 100 MiB `.img`),
//! enforces the mandatory `Mn103Vm` pre-write gate BEFORE touching disk layout, and writes
//! only active sectors + High-LBA mirror with CoW 64 KiB backup & full SHA-256 read-back.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{
    ALLOCATION_DESCRIPTORS_MIRROR_OFFSET, ALLOCATION_DESCRIPTORS_OFFSET,
    LBA_JOURNAL_CATALOG_OFFSET, LBA_MIRROR_MBR_OFFSET, LBA_PARTITION_1_OFFSET,
    LBA_PARTITION_2_OFFSET, LBA_SECTOR_SIZE_BYTES, PANA_EXTRHEAD_DEFAULT_ADLER_STEP,
};
use crate::crypto::{
    ExtrheadCryptMode, pack_extrhead, parse_sd_frm_toc_with_layout, unpack_dvd_frm_to_modules,
    unpack_extrhead,
};
use crate::hdd::capacity_patch::{HddCapacityPatchMode, apply_capacity_patch_in_place};
use crate::hdd::generate_meihdfs_layout;
use crate::nvram::calculate_hashes;
use crate::re::main_passthrough::{
    MAIN_BASE_VA, MainPassthroughMode, apply_main_passthrough_in_place,
    inspect_main_firmware_identity,
};
use crate::re::prewrite_gate::{PreWriteGateReport, PreWriteTarget, require_prewrite_gate};

use super::eh55_flash::backup_verify::{
    HddPreflightParams, preflight_hdd_target_geometry, verify_region_readback_sha256,
    write_chunked_with_progress, write_mandatory_64k_cow_backup,
};
use super::eh55_flash::{Eh55FlashProgress, Eh55FlashStage};
use super::raw_device::BlockFlashDevice;

/// Configuration for streaming MEIHFS HDD composition without a 100 MiB `.img` file.
#[derive(Debug, Clone)]
pub struct ComposedHddFlashRequest {
    pub device_path: String,
    pub confirmation_token: String,
    pub backup_dir: PathBuf,
    pub expected_gb_range: Option<(u32, u32)>,
    pub detected_os_disks: Option<Vec<String>>,
    pub main_source_path: Option<PathBuf>,
    pub drv_source_path: Option<PathBuf>,
    pub apply_dynamic_capacity_unlock: bool,
    pub target_capacity_gb: Option<u32>,
    pub apply_av_passthrough: bool,
    pub custom_guid: Option<[u8; 16]>,
}

/// Verified result of a streaming composed MEIHFS flash operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposedHddFlashReport {
    pub device_path: String,
    pub disk_bytes: u64,
    pub total_sectors: u64,
    pub bus_type_id: u32,
    pub mirror_main_lba: u64,
    pub backup_bin_path: String,
    pub backup_manifest_path: String,
    pub backup_64k_sha256: String,
    pub layout_sha256: String,
    pub p1_main_bytes_written: usize,
    pub p1_main_sha256: Option<String>,
    pub p2_drv_bytes_written: usize,
    pub p2_drv_sha256: Option<String>,
    pub mirror_main_bytes_written: usize,
    pub mirror_main_sha256: Option<String>,
    pub emu_gate: PreWriteGateReport,
    pub readback_verified: bool,
}

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

fn prepare_main_extrhead(path: &Path, req: &ComposedHddFlashRequest) -> Result<Vec<u8>> {
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

fn prepare_drv_extrhead(path: &Path) -> Result<Vec<u8>> {
    let raw = fs::read(path).with_context(|| format!("Reading DRV source {}", path.display()))?;
    let payload = if raw.starts_with(b"EXTRHEAD") {
        raw
    } else if let Some(from_frm) = extract_module_from_frm_bytes(&raw, "DRV")? {
        from_frm
    } else {
        pack_extrhead(
            &raw,
            "DRV ",
            0x9020_2000,
            "S211",
            ExtrheadCryptMode::Drv,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
            1,
        )?
    };
    Ok(pad_to_sector_size(payload))
}

fn preflight_params(req: &ComposedHddFlashRequest) -> HddPreflightParams<'_> {
    HddPreflightParams {
        device_path: &req.device_path,
        confirmation_token: &req.confirmation_token,
        expected_gb_range: req.expected_gb_range,
        detected_os_disks: req.detected_os_disks.as_deref(),
    }
}

/// Streams procedurally synthesized MEIHFS metadata + optional `MAIN`/`DRV` `EXTRHEAD` partitions
/// onto any [`BlockFlashDevice`], gated by `Mn103Vm` pre-write validation and full SHA-256 readback.
pub fn execute_composed_meihfs_hdd_flash<D, F>(
    device: &mut D,
    req: &ComposedHddFlashRequest,
    mut on_progress: F,
) -> Result<ComposedHddFlashReport>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(Eh55FlashProgress),
{
    let geo = preflight_hdd_target_geometry(device, preflight_params(req), &mut on_progress)?;
    let disk_bytes = geo.disk_bytes;
    let total_sectors = geo.total_sectors;
    let bus_type_id = geo.bus_type_id;
    let mirror_main_lba = geo.mirror_main_lba;
    let mirror_offset_bytes = geo.mirror_offset_bytes;

    let main_payload = req
        .main_source_path
        .as_deref()
        .map(|p| prepare_main_extrhead(p, req))
        .transpose()?;
    let drv_payload = req
        .drv_source_path
        .as_deref()
        .map(prepare_drv_extrhead)
        .transpose()?;

    if let Some(ref m) = main_payload
        && LBA_PARTITION_1_OFFSET + (m.len() as u64) > mirror_offset_bytes
    {
        bail!(
            "Composed P1 MAIN ({} B) exceeds mirror LBA {mirror_main_lba}",
            m.len()
        );
    }
    let emu_gate = require_prewrite_gate(PreWriteTarget::ComposedHddStream {
        total_sectors,
        main_extrhead: main_payload.as_deref(),
        drv_extrhead: drv_payload.as_deref(),
    })?;

    let layout = generate_meihdfs_layout(total_sectors, req.custom_guid)?;
    let (_, layout_sha256) = calculate_hashes(&layout.mbr_lba0);
    let (backup_bin_path, backup_manifest_path, backup_64k_sha256) =
        write_mandatory_64k_cow_backup(device, &req.backup_dir)?;

    device.delete_drive_layout()?;
    let layout_regions: &[(u64, &[u8], &str)] = &[
        (0, &layout.mbr_lba0, "MEIHFS MBR LBA 0"),
        (
            ALLOCATION_DESCRIPTORS_OFFSET,
            &layout.descriptors_lba24,
            "MEIHFS Descriptors",
        ),
        (
            LBA_JOURNAL_CATALOG_OFFSET,
            &layout.journal_header_lba16384,
            "MEIHFS Journal",
        ),
        (
            LBA_MIRROR_MBR_OFFSET,
            &layout.mirror_mbr_lba57344,
            "MEIHFS Mirror MBR",
        ),
        (
            ALLOCATION_DESCRIPTORS_MIRROR_OFFSET,
            &layout.mirror_descriptors_lba57368,
            "MEIHFS Mirror Desc",
        ),
    ];
    for &(off, data, label) in layout_regions {
        write_chunked_with_progress(
            device,
            off,
            data,
            Eh55FlashStage::Write100m,
            label,
            &mut on_progress,
        )?;
    }

    let mut p1_sha = None;
    let mut mirror_sha = None;
    if let Some(ref m) = main_payload {
        let (_, sha) = calculate_hashes(m);
        write_chunked_with_progress(
            device,
            LBA_PARTITION_1_OFFSET,
            m,
            Eh55FlashStage::Write100m,
            "P1 MAIN",
            &mut on_progress,
        )?;
        write_chunked_with_progress(
            device,
            mirror_offset_bytes,
            m,
            Eh55FlashStage::WriteMirror,
            "Mirror MAIN",
            &mut on_progress,
        )?;
        p1_sha = Some(sha.clone());
        mirror_sha = Some(sha);
    }
    let mut p2_sha = None;
    if let Some(ref d) = drv_payload {
        let (_, sha) = calculate_hashes(d);
        write_chunked_with_progress(
            device,
            LBA_PARTITION_2_OFFSET,
            d,
            Eh55FlashStage::WriteMirror,
            "P2 DRV",
            &mut on_progress,
        )?;
        p2_sha = Some(sha);
    }
    device.flush_and_update()?;

    for &(off, data, label) in layout_regions {
        let (_, sha) = calculate_hashes(data);
        let _ =
            verify_region_readback_sha256(device, off, data.len(), &sha, label, &mut on_progress)?;
    }
    if let (Some(m), Some(sha)) = (&main_payload, &p1_sha) {
        let _ = verify_region_readback_sha256(
            device,
            LBA_PARTITION_1_OFFSET,
            m.len(),
            sha,
            "P1 MAIN",
            &mut on_progress,
        )?;
        let _ = verify_region_readback_sha256(
            device,
            mirror_offset_bytes,
            m.len(),
            sha,
            "Mirror MAIN",
            &mut on_progress,
        )?;
    }
    if let (Some(d), Some(sha)) = (&drv_payload, &p2_sha) {
        let _ = verify_region_readback_sha256(
            device,
            LBA_PARTITION_2_OFFSET,
            d.len(),
            sha,
            "P2 DRV",
            &mut on_progress,
        )?;
    }

    Ok(ComposedHddFlashReport {
        device_path: req.device_path.clone(),
        disk_bytes,
        total_sectors,
        bus_type_id,
        mirror_main_lba,
        backup_bin_path: backup_bin_path.to_string_lossy().into_owned(),
        backup_manifest_path: backup_manifest_path.to_string_lossy().into_owned(),
        backup_64k_sha256,
        layout_sha256,
        p1_main_bytes_written: main_payload.as_ref().map_or(0, Vec::len),
        p1_main_sha256: p1_sha.clone(),
        p2_drv_bytes_written: drv_payload.as_ref().map_or(0, Vec::len),
        p2_drv_sha256: p2_sha,
        mirror_main_bytes_written: main_payload.as_ref().map_or(0, Vec::len),
        mirror_main_sha256: mirror_sha,
        emu_gate,
        readback_verified: true,
    })
}

#[cfg(test)]
#[path = "composed_flash_tests.rs"]
mod tests;
