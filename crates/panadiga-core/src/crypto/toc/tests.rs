//! Unit tests for `.FRM` TOC parsing, layout discrimination, and Feistel key detection.

use super::super::feistel::feistel_encrypt_buffer;
use super::*;
use crate::constants::{PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256};

#[test]
fn test_interpret_frm_target_flag() {
    assert_eq!(interpret_frm_target_flag("MAIN3410PQ", 1), "FORCE_UPDATE");
    assert_eq!(
        interpret_frm_target_flag("MAIN3060NP", 0),
        "NORMAL_VERSION_CHECK"
    );
    assert_eq!(interpret_frm_target_flag("PROG061 ", 1), "FORMAT_HDD");
    assert_eq!(
        interpret_frm_target_flag("PROG061 ", 0),
        "NORMAL_PRESERVE_HDD"
    );
    assert_eq!(interpret_frm_target_flag("DRV S211", 0), "NORMAL");
    assert_eq!(interpret_frm_target_flag("DRV S211", 1), "FLAG_SET");
}

#[test]
fn parse_sd_frm_toc_with_layout_discriminates_64b_vs_48b() {
    let mut sd64 = vec![0u8; 0x2000];
    sd64[0..8].copy_from_slice(b"DRV S211");
    sd64[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
    let sum64: u16 = sd64[..0x3E]
        .iter()
        .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
    sd64[0x3E..0x40].copy_from_slice(&sum64.to_le_bytes());
    let (layout64, entries64) = parse_sd_frm_toc_with_layout(&sd64);
    assert_eq!(layout64, Some(TocLayout::Sd64B));
    assert_eq!(entries64.len(), 1);

    let mut sd48 = vec![0u8; 0x1800];
    sd48[0..8].copy_from_slice(b"DRV S211");
    sd48[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
    let sum48: u16 = sd48[..0x2E]
        .iter()
        .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
    sd48[0x2E..0x30].copy_from_slice(&sum48.to_le_bytes());
    let (layout48, entries48) = parse_sd_frm_toc_with_layout(&sd48);
    assert_eq!(layout48, Some(TocLayout::Sd48B));
    assert_eq!(entries48.len(), 1);
}

#[test]
fn test_decrypt_frm_toc_with_detected_key_synthetic_dvd64() {
    let mut plain = vec![0u8; 0x1800];
    // Entry 0 (64 bytes)
    plain[0..7].copy_from_slice(b"PROG071");
    plain[0x0C..0x10].copy_from_slice(&0x00002000u32.to_le_bytes()); // offset >= 0x2000
    plain[0x10..0x16].copy_from_slice(b"EH49EG");
    plain[0x18] = b'M';
    plain[0x19] = 1; // target_flag
    plain[0x1A..0x20].copy_from_slice(b"000000");
    plain[0x20..0x24].copy_from_slice(&0x00020000u32.to_le_bytes()); // size
    plain[0x24..0x28].copy_from_slice(&0x0102D54Fu32.to_le_bytes());
    let sum0: u16 = plain[..0x3E]
        .iter()
        .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
    plain[0x3E..0x40].copy_from_slice(&sum0.to_le_bytes());

    // Entry 1 (64 bytes)
    plain[0x40..0x47].copy_from_slice(b"PROG071");
    plain[0x4C..0x50].copy_from_slice(&0x00022000u32.to_le_bytes());
    plain[0x50..0x56].copy_from_slice(b"EH49EC");
    plain[0x58] = b'M';
    plain[0x59] = 1;
    plain[0x5A..0x60].copy_from_slice(b"000000");
    plain[0x60..0x64].copy_from_slice(&0x00020000u32.to_le_bytes());
    let sum1: u16 = plain[0x40..0x7E]
        .iter()
        .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
    plain[0x7E..0x80].copy_from_slice(&sum1.to_le_bytes());

    // Encrypt with old_df72
    let mut encrypted = plain.clone();
    feistel_encrypt_buffer(
        &mut encrypted,
        &PANA_FEISTEL_ROUND_KEY_32B,
        &PANA_FEISTEL_SBOX_256,
    );

    let (key_name, _, layout, entries) = decrypt_frm_toc_with_detected_key(&encrypted);
    assert_eq!(key_name, Some("old_df72"));
    assert_eq!(layout, TocLayout::Dvd64B);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].module_name, "PROG071");
    assert_eq!(entries[0].hardware_family, "EH49EG");
    assert_eq!(entries[0].target_flag, 1);
    assert!(entries[0].list_sum_valid);
    assert_eq!(entries[1].module_name, "PROG071");
    assert_eq!(entries[1].hardware_family, "EH49EC");
    assert!(entries[1].list_sum_valid);
}

#[test]
fn test_decrypt_frm_toc_with_detected_key_real_firmwares() {
    let cases = [
        (
            "firmware_and_images/PANA_DVD.FRM",
            "../../firmware_and_images/PANA_DVD.FRM",
            "old_df72",
            TocLayout::Dvd48B,
        ),
        (
            "firmware_and_images/dmr-eh50/PANA_DVD.FRM",
            "../../firmware_and_images/dmr-eh50/PANA_DVD.FRM",
            "le2011_08e0",
            TocLayout::Dvd48B,
        ),
        (
            "firmware_and_images/dmr-eh49_59_69/eh49/PANA_DVD.FRM",
            "../../firmware_and_images/dmr-eh49_59_69/eh49/PANA_DVD.FRM",
            "old_df72",
            TocLayout::Dvd64B,
        ),
    ];

    for (p1, p2, exp_key, exp_layout) in cases {
        if let Ok(data) = std::fs::read(p1).or_else(|_| std::fs::read(p2)) {
            let (key_name, _, layout, entries) = decrypt_frm_toc_with_detected_key(&data);
            assert_eq!(key_name, Some(exp_key), "Checking key for {p1}");
            assert_eq!(layout, exp_layout, "Checking layout for {p1}");
            assert!(!entries.is_empty(), "Entries must not be empty for {p1}");
            for entry in &entries {
                assert!(
                    entry.list_sum_valid,
                    "Entry {} list_sum must be valid in {p1}",
                    entry.index
                );
                assert!(!entry.module_name.trim().is_empty());
            }
        }
    }
}
