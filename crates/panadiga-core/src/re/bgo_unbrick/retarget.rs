#![deny(unsafe_code)]

//! SSOT-driven `PROG` BGO retargeting engine.
//!
//! Reads `ProgBgoRetargetSpec` and verified compressed `nfrom_drv` payloads dynamically from
//! `panadiga_profiles` (`profiles/firmware_trains/*.json` + `profiles/blobs/*.gz`), keeping
//! zero model-specific descriptor tables, SDRAM relocation pairs, or patch arrays hardcoded in Rust.

use std::io::Read;

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use super::{OFF_BGO_SUM_BEQ, ORIG_SUM_BEQ, PATCH_SUM_BRA, PROG061_EXPECTED_SIZE};
use crate::nvram::calculate_hashes;
use crate::ssot::registry::{
    FirmwareTrainProfile, HardwareProfileRegistry, PanaRebuildRecipe, ProgBgoRetargetSpec,
    decode_base64,
};

/// Default factory baseline BGO geometry identifier for unmodified `4 MiB` donor images.
pub const BASE_FACTORY_BGO_GEOMETRY_ID: &str = "BGO_35SEC_4MB_0X84200000";

/// Detected BGO geometry inside an unpacked `PROG` binary derived from SSOT train specs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedBgoGeometry {
    /// Declarative geometry identifier (e.g. `BGO_35SEC_4MB_0X84200000` or `BGO_45SEC_8MB_0X84130000`).
    pub geometry_id: String,
    /// `true` when matched against an SSOT `bgo_retarget_spec`; `false` for baseline factory geometry.
    pub is_retargeted: bool,
    /// File offset of the BGO sector checksum branch instruction within the unpacked `PROG` image.
    pub bgo_sum_beq_offset: usize,
    /// Matching SSOT `firmware_trains` ID when a retargeted geometry was detected.
    pub train_id: Option<String>,
}

/// Returns the [`ProgBgoRetargetSpec`] for `train_id` from the validated SSOT registry.
pub fn load_train_retarget_spec(train_id: &str) -> Result<&'static ProgBgoRetargetSpec> {
    let bundle = HardwareProfileRegistry::builtin().bundle();
    let train = bundle
        .firmware_trains
        .get(train_id)
        .with_context(|| format!("Unknown firmware_train '{train_id}' in SSOT registry"))?;
    train
        .bgo_retarget_spec
        .as_ref()
        .with_context(|| format!("firmware_train '{train_id}' does not define bgo_retarget_spec"))
}

/// Loads and verifies the decompressed `nfrom_drv` blob referenced by an SSOT [`ProgBgoRetargetSpec`].
pub fn load_retarget_drv_for_spec(spec: &ProgBgoRetargetSpec) -> Result<Vec<u8>> {
    let bundle = HardwareProfileRegistry::builtin().bundle();
    let gz_b64 = bundle
        .blobs
        .get(&spec.nfrom_drv_blob_relpath)
        .with_context(|| {
            format!(
                "Missing SSOT blob '{}' required by geometry '{}'",
                spec.nfrom_drv_blob_relpath, spec.target_geometry_id
            )
        })?;
    let compressed = decode_base64(gz_b64).with_context(|| {
        format!(
            "Decoding base64 SSOT blob '{}'",
            spec.nfrom_drv_blob_relpath
        )
    })?;

    let mut decoder = GzDecoder::new(compressed.as_slice());
    let expected_len = spec.nfrom_drv_uncompressed_bytes as usize;
    let mut nfrom_drv = Vec::with_capacity(expected_len);
    decoder
        .read_to_end(&mut nfrom_drv)
        .with_context(|| format!("Decompressing SSOT blob '{}'", spec.nfrom_drv_blob_relpath))?;
    if nfrom_drv.len() != expected_len {
        bail!(
            "Decompressed SSOT blob '{}' length mismatch: got {} B, expected {expected_len} B",
            spec.nfrom_drv_blob_relpath,
            nfrom_drv.len()
        );
    }
    let (_, actual_sha) = calculate_hashes(&nfrom_drv);
    if actual_sha != spec.nfrom_drv_uncompressed_sha256 {
        bail!(
            "Decompressed SSOT blob '{}' SHA-256 mismatch: expected {}, got {actual_sha}",
            spec.nfrom_drv_blob_relpath,
            spec.nfrom_drv_uncompressed_sha256
        );
    }
    Ok(nfrom_drv)
}

