#![deny(unsafe_code)]

//! SSOT-driven `PROG` BGO retargeting engine, positive allowlist validator, and tier gate.
//!
//! Reads `ProgBgoRetargetSpec` and verified compressed `nfrom_drv` payloads dynamically from
//! `panadiga_profiles` (`profiles/firmware_trains/*.json` + `profiles/blobs/*.gz`), keeping
//! zero model-specific descriptor tables, SDRAM relocation pairs, or patch arrays hardcoded in Rust.

mod apply;

use std::io::Read;

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use flate2::read::GzDecoder;
use panadiga_profiles::{MIN_CRITICAL_IPL_GUARD_BYTES, NorRegionKind, derive_sector_table};
use serde::{Deserialize, Serialize};

use super::{OFF_BGO_SUM_BEQ, PROG061_EXPECTED_SIZE};
use crate::constants::{
    AM34_CS1_NOR_OFFSET_MASK, BGO_SECTOR_SIZE_64K, BGO_TABLE_WORD_IDX_FLASH_BASE,
    BGO_TABLE_WORD_IDX_FLASH_END, BGO_TABLE_WORD_IDX_TOTAL_SECTORS,
};
use crate::nvram::calculate_hashes;
use crate::ssot::registry::{
    EvidenceTier, FirmwareTrainProfile, HardwareProfileRegistry, PanaRebuildRecipe,
    ProgBgoRetargetSpec, decode_base64,
};

pub use apply::{
    retarget_prog_for_geometry_in_place, retarget_prog_for_loader_profile_in_place,
    retarget_prog_for_loader_profile_with_policy, retarget_prog_from_spec_in_place,
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

/// Returns the [`FirmwareTrainProfile`] matching `loader_profile_key` from the validated SSOT registry.
#[must_use]
pub fn find_train_by_loader_key(loader_profile_key: &str) -> Option<&'static FirmwareTrainProfile> {
    HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .find(|train| train.loader_profile_key == loader_profile_key)
}

/// Returns the [`FirmwareTrainProfile`] defining a given SSOT `target_geometry_id`.
#[must_use]
pub fn find_train_by_geometry_id(geometry_id: &str) -> Option<&'static FirmwareTrainProfile> {
    HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .find(|train| {
            train
                .bgo_retarget_spec
                .as_ref()
                .is_some_and(|spec| spec.target_geometry_id == geometry_id)
        })
}

/// Runtime R1 tier-gating policy: blocks `< T3 (PhysicalDumpVerified)` trains from destructive BGO
/// retargeting or SD kit synthesis unless `allow_unverified_profile == true` is explicitly passed.
pub fn enforce_train_evidence_tier(
    loader_profile_key: &str,
    allow_unverified_profile: bool,
) -> Result<EvidenceTier> {
    let Some(train) = find_train_by_loader_key(loader_profile_key) else {
        if allow_unverified_profile {
            return Ok(EvidenceTier::CommunityUnverified);
        }
        bail!(
            "RUNTIME TIER GATE BLOCKED: unknown loader profile '{}' not found in verified SSOT firmware_trains; pass --allow-unverified-profile to override for R&D",
            loader_profile_key
        );
    };
    if train.evidence.tier < EvidenceTier::PhysicalDumpVerified && !allow_unverified_profile {
        bail!(
            "RUNTIME TIER GATE BLOCKED: firmware_train '{}' (loader '{}') has evidence tier {} (< T3 PhysicalDumpVerified); pass --allow-unverified-profile to override for R&D",
            train.id,
            loader_profile_key,
            train.evidence.tier.short_tag()
        );
    }
    Ok(train.evidence.tier)
}

