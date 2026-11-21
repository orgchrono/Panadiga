#![deny(unsafe_code)]

use super::*;
use crate::constants::AM34_FLASH_SIZE;
use crate::crypto::{ExtrheadCryptMode, pack_extrhead};
use crate::re::bgo_unbrick::{Prog061BgoPatchMode, patch_prog061_bgo_in_place};
use crate::re::unbrick_sd_kit::{build_cleanroom_digadiag, synthetic_prog061_fixture};

#[test]
fn gate_approves_cleanroom_digadiag_ver061_and_ver063_and_rejects_corrupt_stub() {
    let art061 = build_cleanroom_digadiag(SdLoaderProfile::Ver061RawSd48).unwrap();
    assert!(art061.emu_gate.passed);
    assert!(art061.emu_gate.ipl_untouched);
    assert!(art061.emu_gate.lethal_mmio_blocked);

    let art063 = build_cleanroom_digadiag(SdLoaderProfile::Ver063ExtrheadSd64).unwrap();
    assert!(art063.emu_gate.passed);

    let mut bad = art061.bytes.clone();
    bad[0..6].fill(0xFF);
    let err = require_prewrite_gate(PreWriteTarget::CleanroomDigadiag {
        bytes: &bad,
        loader_profile: SdLoaderProfile::Ver061RawSd48,
    })
    .unwrap_err();
    assert!(err.to_string().contains("PRE-WRITE EMULATOR GATE BLOCKED"));
}

#[test]
fn gate_approves_patched_prog061_nor_cow_and_blocks_ipl_tamper() {
    let mut prog = synthetic_prog061_fixture();
    patch_prog061_bgo_in_place(&mut prog, Prog061BgoPatchMode::UartMonitor).unwrap();
    let rep = require_prewrite_gate(PreWriteTarget::PatchedProg061 {
        bytes: &prog,
        mode: Prog061BgoPatchMode::UartMonitor,
    })
    .unwrap();
    assert!(rep.passed);

    let orig_nor = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    let mut clean_nor = orig_nor.clone();
    clean_nor[0x7F_0000] = 0xAA;
    let ok_nor = require_prewrite_gate(PreWriteTarget::NorCowRepair {
        original_nor: &orig_nor,
        repaired_nor: &clean_nor,
        allow_upper_nor: false,
    })
    .unwrap();
    assert!(ok_nor.passed);

    let mut tampered_ipl = orig_nor.clone();
    tampered_ipl[0x100] = 0x00;
    assert!(
        require_prewrite_gate(PreWriteTarget::NorCowRepair {
            original_nor: &orig_nor,
            repaired_nor: &tampered_ipl,
            allow_upper_nor: true,
        })
        .is_err()
    );
}

#[test]
fn gate_validates_composed_hdd_stream_with_and_without_extrhead_payloads() {
    let main_eh = pack_extrhead(
        &vec![0x5Au8; 4096],
        "FVU ",
        0x5700_0000,
        "3410PQ",
        ExtrheadCryptMode::Main,
        16,
        1,
    )
    .unwrap();
    let drv_eh = pack_extrhead(
        &vec![0x33u8; 2048],
        "DRV ",
        0x9020_2000,
        "S211",
        ExtrheadCryptMode::Drv,
        16,
        1,
    )
    .unwrap();
    let rep = require_prewrite_gate(PreWriteTarget::ComposedHddStream {
        total_sectors: 312_581_808,
        main_extrhead: Some(&main_eh),
        drv_extrhead: Some(&drv_eh),
    })
    .unwrap();
    assert!(rep.passed);
    assert!(
        rep.checks
            .iter()
            .any(|c| c.id == "composed_main_extrhead" && c.passed)
    );
}
