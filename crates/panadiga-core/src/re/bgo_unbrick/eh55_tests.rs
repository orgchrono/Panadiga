use std::path::Path;

use crate::nvram::calculate_hashes;
use crate::re::mn10300_emu::Mn103Vm;

use super::{
    OFF_BGO_SUM_BEQ_EH55, ORIG_SUM_BEQ, PATCH_SUM_BRA, PROG061_BASE_VA, Prog061BgoPatchMode,
    Prog061BgoTargetGeometry, inspect_prog061_bgo_geometry, inspect_prog061_bgo_sites,
    patch_prog061_bgo_in_place_with_policy, retarget_prog061_bgo_geometry_in_place,
};

fn repo_file(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn call_mn103_fn(vm: &mut Mn103Vm, entry_va: u32, arg_d0: u32, max_steps: u64) -> u32 {
    let call_va = 0x504F_F800u32;
    let rel32 = (entry_va as i64 - call_va as i64) as i32;
    vm.write_u8(call_va, 0xDD).expect("write CALL32");
    for (idx, byte) in rel32.to_le_bytes().iter().enumerate() {
        vm.write_u8(call_va + 1 + idx as u32, *byte)
            .expect("write CALL32 disp");
    }
    vm.write_u8(call_va + 5, 0x00).expect("write CALL32 reg");
    vm.write_u8(call_va + 6, 0x00).expect("write CALL32 arg");
    vm.write_u8(call_va + 7, 0xCB).expect("write NOP sentinel");

    vm.d[0] = arg_d0;
    vm.sp = 0x50FF_FC00;
    vm.pc = call_va;
    vm.run_until(call_va + 7, max_steps)
        .expect("Mn103Vm execution failed");
    vm.d[0]
}

#[test]
fn test_eh55_retarget_is_idempotent_and_preserves_eh50() {
    let prog_path =
        repo_file("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
    if !prog_path.exists() {
        return;
    }
    let factory = std::fs::read(&prog_path).expect("read prog061_dec.bin");
    let (_, factory_sha) = calculate_hashes(&factory);

    assert_eq!(
        inspect_prog061_bgo_geometry(&factory).unwrap(),
        Prog061BgoTargetGeometry::Eh50M6930_4Mb
    );

    let mut eh50 = factory.clone();
    retarget_prog061_bgo_geometry_in_place(&mut eh50, Prog061BgoTargetGeometry::Eh50M6930_4Mb)
        .unwrap();
    assert_eq!(calculate_hashes(&eh50).1, factory_sha);

    let mut eh55 = factory.clone();
    retarget_prog061_bgo_geometry_in_place(&mut eh55, Prog061BgoTargetGeometry::Eh55M6930Lb8Mb)
        .unwrap();
    assert_eq!(
        inspect_prog061_bgo_geometry(&eh55).unwrap(),
        Prog061BgoTargetGeometry::Eh55M6930Lb8Mb
    );
    let (_, eh55_sha) = calculate_hashes(&eh55);

    // Idempotent second pass.
    retarget_prog061_bgo_geometry_in_place(&mut eh55, Prog061BgoTargetGeometry::Eh55M6930Lb8Mb)
        .unwrap();
    assert_eq!(calculate_hashes(&eh55).1, eh55_sha);

    // Rejects in-place downgrade back to EH50.
    assert!(
        retarget_prog061_bgo_geometry_in_place(&mut eh55, Prog061BgoTargetGeometry::Eh50M6930_4Mb)
            .is_err()
    );

    // Supports BypassChecksOnly, UartMonitor, and FactoryOriginal restore on EH55 geometry.
    patch_prog061_bgo_in_place_with_policy(&mut eh55, Prog061BgoPatchMode::BypassChecksOnly, false)
        .unwrap();
    assert_eq!(
        &eh55[OFF_BGO_SUM_BEQ_EH55..OFF_BGO_SUM_BEQ_EH55 + 2],
        &PATCH_SUM_BRA
    );
    let insp = inspect_prog061_bgo_sites(&eh55).unwrap();
    assert_eq!(insp.bgo_target_geometry, "EH55_M6930LB_8MB");
    assert_eq!(insp.detected_mode, "BYPASS_CHECKS_ONLY");
    assert_eq!(insp.bgo_sum_site.state, "BYPASSED_BRA");

    patch_prog061_bgo_in_place_with_policy(&mut eh55, Prog061BgoPatchMode::UartMonitor, false)
        .unwrap();
    let insp_mon = inspect_prog061_bgo_sites(&eh55).unwrap();
    assert_eq!(insp_mon.bgo_target_geometry, "EH55_M6930LB_8MB");
    assert_eq!(insp_mon.detected_mode, "UART_MONITOR_PANAMON1");

    patch_prog061_bgo_in_place_with_policy(&mut eh55, Prog061BgoPatchMode::FactoryOriginal, false)
        .unwrap();
    assert_eq!(
        &eh55[OFF_BGO_SUM_BEQ_EH55..OFF_BGO_SUM_BEQ_EH55 + 2],
        &ORIG_SUM_BEQ
    );
    assert_eq!(calculate_hashes(&eh55).1, eh55_sha);
}

#[test]
fn test_eh55_retargeted_prog061_mounts_real_8mb_bgo_in_vm() {
    let prog_path =
        repo_file("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
    let nor_path = repo_file("firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
    if !prog_path.exists() || !nor_path.exists() {
        return;
    }

    let mut prog = std::fs::read(&prog_path).expect("read prog061_dec.bin");
    let nor = std::fs::read(&nor_path).expect("read eh55_nor_8mb_ic5001_repaired.bin");
    assert_eq!(nor.len(), 0x0080_0000);

    // 1. Descriptor & Call-Graph Integrity against Real 8 MiB NOR (`audit_eh55_bgo_nor_8mb`).
    let nor_audit = crate::re::audit_eh55_bgo_nor_8mb(&nor).unwrap();
    assert!(nor_audit.healthy);
    assert_eq!(nor_audit.total_sectors, 45);
    assert_eq!(nor_audit.p1_active_count, 1);
    assert_eq!(nor_audit.p2_present_ids.len(), 40);
    assert_eq!(nor_audit.ifodev2_model.as_deref(), Some("M6930LB"));

    retarget_prog061_bgo_geometry_in_place(&mut prog, Prog061BgoTargetGeometry::Eh55M6930Lb8Mb)
        .unwrap();
    patch_prog061_bgo_in_place_with_policy(&mut prog, Prog061BgoPatchMode::BypassChecksOnly, false)
        .unwrap();

    let table_word = |idx: usize| -> u32 {
        let off = 0x03_6E2C + idx * 4;
        u32::from_le_bytes(prog[off..off + 4].try_into().unwrap())
    };
    assert_eq!(table_word(3) as usize, nor_audit.p2_present_ids.len());
    assert_eq!(table_word(7), 0x0F24);
    assert_eq!(table_word(12), 0x8413_0000);
    assert_eq!(table_word(13), 0x8440_0000);
    assert_eq!(table_word(31), nor_audit.total_sectors as u32);

    let init2_rel32 = i32::from_le_bytes(prog[0x00_24C7..0x00_24CB].try_into().unwrap());
    assert_eq!(init2_rel32, 0x0001_FE82);
    assert_eq!((0x5040_24C6i64 + init2_rel32 as i64) as u32, 0x5042_2348);

    let bss_zero_end = u32::from_le_bytes(prog[0x00_07C0..0x00_07C4].try_into().unwrap());
    assert_eq!(bss_zero_end, 0x504F_E000);

    // 2. Mn103Vm Execution of FUN_5040a294 (`bgo_init`) Cold & Warm Paths.
    let mut prog_cold = prog.clone();
    let stub_ret0 = [0x00u8, 0xDF, 0x00, 0x00]; // clr d0; ret [], 0 (matches CALL32 convention)
    for callee_off in [0x00_4592usize, 0x00_248E, 0x00_4DCD, 0x00_439B] {
        prog_cold[callee_off..callee_off + stub_ret0.len()].copy_from_slice(&stub_ret0);
    }

    let mut sdram = vec![0u8; 0x0008_0000];
    sdram[..0x0B74].copy_from_slice(&prog[0x03_8920..0x03_9494]);

    let mut vm = Mn103Vm::new();
    vm.enable_sc0_uart(&[]);
    vm.map_segment(PROG061_BASE_VA, prog_cold, false);
    vm.map_segment(0x5048_0000, sdram, true);
    vm.map_segment(0x50FF_0000, vec![0u8; 0x0001_0000], true);

    assert_eq!(vm.read_u8(0x5048_001F).unwrap(), b'i');
    assert_eq!(vm.read_u8(0x5048_009D).unwrap(), b'u');

    let cold_ret = call_mn103_fn(&mut vm, 0x5040_A294, 0, 10_000);
    assert_eq!(cold_ret, 0, "FUN_5040a294 cold path failed");
    assert_eq!(vm.read_u8(0x5048_02B0).unwrap(), 1);

    let warm_ret = call_mn103_fn(&mut vm, 0x5040_A294, 0, 10_000);
    assert_eq!(warm_ret, 0, "FUN_5040a294 warm path failed");

    // 3. Mn103Vm Execution of PANAMON1> Reading Real EH55 8 MiB BGO ("M6930LB" + 'M' & "update").
    let mut prog_mon = std::fs::read(&prog_path).expect("read prog061_dec.bin");
    retarget_prog061_bgo_geometry_in_place(&mut prog_mon, Prog061BgoTargetGeometry::Eh55M6930Lb8Mb)
        .unwrap();
    patch_prog061_bgo_in_place_with_policy(&mut prog_mon, Prog061BgoPatchMode::UartMonitor, false)
        .unwrap();

    let mut vm_mon = Mn103Vm::new();
    vm_mon.enable_sc0_uart(b"R 84150604,09\r\nR 84131220,0A\r\nB");
    vm_mon.map_segment(0x8400_0000, nor, true);
    vm_mon.map_segment(PROG061_BASE_VA, prog_mon, false);
    vm_mon.map_segment(0x5048_0000, vec![0u8; 0x0008_0000], true);
    vm_mon.map_segment(0x50FF_0000, vec![0u8; 0x0001_0000], true);

    let mon_ret = call_mn103_fn(&mut vm_mon, crate::re::PROG061_UART_MONITOR_VA, 0, 500_000);
    let tx = vm_mon.take_uart_tx_utf8();
    assert_eq!(mon_ret, 0, "PANAMON1 failed; tx={tx}");
    assert!(
        tx.contains("4D363933304C42004D"),
        "expected M6930LB\\0M in UART TX, got tx={tx}"
    );
    assert!(
        tx.contains("75706461746500003000"),
        "expected update\\0\\00\\0 in UART TX, got tx={tx}"
    );
}
