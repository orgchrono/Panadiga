use std::path::PathBuf;

use crate::nvram::calculate_hashes;
use crate::re::SataBridgeResetFixMode;
use crate::re::mn10300_emit::{
    emit_ld_w_an_dm, emit_mov_d_to_a, emit_mov_imm32, emit_movbu_mem, emit_st_w_an_dm,
};
use crate::re::mn10300_emu::Mn103Vm;
use crate::re::sata_bridge_fix::{
    ORIG_PROG061_SATA_BRIDGE_RESET_WAIT, apply_prog061_sata_bridge_reset_fix_in_place,
};
pub(super) use crate::re::{
    NOR_AUTOBOOT_CAPSULE_MAGIC_LE, NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET, PROG061_BASE_VA,
    PROG061_EXPECTED_SIZE,
};

use super::asm::HddAutobootAsm;
use super::*;

mod nor_autoboot_tests;

pub(super) const VM_CALL_LOG_BASE: u32 = 0x5048_F000;
pub(super) const VM_CALL_COUNT_ADDR: u32 = 0x5048_EFFC;
pub(super) const VM_CFG_BASE: u32 = 0x5048_E000;
pub(super) const VM_CFG_SD_INIT_RC: u32 = VM_CFG_BASE;
pub(super) const VM_CFG_FAT_MOUNT_RC: u32 = VM_CFG_BASE + 0x04;
pub(super) const VM_CFG_FORCE_OPEN_RC: u32 = VM_CFG_BASE + 0x08;
pub(super) const VM_CFG_PANA_OPEN_RC: u32 = VM_CFG_BASE + 0x0C;
pub(super) const VM_CFG_MAX_LBA_RC: u32 = VM_CFG_BASE + 0x10;
pub(super) const VM_CFG_MAX_LBA_VAL: u32 = VM_CFG_BASE + 0x14;
pub(super) const VM_CFG_PRIMARY_FAIL: u32 = VM_CFG_BASE + 0x18;
pub(super) const VM_CFG_PRIMARY_SRC: u32 = VM_CFG_BASE + 0x1C;
pub(super) const VM_CFG_ENTRY_VA: u32 = VM_CFG_BASE + 0x20;
pub(super) const VM_CFG_BITWALK_FAIL: u32 = VM_CFG_BASE + 0x24;

pub(super) const TAG_PRINT: u32 = 1;
pub(super) const TAG_SLEEP: u32 = 2;
pub(super) const TAG_SD_INIT: u32 = 3;
pub(super) const TAG_FAT_MOUNT: u32 = 4;
pub(super) const TAG_FAT_OPEN_FORCE: u32 = 5;
pub(super) const TAG_FAT_OPEN_PANA: u32 = 6;
pub(super) const TAG_FAT_CLOSE: u32 = 7;
pub(super) const TAG_HDD_MAX_LBA: u32 = 8;
pub(super) const TAG_HDD_READ: u32 = 9;
pub(super) const TAG_FEISTEL: u32 = 10;
pub(super) const TAG_CHK_HDR: u32 = 11;
pub(super) const TAG_GET_ENTRY: u32 = 12;
pub(super) const TAG_GET_SRC: u32 = 13;
pub(super) const TAG_IS_COMP: u32 = 14;
pub(super) const TAG_GET_SIZE: u32 = 15;
pub(super) const TAG_MEMCPY: u32 = 16;
pub(super) const TAG_UNCOMP: u32 = 17;
pub(super) const TAG_DCACHE: u32 = 18;
pub(super) const TAG_STOP_TMR: u32 = 19;
pub(super) const TAG_DISABLE_IRQ: u32 = 20;
pub(super) const TAG_CHK_UPDATE: u32 = 21;

pub(super) const VA_MAIN_BITWALK_TABLE: u32 = 0x5700_0200;

