use super::*;
use crate::re::bgo_unbrick::{PATCH_DEVINFO_BRA, PATCH_SUM_BRA, PROG061_BASE_VA};
use crate::re::main_passthrough::{
    MAIN_BASE_VA, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
    PATCH_FPC_STATUS_CTRL, PATCH_ODD_INIT_STATUS, PATCH_ODD_STATE_BRANCH,
};

#[test]
fn test_main_passthrough_and_bgo_patches_execute_as_designed() {
    // 1. FPC status control: `mov 1, d0; rets; nop; nop` returns to stack-top LR with D0=1.
    let mut vm = Mn103Vm::new();
    let fpc_va = MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32;
    vm.map_segment(fpc_va, PATCH_FPC_STATUS_CTRL.to_vec(), false);
    vm.map_segment(0x5000_0F00, vec![0; 0x200], true);
    vm.sp = 0x5000_1000;
    vm.write_u32(vm.sp, 0x5700_1234).unwrap();
    vm.pc = fpc_va;
    vm.run_until(0x5700_1234, 16).unwrap();
    assert_eq!(vm.d[0], 1);
    assert_eq!(vm.sp, 0x5000_1004);

    // 2. ODD state branch + init status: `jmp16 +0x00D0` -> 0x574F7C5C -> `clr d0 (80 00)` -> D0=0.
    let mut odd_vm = Mn103Vm::new();
    let br_va = MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32;
    let init_va = MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32;
    let span = (init_va - br_va) as usize + PATCH_ODD_INIT_STATUS.len();
    let mut odd_mem = vec![0xCBu8; span];
    odd_mem[..PATCH_ODD_STATE_BRANCH.len()].copy_from_slice(&PATCH_ODD_STATE_BRANCH);
    let rel = (init_va - br_va) as usize;
    odd_mem[rel..rel + PATCH_ODD_INIT_STATUS.len()].copy_from_slice(&PATCH_ODD_INIT_STATUS);
    odd_vm.map_segment(br_va, odd_mem, false);
    odd_vm.d[0] = 0xDEAD_BEEF;
    odd_vm.pc = br_va;
    odd_vm
        .run_until(init_va + PATCH_ODD_INIT_STATUS.len() as u32, 16)
        .unwrap();
    assert_eq!(odd_vm.d[0], 0);

    // 3. BGO unconditional branches: PATCH_SUM_BRA (+0x0A) -> PATCH_DEVINFO_BRA (+0x15 -> 0x1F).
    let mut bgo_vm = Mn103Vm::new();
    bgo_vm.map_segment(PROG061_BASE_VA, vec![0xCB; 0x40], false);
    bgo_vm.segments[0].data[0..2].copy_from_slice(&PATCH_SUM_BRA);
    bgo_vm.segments[0].data[0x0A..0x0C].copy_from_slice(&PATCH_DEVINFO_BRA);
    bgo_vm.pc = PROG061_BASE_VA;
    bgo_vm.run_until(PROG061_BASE_VA + 0x1F, 16).unwrap();
    assert_eq!(bgo_vm.steps, 2);
}

