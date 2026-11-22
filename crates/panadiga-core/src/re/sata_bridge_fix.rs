#![deny(unsafe_code)]

//! Opt-in, fail-closed post-`RESET-` timing patch (`2 ms` -> `1000 ms`) for SATA-to-PATA bridges
//! (e.g. StarTech `IDE2SAT2` / Marvell `88SA8052` + `SAMSUNG HM251JI`) on Panasonic DIGA `Loader ver0.61`.
//!
//! Hardware trace and SDRAM validation proved that after the factory `25 us` `RESET-` pulse,
//! stock `PROG061` (`0x5041D928`) and `MAIN3060NP` (`0x571122B2`) call `udelay(2000)` before
//! polling `BSY`, exiting before `IDE2SAT2` asserts `BSY` (~32 ms) / `DRDY` (~555 ms). Replacing the
//! 11-byte call site with `MOV 100, D0; MOV D2, D1; CALL rtsd_sleep_10ms` yields an exact `1000 ms`
//! wait (`100 x 10 ms` RTOS ticks) in both `PROG061` (`0x50431303`) and `MAIN3060NP` (`0x574E6BBD`).

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::bgo_unbrick::{PROG061_BASE_VA, PROG061_EXPECTED_SIZE};
use super::main_passthrough::MAIN_BASE_VA;
use super::patch_spec::{
    PatchApplyAction, PatchSpec, apply_patch_specs_in_place, classify_patch_site,
};
use super::region_compat::EXPECTED_MAIN_3060NP_SIZE;

/// File offset inside raw `PROG061` (`262,144 B`, `VA 0x5041D928`) of the `2000 us` post-reset wait.
pub const OFF_PROG061_SATA_BRIDGE_RESET_WAIT: usize = 0x0001_D928;
/// Exact 11-byte factory sequence at `PROG061` `0x5041D928` (`MOV 0x07D0,D0; MOV D2,D1; CALL 0x5041B3E6`).
pub const ORIG_PROG061_SATA_BRIDGE_RESET_WAIT: [u8; 11] = [
    0x2C, 0xD0, 0x07, 0x89, 0xDD, 0xBA, 0xDA, 0xFF, 0xFF, 0x00, 0x00,
];
/// Patched 11-byte sequence at `PROG061` `0x5041D928` (`MOV 100,D0; MOV D2,D1; CALL 0x50431303` = `1000 ms`).
pub const PATCH_PROG061_SATA_BRIDGE_RESET_WAIT: [u8; 11] = [
    0x2C, 0x64, 0x00, 0x89, 0xDD, 0xD7, 0x39, 0x01, 0x00, 0x00, 0x00,
];

/// File offset inside unpacked `MAIN3060NP` (`13,427,852 B`, `VA 0x571122B2`) of the `2000 us` post-reset wait.
pub const OFF_MAIN3060NP_SATA_BRIDGE_RESET_WAIT: usize = 0x0011_22B2;
/// Exact 11-byte factory sequence at `MAIN3060NP` `0x571122B2` (`MOV 0x07D0,D0; MOV D2,D1; CALL 0x5710EC5D`).
pub const ORIG_MAIN3060NP_SATA_BRIDGE_RESET_WAIT: [u8; 11] = [
    0x2C, 0xD0, 0x07, 0x89, 0xDD, 0xA7, 0xC9, 0xFF, 0xFF, 0x00, 0x00,
];
/// Patched 11-byte sequence at `MAIN3060NP` `0x571122B2` (`MOV 100,D0; MOV D2,D1; CALL 0x574E6BBD` = `1000 ms`).
pub const PATCH_MAIN3060NP_SATA_BRIDGE_RESET_WAIT: [u8; 11] = [
    0x2C, 0x64, 0x00, 0x89, 0xDD, 0x07, 0x49, 0x3D, 0x00, 0x00, 0x00,
];