/// Logs `[tag, D3 (saved D1), D2 (saved D0)]` at `VM_CALL_LOG_BASE + slot * 16`, preserving `D2`/`D3`.
fn emit_record_call(a: &mut HddAutobootAsm, tag: u32) {
    a.mov_imm32_a(0, VM_CALL_COUNT_ADDR);
    emit_ld_w_an_dm(&mut a.code, 0, 0); // D0 = slot_idx
    emit_mov_d_to_a(&mut a.code, 0, 1); // A1 = slot_idx
    a.add_imm8_d(0, 1);
    emit_st_w_an_dm(&mut a.code, 0, 0); // *(count_addr) = slot_idx + 1
    a.code.extend_from_slice(&[0xF1, 0xD4]); // MOV A1, D0
    a.asl_imm8_d(0, 4); // slot_idx * 16
    a.add_imm32_d(0, VM_CALL_LOG_BASE);
    emit_mov_d_to_a(&mut a.code, 0, 0); // A0 = entry ptr
    a.mov_imm32_d(0, tag);
    emit_st_w_an_dm(&mut a.code, 0, 0); // entry[0] = tag
    a.code.push(0x50); // INC4 A0
    emit_st_w_an_dm(&mut a.code, 3, 0); // entry[1] = saved D1
    a.code.push(0x50); // INC4 A0
    emit_st_w_an_dm(&mut a.code, 2, 0); // entry[2] = saved D0
}

const VM_STUB_POOL_BASE: u32 = 0x5080_0000;
const VM_STUB_SLOT_SIZE: usize = 0x100;

struct VmStubAllocator<'a> {
    prog_seg: &'a mut [u8],
    spec: &'a HddAutobootTrainSpec,
    stub_pool: Vec<u8>,
    slot_idx: usize,
}

impl<'a> VmStubAllocator<'a> {
    fn new(prog_seg: &'a mut [u8], spec: &'a HddAutobootTrainSpec) -> Self {
        Self {
            prog_seg,
            spec,
            stub_pool: vec![0xCB_u8; 0x1_0000],
            slot_idx: 0,
        }
    }

    fn install_custom(
        &mut self,
        entry_va: u32,
        build_fn: impl FnOnce(&mut HddAutobootAsm, &HddAutobootTrainSpec),
    ) {
        let slot_off = self.slot_idx * VM_STUB_SLOT_SIZE;
        let slot_va = VM_STUB_POOL_BASE + slot_off as u32;
        self.slot_idx += 1;

        let mut a = HddAutobootAsm::new(slot_va, self.spec.uart0_printf_va);
        a.code.extend_from_slice(&[0xCF, 0xC0]); // MOVM [D2, D3], (SP)
        a.mov_d_to_d(0, 2); // D2 = saved D0
        a.mov_d_to_d(1, 3); // D3 = saved D1
        build_fn(&mut a, self.spec);
        a.code.extend_from_slice(&[0xDF, 0xC0, 0x08]); // RET [D2, D3], 8
        let body = a.finalize().unwrap();
        assert!(
            body.len() <= VM_STUB_SLOT_SIZE && slot_off + body.len() <= self.stub_pool.len(),
            "VM stub at 0x{entry_va:08X} overflowed slot ({} B)",
            body.len()
        );
        self.stub_pool[slot_off..slot_off + body.len()].copy_from_slice(&body);

        let entry_off = (entry_va - self.spec.base_va) as usize;
        let rel = slot_va.wrapping_sub(entry_va).to_le_bytes();
        self.prog_seg[entry_off..entry_off + 5]
            .copy_from_slice(&[0xDC, rel[0], rel[1], rel[2], rel[3]]);
    }

    fn install_simple(
        &mut self,
        entry_va: u32,
        tag: u32,
        ret_d0: Option<u32>,
        cfg_addr: Option<u32>,
    ) {
        self.install_custom(entry_va, |a, _| {
            emit_record_call(a, tag);
            if let Some(addr) = cfg_addr {
                a.ld_abs32_d(addr, 0);
            } else if let Some(val) = ret_d0 {
                a.mov_imm32_d(0, val);
            } else {
                a.clr_d(0);
            }
        });
    }
}

