#![deny(unsafe_code)]

//! Unit tests for Matsushita MN103S 64-bit Feistel cipher and key detection.

use super::*;
use crate::constants::PANA_FEISTEL_ROUND_KEY_32B;

#[test]
fn test_derive_round_key_from_custom_key_8b_matches_old_df72() {
    let old_df72_key = [0xDF, 0x72, 0x15, 0xC7, 0x42, 0x7F, 0xD0, 0x10];
    let derived = derive_round_key_from_custom_key_8b(&old_df72_key);
    assert_eq!(derived, PANA_FEISTEL_ROUND_KEY_32B);
}

#[test]
fn test_derive_round_key_le2011_distinct_and_deterministic() {
    let le2011_key = [0x08, 0xE0, 0x3D, 0x85, 0x9A, 0xF9, 0xF3, 0xEE];
    let derived = derive_round_key_from_custom_key_8b(&le2011_key);
    assert_ne!(derived, PANA_FEISTEL_ROUND_KEY_32B);
    let derived2 = derive_round_key_from_custom_key_8b(&le2011_key);
    assert_eq!(derived, derived2);
}

#[test]
fn test_detect_frm_feistel_key_probes_slot1_at_64_bytes() {
    let mut plain = vec![0u8; 0x80];
    // Slot 0 unknown, Slot 1 at offset 64 (0x40) carries "MAIN..."
    plain[0..8].copy_from_slice(b"UNKNOWN0");
    plain[64..72].copy_from_slice(b"MAIN3410");
    let custom_rk = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[2].1);
    feistel_encrypt_buffer(
        &mut plain,
        &custom_rk,
        &crate::constants::PANA_FEISTEL_SBOX_256,
    );

    let (label, rk) = detect_frm_feistel_key(&plain).expect("must detect key via slot 1 at +64");
    assert_eq!(label, PANA_DVD_KEY_CATALOG[2].0);
    assert_eq!(rk, custom_rk);
}

#[test]
fn test_feistel_crypt_edges_short_and_split_buffers() {
    let rk = PANA_FEISTEL_ROUND_KEY_32B;
    // Short-buffer regime: span <= len < 2 * span (no overlap double-crypt)
    let span = 0x100usize;
    let orig = (0..0x180usize)
        .map(|i| (i & 0xFF) as u8)
        .collect::<Vec<_>>();
    let mut work = orig.clone();
    feistel_crypt_edges(&mut work, span, &rk, true);
    let mut direct = orig.clone();
    feistel_encrypt_buffer(&mut direct, &rk, &crate::constants::PANA_FEISTEL_SBOX_256);
    assert_eq!(work, direct);
    feistel_crypt_edges(&mut work, span, &rk, false);
    assert_eq!(work, orig);

    // Split-buffer regime: len > 2 * span (head and tail encrypted, middle untouched)
    let big_orig = vec![0x5Au8; 0x300];
    let mut big = big_orig.clone();
    feistel_crypt_edges(&mut big, span, &rk, true);
    assert_eq!(&big[span..0x300 - span], &big_orig[span..0x300 - span]);
    assert_ne!(&big[..span], &big_orig[..span]);
    assert_ne!(&big[0x300 - span..], &big_orig[0x300 - span..]);
    feistel_crypt_edges(&mut big, span, &rk, false);
    assert_eq!(big, big_orig);
}

#[test]
fn test_detect_frm_feistel_key_on_real_files_if_present() {
    let candidate_paths = [
        (
            "firmware_and_images/PANA_DVD.FRM",
            "../../firmware_and_images/PANA_DVD.FRM",
            "old_df72",
        ),
        (
            "firmware_and_images/dmr-eh50/PANA_DVD.FRM",
            "../../firmware_and_images/dmr-eh50/PANA_DVD.FRM",
            "le2011_08e0",
        ),
        (
            "firmware_and_images/dmr-eh49_59_69/eh49/PANA_DVD.FRM",
            "../../firmware_and_images/dmr-eh49_59_69/eh49/PANA_DVD.FRM",
            "old_df72",
        ),
    ];
    for (p1, p2, expected_label) in candidate_paths {
        if let Ok(data) = std::fs::read(p1).or_else(|_| std::fs::read(p2)) {
            let res = detect_frm_feistel_key(&data[..64]);
            assert!(res.is_some(), "Should detect key for {p1}");
            let (label, _) = res.unwrap();
            assert_eq!(label, expected_label);
        }
    }
}
