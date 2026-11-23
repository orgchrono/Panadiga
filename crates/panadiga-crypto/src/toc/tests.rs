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

#[test]
fn test_dvd64_at_0x1000_no_false_fallback_and_derived_module_report() {
    use super::super::feistel::{PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b};
    use super::super::frm_pack::{FrmContainerMode, FrmPackModule, pack_frm_with_key};
    use super::report::analyze_frm_toc_report;

    // 1. Unknown/unencrypted random noise MUST NOT falsely report Some("old_df72").
    let noise = [0xA5u8; 0x200];
    let (k_none, _, _, entries_none) = decrypt_frm_toc_with_detected_key(&noise);
    assert_eq!(k_none, None);
    assert!(entries_none.is_empty());

    // 2. Dvd64B TOC starting at offset 0x1000 inside 0x2000 region with non-default key.
    let mut plain = vec![0u8; 0x2400];
    plain[0x1000..0x1007].copy_from_slice(b"PROG081");
    plain[0x100C..0x1010].copy_from_slice(&0x0000_2000u32.to_le_bytes());
    plain[0x1010..0x1016].copy_from_slice(b"EH59EG");
    plain[0x1018] = b'M';
    plain[0x1019] = 1;
    plain[0x101A..0x1020].copy_from_slice(b"000000");
    plain[0x1020..0x1024].copy_from_slice(&0x0000_0200u32.to_le_bytes());
    let sum: u16 = plain[0x1000..0x103E]
        .iter()
        .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
    plain[0x103E..0x1040].copy_from_slice(&sum.to_le_bytes());

    let alt_key = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[2].1);
    feistel_encrypt_buffer(&mut plain, &alt_key, &PANA_FEISTEL_SBOX_256);
    let (k_det, _, layout, entries) = decrypt_frm_toc_with_detected_key(&plain);
    assert_eq!(k_det, Some(PANA_DVD_KEY_CATALOG[2].0));
    assert_eq!(layout, TocLayout::Dvd64B);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].index, 64);
    assert_eq!(entries[0].module_name, "PROG081");

    // 3. Report module catalog derives exact offsets/sizes from decrypted TOC.
    let (packed, _) = pack_frm_with_key(
        &[
            FrmPackModule::new("DRV S211", vec![0x11; 64]),
            FrmPackModule::new("MAIN3410PQ", vec![0x22; 96]),
        ],
        FrmContainerMode::Dvd64B,
        None,
        &alt_key,
    )
    .unwrap();
    let rep = analyze_frm_toc_report(&packed, "mem:dvd64b.frm").unwrap();
    assert_eq!(rep.toc_format, "dvd-64b-feistel");
    assert_eq!(rep.modules.len(), 2);
    assert_eq!(rep.modules[0].name, "DRV S211");
    assert_eq!(rep.modules[0].start_offset_hex, "0x00002000");
    assert_eq!(rep.modules[0].payload_size_bytes, 64);
    assert_eq!(rep.modules[1].start_offset_hex, "0x00002040");
    assert_eq!(rep.modules[1].payload_size_bytes, 96);
}

