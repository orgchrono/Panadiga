//! `MAIN` TBC (`E-E`) passthrough and `<deviceinfo>` region/model + `DEV.BIN` verification checks.

use anyhow::{Result, bail};

use crate::constants::{PANA_DEV_BIN_MAGIC, PANA_DEV_BIN_SIG, PANA_DEV_BIN_SIZE};
use crate::crypto::codec::write_padded_ascii;
use crate::re::{
    EXPECTED_MAIN_3060NP_SIZE, MAIN_BASE_VA, OFF_FMU_DEVINFO_3060NP, OFF_FMU_DEVINFO_3410PQ,
    OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH, ORIG_FMU_DEVINFO_3060NP,
    ORIG_FMU_DEVINFO_3410PQ, PATCH_FMU_DEVINFO_3060NP, PATCH_FMU_DEVINFO_3410PQ,
    patch_dev_bin_bytes,
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

pub(super) fn verify_region_compat_and_devbin(
    main_factory: &[u8],
    main_patched: &[u8],
) -> Result<PatchVerificationCheck> {
    let is_3060np = main_factory.len() == EXPECTED_MAIN_3060NP_SIZE;
    let pair_3410 = if is_3060np {
        (
            &ORIG_FMU_DEVINFO_3410PQ[..4],
            &PATCH_FMU_DEVINFO_3410PQ[..4],
        )
    } else {
        (
            &main_factory[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 4],
            &main_patched[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 4],
        )
    };
    let pair_3060 = if is_3060np {
        (
            &main_factory[OFF_FMU_DEVINFO_3060NP..OFF_FMU_DEVINFO_3060NP + 4],
            &main_patched[OFF_FMU_DEVINFO_3060NP..OFF_FMU_DEVINFO_3060NP + 4],
        )
    } else {
        (
            &ORIG_FMU_DEVINFO_3060NP[..4],
            &PATCH_FMU_DEVINFO_3060NP[..4],
        )
    };
    for (label, off, fac_pair, pat_pair) in [
        ("3410PQ", OFF_FMU_DEVINFO_3410PQ, pair_3410.0, pair_3410.1),
        ("3060NP", OFF_FMU_DEVINFO_3060NP, pair_3060.0, pair_3060.1),
    ] {
        if fac_pair[..2] != [0xA0, 0x00] || pat_pair[..2] != [0xA0, 0x00] {
            bail!("Unexpected <deviceinfo> compare prefix for {label} @ 0x{off:06X}");
        }
        let d0_fac = run_two_byte_branch_probe(&fac_pair[2..4], 5)?;
        let d0_pat = run_two_byte_branch_probe(&pat_pair[2..4], 5)?;
        if d0_fac != 0xFFFF_FFFF || d0_pat != 0 {
            bail!("Region/model guard {label} mismatch: factory={d0_fac:X} patched={d0_pat:X}");
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