/// Loads `(ProgBgoRetargetSpec, decompressed nfrom_drv)` for `train_id` from the validated SSOT bundle.
pub fn load_train_retarget_spec_and_drv(
    train_id: &str,
) -> Result<(&'static ProgBgoRetargetSpec, Vec<u8>)> {
    let spec = load_train_retarget_spec(train_id)?;
    let nfrom_drv = load_retarget_drv_for_spec(spec)?;
    Ok((spec, nfrom_drv))
}

fn find_train_by_loader_key(loader_profile_key: &str) -> Option<&'static FirmwareTrainProfile> {
    HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .find(|train| train.loader_profile_key == loader_profile_key)
}

/// Looks up an SSOT [`ProgBgoRetargetSpec`] by `FirmwareTrain::loader_profile_key`.
#[must_use]
pub fn find_retarget_spec_by_loader_key(
    loader_profile_key: &str,
) -> Option<&'static ProgBgoRetargetSpec> {
    find_train_by_loader_key(loader_profile_key).and_then(|train| train.bgo_retarget_spec.as_ref())
}

/// Looks up an SSOT [`ProgBgoRetargetSpec`] by canonical `target_geometry_id`.
#[must_use]
pub fn find_retarget_spec_by_geometry_id(
    geometry_id: &str,
) -> Option<&'static ProgBgoRetargetSpec> {
    HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .filter_map(|train| train.bgo_retarget_spec.as_ref())
        .find(|spec| spec.target_geometry_id == geometry_id)
}

/// Looks up an SSOT [`PanaRebuildRecipe`] by `FirmwareTrain::loader_profile_key`.
#[must_use]
pub fn find_recipe_by_loader_key(loader_profile_key: &str) -> Option<&'static PanaRebuildRecipe> {
    find_train_by_loader_key(loader_profile_key)
        .and_then(|train| train.pana_rebuild_recipe.as_ref())
}

fn has_bgo_table_marker(buf: &[u8], spec: &ProgBgoRetargetSpec) -> bool {
    let off = spec.bgo_table_offset as usize;
    buf.len() == spec.expected_prog_size as usize
        && off.saturating_add(4) <= buf.len()
        && LittleEndian::read_u32(&buf[off..off + 4]) == spec.bgo_table_marker
}

/// Returns `true` when `buf` carries a recognized SSOT BGO table marker at its spec offset.
#[must_use]
pub fn prog_has_bgo_table_marker(buf: &[u8]) -> bool {
    HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .filter_map(|train| train.bgo_retarget_spec.as_ref())
        .any(|spec| has_bgo_table_marker(buf, spec))
}

/// Returns `true` when `data` matches `spec`'s target BGO descriptor (`detect_logical_sectors` & `detect_flash_base`).
pub fn inspect_prog_bgo_geometry_with_spec(
    data: &[u8],
    spec: &ProgBgoRetargetSpec,
) -> Result<bool> {
    let expected_len = spec.expected_prog_size as usize;
    if data.len() != expected_len {
        bail!(
            "Expected {expected_len}-byte PROG binary, got {} bytes",
            data.len()
        );
    }
    if !has_bgo_table_marker(data, spec) {
        return Ok(false);
    }
    let tbl = spec.bgo_table_offset as usize;
    let logical_sectors = LittleEndian::read_u32(&data[tbl + 0x0C..tbl + 0x10]);
    let flash_base = LittleEndian::read_u32(&data[tbl + 0x30..tbl + 0x34]);
    Ok(logical_sectors == spec.detect_logical_sectors && flash_base == spec.detect_flash_base)
}

