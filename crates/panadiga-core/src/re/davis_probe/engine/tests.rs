#![deny(unsafe_code)]

use super::*;
use crate::constants::{AM34_DAVIS_SEC_MMIO_BASE, AM34_FORBIDDEN_LETHAL_REG, AM34_WDBC_REG};
use crate::pico::panamon1::{MockPanamon1Transport, Panamon1Client};
use crate::re::davis_probe::ata_identify::{
    build_mock_identify_512, extract_ata_identify_512_from_pio_1024, parse_ata_identify_words,
};
use crate::re::davis_probe::stub_builder::{
    DAVIS_IDENTIFY_BUF_DEFAULT_VA, DAVIS_IDENTIFY_META_DEFAULT_VA, DAVIS_IDENTIFY_SENTINEL,
    build_ata_identify_ec_stub,
};
use crate::re::mn10300_emu::Mn103Vm;

fn pack_512_to_pio_1024(raw_512: &[u8; 512]) -> Vec<u8> {
    let mut pio = vec![0u8; 1024];
    for i in 0..256usize {
        pio[i * 4] = raw_512[i * 2];
        pio[i * 4 + 1] = raw_512[i * 2 + 1];
    }
    pio
}

#[test]
fn ata_identify_ec_stub_executes_in_mn103_vm_until_rets_and_parses_words() {
    let stub = build_ata_identify_ec_stub(
        AM34_DAVIS_SEC_MMIO_BASE,
        0xA0,
        DAVIS_IDENTIFY_BUF_DEFAULT_VA,
    )
    .unwrap();
    assert!(stub.code.ends_with(&[0xF0, 0xFC]));

    let mut vm = Mn103Vm::new();
    vm.map_segment(stub.load_va, stub.code.clone(), false);
    vm.map_segment(0x9050_1000, vec![0u8; 0x3000], true);

    // Mock DAVIS CH1 MMIO (0x8900_0C00..0x8900_0E00): DATA=0x045A.
    let mut mmio = vec![0u8; 0x200];
    mmio[0x120..0x124].copy_from_slice(&0x0000_045Au32.to_le_bytes());
    vm.map_segment(0x8900_0C00, mmio, true);

    let ret_pc = 0x5000_DEADu32;
    vm.map_segment(0x500F_0000, vec![0u8; 0x1000], true);
    vm.sp = 0x500F_0800;
    vm.write_u32(vm.sp, ret_pc).unwrap();
    vm.pc = stub.load_va;

    // Note: stub writes DevCtrl=0 to 0x8900_0D58 before sending CMD=0xEC to 0x8900_0D3C.
    // Model IDE hardware asserting DRQ=1 (0x08) on ALTSTAT once 0xEC reaches CMD.
    let mut drq_asserted = false;
    for _ in 0..250_000u64 {
        if vm.pc == ret_pc {
            break;
        }
        vm.step().unwrap();
        if !drq_asserted && vm.read_u32(0x8900_0D3C) == Ok(0x0000_00EC) {
            vm.write_u32(0x8900_0D58, 0x0000_0008).unwrap();
            drq_asserted = true;
        }
    }
    assert_eq!(
        vm.pc, ret_pc,
        "EC stub failed to return to ret_pc within 250_000 steps (steps={})",
        vm.steps
    );
    assert!(drq_asserted);

    let meta_va = stub.meta_va.unwrap();
    assert_eq!(meta_va, DAVIS_IDENTIFY_META_DEFAULT_VA);
    assert_eq!(vm.read_u32(meta_va).unwrap(), DAVIS_IDENTIFY_SENTINEL);
    assert_eq!(vm.read_u32(meta_va + 4).unwrap(), 0x08);
    assert_eq!(vm.read_u32(0x8900_0D38).unwrap(), 0xA0);
    assert_eq!(vm.read_u32(0x8900_0D3C).unwrap(), 0xEC);

    let mut vm_pio = vec![0u8; 1024];
    for i in 0..256u32 {
        let w = vm.read_u32(DAVIS_IDENTIFY_BUF_DEFAULT_VA + i * 4).unwrap();
        assert_eq!(w, 0x045A);
        vm_pio[i as usize * 4..i as usize * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    let vm_512 = extract_ata_identify_512_from_pio_1024(&vm_pio).unwrap();
    assert_eq!(&vm_512[0..2], &[0x5A, 0x04]);

    let mock_512 =
        build_mock_identify_512("WDC WD2500AAJB", "WD-WCAV1234", "01.03B01", 488_397_168);
    let converted =
        extract_ata_identify_512_from_pio_1024(&pack_512_to_pio_1024(&mock_512)).unwrap();
    let parsed = parse_ata_identify_words(&converted).unwrap();
    assert_eq!(parsed.model, "WDC WD2500AAJB");
    assert_eq!(parsed.mirror_main_lba, Some(488_266_096));
}

#[test]
fn blocks_lethal_register_0xc000100c_before_touching_transport() {
    let mut mock = MockPanamon1Transport::new();
    mock.bridge_active = true;
    let mut client = Panamon1Client::new(mock);
    let mut engine = DavisProbeEngine::new(&mut client, DavisProbeConfig::default());

    let err_direct = engine
        .safe_read_reg(AM34_FORBIDDEN_LETHAL_REG, 4)
        .unwrap_err()
        .to_string();
    assert!(err_direct.contains("0xC000100C"), "err={err_direct}");

    let err_overlap = engine
        .safe_read_reg(AM34_WDBC_REG, 16)
        .unwrap_err()
        .to_string();
    assert!(err_overlap.contains("0xC000100C"), "err={err_overlap}");
    assert!(client.transport().tx_target_log.is_empty());
}

#[test]
fn runs_full_davis_probe_sequence_against_mock_transport() {
    let mut mock = MockPanamon1Transport::new();
    mock.bridge_active = true;
    mock.map_memory(0x8900_0000, vec![0u8; 0x1000]);
    mock.map_memory(0x504F_8234, vec![0u8; 0x40]);
    mock.map_memory(0x9050_0000, vec![0u8; 0x3000]);

    let mut sys = vec![0u8; 0x0C];
    sys[8..10].copy_from_slice(&0x8002u16.to_le_bytes());
    mock.map_memory(0xC000_1000, sys);

    let make_tf = |status: u32, seccnt: u32, lbalo: u32, sentinel: u32| -> Vec<u8> {
        let mut out = Vec::with_capacity(40);
        for w in [0u32, 0, seccnt, lbalo, 0, 0, 0xA0, status, status, sentinel] {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    };
    let mut meta_bytes = DAVIS_IDENTIFY_SENTINEL.to_le_bytes().to_vec();
    meta_bytes.extend_from_slice(&0x50u32.to_le_bytes());
    mock.set_jump_mem_effect(
        DAVIS_STUB_DEFAULT_VA,
        vec![
            (0x9050_1000, make_tf(0x50, 1, 1, 0xCAFE_0001)),
            (0x9050_1100, make_tf(0x50, 0x55, 0xAA, 0xCAFE_02A0)),
            (0x9050_1140, make_tf(0x50, 0x55, 0xAA, 0xCAFE_02A1)),
            (0x9050_1180, make_tf(0x00, 0x00, 0x00, 0xCAFE_02A2)),
            (0x9050_12FC, 0u32.to_le_bytes().to_vec()),
            (0x504F_8234, vec![0, 0, 0, 0, 0, 0, 1, 0]),
            (DAVIS_IDENTIFY_META_DEFAULT_VA, meta_bytes),
            (
                DAVIS_IDENTIFY_BUF_DEFAULT_VA,
                pack_512_to_pio_1024(&build_mock_identify_512(
                    "SAMSUNG HM251JI",
                    "S123456",
                    "2AC1",
                    488_397_168,
                )),
            ),
        ],
    );

    let mut client = Panamon1Client::new(mock);
    let mut engine = DavisProbeEngine::new(&mut client, DavisProbeConfig::default());
    let report = engine.run_probe().unwrap();

    assert!(report.fe_released);
    assert_eq!(report.classification, DavisHangClassification::AtaReady);
    assert_eq!(report.identify_sentinel, Some(DAVIS_IDENTIFY_SENTINEL));
    assert_eq!(
        report.identify_info.as_ref().map(|i| i.model.as_str()),
        Some("SAMSUNG HM251JI")
    );
    assert!(
        !client
            .transport()
            .tx_target_log
            .iter()
            .any(|c| c.contains("C000100C"))
    );
}
