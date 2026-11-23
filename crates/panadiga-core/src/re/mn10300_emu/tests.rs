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
    assert_eq!(vm.sp, 0x5000_1000);

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
    // Benign 16-bit multi-vendor Read-ID/Reset probes at Upper-NOR bank bases are ignored; critical IPL stays E05
    vm.write_u16(0x8420_0000, 0x0090).unwrap();
    vm.write_u16(0x8420_0000, 0xFFFF).unwrap();
    assert!(matches!(
        vm.write_u16(0x8400_0000, 0x0090),
        Err(Mn103Fault::IplWriteProtected(0x8400_0000))
    ));
}

fn run_jedec_erase(vm: &mut Mn103Vm, base: u32, sector_addr: u32) -> Result<(), Mn103Fault> {
    vm.write_u16(base + 0x0AAA, 0x00AA)?;
    vm.write_u16(base + 0x0554, 0x0055)?;
    vm.write_u16(base + 0x0AAA, 0x0080)?;
    vm.write_u16(base + 0x0AAA, 0x00AA)?;
    vm.write_u16(base + 0x0554, 0x0055)?;
    vm.write_u16(sector_addr, 0x0030)
}

fn run_jedec_program_u8(vm: &mut Mn103Vm, base: u32, addr: u32, val: u8) -> Result<(), Mn103Fault> {
    vm.write_u8(base + 0x0AAA, 0xAA)?;
    vm.write_u8(base + 0x0554, 0x55)?;
    vm.write_u8(base + 0x0AAA, 0xA0)?;
    vm.write_u8(addr, val)
}

#[test]
fn test_spansion_jedec_erase_program_and_two_layer_policy() {
    let mut vm = Mn103Vm::new();
    vm.map_segment(0x8400_0000, vec![0x5Au8; 0x80_0000], true);
    assert_eq!(vm.nor_write_policy, NorWritePolicy::NvramOnly);
    assert!(is_critical_ipl_boot_va(0x8400_0000));
    assert!(is_critical_ipl_boot_va(0x8003_FFFF));
    assert!(!is_critical_ipl_boot_va(0x8404_0000));

    // Default NvramOnly: blocks SA4..SA133 erase/program, allows 8 KiB NVRAM SA134..SA141 + NAND-like &=
    assert!(matches!(
        run_jedec_erase(&mut vm, 0x8400_0000, 0x8420_0000),
        Err(Mn103Fault::IplWriteProtected(0x8420_0000))
    ));
    assert!(matches!(
        run_jedec_program_u8(&mut vm, 0x8400_0000, 0x8404_0010, 0x12),
        Err(Mn103Fault::IplWriteProtected(0x8404_0010))
    ));
    run_jedec_program_u8(&mut vm, 0x8400_0000, 0x847F_0010, 0xA5).unwrap();
    assert_eq!(vm.read_u8(0x847F_0010).unwrap(), 0x5A & 0xA5); // 1->0 physical NOR rule without erase
    run_jedec_erase(&mut vm, 0x8400_0000, 0x847F_2000).unwrap();
    assert_eq!(vm.read_u8(0x847F_1FFF).unwrap(), 0x5A);
    assert_eq!(vm.read_u8(0x847F_2000).unwrap(), 0xFF);
    run_jedec_program_u8(&mut vm, 0x8400_0000, 0x847F_2004, 0xF0).unwrap(); // 0xF0 data during ProgramPending
    assert_eq!(vm.read_u8(0x847F_2004).unwrap(), 0xF0);

    // Explicit unlock: SA4..SA133 64 KiB erase + program works; SA0..SA3 permanently E05 without mutating IPL
    vm.nor_write_policy = NorWritePolicy::UpperNorAppUnlocked;
    assert!(matches!(
        run_jedec_erase(&mut vm, 0x8400_0000, 0x8403_FFFE),
        Err(Mn103Fault::IplWriteProtected(0x8403_FFFE))
    ));
    assert!(matches!(
        run_jedec_program_u8(&mut vm, 0x8400_0000, 0x8400_0100, 0x00),
        Err(Mn103Fault::IplWriteProtected(0x8400_0100))
    ));
    assert_eq!(vm.read_u8(0x8400_0AAA).unwrap(), 0x5A); // unlock cycles didn't clobber SA0
    run_jedec_erase(&mut vm, 0x8400_0000, 0x8420_1234).unwrap();
    assert_eq!(vm.read_u8(0x841F_FFFF).unwrap(), 0x5A);
    assert_eq!(vm.read_u8(0x8420_0000).unwrap(), 0xFF);
    assert_eq!(vm.read_u8(0x8420_FFFF).unwrap(), 0xFF);
    assert_eq!(vm.read_u8(0x8421_0000).unwrap(), 0x5A);
    run_jedec_program_u8(&mut vm, 0x8400_0000, 0x8420_0020, 0x3C).unwrap();
    assert_eq!(vm.read_u8(0x8420_0020).unwrap(), 0x3C);

    // Bottom 8x8 KiB sectors (SA0..SA7) + CFI Query (0x98) + Lethal 0xC000_100C fail-closed fault
    assert_eq!(spansion_nor::nor_sector_bounds(0x0020), (0x0000, 0x2000));
    assert_eq!(spansion_nor::nor_sector_bounds(0x2100), (0x2000, 0x2000));
    assert_eq!(
        spansion_nor::nor_sector_bounds(0x1_0100),
        (0x1_0000, 0x1_0000)
    );
    vm.write_u16(0x8400_00AA, 0x0098).unwrap();
    assert_eq!(vm.read_u16(0x8400_0020).unwrap(), 0x0051);
    assert_eq!(vm.read_u16(0x8400_0022).unwrap(), 0x0052);
    assert_eq!(vm.read_u16(0x8400_0024).unwrap(), 0x0059);
    assert_eq!(vm.read_u16(0x8400_004E).unwrap(), 0x0017);
    vm.write_u16(0x8400_0000, 0x00F0).unwrap();
    assert_eq!(
        vm.read_u32(crate::constants::AM34_FORBIDDEN_LETHAL_REG),
        Err(Mn103Fault::LethalRegisterRead(
            crate::constants::AM34_FORBIDDEN_LETHAL_REG
        ))
    );
}