/// Detects whether a decoded `PROG` binary matches any SSOT `bgo_retarget_spec` or factory baseline.
pub fn detect_prog_bgo_geometry(data: &[u8]) -> Result<DetectedBgoGeometry> {
    let bundle = HardwareProfileRegistry::builtin().bundle();
    for train in bundle.firmware_trains.values() {
        if let Some(spec) = train.bgo_retarget_spec.as_ref()
            && inspect_prog_bgo_geometry_with_spec(data, spec)?
        {
            return Ok(DetectedBgoGeometry {
                geometry_id: spec.target_geometry_id.clone(),
                is_retargeted: true,
                bgo_sum_beq_offset: spec.bgo_sum_beq_offset as usize,
                train_id: Some(train.id.clone()),
            });
        }
    }
    if data.len() != PROG061_EXPECTED_SIZE {
        bail!(
            "invalid PROG payload size {} (expected {} bytes)",
            data.len(),
            PROG061_EXPECTED_SIZE
        );
    }
    Ok(DetectedBgoGeometry {
        geometry_id: BASE_FACTORY_BGO_GEOMETRY_ID.to_string(),
        is_retargeted: false,
        bgo_sum_beq_offset: OFF_BGO_SUM_BEQ,
        train_id: None,
    })
}

fn relocate_hi_ram_ranges(buf: &mut [u8], spec: &ProgBgoRetargetSpec) {
    if spec.hi_ram_relocs.is_empty() {
        return;
    }
    for rng in &spec.hi_ram_ranges {
        let start = rng.start as usize;
        let limit = (rng.end as usize).saturating_sub(3);
        for off in start..limit {
            let word = LittleEndian::read_u32(&buf[off..off + 4]);
            if let Some(reloc) = spec.hi_ram_relocs.iter().find(|r| r.from == word) {
                LittleEndian::write_u32(&mut buf[off..off + 4], reloc.to);
            }
        }
    }
}

/// Applies an arbitrary SSOT [`ProgBgoRetargetSpec`] and validated `nfrom_drv` blob idempotently in place.
pub fn retarget_prog_from_spec_in_place(
    raw_prog: &mut [u8],
    spec: &ProgBgoRetargetSpec,
    nfrom_drv: &[u8],
) -> Result<()> {
    if inspect_prog_bgo_geometry_with_spec(raw_prog, spec)? || !has_bgo_table_marker(raw_prog, spec)
    {
        return Ok(());
    }
    if nfrom_drv.len() != spec.nfrom_drv_uncompressed_bytes as usize {
        bail!(
            "nfrom_drv payload length {} B != spec expected {} B",
            nfrom_drv.len(),
            spec.nfrom_drv_uncompressed_bytes
        );
    }

    let orig_sum_off = OFF_BGO_SUM_BEQ;
    let bgo_sum_was_bypassed =
        raw_prog.get(orig_sum_off..orig_sum_off + 2) == Some(PATCH_SUM_BRA.as_slice());

    // 1. Validate every single-byte site before mutating anything.
    for bp in &spec.byte_patches {
        let off = bp.offset as usize;
        if off >= raw_prog.len() {
            bail!(
                "Retarget '{}' byte patch offset 0x{off:06X} exceeds image size {}",
                spec.target_geometry_id,
                raw_prog.len()
            );
        }
        if raw_prog[off] != bp.expect && raw_prog[off] != bp.replace {
            bail!(
                "Retarget '{}' mismatch at 0x{off:06X}: expected 0x{:02X}, found 0x{:02X}",
                spec.target_geometry_id,
                bp.expect,
                raw_prog[off]
            );
        }
    }

    // 2. Write 64-word BGO descriptor table from SSOT.
    let table_off = spec.bgo_table_offset as usize;
    for (idx, &word) in spec.bgo_table_words.iter().enumerate() {
        let off = table_off + idx * 4;
        LittleEndian::write_u32(&mut raw_prog[off..off + 4], word);
    }

    // 3. Transplant relocated `nfrom_drv` payload and NOP-fill slack to window end.
    let drv_start = spec.nfrom_drv_start as usize;
    let drv_end = spec.nfrom_drv_end as usize;
    let drv_write_end = drv_start + nfrom_drv.len();
    if drv_write_end > drv_end || drv_end > raw_prog.len() {
        bail!(
            "Retarget '{}' nfrom_drv window [0x{drv_start:06X}..0x{drv_end:06X}) invalid for {} B payload",
            spec.target_geometry_id,
            nfrom_drv.len()
        );
    }
    raw_prog[drv_start..drv_write_end].copy_from_slice(nfrom_drv);
    raw_prog[drv_write_end..drv_end].fill(spec.nop_fill_byte);

    let sum_off = spec.bgo_sum_beq_offset as usize;
    if bgo_sum_was_bypassed {
        raw_prog[sum_off..sum_off + 2].copy_from_slice(&PATCH_SUM_BRA);
    } else if raw_prog[sum_off..sum_off + 2] != ORIG_SUM_BEQ {
        bail!("Unexpected byte sequence at retargeted BGO sum offset 0x{sum_off:06X}");
    }

    // 4. Relocate init_2 CALL32 displacement (`FUN_5040248e`).
    let init2_off = spec.init2_call_offset as usize;
    if LittleEndian::read_i32(&raw_prog[init2_off + 1..init2_off + 5]) == spec.orig_init2_rel32 {
        LittleEndian::write_i32(
            &mut raw_prog[init2_off + 1..init2_off + 5],
            spec.target_init2_rel32,
        );
    }

    // 5. Relocate BGO RAM pointers in configured ranges and apply verified byte patches.
    relocate_hi_ram_ranges(raw_prog, spec);
    for bp in &spec.byte_patches {
        raw_prog[bp.offset as usize] = bp.replace;
    }

    // 6. Expand `_start` BSS clear end so relocated tables zero cleanly on boot.
    let bss_off = spec.bss_zero_end_offset as usize;
    if LittleEndian::read_u32(&raw_prog[bss_off..bss_off + 4]) == spec.orig_bss_zero_end {
        LittleEndian::write_u32(
            &mut raw_prog[bss_off..bss_off + 4],
            spec.target_bss_zero_end,
        );
    }

    Ok(())
}