#[derive(Clone, Copy)]
pub(super) enum NorFromTestState {
    Blank,
    RawAutoboot,
    NorAutobootSa11Magic,
}

pub(super) struct HddAutobootVmHarness {
    pub(super) vm: Mn103Vm,
}

impl HddAutobootVmHarness {
    fn new(prog061: &[u8]) -> Self {
        Self::new_full(prog061, NorFromTestState::Blank, None)
    }

    fn new_with_nor(prog061: &[u8], nor_has_autoboot: bool) -> Self {
        let state = if nor_has_autoboot {
            NorFromTestState::RawAutoboot
        } else {
            NorFromTestState::Blank
        };
        Self::new_full(prog061, state, None)
    }

    pub(super) fn new_with_nor_state(prog061: &[u8], nor_state: NorFromTestState) -> Self {
        Self::new_full(prog061, nor_state, None)
    }

    pub(super) fn new_full(
        prog061: &[u8],
        nor_state: NorFromTestState,
        real_main: Option<Vec<u8>>,
    ) -> Self {
        let has_real_main = real_main.is_some();
        let s = &hdd_autoboot_spec_for_loader_profile("ver061").unwrap();
        let mut vm = Mn103Vm::new();
        let mut prog_seg = prog061.to_vec();

        let real_getc_nb = [
            0xCF, 0x10, 0x3F, 0xFC, 0xAC, 0x0C, 0x20, 0x00, 0xD4, 0x81, 0xF8, 0xE1, 0x10, 0x84,
            0x1C, 0xA0, 0x00, 0xC8, 0x0D, 0xFC, 0xA8, 0x09, 0x20, 0x00, 0xD4, 0x81, 0x15, 0x84,
            0xCA, 0x05, 0x00, 0xCA, 0x02, 0xF2, 0xFC, 0xDF, 0x10, 0x04,
        ];
        let getc_off = (s.uart0_getc_nb_va - s.base_va) as usize;
        prog_seg[getc_off..getc_off + real_getc_nb.len()].copy_from_slice(&real_getc_nb);

        let stub_pool = {
            let mut alloc = VmStubAllocator::new(&mut prog_seg, s);
            alloc.install_simple(s.uart0_printf_va, TAG_PRINT, Some(0), None);
            alloc.install_simple(s.sleep_10ms_va, TAG_SLEEP, Some(0), None);
            alloc.install_simple(s.sd_init_va, TAG_SD_INIT, None, Some(VM_CFG_SD_INIT_RC));
            alloc.install_simple(
                s.fat_mount_va,
                TAG_FAT_MOUNT,
                None,
                Some(VM_CFG_FAT_MOUNT_RC),
            );
            alloc.install_simple(s.fat_close_va, TAG_FAT_CLOSE, Some(0), None);
            alloc.install_simple(s.feistel_decrypt_va, TAG_FEISTEL, Some(0), None);
            alloc.install_simple(s.extrhead_check_hdr_va, TAG_CHK_HDR, Some(1), None);
            alloc.install_simple(
                s.extrhead_get_entry_va,
                TAG_GET_ENTRY,
                None,
                Some(VM_CFG_ENTRY_VA),
            );
            alloc.install_simple(s.extrhead_is_comp_va, TAG_IS_COMP, Some(1), None);
            alloc.install_simple(
                s.extrhead_get_size_va,
                TAG_GET_SIZE,
                Some(0x0002_0000),
                None,
            );
            alloc.install_simple(s.memcpy_va, TAG_MEMCPY, Some(0x9400_0000), None);
            alloc.install_simple(s.dcache_purge_va, TAG_DCACHE, Some(0), None);
            alloc.install_simple(s.stop_timer_irq_va, TAG_STOP_TMR, Some(0), None);
            alloc.install_simple(s.disable_irq_va, TAG_DISABLE_IRQ, Some(0), None);
            alloc.install_simple(s.check_update_flag_va, TAG_CHK_UPDATE, Some(0), None);

            alloc.install_custom(s.fat_open_va, |a, _| {
                emit_mov_d_to_a(&mut a.code, 3, 0);
                emit_movbu_mem(&mut a.code, 0, 0);
                a.cmp_imm8_d(0, b'F' as i8);
                a.jcc(0xC8, "open_force");
                emit_record_call(a, TAG_FAT_OPEN_PANA);
                a.ld_abs32_d(VM_CFG_PANA_OPEN_RC, 0);
                a.jmp("open_done");
                a.label("open_force");
                emit_record_call(a, TAG_FAT_OPEN_FORCE);
                a.ld_abs32_d(VM_CFG_FORCE_OPEN_RC, 0);
                a.label("open_done");
            });

            alloc.install_custom(s.hdd_max_lba_va, |a, _| {
                emit_record_call(a, TAG_HDD_MAX_LBA);
                emit_mov_d_to_a(&mut a.code, 2, 0);
                a.ld_abs32_d(VM_CFG_MAX_LBA_VAL, 0);
                emit_st_w_an_dm(&mut a.code, 0, 0);
                a.ld_abs32_d(VM_CFG_MAX_LBA_RC, 0);
            });

            alloc.install_custom(s.hdd_read_va, |a, s| {
                emit_record_call(a, TAG_HDD_READ);
                a.cmp_imm8_d(2, 0);
                a.jcc(0xC9, "read_done");
                a.mov_imm32_a(0, s.hdd_dma_buf_va);
                emit_st_w_an_dm(&mut a.code, 3, 0);
                a.label("read_done");
                a.clr_d(0);
            });

            alloc.install_custom(s.extrhead_get_src_va, |a, s| {
                emit_record_call(a, TAG_GET_SRC);
                a.ld_abs32_d(s.hdd_dma_buf_va, 0);
                a.cmp_imm32_d0(s.hdd_primary_lba);
                a.jcc(0xC9, "ret_primary_src");
                a.ld_abs32_d(VM_CFG_PRIMARY_FAIL, 0);
                a.cmp_imm8_d(0, 0);
                a.jcc(0xC8, "ret_primary_src");
                a.clr_d(0);
                a.jmp("get_src_done");
                a.label("ret_primary_src");
                a.ld_abs32_d(VM_CFG_PRIMARY_SRC, 0);
                a.label("get_src_done");
            });

            if has_real_main {
                alloc.install_simple(s.extrhead_uncomp_va, TAG_UNCOMP, Some(1), None);
            } else {
                alloc.install_custom(s.extrhead_uncomp_va, |a, _| {
                    emit_record_call(a, TAG_UNCOMP);
                    emit_mov_imm32(&mut a.code, 0xA0, VM_CFG_ENTRY_VA); // A0 = *(entry_va)
                    a.code.extend_from_slice(&[0x20, 0x08]); // ADD 8, A0
                    a.mov_imm32_d(0, VA_MAIN_BITWALK_TABLE);
                    emit_st_w_an_dm(&mut a.code, 0, 0); // *(entry + 8) = bitwalk_table
                    a.ld_abs32_d(VM_CFG_BITWALK_FAIL, 0);
                    a.cmp_imm8_d(0, 0);
                    a.jcc(0xC8, "uncomp_ok");
                    a.mov_imm32_a(0, VA_MAIN_BITWALK_TABLE);
                    a.clr_d(0);
                    emit_st_w_an_dm(&mut a.code, 0, 0); // corrupt word 0
                    a.label("uncomp_ok");
                    a.mov_imm8_d(0, 1);
                });
            }

            alloc.stub_pool
        };

        let ram_entry_va = s.fvu_load_uncached_va.wrapping_add(s.payload_offset as u32);
        let nor_entry_va = s.norfrom_fvu_base_va.wrapping_add(s.payload_offset as u32);
        let ram_entry_bytes = prog061[s.payload_offset..s.payload_offset + 0x100].to_vec();
        let mut nor_base_bytes = vec![0xFFu8; 0x100];
        let nor_entry_bytes = match nor_state {
            NorFromTestState::Blank => vec![0xFFu8; 0x100],
            NorFromTestState::RawAutoboot => ram_entry_bytes.clone(),
            NorFromTestState::NorAutobootSa11Magic => {
                nor_base_bytes
                    [NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET..NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET + 4]
                    .copy_from_slice(&NOR_AUTOBOOT_CAPSULE_MAGIC_LE.to_le_bytes());
                vec![0xFFu8; 0x100]
            }
        };
        vm.map_segment(s.base_va, prog_seg, false);
        vm.map_segment(VM_STUB_POOL_BASE, stub_pool, false);
        vm.map_segment(0x5048_0000, vec![0u8; 0x1_0000], true);
        vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
        vm.map_segment(ram_entry_va, ram_entry_bytes, false);
        vm.map_segment(s.norfrom_fvu_base_va, nor_base_bytes, false);
        vm.map_segment(nor_entry_va, nor_entry_bytes, false);
        vm.map_segment(s.hdd_dma_buf_va, vec![0u8; 0x200], true);
        if let Some(main_img) = real_main {
            vm.map_segment(0x5700_0000, main_img, false);
        } else {
            vm.map_segment(0x5700_0000, vec![0xCBu8; 0x1000], true);
            for bit in 0..32u32 {
                vm.write_u32(VA_MAIN_BITWALK_TABLE + bit * 4, 1u32 << bit)
                    .unwrap();
            }
        }
        vm.sp = 0x50FF_FF00;
        vm.pc = s.base_va + s.hook_offset as u32;
        vm.enable_sc0_uart(&[]);
        vm.write_u32(VM_CFG_SD_INIT_RC, (-90001i32) as u32).unwrap();
        vm.write_u32(VM_CFG_FAT_MOUNT_RC, 1).unwrap();
        vm.write_u32(VM_CFG_FORCE_OPEN_RC, (-1i32) as u32).unwrap();
        vm.write_u32(VM_CFG_PANA_OPEN_RC, (-1i32) as u32).unwrap();
        vm.write_u32(VM_CFG_MAX_LBA_RC, 0).unwrap();
        vm.write_u32(VM_CFG_MAX_LBA_VAL, 0x1D1C_5970).unwrap();
        vm.write_u32(VM_CFG_PRIMARY_FAIL, 0).unwrap();
        vm.write_u32(VM_CFG_PRIMARY_SRC, 0x9400_0000).unwrap();
        vm.write_u32(VM_CFG_ENTRY_VA, 0x5700_0000).unwrap();
        vm.write_u32(VM_CFG_BITWALK_FAIL, 0).unwrap();
        Self { vm }
    }