/// Canonical inspected site states for [`SataBridgeResetSiteStatus::state`].
pub const SATA_BRIDGE_STATE_FACTORY_2MS: &str = "FACTORY_2MS";
pub const SATA_BRIDGE_STATE_WAIT_1000MS: &str = "SATA_BRIDGE_1000MS";
pub const SATA_BRIDGE_STATE_MISMATCH: &str = "UNKNOWN_OR_MISMATCH";

const SPEC_PROG061_SATA_BRIDGE_RESET_WAIT: PatchSpec<'static> = PatchSpec {
    name: "prog061_sata_bridge_reset_wait",
    description: "PROG061 FUN_5041d8e4 post-RESET- 2 ms udelay -> 1000 ms RTOS wait (0x50431303)",
    va: PROG061_BASE_VA + OFF_PROG061_SATA_BRIDGE_RESET_WAIT as u32,
    base_va: PROG061_BASE_VA,
    expect: &ORIG_PROG061_SATA_BRIDGE_RESET_WAIT,
    replace: &PATCH_PROG061_SATA_BRIDGE_RESET_WAIT,
    required_len: Some(PROG061_EXPECTED_SIZE),
    input_sha256: None,
};

const SPEC_MAIN3060NP_SATA_BRIDGE_RESET_WAIT: PatchSpec<'static> = PatchSpec {
    name: "main3060np_sata_bridge_reset_wait",
    description: "MAIN3060NP FUN_5711226E post-RESET- 2 ms udelay -> 1000 ms RTOS wait (0x574E6BBD)",
    va: MAIN_BASE_VA + OFF_MAIN3060NP_SATA_BRIDGE_RESET_WAIT as u32,
    base_va: MAIN_BASE_VA,
    expect: &ORIG_MAIN3060NP_SATA_BRIDGE_RESET_WAIT,
    replace: &PATCH_MAIN3060NP_SATA_BRIDGE_RESET_WAIT,
    required_len: Some(EXPECTED_MAIN_3060NP_SIZE),
    input_sha256: None,
};

/// Mode selecting factory `2 ms` wait vs SATA-bridge `1000 ms` post-reset wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SataBridgeResetFixMode {
    #[default]
    FactoryOriginal,
    Wait1000Ms,
}

/// Deterministic inspection result for a single SATA-bridge hard-reset timing site.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SataBridgeResetSiteStatus {
    pub target: String,
    pub va_hex: String,
    pub file_offset_hex: String,
    pub state: String,
    pub bytes_hex: String,
}

fn inspect_site(
    target: &str,
    buf: &[u8],
    spec: &PatchSpec<'_>,
) -> Result<SataBridgeResetSiteStatus> {
    let cls = classify_patch_site(buf, spec, SATA_BRIDGE_STATE_WAIT_1000MS)?;
    let state = if cls.state == "FACTORY" {
        SATA_BRIDGE_STATE_FACTORY_2MS
    } else {
        cls.state
    };
    Ok(SataBridgeResetSiteStatus {
        target: target.to_string(),
        va_hex: format!("0x{:08X}", cls.va),
        file_offset_hex: format!("0x{:06X}", cls.file_offset),
        state: state.to_string(),
        bytes_hex: hex::encode(cls.actual_bytes),
    })
}

fn apply_site(buf: &mut [u8], spec: &PatchSpec<'_>, mode: SataBridgeResetFixMode) -> Result<()> {
    let action = match mode {
        SataBridgeResetFixMode::Wait1000Ms => PatchApplyAction::ApplyReplace,
        SataBridgeResetFixMode::FactoryOriginal => PatchApplyAction::RestoreExpect,
    };
    apply_patch_specs_in_place(buf, &[(spec, action)])
}

/// Inspects `PROG061` `0x5041D928` for factory `2 ms` vs `SATA_BRIDGE_1000MS` (`fail-closed` bounds).
pub fn inspect_prog061_sata_bridge_reset_site(buf: &[u8]) -> Result<SataBridgeResetSiteStatus> {
    inspect_site("PROG061", buf, &SPEC_PROG061_SATA_BRIDGE_RESET_WAIT)
}