#[test]
fn test_am33_extended_registers_movm_and_prefixes() {
    let mut vm = Mn103Vm::new();
    let code_base = 0x5000_0000u32;
    #[rustfmt::skip]
    let code = vec![
        // Save all AM33 extended groups EXREG0(0x04) + EXREG1(0x02) + EXOTHER(0x01) = 48 bytes
        0xCF, 0x07,                         // movm [exreg0, exreg1, exother], (sp)
        // F5: MOV A1, R0 (0x10), MOV D2, R5 (0x65)
        0xF5, 0x10,                         // mov a1, r0
        0xF5, 0x65,                         // mov d2, r5
        // FB: MOV simm8 (-5=0xFB), R2 (0x08 0x02 0xFB); MOVU 0x90, R3 (0x18 0x03 0x90)
        0xFB, 0x08, 0x02, 0xFB,             // mov -5, r2 -> 0xFFFFFFFB
        0xFB, 0x18, 0x03, 0x90,             // movu 0x90, r3 -> 0x00000090
        // FB 9C: SUB Rm2(D0=C), Rn0(R3=3), Rd2(A2=A) -> A2 = R3 - D0 = 0x90 - 0x10 = 0x80
        0xFB, 0x9C, 0xC3, 0xA0,             // sub d0, r3, a2
        // FB 9A / AA / CA with unsigned u8 stack offset d8 = 0x84 (> 127!)
        0xFB, 0x9A, 0xA0, 0x84,             // mov a2, (0x84, sp)
        0xFB, 0xAA, 0x40, 0x84,             // movbu (0x84, sp), r4 -> 0x80
        // FB indexed memory: MOV R4, (R2, R3)? Use A1=0x5002_0000 (9), R5=4 (5) -> write/read
        0xF9, 0x08, 0x34,                   // mov r3, r4 (r4 = 0x90)
        0xFB, 0x78, 0x44, 0x05,             // add 5, r4 -> 0x95
        0xFB, 0x9E, 0x95, 0x40,             // mov r4, (a1, r5) -> [0x50020004] = 0x95
        0xFB, 0xCE, 0x95, 0x60,             // movhu (a1, r5), r6 -> r6 = 0x95
        // FD imm24 + FE imm32
        0xFD, 0x08, 0x77, 0x34, 0x12, 0x80, // mov 0xFF801234, r7 (sext imm24)
        0xFE, 0x1E, 0x70, 0x10, 0x00, 0x02, 0x50, // mov r7, (0x50020010)
        // Base F3/F4 indexed (D0/D1, A1) & FA 16-bit
        0xF3, 0x01,                         // mov (d0, a1), d0 -> d0 = [0x50020010]
        0xF4, 0x05,                         // movbu (d1, a1), d0 -> d0 = [0x50020010] & 0xFF = 0x34
        0xFA, 0xC0, 0x10, 0x00,             // add 0x10, d0
        // F9 MULU / DIVU: 100 * 3 = 300, 300 / 7 -> q=42, mdr=6
        0xFB, 0x18, 0x11, 100,              // movu 100, r1
        0xFB, 0x18, 0x22, 3,                // movu 3, r2
        0xF9, 0xB9, 0x21,                   // mulu r2, r1 -> r1=300, mdr=0
        0xFB, 0x18, 0x33, 7,                // movu 7, r3
        0xF9, 0xD9, 0x31,                   // divu r3, r1 -> r1=42, mdr=6
        // Restore saved AM33 registers and verify sentinel intact
        0xCE, 0x07,                         // movm (sp), [exreg0, exreg1, exother]
        0xCB,                               // nop
    ];

    let stop_pc = code_base + code.len() as u32 - 1;
    vm.map_segment(code_base, code, false);
    vm.map_segment(0x5002_0000, vec![0; 0x1000], true);
    vm.r = [
        0x1111_0000,
        0x2222_0001,
        0x3333_0002,
        0x4444_0003,
        0x5555_0004,
        0x6666_0005,
        0x7777_0006,
        0x8888_0007,
    ];
    vm.mdrq = 0xAAAA_BBBB;
    vm.a[1] = 0x5002_0000;
    vm.d[0] = 0x10;
    vm.d[1] = 0x10;
    vm.d[2] = 4;
    vm.sp = 0x5002_0800;
    vm.pc = code_base;

    vm.run_until(stop_pc, 128).unwrap();

    assert_eq!(vm.sp, 0x5002_0800);
    assert_eq!(vm.r[0], 0x1111_0000);
    assert_eq!(vm.r[2], 0x3333_0002);
    assert_eq!(vm.r[7], 0x8888_0007);
    assert_eq!(vm.mdrq, 0xAAAA_BBBB);
    assert_eq!(vm.a[2], 0x80);
    assert_eq!(vm.read_u32(0x5002_0004).unwrap(), 0x95);
    assert_eq!(vm.read_u32(0x5002_0010).unwrap(), 0xFF80_1234);
    assert_eq!(vm.d[0], 0x44); // 0x34 + 0x10
    assert_eq!(vm.mdr, 6);
}