/// Positive mathematical allowlist invariant for R3: proves that a `ProgBgoRetargetSpec` interval
/// `[start_off, end_incl_off]` is:
/// 1. Strictly contained inside the SSOT `nor_layout.regions[kind == Bgo]` span;
/// 2. Aligned at both boundaries to physical sectors of `nor_layout.flash_ref` (`derive_sector_table`);
/// 3. Strictly disjoint (`∩ = ∅`) from every non-BGO region (`Ipl`, `Extrhead`, `NorFs`, `Eemul`, `Erased`, etc.).
pub fn validate_bgo_allowlist_invariant(
    train: &FirmwareTrainProfile,
    spec: &ProgBgoRetargetSpec,
) -> Result<()> {
    let bundle = HardwareProfileRegistry::builtin().bundle();
    let layout = bundle
        .nor_layouts
        .values()
        .find(|l| l.train_ref == train.id)
        .with_context(|| {
            format!(
                "BGO allowlist invariant failed: no nor_layout in SSOT binds train_ref '{}'",
                train.id
            )
        })?;
    let flash = bundle.flash_chips.get(&layout.flash_ref).with_context(|| {
        format!(
            "BGO allowlist invariant failed: unknown flash_ref '{}' in nor_layout '{}'",
            layout.flash_ref, layout.id
        )
    })?;

    let table_base = *spec
        .bgo_table_words
        .get(BGO_TABLE_WORD_IDX_FLASH_BASE)
        .context("bgo_table_words missing word[12] (FLASH_BASE)")?;
    if table_base != spec.detect_flash_base {
        bail!(
            "BGO allowlist invariant failed for '{}': word[12]=0x{table_base:08X} != detect_flash_base=0x{:08X}",
            spec.target_geometry_id,
            spec.detect_flash_base
        );
    }
    let table_end_va = *spec
        .bgo_table_words
        .get(BGO_TABLE_WORD_IDX_FLASH_END)
        .context("bgo_table_words missing word[13] (FLASH_END)")?;
    let total_secs = *spec
        .bgo_table_words
        .get(BGO_TABLE_WORD_IDX_TOTAL_SECTORS)
        .context("bgo_table_words missing word[31] (TOTAL_SECTORS)")?;

    let start_off = spec.detect_flash_base & AM34_CS1_NOR_OFFSET_MASK;
    let end_excl_off = table_end_va & AM34_CS1_NOR_OFFSET_MASK;
    if start_off < MIN_CRITICAL_IPL_GUARD_BYTES {
        bail!(
            "BGO allowlist invariant failed: start_off 0x{start_off:06X} enters 256 KiB IPL guard (< 0x{MIN_CRITICAL_IPL_GUARD_BYTES:06X})"
        );
    }
    if end_excl_off <= start_off || end_excl_off > flash.total_bytes {
        bail!(
            "BGO allowlist invariant failed: span [0x{start_off:06X}..0x{end_excl_off:06X}) invalid for flash '{}' ({} B)",
            flash.part_number,
            flash.total_bytes
        );
    }
    let expected_end_excl = total_secs
        .checked_mul(BGO_SECTOR_SIZE_64K)
        .and_then(|span| start_off.checked_add(span))
        .context("BGO allowlist sector span arithmetic overflow")?;
    if expected_end_excl != end_excl_off {
        bail!(
            "BGO allowlist invariant failed: 0x{start_off:06X} + {total_secs}*64KiB = 0x{expected_end_excl:06X} != table FLASH_END off 0x{end_excl_off:06X}"
        );
    }
    let end_incl_off = end_excl_off - 1;

    let sectors = derive_sector_table(flash)?;
    if !sectors.iter().any(|s| s.start_offset == start_off) {
        bail!(
            "BGO allowlist invariant failed: start_off 0x{start_off:06X} is not aligned to any '{}' physical sector boundary",
            flash.part_number
        );
    }
    if !sectors
        .iter()
        .any(|s| s.end_offset_inclusive == end_incl_off)
    {
        bail!(
            "BGO allowlist invariant failed: end_off 0x{end_incl_off:06X} is not aligned to any '{}' physical sector boundary",
            flash.part_number
        );
    }

    let bgo_region = layout
        .regions
        .iter()
        .find(|r| r.kind == NorRegionKind::Bgo)
        .with_context(|| {
            format!(
                "BGO allowlist invariant failed: nor_layout '{}' lacks a NorRegionKind::Bgo entry",
                layout.id
            )
        })?;
    if start_off < bgo_region.start_offset || end_incl_off > bgo_region.end_offset {
        bail!(
            "BGO allowlist invariant failed: spec span [0x{start_off:06X}..=0x{end_incl_off:06X}] exceeds SSOT BGO allowlist [0x{:06X}..=0x{:06X}] in '{}'",
            bgo_region.start_offset,
            bgo_region.end_offset,
            layout.id
        );
    }

    for reg in layout
        .regions
        .iter()
        .filter(|r| r.kind != NorRegionKind::Bgo)
    {
        let overlaps = !(end_incl_off < reg.start_offset || start_off > reg.end_offset);
        if overlaps {
            bail!(
                "BGO allowlist invariant failed: spec span [0x{start_off:06X}..=0x{end_incl_off:06X}] intersects non-BGO {:?} region [0x{:06X}..=0x{:06X}] in '{}'",
                reg.kind,
                reg.start_offset,
                reg.end_offset,
                layout.id
            );
        }
    }

    Ok(())
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

pub(super) fn has_bgo_table_marker(buf: &[u8], spec: &ProgBgoRetargetSpec) -> bool {
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
