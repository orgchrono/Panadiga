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
fn obsolete_48b_sd_toc_is_marked_fail_with_48b_stride() {
    let mut sd48 = vec![0u8; 0x1880];
    sd48[0..8].copy_from_slice(b"DRV S211");
    sd48[0x0C..0x10].copy_from_slice(&0x1800u32.to_le_bytes());
    sd48[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
    let sum: u16 = sd48[..0x2E]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
    sd48[0x2E..0x30].copy_from_slice(&sum.to_le_bytes());

    let rep = verify_sd_buffers("/mem/sd48", None, Some(&sd48));
    assert!(rep.pana_sd_present);
    assert!(rep.pana_sd_list_sums_ok);
    assert_eq!(rep.overall_status, "INCOMPLETE_OR_INVALID");
    let sd_chk = rep
        .checks
        .iter()
        .find(|c| c.id == "pana_sd_container")
        .unwrap();
    assert_eq!(sd_chk.severity, "fail");
    assert!(sd_chk.title.contains("48B Obsolete"));
    assert!(sd_chk.detail.contains("entry_stride=48B"));
}