    pub(super) fn run(&mut self, stop_pc: u32) -> Result<(), crate::re::Mn103Fault> {
        self.vm.run_until(stop_pc, 250_000)
    }

    pub(super) fn calls(&mut self) -> Vec<(u32, u32, u32)> {
        let n = self.vm.read_u32(VM_CALL_COUNT_ADDR).unwrap();
        (0..n)
            .map(|i| {
                let base = VM_CALL_LOG_BASE + i * 16;
                (
                    self.vm.read_u32(base).unwrap(),
                    self.vm.read_u32(base + 4).unwrap(),
                    self.vm.read_u32(base + 8).unwrap(),
                )
            })
            .collect()
    }
}

fn build_hdd_autoboot_prog_fixture() -> Vec<u8> {
    let mut buf = vec![0u8; PROG061_EXPECTED_SIZE];
    buf[PROG061_HDD_AUTOBOOT_PAYLOAD_OFFSET..PROG061_EXPECTED_SIZE - PROG_TRAILER_RESERVED_BYTES]
        .fill(PROG_SLACK_FILL_BYTE);
    buf[0x01_D928..0x01_D928 + 11].copy_from_slice(&ORIG_PROG061_SATA_BRIDGE_RESET_WAIT);
    buf[OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE..OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE + 7]
        .copy_from_slice(&ORIG_PROG061_HDD_AUTOBOOT_HOOK_BYTES);
    apply_prog061_sata_bridge_reset_fix_in_place(&mut buf, SataBridgeResetFixMode::Wait1000Ms)
        .unwrap();
    apply_prog061_hdd_autoboot_in_place(&mut buf, true).unwrap();
    buf
}