#[test]
fn test_call_ret_setlb_and_inc_an_preserve_flags() {
    // Verify CALL16 / RET, SETLB..LNE countdown, and INC4 An not clobbering PSW flags.
    let code_base = 0x4400_0000u32;
    let mut code = vec![0xCB; 0x60];
    // 0x00: CALL16 +0x20, regs=0x80 (d2), frame=0x08 -> target 0x20, ret 0x05
    code[0x00] = 0xCD;
    code[0x01..0x03].copy_from_slice(&(0x0020i16).to_le_bytes());
    code[0x03] = 0x80;
    code[0x04] = 0x08;
    // 0x20 (callee): MOV 3, D0; SETLB; DEC D0 via ADD -1,D0; INC4 A0; LNE
    code[0x20..0x22].copy_from_slice(&[0x80, 0x03]); // mov 3, d0
    code[0x22] = 0xDB; // setlb -> LAR = 0x27, loop body = 0x23
    code[0x23..0x25].copy_from_slice(&[0x28, 0xFF]); // add -1, d0 (updates Z)
    code[0x25] = 0x50; // inc4 a0 (must NOT overwrite PSW flags!)
    code[0x26] = 0xCB; // nop (fills 4 bytes after setlb)
    code[0x27] = 0xD9; // lne -> if !Z jump LAR-4 = 0x23
    // 0x28: RET 0x80, 0x08
    code[0x28..0x2B].copy_from_slice(&[0xDF, 0x80, 0x08]);

    let mut vm = Mn103Vm::new();
    vm.map_segment(code_base, code, false);
    vm.map_segment(0x4401_0000, vec![0; 0x200], true);
    vm.sp = 0x4401_0100;
    vm.d[2] = 0x1234_5678;
    vm.a[0] = 0x1000;
    vm.pc = code_base;
    vm.run_until(code_base + 0x05, 64).unwrap();
    assert_eq!(vm.d[0], 0);
    assert_eq!(vm.a[0], 0x100C);
    assert_eq!(vm.d[2], 0x1234_5678);
    assert_eq!(vm.sp, 0x4401_0100);
}

#[test]
fn test_real_prog061_bgo_init_fastpath_and_fail_path_execution() {
    // Real PROG061 bytes at VA 0x5040A294..0x5040A316 (file offset 0xA294..0xA316, 130 bytes).
    #[rustfmt::skip]
    const PROG061_A294_WINDOW: &[u8] = &[
        0xcf, 0x10, 0x3f, 0xf8, 0xfe, 0xe8, 0xfc, 0xa4, 0xb0, 0x02, 0x48, 0x50, 0xa0, 0x01, 0xc8, 0x6c,
        0xfc, 0xcc, 0x00, 0x00, 0x48, 0x50, 0xfc, 0xcd, 0x02, 0x00, 0x48, 0x50, 0xdd, 0xe2, 0xa2, 0xff,
        0xff, 0x00, 0x00, 0x00, 0x42, 0x0c, 0x00, 0x04, 0xdd, 0xd2, 0x81, 0xff, 0xff, 0x00, 0x00, 0xf8,
        0x13, 0xfc, 0xf8, 0x03, 0xfc, 0xa0, 0x00, 0xc8, 0x16, 0xfc, 0xcc, 0xb0, 0x3e, 0x43, 0x50, 0xf8,
        0x07, 0xfc, 0xdd, 0xf7, 0xaa, 0xff, 0xff, 0x00, 0x00, 0x80, 0xff, 0xca, 0x32, 0xdd, 0xba, 0xa0,
        0xff, 0xff, 0x00, 0x00, 0xf8, 0x13, 0xf8, 0xf8, 0x03, 0xf8, 0xa0, 0x00, 0xc8, 0x16, 0xfc, 0xcc,
        0xcd, 0x3e, 0x43, 0x50, 0xf8, 0x07, 0xf8, 0xdd, 0xd2, 0xaa, 0xff, 0xff, 0x00, 0x00, 0x80, 0xff,
        0xca, 0x0d, 0x80, 0x01, 0xfc, 0x81, 0xb0, 0x02, 0x48, 0x50, 0x00, 0xca, 0x02, 0xf2, 0xfc, 0xdf,
        0x10, 0x04,
    ];
    let mut slice = PROG061_A294_WINDOW.to_vec();
    let real_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
    if let Ok(disk_bytes) = std::fs::read(&real_path) {
        let disk_slice = &disk_bytes[0xA294..0xA316];
        assert_eq!(disk_slice, PROG061_A294_WINDOW);
        slice = disk_slice.to_vec();
    }

    // Case A: fast-path `*(0x504802B0) == 1` executes cleanly to caller return with D0=0.
    let mut fast = Mn103Vm::new();
    fast.map_segment(0x5040_A294, slice.clone(), false);
    fast.map_segment(0x5048_0000, vec![0; 0x400], true);
    fast.map_segment(0x504F_0000, vec![0; 0x400], true);
    fast.write_u32(0x5048_02B0, 1).unwrap();
    fast.sp = 0x504F_0200;
    fast.write_u32(fast.sp, 0x5040_BEEF).unwrap();
    fast.a[3] = 0xCAFE_BABE;
    fast.pc = 0x5040_A294;
    fast.run_until(0x5040_BEEF, 32).unwrap();
    assert_eq!(fast.d[0], 0);
    assert_eq!(fast.a[3], 0xCAFE_BABE);
    assert_eq!(fast.sp, 0x504F_0200);

    // Case B: uninitialized `[0x504802B0] == 0` walks through MOVM/ADD SP/MOV imm32/CMP/MOV imm32
    // and halts cleanly at the unmapped callee `0x50404592` instead of `InvalidOpcode(0xCF)`.
    let mut cold = Mn103Vm::new();
    cold.map_segment(0x5040_A294, slice, false);
    cold.map_segment(0x5048_0000, vec![0; 0x400], true);
    cold.map_segment(0x504F_0000, vec![0; 0x400], true);
    cold.sp = 0x504F_0200;
    cold.write_u32(cold.sp, 0x5040_BEEF).unwrap();
    cold.pc = 0x5040_A294;
    let err = cold.run_until(0x5040_BEEF, 32).unwrap_err();
    assert_eq!(err, Mn103Fault::UnmappedRead(0x5040_4592));
    assert_eq!(cold.d[0], 0x5048_0000);
    assert_eq!(cold.d[1], 0x5048_0002);
    assert_eq!(cold.sp, 0x504F_01E4);
}

