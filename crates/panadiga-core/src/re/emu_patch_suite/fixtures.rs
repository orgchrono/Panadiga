//! Synthetic `PROG061`/`MAIN3410PQ` layouts, Spansion NOR VM setup, and MN103S branch probes.

use anyhow::Result;

use crate::constants::{AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};
use crate::hdd::capacity_patch::specs::HDD_CMD_CLAMP_WINDOW_TEMPLATE;
use crate::re::{
    EXPECTED_MAIN_3410PQ_SIZE, Mn103Vm, OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ,
    OFF_DEVINFO_CHECK_BEQ, OFF_FMU_DEVINFO_3410PQ, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS,
    OFF_ODD_STATE_BRANCH, ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ,
    ORIG_FMU_DEVINFO_3410PQ, ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH,
    ORIG_SUM_BEQ, PROG061_EXPECTED_SIZE,
};

const TEST_MAIN_HDD_CMD_OFFSET: usize = 0x004C_2600;
pub(super) const TEST_RAM_WORK_BASE: u32 = 0x5000_0000;
const TEST_RAM_WORK_SIZE: usize = 0x2_0000;
const TEST_BSS_BASE: u32 = 0x5311_B000;
const TEST_BSS_SIZE: usize = 0x1000;

pub(super) fn build_synthetic_prog061() -> Vec<u8> {
    let mut img = vec![0xFFu8; PROG061_EXPECTED_SIZE];
    img[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + ORIG_SUM_BEQ.len()].copy_from_slice(&ORIG_SUM_BEQ);
    img[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + ORIG_DEVINFO_BEQ.len()]
        .copy_from_slice(&ORIG_DEVINFO_BEQ);
    img[OFF_BGO_INIT_CALL..OFF_BGO_INIT_CALL + ORIG_A2BC_BGO_CALL.len()]
        .copy_from_slice(&ORIG_A2BC_BGO_CALL);
    img[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + ORIG_A2CD_HOOK.len()]
        .copy_from_slice(&ORIG_A2CD_HOOK);
    img
}

pub(super) fn build_synthetic_main_3410pq() -> Vec<u8> {
    let mut img = vec![0xCBu8; EXPECTED_MAIN_3410PQ_SIZE];
    img[OFF_FPC_STATUS_CTRL..OFF_FPC_STATUS_CTRL + ORIG_FPC_STATUS_CTRL.len()]
        .copy_from_slice(&ORIG_FPC_STATUS_CTRL);
    img[OFF_ODD_STATE_BRANCH..OFF_ODD_STATE_BRANCH + ORIG_ODD_STATE_BRANCH.len()]
        .copy_from_slice(&ORIG_ODD_STATE_BRANCH);
    img[OFF_ODD_INIT_STATUS..OFF_ODD_INIT_STATUS + ORIG_ODD_INIT_STATUS.len()]
        .copy_from_slice(&ORIG_ODD_INIT_STATUS);
    img[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + ORIG_FMU_DEVINFO_3410PQ.len()]
        .copy_from_slice(&ORIG_FMU_DEVINFO_3410PQ);
    img[TEST_MAIN_HDD_CMD_OFFSET..TEST_MAIN_HDD_CMD_OFFSET + HDD_CMD_CLAMP_WINDOW_TEMPLATE.len()]
        .copy_from_slice(&HDD_CMD_CLAMP_WINDOW_TEMPLATE);
    img
}

pub(super) fn new_spansion_nor_vm() -> Mn103Vm {
    let mut vm = Mn103Vm::new();
    let nor_image = vec![0xA5u8; AM34_FLASH_SIZE as usize];
    vm.map_segment(AM34_FLASH_UNCACHED_BASE, nor_image, true);
    vm.map_segment(TEST_RAM_WORK_BASE, vec![0u8; TEST_RAM_WORK_SIZE], true);
    vm.map_segment(TEST_BSS_BASE, vec![0u8; TEST_BSS_SIZE], true);
    vm
}

pub(super) fn run_two_byte_branch_probe(branch_2b: &[u8], jump_disp: usize) -> Result<u32> {
    let mut vm = Mn103Vm::new();
    let code_va = TEST_RAM_WORK_BASE + 0x10_0000;
    let mut code = vec![0xCBu8; jump_disp + 16];
    code[0..4].copy_from_slice(&[0x80, 0x01, 0xA0, 0x00]); // mov 1, d0; cmp 0, d0 -> Z=0
    code[4..6].copy_from_slice(branch_2b);
    code[6..9].copy_from_slice(&[0x80, 0xFF, 0xCB]); // fallthrough: mov -1, d0; nop
    let target = 4 + jump_disp;
    code[target..target + 2].copy_from_slice(&[0x00, 0xCB]); // taken: clr d0; nop
    let stop_pass = code_va + target as u32 + 1;

    vm.map_segment(code_va, code, false);
    vm.pc = code_va;
    let mut steps = 0u64;
    while vm.pc != stop_pass && vm.pc != code_va + 8 && steps < 16 {
        vm.step()?;
        steps += 1;
    }
    Ok(vm.d[0])
}