#[test]
fn hook_bytes_match_ver061_call32_displacement() {
    assert_eq!(
        compute_hdd_autoboot_hook_bytes(&hdd_autoboot_spec_for_loader_profile("ver061").unwrap()),
        [0xDD, 0x2D, 0x45, 0x03, 0x00, 0x00, 0x00]
    );
}

#[test]
fn mn103vm_executes_hdd_autoboot_primary_and_mirror_paths() {
    let prog = build_hdd_autoboot_prog_fixture();

    // 1. Direct SD PANA__SD.FRM happy path while Nor-FROM (0x8407_9600) is virgin 0xFF
    //    -> must NOT trigger Nor-FROM+SD fallback, must boot MAIN @ 0x5700_0000 with Loader teardown.
    let mut h = HddAutobootVmHarness::new(&prog);
    h.vm.write_u32(VM_CFG_SD_INIT_RC, 0).unwrap();
    h.vm.write_u32(VM_CFG_FAT_MOUNT_RC, 1).unwrap();
    h.vm.write_u32(VM_CFG_PANA_OPEN_RC, 0xC8).unwrap();
    h.run(0x5700_0000).unwrap();
    let calls = h.calls();
    assert!(!calls.iter().any(|&(t, _, _)| t == TAG_FAT_OPEN_PANA));
    assert!(
        calls
            .iter()
            .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x10000 && off == 0)
    );
    assert!(calls.iter().any(|&(t, _, _)| t == TAG_UNCOMP));
    let idx_dc = calls.iter().position(|c| c.0 == TAG_DCACHE).unwrap();
    let idx_tmr = calls.iter().position(|c| c.0 == TAG_STOP_TMR).unwrap();
    let idx_irq = calls.iter().position(|c| c.0 == TAG_DISABLE_IRQ).unwrap();
    assert!(idx_dc < idx_tmr && idx_tmr < idx_irq);

    // 2. Primary src_addr == 0 rejected -> outer mirror fallback (0x1D1A_5970) succeeds.
    let mut h_mir = HddAutobootVmHarness::new(&prog);
    h_mir.vm.write_u32(VM_CFG_PRIMARY_FAIL, 1).unwrap();
    h_mir.run(0x5700_0000).unwrap();
    let mir_calls = h_mir.calls();
    assert!(
        mir_calls
            .iter()
            .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x1D1A_5970 && off == 0)
    );

    // 3. Nor-FROM happy path: *(0x8407_9600) == *(0x9043_9600) with no SD card -> boots HDD MAIN.
    let mut h_nf_ok = HddAutobootVmHarness::new_with_nor(&prog, true);
    h_nf_ok.run(0x5700_0000).unwrap();
    assert!(
        h_nf_ok
            .calls()
            .iter()
            .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x10000 && off == 0)
    );
}

