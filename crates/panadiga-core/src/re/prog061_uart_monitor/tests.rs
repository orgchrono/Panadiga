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

#[test]
fn panamon1_executes_guarded_rw_jump_erase_and_blocks_ipl_on_mn103_vm() {
    let payload = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA).unwrap();
    let mut vm = Mn103Vm::new();
    vm.enable_sc0_uart(&[]);

    vm.map_segment(PROG061_UART_MONITOR_VA, payload, false);
    let mut ram = vec![0u8; 0x1_0000];
    let call_va = 0x5000_0000u32;
    let rel32 = (PROG061_UART_MONITOR_VA as i64 - call_va as i64) as i32;
    ram[0] = 0xDD;
    ram[1..5].copy_from_slice(&rel32.to_le_bytes());
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
    vm.map_segment(0x847F_0000, vec![0x5Au8; 0x1_0000], true);
    vm.sp = 0x5000_F000;
    vm.pc = call_va;

    vm.queue_uart_rx(
        b"R 80000000,04\r\n\
          W 80000000,AA\r\n\
          W 847EFFFF,AA\r\n\
          E 847E0000\r\n\
          W 847F0010,A5\r\n\
          R 847F0010,04\r\n\
          J 50001800\r\n\
          E 847FE000\r\n\
          B",
    );

    if let Err(e) = vm.run_until(call_va + 7, 250_000) {
        let tx = vm.take_uart_tx_utf8();
        panic!(
            "PANAMON1 VM err={e:?} pc={:#010x} sp={:#010x} a0={:#010x} a2={:#010x} d0={:#010x} d1={:#010x} d2={:#010x} tx={tx:?}",
            vm.pc, vm.sp, vm.a[0], vm.a[2], vm.d[0], vm.d[1], vm.d[2]
        );
    }
    assert_eq!(vm.d[0], 0);

    let tx = vm.take_uart_tx_utf8();
    assert!(tx.contains("PANAMON1>[B/R/W/D/F/J/E/Q]"), "tx={tx}");
    assert!(tx.contains("DEADBEEF"), "expected IPL read hex: {tx}");
    assert_eq!(tx.matches("ERR:E05_IPL_RO").count(), 3, "tx={tx}");
    assert!(tx.contains("OK:WRITE"), "tx={tx}");
    assert!(tx.contains("A55A5A5A"), "tx={tx}");
    assert!(tx.contains("OK:DONE"), "tx={tx}");
    assert!(tx.contains("OK:ERASE"), "tx={tx}");
    assert!(tx.contains("OK:BYPASS"), "tx={tx}");

    assert_eq!(vm.read_u8(0x8000_0000).unwrap(), 0xDE);
    assert_eq!(vm.read_u8(0x5000_0100).unwrap(), 0x77);
    assert_eq!(vm.read_u8(0x847F_0010).unwrap(), 0xA5);
    assert_eq!(vm.read_u8(0x847F_DFFFu32).unwrap(), 0x5A);
    assert_eq!(vm.read_u8(0x847F_E000u32).unwrap(), 0xFF);
    assert_eq!(vm.read_u8(0x847F_FFFFu32).unwrap(), 0xFF);
}