/// Inspects unpacked `MAIN3060NP` `0x571122B2` for factory `2 ms` vs `SATA_BRIDGE_1000MS`.
pub fn inspect_main3060np_sata_bridge_reset_site(buf: &[u8]) -> Result<SataBridgeResetSiteStatus> {
    inspect_site("MAIN3060NP", buf, &SPEC_MAIN3060NP_SATA_BRIDGE_RESET_WAIT)
}

/// Applies or restores the `1000 ms` SATA-bridge reset wait in `PROG061` (`fail-closed`).
pub fn apply_prog061_sata_bridge_reset_fix_in_place(
    buf: &mut [u8],
    mode: SataBridgeResetFixMode,
) -> Result<()> {
    apply_site(buf, &SPEC_PROG061_SATA_BRIDGE_RESET_WAIT, mode)
}

/// Applies or restores the `1000 ms` SATA-bridge reset wait in `MAIN3060NP` (`fail-closed`).
pub fn apply_main3060np_sata_bridge_reset_fix_in_place(
    buf: &mut [u8],
    mode: SataBridgeResetFixMode,
) -> Result<()> {
    apply_site(buf, &SPEC_MAIN3060NP_SATA_BRIDGE_RESET_WAIT, mode)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::nvram::calculate_hashes;
    use crate::re::main_passthrough::EXPECTED_MAIN_3410PQ_SIZE;

    use super::*;

    type InspectFn = fn(&[u8]) -> Result<SataBridgeResetSiteStatus>;
    type ApplyFn = fn(&mut [u8], SataBridgeResetFixMode) -> Result<()>;

    fn verify_call32_encoding(
        patch: &[u8; 11],
        call_va: u32,
        expected_target_va: u32,
        expected_ticks: u16,
    ) {
        assert_eq!(patch[0], 0x2C);
        assert_eq!(u16::from_le_bytes([patch[1], patch[2]]), expected_ticks);
        assert_eq!(patch[3], 0x89);
        assert_eq!(patch[4], 0xDD);
        let disp = i32::from_le_bytes([patch[5], patch[6], patch[7], patch[8]]);
        let target = call_va.wrapping_add_signed(disp);
        assert_eq!(target, expected_target_va);
        assert_eq!(&patch[9..11], &[0x00, 0x00]);
    }

    fn assert_synthetic_roundtrip_and_fail_closed(
        size: usize,
        offset: usize,
        orig: &[u8; 11],
        patch: &[u8; 11],
        inspect_fn: InspectFn,
        apply_fn: ApplyFn,
    ) {
        let mut buf = vec![0xA5u8; size];
        assert!(apply_fn(&mut buf, SataBridgeResetFixMode::Wait1000Ms).is_err());
        assert_eq!(&buf[offset..offset + 11], &[0xA5u8; 11]);

        buf[offset..offset + 11].copy_from_slice(orig);
        assert_eq!(
            inspect_fn(&buf).unwrap().state,
            SATA_BRIDGE_STATE_FACTORY_2MS
        );

        apply_fn(&mut buf, SataBridgeResetFixMode::Wait1000Ms).unwrap();
        assert_eq!(&buf[offset..offset + 11], patch);
        assert_eq!(
            inspect_fn(&buf).unwrap().state,
            SATA_BRIDGE_STATE_WAIT_1000MS
        );

        // Idempotent reapply must succeed; corrupting a single byte must fail closed without mutation.
        apply_fn(&mut buf, SataBridgeResetFixMode::Wait1000Ms).unwrap();
        buf[offset + 5] ^= 0xFF;
        let corrupted = buf[offset..offset + 11].to_vec();
        assert!(apply_fn(&mut buf, SataBridgeResetFixMode::FactoryOriginal).is_err());
        assert_eq!(&buf[offset..offset + 11], corrupted.as_slice());

        buf[offset + 5] ^= 0xFF;
        apply_fn(&mut buf, SataBridgeResetFixMode::FactoryOriginal).unwrap();
        assert_eq!(
            inspect_fn(&buf).unwrap().state,
            SATA_BRIDGE_STATE_FACTORY_2MS
        );
    }

    #[test]
    fn verifies_mn103s_encoding_and_fail_closed_roundtrips() {
        verify_call32_encoding(
            &PATCH_PROG061_SATA_BRIDGE_RESET_WAIT,
            0x5041_D92C,
            0x5043_1303,
            100,
        );
        verify_call32_encoding(
            &PATCH_MAIN3060NP_SATA_BRIDGE_RESET_WAIT,
            0x5711_22B6,
            0x574E_6BBD,
            100,
        );

        assert_synthetic_roundtrip_and_fail_closed(
            PROG061_EXPECTED_SIZE,
            OFF_PROG061_SATA_BRIDGE_RESET_WAIT,
            &ORIG_PROG061_SATA_BRIDGE_RESET_WAIT,
            &PATCH_PROG061_SATA_BRIDGE_RESET_WAIT,
            inspect_prog061_sata_bridge_reset_site,
            apply_prog061_sata_bridge_reset_fix_in_place,
        );
        assert_synthetic_roundtrip_and_fail_closed(
            EXPECTED_MAIN_3060NP_SIZE,
            OFF_MAIN3060NP_SATA_BRIDGE_RESET_WAIT,
            &ORIG_MAIN3060NP_SATA_BRIDGE_RESET_WAIT,
            &PATCH_MAIN3060NP_SATA_BRIDGE_RESET_WAIT,
            inspect_main3060np_sata_bridge_reset_site,
            apply_main3060np_sata_bridge_reset_fix_in_place,
        );

        // Wrong MAIN build size (e.g. MAIN3410PQ) must fail closed even if bytes match at offset.
        let mut wrong_size_main = vec![0u8; EXPECTED_MAIN_3410PQ_SIZE];
        wrong_size_main
            [OFF_MAIN3060NP_SATA_BRIDGE_RESET_WAIT..OFF_MAIN3060NP_SATA_BRIDGE_RESET_WAIT + 11]
            .copy_from_slice(&ORIG_MAIN3060NP_SATA_BRIDGE_RESET_WAIT);
        assert!(
            apply_main3060np_sata_bridge_reset_fix_in_place(
                &mut wrong_size_main,
                SataBridgeResetFixMode::Wait1000Ms
            )
            .is_err()
        );
    }

    #[test]
    fn optional_real_workspace_binaries_roundtrip_exactly() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("firmware_and_images/extracted_partitions/frm_unpacked");
        for (rel, inspect_fn, apply_fn) in [
            (
                "prog061_dec.bin",
                inspect_prog061_sata_bridge_reset_site as InspectFn,
                apply_prog061_sata_bridge_reset_fix_in_place as ApplyFn,
            ),
            (
                "main3060np_unpacked.bin",
                inspect_main3060np_sata_bridge_reset_site as InspectFn,
                apply_main3060np_sata_bridge_reset_fix_in_place as ApplyFn,
            ),
        ] {
            let path = root.join(rel);
            if !path.is_file() {
                continue;
            }
            let orig = std::fs::read(&path).unwrap();
            let (_, orig_sha) = calculate_hashes(&orig);
            let mut work = orig;
            assert_eq!(
                inspect_fn(&work).unwrap().state,
                SATA_BRIDGE_STATE_FACTORY_2MS
            );
            apply_fn(&mut work, SataBridgeResetFixMode::Wait1000Ms).unwrap();
            assert_eq!(
                inspect_fn(&work).unwrap().state,
                SATA_BRIDGE_STATE_WAIT_1000MS
            );
            apply_fn(&mut work, SataBridgeResetFixMode::FactoryOriginal).unwrap();
            assert_eq!(calculate_hashes(&work).1, orig_sha);
        }
    }
}