#[test]
fn test_calls_and_rets_preserve_sp_contract() {
    let mut vm = Mn103Vm::new();
    let code_base = 0x5000_0000u32;
    let mut code = vec![0xCB; 0x80];
    // +0x00: CALL +0x20, [], 0 -> leaf @ +0x20 ending in RETS; returns to +0x05 with SP unchanged
    code[0x00..0x05].copy_from_slice(&[0xCD, 0x20, 0x00, 0x00, 0x00]);
    // +0x05: MOV 0x50000040, A0; CALLS (A0); NOP (+0x0D)
    code[0x05..0x0B].copy_from_slice(&[0xFC, 0xDC, 0x40, 0x00, 0x00, 0x50]);
    code[0x0B..0x0D].copy_from_slice(&[0xF0, 0xF0]);
    code[0x0D] = 0xCB;
    // +0x20 (leaf): MOV 0x42, D0; RETS
    code[0x20..0x24].copy_from_slice(&[0x80, 0x42, 0xF0, 0xFC]);
    // +0x40 (framed): MOVM [d2], (SP); ADD -8, SP; MOV 0x77, D1; RET [d2], 12
    code[0x40..0x4A].copy_from_slice(&[0xCF, 0x80, 0xF8, 0xFE, 0xF8, 0x85, 0x77, 0xDF, 0x80, 0x0C]);

    vm.map_segment(code_base, code, false);
    vm.map_segment(0x5001_0000, vec![0; 0x200], true);
    vm.sp = 0x5001_0100;
    vm.d[2] = 0xDEAD_BEEF;
    vm.pc = code_base;

    vm.run_until(code_base + 0x0D, 64).unwrap();
    assert_eq!(vm.d[0], 0x42);
    assert_eq!(vm.d[1], 0x77);
    assert_eq!(vm.d[2], 0xDEAD_BEEF);
    assert_eq!(vm.sp, 0x5001_0100);
}
