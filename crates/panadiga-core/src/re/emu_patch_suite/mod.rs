//! End-to-End Patcher + Spansion NOR Flash + `Mn103Vm` Emulator Verification Suite.
//!
//! Validates the complete Panadiga patch lifecycle offline using Matsushita MN103S execution
//! (`Mn103Vm`), simulated 8 MiB Spansion `S29JL064H` NOR flash (CFI + IPL read-only protection +
//! 64 KiB NVRAM), and real `.FRM` pack/retarget/decrypt/unpack roundtrips across:
//! 1. Spansion NOR Flash CFI autoselect & IPL (`0x8000_0000..0x807E_FFFF`) hardware write lockout
//! 2. `PANA_DVD.FRM` (Feistel) & `PANA__SD.FRM` (plaintext 64B) pack -> retarget -> unpack -> flash
//! 3. `PROG061` BGO unbrick bypass + interactive `PANAMON1>` UART0 (`SC0`) command functions
//! 4. `MAIN` TBC (`E-E`) passthrough routines (`FPC` + `ODD` state branch/init)
//! 5. `MAIN` `<deviceinfo>` region/model unlock (`3410PQ` & `3060NP`) + `DEV.BIN` repacking
//! 6. `MAIN` `src/hdd_cmd.c` >200 GB LBA48 resize (500 GB drive clamped vs preserved)

mod checks_hdd_lba;
mod checks_nor_frm;
mod checks_passthrough_region;
#[cfg(test)]
mod checks_real_nor;
mod fixtures;
mod types;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};
use crate::nvram::calculate_hashes;
use crate::re::{
    EXPECTED_MAIN_3060NP_SIZE, MainPassthroughMode, Prog061BgoPatchMode, RegionCompatMode,
    apply_main_passthrough_in_place, inspect_main_firmware_identity,
    patch_main_region_compat_in_place, patch_prog061_bgo_in_place_with_policy,
};

use checks_hdd_lba::verify_hdd_capacity_unlock;
use checks_nor_frm::{
    verify_bgo_and_panamon_suite, verify_frm_pack_retarget_and_flash, verify_nor_flash_and_cfi,
};
use checks_passthrough_region::{verify_region_compat_and_devbin, verify_tbc_passthrough_routines};
use fixtures::{build_synthetic_main_3410pq, build_synthetic_prog061};

pub use types::{NorEmuPatchSuiteReport, PatchVerificationCheck};

/// Runs the full offline end-to-end Patcher + `.FRM` Pack/Unpack + Spansion NOR + `Mn103Vm` suite.
///
/// When `prog061_path` / `main_path` are omitted, audited real MN103S byte windows are embedded into
/// synthetic `PROG061`/`MAIN3410PQ` layouts so CI verifies deterministically without proprietary ROMs.
pub fn run_nor_emu_patch_verification_suite(
    prog061_path: Option<&Path>,
    main_path: Option<&Path>,
) -> Result<NorEmuPatchSuiteReport> {
    let (prog061_factory, source_prog) = match prog061_path {
        Some(p) => (
            fs::read(p).with_context(|| format!("Reading PROG061 binary {}", p.display()))?,
            p.display().to_string(),
        ),
        None => (
            build_synthetic_prog061(),
            "synthetic_audited_windows".to_string(),
        ),
    };
    let (main_factory, source_main) = match main_path {
        Some(p) => (
            fs::read(p).with_context(|| format!("Reading MAIN binary {}", p.display()))?,
            p.display().to_string(),
        ),
        None => (
            build_synthetic_main_3410pq(),
            "synthetic_audited_windows".to_string(),
        ),
    };
    let source_mode = if prog061_path.is_some() || main_path.is_some() {
        format!("real_binaries ({source_prog} + {source_main})")
    } else {
        source_main
    };
    run_nor_emu_patch_verification_suite_from_slices(&prog061_factory, &main_factory, &source_mode)
}

/// Executes the end-to-end offline patch + `Mn103Vm` verification suite on in-memory firmware slices.
pub fn run_nor_emu_patch_verification_suite_from_slices(
    prog061_factory: &[u8],
    main_factory: &[u8],
    source_mode: &str,
) -> Result<NorEmuPatchSuiteReport> {
    // 1. Apply all patches cleanly in memory to working copies (CoW invariant preserved).
    let mut prog061_patched = prog061_factory.to_vec();
    patch_prog061_bgo_in_place_with_policy(
        &mut prog061_patched,
        Prog061BgoPatchMode::UartMonitor,
        false,
    )?;

    let main_ident = inspect_main_firmware_identity(main_factory, None);
    let mut main_patched = main_factory.to_vec();
    if main_ident.supports_3410pq_tbc_patch || main_factory.len() != EXPECTED_MAIN_3060NP_SIZE {
        apply_main_passthrough_in_place(&mut main_patched, MainPassthroughMode::AvPassthrough)?;
    }
    patch_main_region_compat_in_place(&mut main_patched, RegionCompatMode::BypassModelAndOem)?;
    apply_capacity_patch_in_place(
        &mut main_patched,
        HddCapacityPatchMode::DynamicUnclamped,
        None,
    )?;

    // 2. Wrap into EXTRHEAD, pack into PANA_DVD.FRM (Feistel) & PANA__SD.FRM (64B plaintext), retarget & verify!
    let frm = verify_frm_pack_retarget_and_flash(&prog061_patched, &main_patched)?;

    // 3. Flash unpacked binaries into NOR/SDRAM inside Mn103Vm & run every patched routine!
    let (mfg, dev, nor_check) = verify_nor_flash_and_cfi()?;
    let bgo_check = verify_bgo_and_panamon_suite(prog061_factory, &frm.prog061)?;
    let tbc_check = if main_ident.supports_3410pq_tbc_patch {
        verify_tbc_passthrough_routines(&frm.main)?
    } else {
        PatchVerificationCheck {
            id: "tbc_ee_passthrough_routines".to_string(),
            passed: true,
            detail: format!(
                "Skipped cleanly: MAIN build {} ({} B) is non-3410PQ; TBC passthrough targets 3410PQ",
                main_ident.build_id,
                main_factory.len()
            ),
        }
    };
    let region_check = verify_region_compat_and_devbin(main_factory, &frm.main)?;
    let hdd_check = verify_hdd_capacity_unlock(main_factory, &frm.main)?;

    let checks = vec![
        nor_check,
        frm.check,
        bgo_check,
        tbc_check,
        region_check,
        hdd_check,
    ];
    let all_passed = checks.iter().all(|c| c.passed);
    let (_, prog_sha) = calculate_hashes(&frm.prog061);
    let (_, main_sha) = calculate_hashes(&frm.main);

    Ok(NorEmuPatchSuiteReport {
        source_mode: source_mode.to_string(),
        cfi_manufacturer_id_hex: format!("0x{mfg:04X}"),
        cfi_device_id_hex: format!("0x{dev:04X}"),
        frm_dvd_bytes: frm.dvd_bytes,
        frm_sd_bytes: frm.sd_bytes,
        prog061_sha256: prog_sha,
        main_sha256: main_sha,
        checks,
        all_passed,
    })
}
