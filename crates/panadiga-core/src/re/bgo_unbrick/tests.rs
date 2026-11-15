use std::fs;
use std::path::Path;

use super::*;
use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::crypto::{ExtrheadCryptMode, build_digadiag_frm, unpack_extrhead};

fn synthetic_factory_prog061() -> Vec<u8> {
    let mut buf = vec![0u8; PROG061_EXPECTED_SIZE];
    buf[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
    buf[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2].copy_from_slice(&ORIG_DEVINFO_BEQ);
    buf[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20].copy_from_slice(&ORIG_A2CD_HOOK);
    buf
}

#[test]
fn test_prog061_bgo_patch_modes_and_digadiag_roundtrip() {
    let factory = synthetic_factory_prog061();
    let insp = inspect_prog061_bgo_sites(&factory).unwrap();
    assert_eq!(insp.detected_mode, "FACTORY_ORIGINAL");

    let mut patched = factory.clone();
    patch_prog061_bgo_in_place(&mut patched, Prog061BgoPatchMode::BypassChecksOnly).unwrap();
    assert_eq!(
        inspect_prog061_bgo_sites(&patched).unwrap().detected_mode,
        "BYPASS_CHECKS_ONLY"
    );

    patch_prog061_bgo_in_place(&mut patched, Prog061BgoPatchMode::UartMonitor).unwrap();
    assert_eq!(
        inspect_prog061_bgo_sites(&patched).unwrap().detected_mode,
        "UART_MONITOR_PANAMON1"
    );

    patch_prog061_bgo_in_place(&mut patched, Prog061BgoPatchMode::FactoryOriginal).unwrap();
    assert_eq!(
        inspect_prog061_bgo_sites(&patched).unwrap().detected_mode,
        "FACTORY_ORIGINAL"
    );

    // AutoFormatBgo MUST fail-closed without explicit unsafe policy override (D4/D5).
    assert!(patch_prog061_bgo_in_place(&mut patched, Prog061BgoPatchMode::AutoFormatBgo).is_err());

    // Raw 128 KiB container without EXTRHEAD and oversized buffers must fail decoding cleanly.
    assert!(decode_prog061_payload(&vec![0u8; PANA_DIGADIAG_FRM_SIZE]).is_err());
    let bad_main = vec![0u8; 300_000];
    assert!(inspect_prog061_bgo_sites(&bad_main).is_err());

    // Explicit unsafe override remains testable for regression auditing.
    patch_prog061_bgo_in_place_with_policy(&mut patched, Prog061BgoPatchMode::AutoFormatBgo, true)
        .unwrap();
    assert_eq!(
        inspect_prog061_bgo_sites(&patched).unwrap().detected_mode,
        "AUTO_FORMAT_BGO"
    );

    let dir = std::env::temp_dir().join("panadiga_bgo_unbrick_test");
    let _ = fs::create_dir_all(&dir);
    let raw_in = dir.join("prog061_raw.bin");
    let diag_out = dir.join("DIGADIAG.FRM");
    fs::write(&raw_in, &factory).unwrap();

    assert!(
        patch_prog061_or_build_digadiag(
            &raw_in,
            &diag_out,
            Prog061BgoPatchMode::AutoFormatBgo,
            true,
        )
        .is_err()
    );

    let report = patch_prog061_or_build_digadiag(
        &raw_in,
        &diag_out,
        Prog061BgoPatchMode::BypassChecksOnly,
        true,
    )
    .unwrap();
    assert!(report.digadiag_wrapped);
    assert_eq!(report.output_size, 0x2_0000);
    assert_eq!(report.inspection_before.detected_mode, "FACTORY_ORIGINAL");
    assert_eq!(report.inspection_after.detected_mode, "BYPASS_CHECKS_ONLY");

    let unpacked =
        unpack_extrhead(&fs::read(&diag_out).unwrap(), ExtrheadCryptMode::Plain).unwrap();
    assert_eq!(unpacked.header.dst_addr, PROG061_BASE_VA);
    assert_eq!(
        inspect_prog061_bgo_sites(&unpacked.decompressed)
            .unwrap()
            .detected_mode,
        "BYPASS_CHECKS_ONLY"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_prog061_real_binary_if_present() {
    let local = Path::new(r"C:\ghidra\binaries\unpacked\eh55_frm_PROG061_BOOT.bin");
    if !local.is_file() {
        return;
    }
    let mut bytes = fs::read(local).unwrap();
    let insp = inspect_prog061_bgo_sites(&bytes).unwrap();
    assert_eq!(insp.detected_mode, "FACTORY_ORIGINAL");

    patch_prog061_bgo_in_place(&mut bytes, Prog061BgoPatchMode::UartMonitor).unwrap();
    let (frm, _) =
        build_digadiag_frm(&bytes, PROG061_BASE_VA, "DIAG", "PANAMON1", 4, 0x2_0000).unwrap();
    assert_eq!(frm.len(), 0x2_0000);
    let unpacked = unpack_extrhead(&frm, ExtrheadCryptMode::Plain).unwrap();
    assert_eq!(
        inspect_prog061_bgo_sites(&unpacked.decompressed)
            .unwrap()
            .detected_mode,
        "UART_MONITOR_PANAMON1"
    );
}