#[test]
fn eh50_drv_j147_slot0_offset_quirk_resolves_to_0x1800() {
    use super::super::feistel::{PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b};

    let mut plain = vec![0u8; 0x1800];
    plain[..8].copy_from_slice(b"DRV J147");
    plain[0x08..0x0C].copy_from_slice(&0x0000_1800u32.to_le_bytes());
    plain[0x0C..0x10].copy_from_slice(&0xCDF5_8588u32.to_le_bytes());
    plain[0x10..0x15].copy_from_slice(b"M7820");
    plain[0x18] = b'M';
    plain[0x1A..0x20].copy_from_slice(b"NA-348");
    plain[0x20..0x24].copy_from_slice(&0x0001_0000u32.to_le_bytes());
    let sum0: u16 = plain[..0x2E]
        .iter()
        .map(|&b| u16::from(b))
        .fold(0u16, |acc, x| acc.wrapping_add(x));
    plain[0x2E..0x30].copy_from_slice(&sum0.to_le_bytes());

    let rk = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[1].1);
    let mut synthetic_frm = vec![0u8; 0x1800 + 0x10000];
    synthetic_frm[..0x1800].copy_from_slice(&plain);
    feistel_encrypt_buffer(&mut synthetic_frm[..0x1800], &rk, &PANA_FEISTEL_SBOX_256);

    let (key_name, _, layout, entries) = decrypt_frm_toc_with_detected_key(&synthetic_frm);
    assert_eq!(key_name, Some("le2011_08e0"));
    assert_eq!(layout, TocLayout::Dvd48B);
    assert_eq!(entries[0].module_name, "DRV J147");
    assert_eq!(entries[0].module_offset, 0x1800);

    for p in [
        "firmware_and_images/dmr-eh50/PANA_DVD.FRM",
        "../../firmware_and_images/dmr-eh50/PANA_DVD.FRM",
    ] {
        let path = std::path::Path::new(p);
        if path.exists() {
            let real = std::fs::read(path).expect("read eh50 FRM");
            let (_, _, real_layout, real_entries) = decrypt_frm_toc_with_detected_key(&real);
            assert_eq!(real_layout, TocLayout::Dvd48B);
            assert_eq!(real_entries[0].module_name, "DRV J147");
            assert_eq!(real_entries[0].module_offset, 0x1800);
            assert!(
                (real_entries[0].module_offset as usize + real_entries[0].module_size as usize)
                    <= real.len()
            );
            break;
        }
    }
}

#[test]
fn decrypt_toc_supports_panasonic_48b_outer_header_and_adler32_checksum() {
    use super::super::feistel::{
        PANA_DVD_KEY_CATALOG, adler32_panasonic_strided, derive_round_key_from_custom_key_8b,
    };

    let mut toc_plain = vec![0u8; 0x1800];
    toc_plain[0..8].copy_from_slice(b"PROG0.00");
    toc_plain[0x0C..0x10].copy_from_slice(&0x0000_2000u32.to_le_bytes());
    toc_plain[0x10..0x17].copy_from_slice(b"M2863EG");
    toc_plain[0x1A..0x20].copy_from_slice(b"ZE-340");
    toc_plain[0x20..0x24].copy_from_slice(&0x0002_8000u32.to_le_bytes());
    let adler0 = adler32_panasonic_strided(&toc_plain[..0x2C], 1);
    toc_plain[0x2C..0x30].copy_from_slice(&adler0.to_le_bytes());

    let rk = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[1].1);
    feistel_encrypt_buffer(&mut toc_plain, &rk, &PANA_FEISTEL_SBOX_256);

    let mut container = vec![0u8; 0x30 + toc_plain.len() + 0x28000];
    container[0..12].copy_from_slice(b"PANASONIC\0\0\0");
    container[0x30..0x30 + toc_plain.len()].copy_from_slice(&toc_plain);

    let (key_name, _, layout, entries) = decrypt_frm_toc_with_detected_key(&container);
    assert_eq!(key_name, Some("le2011_08e0"));
    assert_eq!(layout, TocLayout::Dvd48B);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].module_name, "PROG0.00");
    assert_eq!(entries[0].hardware_family, "M2863EG");
    assert_eq!(entries[0].model_group, "ZE-340");
    assert_eq!(entries[0].module_offset, 0x2000);
    assert_eq!(entries[0].module_size, 0x28000);
    assert!(entries[0].list_sum_valid);

    for p in [
        "firmware_and_images/dmr-xw_bd_series/bs885_bs785_xs385eg/PANA_DVD.FRM",
        "../../firmware_and_images/dmr-xw_bd_series/bs885_bs785_xs385eg/PANA_DVD.FRM",
    ] {
        let path = std::path::Path::new(p);
        if path.exists() {
            let real = std::fs::read(path).expect("read bs885 FRM");
            let (k, _, real_layout, real_entries) = decrypt_frm_toc_with_detected_key(&real);
            assert_eq!(k, Some("le2011_08e0"));
            assert_eq!(real_layout, TocLayout::Dvd48B);
            assert_eq!(real_entries.len(), 2);
            assert_eq!(real_entries[0].module_name, "PROG0.00");
            assert_eq!(real_entries[1].module_name, "MAIN3400");
            assert!(real_entries.iter().all(|e| e.list_sum_valid));
            break;
        }
    }
}
