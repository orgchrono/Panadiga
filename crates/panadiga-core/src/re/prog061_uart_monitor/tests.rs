use super::*;
use crate::re::mn10300_emu::Mn103Vm;

#[test]
fn uart_monitor_payload_assembles_cleanly_within_slack() {
    let payload = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA).unwrap();
    assert!(payload.len() <= 2048);
    assert!(payload.len() <= UART_MONITOR_MAX_SLACK_BYTES);
    assert_eq!(&payload[..2], &[0xCF, 0xF0]);
    assert!(payload.windows(16).any(|w| w == b"PANAMON1>[B/R/W/"));
    assert!(payload.windows(8).any(|w| w == b"PANADMP1"));
    assert!(payload.windows(8).any(|w| w == b"PANADONE"));
    let rel = i32::from_le_bytes(PATCH_A2CD_UART_MONITOR[1..5].try_into().unwrap());
    assert_eq!(
        (0x5040_A2CDi64 + rel as i64) as u32,
        PROG061_UART_MONITOR_VA
    );
}

fn make_panamon1_call_ram(call_va: u32) -> Vec<u8> {
    let mut ram = vec![0u8; 0x1_0000];
    let rel32 = (PROG061_UART_MONITOR_VA as i64 - call_va as i64) as i32;
    ram[0] = 0xDD;
    ram[1..5].copy_from_slice(&rel32.to_le_bytes());
    ram
}

#[test]
fn panamon1_executes_guarded_rw_jump_erase_and_blocks_ipl_on_mn103_vm() {
    let payload = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA).unwrap();
    let mut vm = Mn103Vm::new();
    // Authorize Upper NOR hardware writes on the VM so PANAMON1's own software U-lock
    // (`U LOCK` -> `ERR:E05_IPL_RO` vs `U CONFIRM_UPPER_NOR` -> `OK`) and permanent SA0..SA3 lock are tested.
    vm.nor_write_policy = crate::pico::rsp_codec::policy::NorWritePolicy::UpperNorAppUnlocked;
    vm.enable_sc0_uart(&[]);

    vm.map_segment(PROG061_UART_MONITOR_VA, payload, false);
    let call_va = 0x5000_0000u32;
    let mut ram = make_panamon1_call_ram(call_va);
    ram[5] = 0x00;
    ram[6] = 0x00;
    ram[0x1800..0x180C].copy_from_slice(&[
        0xFC, 0xDE, 0x00, 0x01, 0x00, 0x50, // mov 0x50000100, a2
        0x8A, 0x77, // mov 0x77, d2
        0xF0, 0x5A, // movb d2, (a2)
        0xF0, 0xFC, // rets
    ]);
    vm.map_segment(0x5000_0000, ram, true);
    vm.map_segment(0x8000_0000, vec![0xDE, 0xAD, 0xBE, 0xEF], false);
    vm.map_segment(0x8400_0000, vec![0xCA, 0xFE, 0xBA, 0xBE], false);
    vm.map_segment(0x8420_0000, vec![0x5Au8; 0x1_0000], true);
    vm.map_segment(0x847F_0000, vec![0x5Au8; 0x1_0000], true);
    vm.sp = 0x5000_F000;
    vm.pc = call_va;

    vm.queue_uart_rx(
        b"R 80000000,04\r\n\
          F\r\n\
          W 80000000,AA\r\n\
          W 847EFFFF,AA\r\n\
          E 847E0000\r\n\
          E 847F0000\r\n\
          W 847F0010,A5\r\n\
          R 847F0010,04\r\n\
          U WRONG\r\n\
          W 84200010,12\r\n\
          U CONFIRM_UPPER_NOR\r\n\
          F\r\n\
          W 84000000,AA\r\n\
          E 84030000\r\n\
          E 84200000\r\n\
          W 84200010,A5\r\n\
          W 84200011,5A\r\n\
          U LOCK\r\n\
          W 84200012,11\r\n\
          W 50000200,42\r\n\
          J 50001800\r\n\
          E 847FE000\r\n\
          B",
    );

    if let Err(e) = vm.run_until(call_va + 7, 500_000) {
        let tx = vm.take_uart_tx_utf8();
        panic!(
            "PANAMON1 VM err={e:?} pc={:#010x} sp={:#010x} a0={:#010x} a2={:#010x} d0={:#010x} d1={:#010x} d2={:#010x} tx={tx:?}",
            vm.pc, vm.sp, vm.a[0], vm.a[2], vm.d[0], vm.d[1], vm.d[2]
        );
    }
    assert_eq!(vm.d[0], 0);

    let tx = vm.take_uart_tx_utf8();
    assert!(tx.contains("PANAMON1>[B/R/W/D/F/J/E/U/Q]"), "tx={tx}");
    assert!(tx.contains("DEADBEEF"), "expected IPL read hex: {tx}");
    assert_eq!(tx.matches("ERR:E05_IPL_RO").count(), 9, "tx={tx}");
    assert!(
        !tx.contains("OK:FORMAT"),
        "F command must be disabled fail-closed: {tx}"
    );
    assert!(tx.contains("?\r\n"), "tx={tx}");
    assert!(tx.contains("OK:NOR_UNLOCK"), "tx={tx}");
    assert!(tx.contains("OK:NOR_LOCK"), "tx={tx}");
    assert!(tx.contains("OK:WRITE"), "tx={tx}");
    assert!(tx.contains("A5FFFFFF"), "tx={tx}");
    assert!(tx.contains("OK:DONE"), "tx={tx}");
    assert!(tx.contains("OK:ERASE"), "tx={tx}");
    assert!(tx.contains("OK:BYPASS"), "tx={tx}");

    assert_eq!(vm.read_u8(0x8000_0000).unwrap(), 0xDE);
    assert_eq!(vm.read_u8(0x5000_0100).unwrap(), 0x77);
    assert_eq!(vm.read_u8(0x5000_0200).unwrap(), 0x42);
    assert_eq!(vm.read_u8(0x8420_0000).unwrap(), 0xFF);
    assert_eq!(vm.read_u8(0x8420_0010).unwrap(), 0xA5);
    assert_eq!(vm.read_u8(0x8420_0011).unwrap(), 0x5A);
    assert_eq!(vm.read_u8(0x8420_FFFF).unwrap(), 0xFF);
    assert_eq!(vm.read_u8(0x847F_0010).unwrap(), 0xA5);
    assert_eq!(vm.read_u8(0x847F_DFFFu32).unwrap(), 0x5A);
    assert_eq!(vm.read_u8(0x847F_E000u32).unwrap(), 0xFF);
    assert_eq!(vm.read_u8(0x847F_FFFFu32).unwrap(), 0xFF);
}