#[test]
fn mn103vm_executes_escape_hatches_and_fail_closed_fallbacks() {
    let prog = build_hdd_autoboot_prog_fixture();
    let hook_ret_pc = PROG061_BASE_VA + OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE as u32 + 7;

    // 1. UART0 'F' abort via real SC0 RX code -> fallback FVU (no HDD reads or teardown).
    let mut h_uart = HddAutobootVmHarness::new(&prog);
    h_uart.vm.queue_uart_rx(b"F");
    h_uart.run(hook_ret_pc).unwrap();
    let u_calls = h_uart.calls();
    assert_eq!(h_uart.vm.sp, 0x50FF_FF00);
    assert!(
        u_calls
            .iter()
            .any(|&(t, _, d0)| t == TAG_CHK_UPDATE && d0 == 0x10)
    );
    assert!(
        !u_calls
            .iter()
            .any(|c| c.0 == TAG_HDD_MAX_LBA || c.0 == TAG_DISABLE_IRQ)
    );

    // 2. SD FORCEFVU.TXT with FAT_MOUNT == 0 (regression check: mount_id == 0 must succeed).
    let mut h_force = HddAutobootVmHarness::new(&prog);
    h_force.vm.write_u32(VM_CFG_SD_INIT_RC, 0).unwrap();
    h_force.vm.write_u32(VM_CFG_FAT_MOUNT_RC, 0).unwrap();
    h_force.vm.write_u32(VM_CFG_FORCE_OPEN_RC, 0xC8).unwrap();
    h_force.run(hook_ret_pc).unwrap();
    let f_calls = h_force.calls();
    assert!(
        f_calls
            .iter()
            .any(|&(t, _, fd)| t == TAG_FAT_CLOSE && fd == 0xC8)
    );
    assert!(!f_calls.iter().any(|c| c.0 == TAG_HDD_MAX_LBA));

    // 3. Nor-FROM boots (raw SA11..SA14 match or SA11..SA12 compressed 'P4ZU') with/without SD.
    for nor_state in [
        NorFromTestState::RawAutoboot,
        NorFromTestState::NorAutobootSa11Magic,
    ] {
        assert_hdd_autoboot_ram_boots_and_escapes(&prog, nor_state);
    }

    // 4. Both mirrors invalid (src_addr == 0 everywhere) -> clean fallback to FVU.
    let mut h_bad = HddAutobootVmHarness::new(&prog);
    h_bad.vm.write_u32(VM_CFG_PRIMARY_SRC, 0).unwrap();
    h_bad.run(hook_ret_pc).unwrap();
    let bad_calls = h_bad.calls();
    assert_eq!(bad_calls.iter().filter(|c| c.0 == TAG_HDD_READ).count(), 2);
    assert!(bad_calls.iter().any(|c| c.0 == TAG_CHK_UPDATE));
    assert!(!bad_calls.iter().any(|c| c.0 == TAG_DISABLE_IRQ));

    // 5. Bit-walk failure at *(entry + 8) on both mirrors -> clean fallback to FVU.
    let mut h_bw = HddAutobootVmHarness::new(&prog);
    h_bw.vm.write_u32(VM_CFG_BITWALK_FAIL, 1).unwrap();
    h_bw.run(hook_ret_pc).unwrap();
    let bw_calls = h_bw.calls();
    assert_eq!(bw_calls.iter().filter(|c| c.0 == TAG_UNCOMP).count(), 2);
    assert!(bw_calls.iter().any(|c| c.0 == TAG_CHK_UPDATE));
    assert!(!bw_calls.iter().any(|c| c.0 == TAG_DISABLE_IRQ));
}

