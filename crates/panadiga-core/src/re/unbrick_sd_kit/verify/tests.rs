use super::*;

#[test]
fn missing_and_non_extrhead_buffers_fail_cleanly() {
    let rep_missing = verify_sd_buffers("/mem/sd", None, None);
    assert_eq!(rep_missing.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(!rep_missing.digadiag_present);

    let bad = vec![0xAAu8; 64];
    let rep_bad = verify_sd_buffers("/mem/sd", Some(Ok(&bad)), None);
    assert_eq!(rep_bad.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(rep_bad.digadiag_present);
    assert!(!rep_bad.digadiag_exact_size_ok);
}

#[test]
fn ver061_48b_sd_toc_and_raw_digadiag_pass_and_detect_mismatch() {
    let mut sd48 = vec![0u8; 0x1880];
    sd48[0..8].copy_from_slice(b"DRV S211");
    sd48[0x0C..0x10].copy_from_slice(&0x1800u32.to_le_bytes());
    sd48[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
    let sum: u16 = sd48[..0x2E]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
    sd48[0x2E..0x30].copy_from_slice(&sum.to_le_bytes());

    let mut raw_diag = vec![0u8; crate::constants::PANA_DIGADIAG_RAW_VER061_SIZE];
    raw_diag[0..4].copy_from_slice(&[0xDC, 0x14, 0x00, 0x00]);
    let rep = verify_sd_buffers("/mem/sd48", Some(Ok(&raw_diag)), Some(&sd48));
    assert!(rep.digadiag_present);
    assert!(rep.digadiag_exact_size_ok);
    assert!(rep.pana_sd_present);
    assert!(rep.pana_sd_list_sums_ok);
    assert_eq!(rep.overall_status, "READY_WITH_HW_WARNINGS");
    let sd_chk = rep
        .checks
        .iter()
        .find(|c| c.id == "pana_sd_container")
        .unwrap();
    assert_eq!(sd_chk.severity, "pass");
    assert!(sd_chk.title.contains("48B Plaintext SD TOC"));
    assert!(sd_chk.detail.contains("entry_stride=48B"));

    // Pairing 256 KiB raw ver0.61 DIGADIAG.FRM with 64B sd64 PANA__SD.FRM must fail consistency.
    let mut sd64 = vec![0u8; 0x2080];
    sd64[0..8].copy_from_slice(b"DRV S211");
    sd64[0x0C..0x10].copy_from_slice(&0x2000u32.to_le_bytes());
    sd64[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
    let sum64: u16 = sd64[..0x3E]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
    sd64[0x3E..0x40].copy_from_slice(&sum64.to_le_bytes());
    let rep_mismatch = verify_sd_buffers("/mem/mismatch", Some(Ok(&raw_diag)), Some(&sd64));
    assert_eq!(rep_mismatch.overall_status, "INCOMPLETE_OR_INVALID");
    assert!(
        rep_mismatch
            .checks
            .iter()
            .any(|c| c.id == "loader_profile_mismatch" && c.severity == "fail")
    );
}