#[test]
fn test_movm_f8_and_fc_subset_roundtrip() {
    // Exercise CF/CE MOVM + F8 0x/1x/4x/5x/6x/7x/9x/Bx/Cx/Ex + FC 8x/Ax/FE.
    let mut vm = Mn103Vm::new();
    #[rustfmt::skip]
    let code = vec![
        0xCF, 0x90,             // movm [d2, a3], (sp) -> sp -= 8
        0xFC, 0xFE, 0xF0, 0xFF, 0xFF, 0xFF, // add -16, sp -> sp -= 16
        0x3F,                   // mov sp, a3 -> a3 = sp
        0xFC, 0xCC, 0x78, 0x56, 0x34, 0x12, // mov 0x12345678, d0
        0xF8, 0x13, 0x04,       // mov d0, (4, a3)
        0xF8, 0x07, 0x04,       // mov (4, a3), d1
        0xF8, 0x53, 0x08,       // movbu d0, (8, a3) -> 0x78
        0xF8, 0x4B, 0x08,       // movbu (8, a3), d2 -> d2 = 0x78
        0xF8, 0x9E, 0x0C,       // movbu d3, (0x0c, sp)
        0xF8, 0xBA, 0x0C,       // movbu (0x0c, sp), d2 -> d2 = d3&ff
        0xFC, 0x81, 0x00, 0x02, 0x01, 0x50, // mov d0, (0x50010200)
        0xFC, 0xA5, 0x00, 0x02, 0x01, 0x50, // mov (0x50010200), d1
        0xF8, 0xC4, 0x04,       // lsr 4, d0 -> 0x01234567
        0xF8, 0xEC, 0x0F,       // btst 0x0f, d0 -> non-zero
        0xF8, 0xE8, 0x04,       // bvc +4 -> skip clr d0 (since V=0 after btst!)
        0x00,                   // clr d0 (should be skipped)
        0xF8, 0xFE, 0x10,       // add 16, sp
        0xCE, 0x90,             // movm (sp), [d2, a3] -> sp += 8
        0xCB,                   // nop (stop)
    ];
    let stop_pc = 0x5000_0000 + code.len() as u32 - 1;
    vm.map_segment(0x5000_0000, code, false);
    vm.map_segment(0x5001_0000, vec![0; 0x300], true);
    vm.sp = 0x5001_0100;
    vm.d[2] = 0xAABB_CCDD;
    vm.d[3] = 0x0000_005A;
    vm.a[3] = 0x1122_3344;
    vm.pc = 0x5000_0000;
    vm.run_until(stop_pc, 32).unwrap();
    assert_eq!(vm.d[0], 0x0123_4567);
    assert_eq!(vm.d[1], 0x1234_5678);
    assert_eq!(vm.d[2], 0xAABB_CCDD);
    assert_eq!(vm.a[3], 0x1122_3344);
    assert_eq!(vm.sp, 0x5001_0100);
    assert_eq!(vm.read_u32(0x5001_0200).unwrap(), 0x1234_5678);
}