#[test]
fn panamon1_matches_compiler_call_zero_imm_and_preserves_callee_regs_and_sp() {
    let payload = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA).unwrap();
    // Prologue/epilogue match real PROG061 `0x50415CAD` (`MOVM [D2..A3]; ADD -16,SP ... RET [D2..A3],32`).
    assert_eq!(
        &payload[..8],
        &[0xCF, 0xF0, 0xF8, 0xFE, 0xF0, 0x00, 0x42, 0x0C]
    );
    // Every `PUTC_D3_BYTES` copy must be embedded with bounded timeout + `SC0STR & 0xA0`.
    assert!(
        payload
            .windows(crate::re::nor_dumper::PUTC_D3_BYTES.len())
            .any(|w| w == crate::re::nor_dumper::PUTC_D3_BYTES)
    );
    // Must never emit legacy non-compiler `CALL16 ..., [], 0x04` or `RET [], 0x04` patterns in code region.
    let code_end = payload
        .windows(16)
        .position(|w| w == b"PANAMON1>[B/R/W/")
        .expect("prompt string present");
    assert!(
        !payload[..code_end]
            .windows(3)
            .any(|w| w == [0xDF, 0x00, 0x04]),
        "legacy `RET [], 4` must not appear in PANAMON1 code"
    );

    // Execute starting at `0x5040A2BC` (`PATCH_A2BC_BRA_MONITOR`) with `D0 == 0` (simulating bypassed BGO)
    // and verify unconditional `bra +17` skips `BEQ 0x5040A2E1`, runs `PATCH_A2CD_UART_MONITOR`, and preserves regs/SP.
    let mut vm = Mn103Vm::new();
    vm.enable_sc0_uart(b"Q");
    vm.map_segment(PROG061_UART_MONITOR_VA, payload, false);
    let mut hook_page = vec![0xCB; 0x100];
    hook_page[0xBC..0xC3].copy_from_slice(&crate::re::PATCH_A2BC_BRA_MONITOR);
    // Real `0x5040A2C3..0x5040A2CC` conditional bypass guard: if 0xA2BC fell through with D0=0, BEQ would skip 0xA2CD!
    hook_page[0xC3..0xCD]
        .copy_from_slice(&[0xF8, 0x13, 0xFC, 0xF8, 0x03, 0xFC, 0xA0, 0x00, 0xC8, 0x16]);
    hook_page[0xCD..0xE1].copy_from_slice(&PATCH_A2CD_UART_MONITOR);
    vm.map_segment(0x5040_A200, hook_page, false);
    vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
    vm.sp = 0x50FF_F800;
    vm.a[3] = 0x50FF_F900;
    vm.d[0] = 0;
    vm.d[2] = 0x1122_3344;
    vm.d[3] = 0x5566_7788;
    vm.a[2] = 0x99AA_BBCC;
    vm.pc = 0x5040_A2BC;

    // `Q` returns -1, which PATCH_A2CD_UART_MONITOR dispatches via `MOV -1, D0; BRA 0x5040A311`
    // (stop at 0x5040A2DE before the branch leaves hook_page).
    vm.run_until(0x5040_A2DE, 50_000).unwrap();
    assert_eq!(vm.d[0], 0xFFFF_FFFF);
    assert_eq!(vm.sp, 0x50FF_F800);
    assert_eq!(vm.d[2], 0x1122_3344);
    assert_eq!(vm.d[3], 0x5566_7788);
    assert_eq!(vm.a[2], 0x99AA_BBCC);
    assert_eq!(vm.a[3], 0x50FF_F900);
    let tx = vm.take_uart_tx_utf8();
    assert!(
        tx.starts_with("PANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n"),
        "tx={tx:?}"
    );
    assert!(tx.contains("OK:ABORT\r\n"), "tx={tx:?}");
}