#[test]
fn hdd_autoboot_patch_enforces_fail_closed_preconditions_and_real_prog061_roundtrip() {
    let mut raw_prog = vec![0u8; PROG061_EXPECTED_SIZE];
    raw_prog
        [PROG061_HDD_AUTOBOOT_PAYLOAD_OFFSET..PROG061_EXPECTED_SIZE - PROG_TRAILER_RESERVED_BYTES]
        .fill(PROG_SLACK_FILL_BYTE);
    raw_prog[0x01_D928..0x01_D928 + 11].copy_from_slice(&ORIG_PROG061_SATA_BRIDGE_RESET_WAIT);
    raw_prog[OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE..OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE + 7]
        .copy_from_slice(&ORIG_PROG061_HDD_AUTOBOOT_HOOK_BYTES);
    assert!(apply_prog061_hdd_autoboot_in_place(&mut raw_prog, true).is_err());

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
    if path.is_file() {
        let orig = std::fs::read(&path).unwrap();
        let (_, orig_sha) = calculate_hashes(&orig);
        let mut work = orig;
        assert_eq!(
            inspect_prog061_hdd_autoboot_site(&work).unwrap().state,
            PROG061_HDD_AUTOBOOT_STATE_FACTORY
        );
        apply_prog061_sata_bridge_reset_fix_in_place(&mut work, SataBridgeResetFixMode::Wait1000Ms)
            .unwrap();
        apply_prog061_hdd_autoboot_in_place(&mut work, true).unwrap();
        let status = inspect_prog061_hdd_autoboot_site(&work).unwrap();
        assert_eq!(status.state, PROG061_HDD_AUTOBOOT_STATE_ENABLED);
        assert!(status.payload_present);
        apply_prog061_hdd_autoboot_in_place(&mut work, false).unwrap();
        apply_prog061_sata_bridge_reset_fix_in_place(
            &mut work,
            SataBridgeResetFixMode::FactoryOriginal,
        )
        .unwrap();
        assert_eq!(calculate_hashes(&work).1, orig_sha);
    }
}