/// Retargets a decoded `PROG` binary to an explicit SSOT `target_geometry_id` idempotently.
///
/// Preserves baseline [`BASE_FACTORY_BGO_GEOMETRY_ID`] untouched and refuses to downgrade an
/// already-retargeted image back to baseline in place.
pub fn retarget_prog_for_geometry_in_place(
    raw_prog: &mut [u8],
    target_geometry_id: &str,
) -> Result<DetectedBgoGeometry> {
    let current = detect_prog_bgo_geometry(raw_prog)?;
    if target_geometry_id == BASE_FACTORY_BGO_GEOMETRY_ID {
        if current.is_retargeted {
            bail!(
                "Cannot downgrade already-retargeted PROG ({}) back to factory baseline ({}) in-place; start from clean donor PROG",
                current.geometry_id,
                BASE_FACTORY_BGO_GEOMETRY_ID
            );
        }
        return Ok(current);
    }
    let spec = find_retarget_spec_by_geometry_id(target_geometry_id).with_context(|| {
        format!("Unknown SSOT BGO retarget geometry '{target_geometry_id}' in firmware_trains")
    })?;
    if current.is_retargeted && current.geometry_id != spec.target_geometry_id {
        bail!(
            "Cannot retarget PROG from {} to {} without clean factory baseline donor",
            current.geometry_id,
            spec.target_geometry_id
        );
    }
    if !has_bgo_table_marker(raw_prog, spec) {
        return Ok(current);
    }
    let nfrom_drv = load_retarget_drv_for_spec(spec)?;
    retarget_prog_from_spec_in_place(raw_prog, spec, &nfrom_drv)?;
    detect_prog_bgo_geometry(raw_prog)
}

/// Retargets a decoded `PROG` binary according to an SSOT `FirmwareTrain::loader_profile_key`.
///
/// Returns `Ok(Some(target_geometry_id))` if a retargeted geometry is active after the call, or
/// `Ok(None)` if the loader profile stays on factory baseline geometry (or the payload lacks a
/// real BGO table marker).
pub fn retarget_prog_for_loader_profile_in_place(
    raw_prog: &mut [u8],
    loader_profile_key: &str,
) -> Result<Option<String>> {
    let target_geo = find_retarget_spec_by_loader_key(loader_profile_key)
        .map_or(BASE_FACTORY_BGO_GEOMETRY_ID, |spec| {
            spec.target_geometry_id.as_str()
        });
    let detected = retarget_prog_for_geometry_in_place(raw_prog, target_geo)?;
    Ok(detected.is_retargeted.then_some(detected.geometry_id))
}