#[test]
fn computes_exact_uncached_sdram_nor_dumper_patch_offsets() {
    let (payload, offsets) =
        build_prog061_uart_monitor_with_offsets(PROG061_UART_MONITOR_VA).unwrap();
    let (entry_u, start_u, end_u) = offsets.uncached_patch_vas(PROG061_UART_MONITOR_VA);
    assert_eq!(entry_u, 0x9043_94FA);
    assert_eq!(start_u, 0x9043_9510);
    assert_eq!(end_u, 0x9043_9516);
    assert_eq!(offsets.block_size, 4096);
    assert_eq!(
        &payload[offsets.start_imm_off - 2..offsets.start_imm_off],
        &[0xFC, 0xDC]
    );
    assert_eq!(
        &payload[offsets.end_imm_off - 2..offsets.end_imm_off],
        &[0xFC, 0xDD]
    );
}

#[test]
fn panamon1_honors_multicycle_dq7_busy_and_rejects_dq5_timeout_without_false_ok() {
    let payload = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA).unwrap();

    // Case 1: Multi-cycle Spansion busy polling succeeds cleanly after 5 polls
    let mut vm_ok = Mn103Vm::new();
    vm_ok.nor_write_policy = crate::pico::rsp_codec::policy::NorWritePolicy::UpperNorAppUnlocked;
    vm_ok.nor_busy_cycles_config = 5;
    vm_ok.enable_sc0_uart(b"E 847F0000\r\nW 847F0010,55\r\nB");
    vm_ok.map_segment(PROG061_UART_MONITOR_VA, payload.clone(), false);
    vm_ok.map_segment(0x8400_0000, vec![0u8; 0x1000], false);
    vm_ok.map_segment(0x847F_0000, vec![0x12u8; 0x1_0000], true);
    let call_va = 0x5000_0000u32;
    let ram = make_panamon1_call_ram(call_va);
    vm_ok.map_segment(0x5000_0000, ram.clone(), true);
    vm_ok.sp = 0x5000_F000;
    vm_ok.pc = call_va;
    vm_ok.run_until(call_va + 7, 200_000).unwrap();
    let tx_ok = vm_ok.take_uart_tx_utf8();
    assert!(tx_ok.contains("OK:ERASE"), "tx={tx_ok}");
    assert!(tx_ok.contains("OK:WRITE"), "tx={tx_ok}");
    assert!(!tx_ok.contains("ERR:ERASE"), "tx={tx_ok}");
    assert!(!tx_ok.contains("ERR:WRITE"), "tx={tx_ok}");
    assert_eq!(vm_ok.read_u8(0x847F_0010).unwrap(), 0x55);

    // Case 2: DQ5 exceeded-timing fault MUST emit ERR:ERASE / ERR:WRITE and NEVER false OK
    let mut vm_dq5 = Mn103Vm::new();
    vm_dq5.nor_write_policy = crate::pico::rsp_codec::policy::NorWritePolicy::UpperNorAppUnlocked;
    vm_dq5.nor_force_dq5_timeout = true;
    vm_dq5.enable_sc0_uart(b"E 847F0000\r\nW 847F0010,55\r\nB");
    vm_dq5.map_segment(PROG061_UART_MONITOR_VA, payload, false);
    vm_dq5.map_segment(0x8400_0000, vec![0u8; 0x1000], false);
    vm_dq5.map_segment(0x847F_0000, vec![0x12u8; 0x1_0000], true);
    vm_dq5.map_segment(0x5000_0000, ram, true);
    vm_dq5.sp = 0x5000_F000;
    vm_dq5.pc = call_va;
    vm_dq5.run_until(call_va + 7, 200_000).unwrap();
    let tx_dq5 = vm_dq5.take_uart_tx_utf8();
    assert!(tx_dq5.contains("ERR:ERASE"), "tx={tx_dq5}");
    assert!(tx_dq5.contains("ERR:WRITE"), "tx={tx_dq5}");
    assert!(!tx_dq5.contains("OK:ERASE"), "tx={tx_dq5}");
    assert!(!tx_dq5.contains("OK:WRITE"), "tx={tx_dq5}");
}
