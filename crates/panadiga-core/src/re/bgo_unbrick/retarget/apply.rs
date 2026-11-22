#![deny(unsafe_code)]

//! In-place BGO retarget application routines enforcing SSOT allowlist & tier invariants.

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use panadiga_profiles::MIN_CRITICAL_IPL_GUARD_BYTES;

use super::super::{OFF_BGO_SUM_BEQ, ORIG_SUM_BEQ, PATCH_SUM_BRA};
use super::{
    BASE_FACTORY_BGO_GEOMETRY_ID, DetectedBgoGeometry, detect_prog_bgo_geometry,
    enforce_train_evidence_tier, find_retarget_spec_by_geometry_id,
    find_retarget_spec_by_loader_key, find_train_by_geometry_id, has_bgo_table_marker,
    inspect_prog_bgo_geometry_with_spec, load_retarget_drv_for_spec,
    validate_bgo_allowlist_invariant,
};
use crate::constants::{AM34_CS1_NOR_OFFSET_MASK, SPANSION_NVRAM_BASE_OFFSET};
use crate::ssot::registry::ProgBgoRetargetSpec;

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
///
/// Enforces Layer-1 IPL non-overlap and EEMUL NVRAM non-overlap unconditionally, plus full positive
/// SSOT BGO allowlist verification whenever `spec.target_geometry_id` belongs to a registered train.
pub fn retarget_prog_from_spec_in_place(
    raw_prog: &mut [u8],
    spec: &ProgBgoRetargetSpec,
    nfrom_drv: &[u8],
) -> Result<()> {
    let base_off = spec.detect_flash_base & AM34_CS1_NOR_OFFSET_MASK;
    if base_off < MIN_CRITICAL_IPL_GUARD_BYTES || base_off >= SPANSION_NVRAM_BASE_OFFSET as u32 {
        bail!(
            "BGO retarget '{}' flash base 0x{:08X} violates Layer-1 IPL/NVRAM NOR protection boundaries",
            spec.target_geometry_id,
            spec.detect_flash_base
        );
    }
    if let Some(train) = find_train_by_geometry_id(&spec.target_geometry_id) {
        validate_bgo_allowlist_invariant(train, spec)?;
    }

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

    let table_off = spec.bgo_table_offset as usize;
    for (idx, &word) in spec.bgo_table_words.iter().enumerate() {
        let off = table_off + idx * 4;
        LittleEndian::write_u32(&mut raw_prog[off..off + 4], word);
    }

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

    let init2_off = spec.init2_call_offset as usize;
    if LittleEndian::read_i32(&raw_prog[init2_off + 1..init2_off + 5]) == spec.orig_init2_rel32 {
        LittleEndian::write_i32(
            &mut raw_prog[init2_off + 1..init2_off + 5],
            spec.target_init2_rel32,
        );
    }

    relocate_hi_ram_ranges(raw_prog, spec);
    for bp in &spec.byte_patches {
        raw_prog[bp.offset as usize] = bp.replace;
    }

    let bss_off = spec.bss_zero_end_offset as usize;
    if LittleEndian::read_u32(&raw_prog[bss_off..bss_off + 4]) == spec.orig_bss_zero_end {
        LittleEndian::write_u32(
            &mut raw_prog[bss_off..bss_off + 4],
            spec.target_bss_zero_end,
        );
    }

    Ok(())
}

fn retarget_prog_for_geometry_with_policy(
    raw_prog: &mut [u8],
    target_geometry_id: &str,
    allow_unverified_profile: bool,
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
    let train = find_train_by_geometry_id(target_geometry_id).with_context(|| {
        format!("Unknown firmware_train for BGO retarget geometry '{target_geometry_id}'")
    })?;
    enforce_train_evidence_tier(&train.loader_profile_key, allow_unverified_profile)?;
    validate_bgo_allowlist_invariant(train, spec)?;

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

/// Retargets a decoded `PROG` binary to an explicit SSOT `target_geometry_id` idempotently.
///
/// Validates runtime tier (`>= T3`) and full SSOT BGO allowlist inclusion before retargeting.
pub fn retarget_prog_for_geometry_in_place(
    raw_prog: &mut [u8],
    target_geometry_id: &str,
) -> Result<DetectedBgoGeometry> {
    retarget_prog_for_geometry_with_policy(raw_prog, target_geometry_id, false)
}

/// Retargets a decoded `PROG` binary according to an SSOT `FirmwareTrain::loader_profile_key`,
/// enforcing the R1 runtime `< T3` tier-gate policy before emitting any payload.
pub fn retarget_prog_for_loader_profile_with_policy(
    raw_prog: &mut [u8],
    loader_profile_key: &str,
    allow_unverified_profile: bool,
) -> Result<Option<String>> {
    enforce_train_evidence_tier(loader_profile_key, allow_unverified_profile)?;
    let target_geo = find_retarget_spec_by_loader_key(loader_profile_key)
        .map_or(BASE_FACTORY_BGO_GEOMETRY_ID, |spec| {
            spec.target_geometry_id.as_str()
        });
    let detected =
        retarget_prog_for_geometry_with_policy(raw_prog, target_geo, allow_unverified_profile)?;
    Ok(detected.is_retargeted.then_some(detected.geometry_id))
}

/// Fail-closed wrapper around [`retarget_prog_for_loader_profile_with_policy`] (`allow_unverified_profile = false`).
pub fn retarget_prog_for_loader_profile_in_place(
    raw_prog: &mut [u8],
    loader_profile_key: &str,
) -> Result<Option<String>> {
    retarget_prog_for_loader_profile_with_policy(raw_prog, loader_profile_key, false)
}
