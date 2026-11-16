//! `MAIN` updater (`3410PQ` & `3060NP`) `<deviceinfo>` model/OEM guard patcher.

use anyhow::{Result, bail};

use crate::constants::EXPECTED_MAIN_3060NP_SIZE_BYTES;
use crate::nvram::calculate_hashes;

use super::super::main_passthrough::{EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA};
use super::super::patch_spec::{
    PatchApplyAction, PatchSpec, apply_patch_specs_in_place, classify_patch_site,
};
use super::types::{MainRegionCompatReport, RegionCompatMode};

/// Exact unpacked byte size of `MAIN3060NP` (`PANA_DVD.FRM` slot 2 / `PANA__SD.FRM`).
pub const EXPECTED_MAIN_3060NP_SIZE: usize = EXPECTED_MAIN_3060NP_SIZE_BYTES;

/// `3410PQ` `src/fmupre/pre_update_firm.c` model/OEM comparison pair (`VA 0x57498738`, 21 B).
pub const OFF_FMU_DEVINFO_3410PQ: usize = 0x0049_8738;
/// `3060NP` `src/fmupre/pre_update_firm.c` model/OEM comparison pair (`VA 0x574A01AA`, 21 B).
pub const OFF_FMU_DEVINFO_3060NP: usize = 0x004A_01AA;

pub const ORIG_FMU_DEVINFO_3410PQ: [u8; 21] = [
    0xA0, 0x00, 0xC8, 0x05, 0xCC, 0xE2, 0x00, 0xFB, 0xAA, 0x90, 0x64, 0xF8, 0xBB, 0x2B, 0xF1, 0xAD,
    0xC8, 0x05, 0xCC, 0xD4, 0x00,
];
pub const PATCH_FMU_DEVINFO_3410PQ: [u8; 21] = [
    0xA0, 0x00, 0xCA, 0x05, 0xCC, 0xE2, 0x00, 0xFB, 0xAA, 0x90, 0x64, 0xF8, 0xBB, 0x2B, 0xF1, 0xAD,
    0xCA, 0x05, 0xCC, 0xD4, 0x00,
];

pub const ORIG_FMU_DEVINFO_3060NP: [u8; 21] = [
    0xA0, 0x00, 0xC8, 0x05, 0xCC, 0x0D, 0x01, 0xF8, 0xBA, 0x48, 0xFB, 0xAA, 0x90, 0x23, 0xF1, 0x96,
    0xC8, 0x05, 0xCC, 0xFF, 0x00,
];
pub const PATCH_FMU_DEVINFO_3060NP: [u8; 21] = [
    0xA0, 0x00, 0xCA, 0x05, 0xCC, 0x0D, 0x01, 0xF8, 0xBA, 0x48, 0xFB, 0xAA, 0x90, 0x23, 0xF1, 0x96,
    0xCA, 0x05, 0xCC, 0xFF, 0x00,
];

const SPEC_DEVINFO_3410PQ: PatchSpec<'static> = PatchSpec {
    name: "fmu_devinfo_3410pq",
    description: "3410PQ FUN_5749859a <deviceinfo> model + oem beq +5 -> bra +5",
    va: MAIN_BASE_VA + OFF_FMU_DEVINFO_3410PQ as u32,
    base_va: MAIN_BASE_VA,
    expect: &ORIG_FMU_DEVINFO_3410PQ,
    replace: &PATCH_FMU_DEVINFO_3410PQ,
    required_len: Some(EXPECTED_MAIN_3410PQ_SIZE),
    input_sha256: None,
};

const SPEC_DEVINFO_3060NP: PatchSpec<'static> = PatchSpec {
    name: "fmu_devinfo_3060np",
    description: "3060NP pre_update_firm <deviceinfo> model + oem beq +5 -> bra +5",
    va: MAIN_BASE_VA + OFF_FMU_DEVINFO_3060NP as u32,
    base_va: MAIN_BASE_VA,
    expect: &ORIG_FMU_DEVINFO_3060NP,
    replace: &PATCH_FMU_DEVINFO_3060NP,
    required_len: Some(EXPECTED_MAIN_3060NP_SIZE),
    input_sha256: None,
};

fn resolve_main_spec(len: usize) -> Result<(&'static str, &'static PatchSpec<'static>)> {
    match len {
        EXPECTED_MAIN_3410PQ_SIZE => Ok(("3410PQ", &SPEC_DEVINFO_3410PQ)),
        EXPECTED_MAIN_3060NP_SIZE => Ok(("3060NP", &SPEC_DEVINFO_3060NP)),
        other => bail!(
            "Unsupported MAIN size {other} B (expected 3410PQ={} B or 3060NP={} B)",
            EXPECTED_MAIN_3410PQ_SIZE,
            EXPECTED_MAIN_3060NP_SIZE
        ),
    }
}

/// Inspects the `MAIN` `<deviceinfo>` model/OEM comparison site deterministically.
pub fn inspect_main_region_compat(bin: &[u8]) -> Result<MainRegionCompatReport> {
    let (profile, spec) = resolve_main_spec(bin.len())?;
    let cls = classify_patch_site(bin, spec, "BYPASS_MODEL_AND_OEM")?;
    let (_, sha256) = calculate_hashes(bin);
    Ok(MainRegionCompatReport {
        build_profile: profile.to_string(),
        size_bytes: bin.len(),
        va_hex: format!("0x{:08X}", cls.va),
        file_offset_hex: format!("0x{:08X}", cls.file_offset),
        state: cls.state.to_string(),
        actual_hex: hex::encode(cls.actual_bytes),
        sha256,
    })
}

/// Applies or restores `RegionCompatMode` atomically on an unpacked `MAIN` image in memory.
pub fn patch_main_region_compat_in_place(
    bin: &mut [u8],
    mode: RegionCompatMode,
) -> Result<MainRegionCompatReport> {
    let (_, spec) = resolve_main_spec(bin.len())?;
    let action = match mode {
        RegionCompatMode::BypassModelAndOem => PatchApplyAction::ApplyReplace,
        RegionCompatMode::FactoryStrict => PatchApplyAction::RestoreExpect,
    };
    apply_patch_specs_in_place(bin, &[(spec, action)])?;
    inspect_main_region_compat(bin)
}