#[test]
fn test_bitops_cfi_autoselect_and_ipl_write_protection() {
    // Exercise 0xFE/0xFA/0xF0 BSET/BCLR/BTST, EXTHU, MOVHU, Spansion CFI autoselect, and IPL air-gap.
    let mut vm = Mn103Vm::new();
    #[rustfmt::skip]
    let code = vec![
        // TEST 1: FE 80/81/82 -> BSET/BCLR/BTST imm8, (imm16) at 0x0D78 (like S211 PASS unlock!)
        0xFE, 0x80, 0x78, 0x0D, 0x01, // bset 0x01, (0x0d78)
        0xFE, 0x80, 0x78, 0x0D, 0x08, // bset 0x08, (0x0d78) -> 0x09
        0xFE, 0x81, 0x78, 0x0D, 0x01, // bclr 0x01, (0x0d78) -> 0x08
        // TEST 2: F0 90 BCLR D0,(A0) + FA F0 BSET imm8,(d8,A1)
        0x94, 0x00,                   // mov 0, a1 (placeholder; set below)
        0xFC, 0xDD, 0x00, 0x10, 0x00, 0x00, // mov 0x00001000, a1
        0xFA, 0xF1, 0x04, 0xA5,       // bset 0xa5, (4, a1) -> [0x1004] |= 0xA5
        // TEST 3: EXTHU D0
        0xFC, 0xCC, 0xEF, 0xBE, 0xAD, 0xDE, // mov 0xDEADBEEF, d0
        0x1C,                               // exthu d0 -> 0x0000BEEF
        0xCB,                               // nop (stop)
    ];
    let stop_pc = 0x5000_0000 + code.len() as u32 - 1;
    vm.map_segment(0x5000_0000, code, false);
    vm.map_segment(0x0000_0000, vec![0; 0x2000], true);
    vm.map_segment(0x8400_0000, vec![0xEA; 0x80_0000], false);
    vm.pc = 0x5000_0000;
    vm.run_until(stop_pc, 32).unwrap();
    assert_eq!(vm.read_u8(0x0D78).unwrap(), 0x08);
    assert_eq!(vm.read_u8(0x1004).unwrap(), 0xA5);
    assert_eq!(vm.d[0], 0x0000_BEEF);

    // Verify Spansion CFI autoselect (0x84200554 / 0x84200AAA -> 0x0001 / 0x227E)
    vm.write_u8(0x8420_0554, 0xAA).unwrap();
    vm.write_u8(0x8420_0AAA, 0x55).unwrap();
    vm.write_u8(0x8420_0554, 0x90).unwrap();
    assert_eq!(vm.cfi_stage, CfiStage::Autoselect);
    assert_eq!(vm.read_u16(0x8420_0000).unwrap(), 0x0001);
    assert_eq!(vm.read_u16(0x8420_0002).unwrap(), 0x227E);
    vm.write_u8(0x8420_0000, 0xF0).unwrap();
    assert_eq!(vm.cfi_stage, CfiStage::Normal);
    assert_eq!(vm.read_u8(0x8420_0000).unwrap(), 0xEA);

    // Normal writes to IPL NOR fail closed with IplWriteProtected
    assert!(matches!(
        vm.write_u32(0x8400_0000, 0x1234_5678),
        Err(Mn103Fault::IplWriteProtected(0x8400_0000))
    ));
}
