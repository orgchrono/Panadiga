use std::path::Path;

use crate::nvram::calculate_hashes;
use crate::re::mn10300_emu::Mn103Vm;

use crate::ssot::registry::{AddressReloc, BytePatchEntry, OffsetRange, ProgBgoRetargetSpec};

use super::{
    BASE_FACTORY_BGO_GEOMETRY_ID, ORIG_SUM_BEQ, PATCH_SUM_BRA, PROG061_BASE_VA,
    Prog061BgoPatchMode, detect_prog_bgo_geometry, find_retarget_spec_by_loader_key,
    inspect_prog_bgo_geometry_with_spec, inspect_prog061_bgo_sites,
    patch_prog061_bgo_in_place_with_policy, retarget_prog_for_geometry_in_place,
    retarget_prog_for_loader_profile_in_place, retarget_prog_from_spec_in_place,
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
fn test_ssot_retarget_is_idempotent_and_preserves_baseline() {
    let prog_path =
        repo_file("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
    if !prog_path.exists() {
        return;
    }
    let factory = std::fs::read(&prog_path).expect("read prog061_dec.bin");
    let (_, factory_sha) = calculate_hashes(&factory);
    let target_spec = find_retarget_spec_by_loader_key("ver061").expect("SSOT ver061 spec");

    let base_geo = detect_prog_bgo_geometry(&factory).unwrap();
    assert_eq!(base_geo.geometry_id, BASE_FACTORY_BGO_GEOMETRY_ID);
    assert!(!base_geo.is_retargeted);

    let mut baseline = factory.clone();
    retarget_prog_for_geometry_in_place(&mut baseline, BASE_FACTORY_BGO_GEOMETRY_ID).unwrap();
    assert_eq!(calculate_hashes(&baseline).1, factory_sha);

    let mut eh55 = factory.clone();
    retarget_prog_for_loader_profile_in_place(&mut eh55, "ver061").unwrap();
    let retargeted_geo = detect_prog_bgo_geometry(&eh55).unwrap();
    assert_eq!(retargeted_geo.geometry_id, target_spec.target_geometry_id);
    assert!(retargeted_geo.is_retargeted);
    let (_, eh55_sha) = calculate_hashes(&eh55);

    // Idempotent second pass.
    retarget_prog_for_loader_profile_in_place(&mut eh55, "ver061").unwrap();
    assert_eq!(calculate_hashes(&eh55).1, eh55_sha);

    // Rejects in-place downgrade back to baseline.
    assert!(retarget_prog_for_geometry_in_place(&mut eh55, BASE_FACTORY_BGO_GEOMETRY_ID).is_err());

    // Supports BypassChecksOnly, UartMonitor, and FactoryOriginal restore on retargeted geometry.
    let eh55_sum_off = target_spec.bgo_sum_beq_offset as usize;
    patch_prog061_bgo_in_place_with_policy(&mut eh55, Prog061BgoPatchMode::BypassChecksOnly, false)
        .unwrap();
    assert_eq!(&eh55[eh55_sum_off..eh55_sum_off + 2], &PATCH_SUM_BRA);
    let insp = inspect_prog061_bgo_sites(&eh55).unwrap();
    assert_eq!(insp.bgo_target_geometry, target_spec.target_geometry_id);
    assert_eq!(insp.detected_mode, "BYPASS_CHECKS_ONLY");
    assert_eq!(insp.bgo_sum_site.state, "BYPASSED_BRA");

    patch_prog061_bgo_in_place_with_policy(&mut eh55, Prog061BgoPatchMode::UartMonitor, false)
        .unwrap();
    let insp_mon = inspect_prog061_bgo_sites(&eh55).unwrap();
    assert_eq!(insp_mon.bgo_target_geometry, target_spec.target_geometry_id);
    assert_eq!(insp_mon.detected_mode, "UART_MONITOR_PANAMON1");

    patch_prog061_bgo_in_place_with_policy(&mut eh55, Prog061BgoPatchMode::FactoryOriginal, false)
        .unwrap();
    assert_eq!(&eh55[eh55_sum_off..eh55_sum_off + 2], &ORIG_SUM_BEQ);
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

    retarget_prog_for_loader_profile_in_place(&mut prog, "ver061").unwrap();
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
    retarget_prog_for_loader_profile_in_place(&mut prog_mon, "ver061").unwrap();
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

#[test]
fn test_generic_ssot_retarget_executes_synthetic_custom_spec() {
    let mut table_words = vec![0xFFFF_FFFFu32; 64];
    table_words[0] = 0x8000_0000;
    table_words[3] = 0x0000_0030;
    table_words[12] = 0x8415_0000;

    let mut drv = vec![0xAAu8; 32];
    // Simulate BGO_SUM_BEQ at nfrom_drv_start + 8 -> 0x0208
    drv[8..10].copy_from_slice(&ORIG_SUM_BEQ);

    let spec = ProgBgoRetargetSpec {
        target_geometry_id: "SYNTHETIC_CUSTOM_8MB".to_string(),
        expected_prog_size: 0x1000,
        bgo_table_offset: 0x0800,
        bgo_table_marker: 0x8000_0000,
        detect_logical_sectors: 0x30,
        detect_flash_base: 0x8415_0000,
        bgo_table_words: table_words,
        bgo_sum_beq_offset: 0x0208,
        nfrom_drv_start: 0x0200,
        nfrom_drv_end: 0x0230,
        nfrom_drv_uncompressed_bytes: 32,
        nfrom_drv_blob_relpath: "blobs/synthetic.bin.gz".to_string(),
        nfrom_drv_uncompressed_sha256: "0".repeat(64),
        nop_fill_byte: 0xCB,
        bss_zero_end_offset: 0x0080,
        orig_bss_zero_end: 0x504F_1000,
        target_bss_zero_end: 0x504F_9000,
        init2_call_offset: 0x0100,
        orig_init2_rel32: 0x0000_0500,
        target_init2_rel32: 0x0000_0480,
        hi_ram_ranges: vec![OffsetRange {
            start: 0x0120,
            end: 0x0140,
        }],
        hi_ram_relocs: vec![AddressReloc {
            from: 0x5049_1111,
            to: 0x504F_BBBB,
        }],
        byte_patches: vec![BytePatchEntry {
            offset: 0x0128,
            expect: 0x0A,
            replace: 0x0F,
            note: Some("synthetic_custom_patch".to_string()),
        }],
    };

    let mut image = vec![0u8; 0x1000];
    // Original table marker at 0x0800 (differs from target)
    image[0x0800..0x0804].copy_from_slice(&0x8000_0000u32.to_le_bytes());
    // Prior BGO_SUM already bypassed at OFF_BGO_SUM_BEQ? No, test buffer smaller than 0x230F0 so test below checks non-bypassed:
    // Note: retarget_prog_from_spec_in_place checks super::OFF_BGO_SUM_BEQ (0x230F0) only when image >= OFF_BGO_SUM_BEQ+2.
    // Let's use full PROG061_EXPECTED_SIZE so that contract is exercised faithfully.
    image.resize(super::PROG061_EXPECTED_SIZE, 0);
    let mut full_spec = spec;
    full_spec.expected_prog_size = super::PROG061_EXPECTED_SIZE as u32;
    image[super::OFF_BGO_SUM_BEQ..super::OFF_BGO_SUM_BEQ + 2].copy_from_slice(&PATCH_SUM_BRA);
    image[0x0080..0x0084].copy_from_slice(&0x504F_1000u32.to_le_bytes());
    image[0x0101..0x0105].copy_from_slice(&0x0000_0500i32.to_le_bytes());
    image[0x0124..0x0128].copy_from_slice(&0x5049_1111u32.to_le_bytes());
    image[0x0128] = 0x0A;

    assert!(!inspect_prog_bgo_geometry_with_spec(&image, &full_spec).unwrap());
    retarget_prog_from_spec_in_place(&mut image, &full_spec, &drv).unwrap();
    assert!(inspect_prog_bgo_geometry_with_spec(&image, &full_spec).unwrap());
    assert_eq!(&image[0x0208..0x020A], &PATCH_SUM_BRA);
    assert!(image[0x0220..0x0230].iter().all(|&b| b == 0xCB));
    assert_eq!(
        u32::from_le_bytes(image[0x0080..0x0084].try_into().unwrap()),
        0x504F_9000
    );
    assert_eq!(
        i32::from_le_bytes(image[0x0101..0x0105].try_into().unwrap()),
        0x0000_0480
    );
    assert_eq!(
        u32::from_le_bytes(image[0x0124..0x0128].try_into().unwrap()),
        0x504F_BBBB
    );
    assert_eq!(image[0x0128], 0x0F);
}
