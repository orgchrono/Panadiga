//! `MAIN` TBC (`E-E`) passthrough and `<deviceinfo>` region/model + `DEV.BIN` verification checks.

use anyhow::{Result, bail};

use crate::constants::{PANA_DEV_BIN_MAGIC, PANA_DEV_BIN_SIG, PANA_DEV_BIN_SIZE};
use crate::crypto::codec::write_padded_ascii;
use crate::re::{
    EXPECTED_MAIN_3060NP_SIZE, MAIN_BASE_VA, Mn103Vm, OFF_FMU_DEVINFO_3060NP,
    OFF_FMU_DEVINFO_3410PQ, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
    ORIG_FMU_DEVINFO_3060NP, ORIG_FMU_DEVINFO_3410PQ, PATCH_FMU_DEVINFO_3060NP,
    PATCH_FMU_DEVINFO_3410PQ, patch_dev_bin_bytes,
};

use super::fixtures::{TEST_RAM_WORK_BASE, new_spansion_nor_vm, run_two_byte_branch_probe};
use super::types::PatchVerificationCheck;

const TEST_RET_SENTINEL_VA: u32 = 0x577F_BEEF;

pub(super) fn verify_tbc_passthrough_routines(
    main_patched: &[u8],
) -> Result<PatchVerificationCheck> {
    let mut vm = new_spansion_nor_vm();
    vm.map_segment(MAIN_BASE_VA, main_patched.to_vec(), false);

    // Routine 1: FPC status control -> mov 1, d0; rets
    vm.sp = TEST_RAM_WORK_BASE + 0xE000;
    vm.write_u32(vm.sp, TEST_RET_SENTINEL_VA)?;
    vm.pc = MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32;
    vm.run_until(TEST_RET_SENTINEL_VA, 16)?;
    if vm.d[0] != 1 {
        bail!("FPC status ctrl returned D0={}, expected 1", vm.d[0]);
    }

    // Routine 2: ODD state branch -> jmp +0x00D0 -> ODD init status -> clr d0
    let odd_end = MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32 + 7;
    vm.d[0] = 0xDEAD_BEEF;
    vm.pc = MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32;
    vm.run_until(odd_end, 16)?;
    if vm.d[0] != 0 {
        bail!(
            "ODD state branch/init returned D0=0x{:X}, expected 0",
            vm.d[0]
        );
    }

    Ok(PatchVerificationCheck {
        id: "tbc_ee_passthrough_routines".to_string(),
        passed: true,
        detail: "FPC returns D0=1 and ODD branch skips drive init with D0=0 in MAIN SDRAM"
            .to_string(),
    })
}

pub(super) fn run_full_21b_fmu_devinfo_window_probe(
    win_21b: &[u8],
    sp_off_a: u32,
    sp_off_b: u32,
    d0_model_cmp: u32,
    oem_match: bool,
) -> Result<bool> {
    if win_21b.len() != 21 {
        bail!(
            "Expected 21-byte <deviceinfo> window, got {} B",
            win_21b.len()
        );
    }
    let mut vm = Mn103Vm::new();
    let code_va = 0x5749_8700u32;
    let mut code = vec![0xCBu8; 0x400];
    code[..21].copy_from_slice(win_21b);
    vm.map_segment(code_va, code, false);
    vm.map_segment(TEST_RAM_WORK_BASE, vec![0u8; 0x2000], true);
    vm.sp = TEST_RAM_WORK_BASE + 0x1000;
    vm.write_u8(vm.sp + sp_off_a, b'M')?;
    vm.write_u8(vm.sp + sp_off_b, if oem_match { b'M' } else { b'E' })?;
    vm.d[0] = d0_model_cmp;
    vm.pc = code_va;

    for _ in 0..8 {
        if vm.pc == code_va + 21 {
            return Ok(true);
        }
        if vm.pc >= code_va + 0x40 {
            return Ok(false);
        }
        vm.step()?;
    }
    Ok(vm.pc == code_va + 21)
}