pub(super) fn build_nor_autoboot_prog(prog_path: &std::path::Path) -> (Vec<u8>, String) {
    use crate::re::bgo_unbrick::{Prog061BgoPatchMode, patch_prog061_bgo_in_place};

    let mut autoboot_prog = std::fs::read(prog_path).unwrap();
    patch_prog061_bgo_in_place(&mut autoboot_prog, Prog061BgoPatchMode::BypassChecksOnly).unwrap();
    apply_prog061_sata_bridge_reset_fix_in_place(
        &mut autoboot_prog,
        SataBridgeResetFixMode::Wait1000Ms,
    )
    .unwrap();
    apply_prog061_hdd_autoboot_in_place(&mut autoboot_prog, true).unwrap();
    let sha = calculate_hashes(&autoboot_prog).1;
    (autoboot_prog, sha)
}

pub(super) fn assert_hdd_autoboot_ram_boots_and_escapes(
    pos_ram: &[u8],
    nor_state: NorFromTestState,
) {
    let mut h_boot = HddAutobootVmHarness::new_with_nor_state(pos_ram, nor_state);
    h_boot.run(0x5700_0000).unwrap();
    assert!(
        h_boot
            .calls()
            .iter()
            .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x10000 && off == 0)
    );
    let mut h_escape = HddAutobootVmHarness::new_with_nor_state(pos_ram, nor_state);
    h_escape.vm.write_u32(VM_CFG_SD_INIT_RC, 0).unwrap();
    h_escape.vm.write_u32(VM_CFG_FAT_MOUNT_RC, 0).unwrap();
    h_escape.vm.write_u32(VM_CFG_PANA_OPEN_RC, 0xC8).unwrap();
    let hook_ret_pc = PROG061_BASE_VA + OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE as u32 + 7;
    h_escape.run(hook_ret_pc).unwrap();
    let esc_calls = h_escape.calls();
    assert!(esc_calls.iter().any(|c| c.0 == TAG_FAT_OPEN_PANA));
    assert!(
        esc_calls
            .iter()
            .any(|&(t, _, id)| t == TAG_FAT_CLOSE && id == 0xC8)
    );
    assert!(esc_calls.iter().any(|c| c.0 == TAG_CHK_UPDATE));
    assert!(!esc_calls.iter().any(|c| c.0 == TAG_HDD_MAX_LBA));
}