pub(super) fn verify_region_compat_and_devbin(
    main_factory: &[u8],
    main_patched: &[u8],
) -> Result<PatchVerificationCheck> {
    let is_3060np = main_factory.len() == EXPECTED_MAIN_3060NP_SIZE;
    let win_3410 = if is_3060np {
        (&ORIG_FMU_DEVINFO_3410PQ[..], &PATCH_FMU_DEVINFO_3410PQ[..])
    } else {
        (
            &main_factory[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 21],
            &main_patched[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 21],
        )
    };
    let win_3060 = if is_3060np {
        (
            &main_factory[OFF_FMU_DEVINFO_3060NP..OFF_FMU_DEVINFO_3060NP + 21],
            &main_patched[OFF_FMU_DEVINFO_3060NP..OFF_FMU_DEVINFO_3060NP + 21],
        )
    } else {
        (&ORIG_FMU_DEVINFO_3060NP[..], &PATCH_FMU_DEVINFO_3060NP[..])
    };
    for (label, off, fac_win, pat_win, sp_a, sp_b) in [
        (
            "3410PQ",
            OFF_FMU_DEVINFO_3410PQ,
            win_3410.0,
            win_3410.1,
            0x64u32,
            0x2Bu32,
        ),
        (
            "3060NP",
            OFF_FMU_DEVINFO_3060NP,
            win_3060.0,
            win_3060.1,
            0x48u32,
            0x23u32,
        ),
    ] {
        if fac_win[..2] != [0xA0, 0x00] || pat_win[..2] != [0xA0, 0x00] {
            bail!("Unexpected <deviceinfo> compare prefix for {label} @ 0x{off:06X}");
        }
        let d0_fac = run_two_byte_branch_probe(&fac_win[2..4], 5)?;
        let d0_pat = run_two_byte_branch_probe(&pat_win[2..4], 5)?;
        if d0_fac != 0xFFFF_FFFF || d0_pat != 0 {
            bail!("Region/model guard {label} mismatch: factory={d0_fac:X} patched={d0_pat:X}");
        }
        // Full 21-byte <deviceinfo> execution (both model D0!=0 and OEM byte mismatch)
        let fac_rej_model = run_full_21b_fmu_devinfo_window_probe(fac_win, sp_a, sp_b, 1, false)?;
        let fac_rej_oem = run_full_21b_fmu_devinfo_window_probe(fac_win, sp_a, sp_b, 0, false)?;
        let fac_acc_match = run_full_21b_fmu_devinfo_window_probe(fac_win, sp_a, sp_b, 0, true)?;
        let pat_acc_both = run_full_21b_fmu_devinfo_window_probe(pat_win, sp_a, sp_b, 1, false)?;
        let pat_acc_oem = run_full_21b_fmu_devinfo_window_probe(pat_win, sp_a, sp_b, 0, false)?;
        if fac_rej_model || fac_rej_oem || !fac_acc_match || !pat_acc_both || !pat_acc_oem {
            bail!(
                "Full 21B <deviceinfo> execution failed for {label}: fac=({fac_rej_model},{fac_rej_oem},{fac_acc_match}) pat=({pat_acc_both},{pat_acc_oem})"
            );
        }
    }

    // DEV.BIN descriptor repacking validation
    let mut dev = vec![0xFFu8; PANA_DEV_BIN_SIZE];
    dev[0..4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
    write_padded_ascii(&mut dev[0x04..0x0C], "EH58EE", 0);
    dev[0x0C] = b'M';
    dev[0x20..0x2C].copy_from_slice(PANA_DEV_BIN_SIG);
    let (dev_out, dev_rep) = patch_dev_bin_bytes(&dev, "EH55EG", Some('E'))?;
    if dev_out.len() != PANA_DEV_BIN_SIZE || dev_rep.model != "EH55EG" || dev_rep.oem != "E" {
        bail!("DEV.BIN retarget check failed");
    }

    Ok(PatchVerificationCheck {
        id: "region_model_compat_unlock".to_string(),
        passed: true,
        detail: "3410PQ & 3060NP <deviceinfo> guard bypassed (D0=0 on mismatch) + DEV.BIN repacked"
            .to_string(),
    })
}
