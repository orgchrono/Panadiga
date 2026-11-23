#![deny(unsafe_code)]

//! Unit and workspace integration tests for `universal_frm`.

use super::super::aes128::FrmKeyring;
use super::super::codec::write_padded_ascii;
use super::super::extrhead::ExtrheadCompressionAlgo;
use super::super::feistel::{
    adler32_panasonic_strided, derive_round_key_from_custom_key_8b, feistel_encrypt_buffer,
};
use super::super::pana_fw_go::tests::{build_synthetic_cramfs, build_synthetic_romfs};
use super::super::pana_fw_go::{
    CramFsHeaderInfo, PartitionFsKind, RomFsHeaderInfo, pack_main_list, split_main_partitions,
    wrap_aes_frm,
};
use super::*;
use crate::constants::{PANA_FEISTEL_SBOX_256, resolve_workspace_root};

fn build_synthetic_bluray_feistel_frm(
    round_key: &[u8; 32],
    has_outer_48b: bool,
    prog_bytes: &[u8],
    main_plain: &[u8],
) -> Vec<u8> {
    let first_off = if has_outer_48b {
        0x2000usize
    } else {
        0x1800usize
    };
    let toc_start = if has_outer_48b { 48usize } else { 0usize };
    let mut img = vec![0u8; first_off];
    if has_outer_48b {
        img[..12].copy_from_slice(b"PANASONIC\0\0\0");
        img[43] = 1;
    }

    let mut enc_prog = prog_bytes.to_vec();
    enc_prog.resize((enc_prog.len() + 15) & !15, 0);
    let prog_sum = adler32_panasonic_strided(prog_bytes, 1);
    feistel_encrypt_buffer(&mut enc_prog, round_key, &PANA_FEISTEL_SBOX_256);

    let main_list = pack_main_list(
        main_plain,
        "DRV ",
        "BD_TEST",
        ExtrheadCompressionAlgo::Gzip,
        16 * 1024 * 1024,
        Some(round_key),
        16,
        1,
    )
    .unwrap();
    let main_sum = adler32_panasonic_strided(&main_list, 1);

    let prog_off = first_off;
    img.extend_from_slice(&enc_prog);
    let main_off = ((img.len() + 0xFFF) & !0xFFF).max(img.len());
    img.resize(main_off, 0);
    img.extend_from_slice(&main_list);

    let write_slot =
        |dst: &mut [u8], tag: &str, ver: &str, off: u32, sz: u32, dsum: u32, ct: u8| {
            dst.fill(0);
            write_padded_ascii(&mut dst[0..4], tag, 0);
            write_padded_ascii(&mut dst[4..8], ver, 0);
            dst[12..16].copy_from_slice(&off.to_le_bytes());
            write_padded_ascii(&mut dst[16..24], "M8500", 0);
            dst[24] = b'M';
            write_padded_ascii(&mut dst[26..32], "M16360", 0);
            dst[32..36].copy_from_slice(&sz.to_le_bytes());
            dst[36..40].copy_from_slice(&dsum.to_le_bytes());
            dst[43] = ct;
            let hsum = adler32_panasonic_strided(&dst[..44], 1);
            dst[44..48].copy_from_slice(&hsum.to_le_bytes());
        };

    write_slot(
        &mut img[toc_start..toc_start + 48],
        "PROG",
        "0.00",
        prog_off as u32,
        prog_bytes.len() as u32,
        prog_sum,
        0,
    );
    write_slot(
        &mut img[toc_start + 48..toc_start + 96],
        "MAIN",
        "1600",
        main_off as u32,
        main_list.len() as u32,
        main_sum,
        1,
    );

    if has_outer_48b {
        let csz = (img.len() - 48) as u32;
        img[36..40].copy_from_slice(&csz.to_le_bytes());
        let osum = adler32_panasonic_strided(&img[..44], 1);
        img[44..48].copy_from_slice(&osum.to_le_bytes());
    }

    feistel_encrypt_buffer(
        &mut img[toc_start..first_off],
        round_key,
        &PANA_FEISTEL_SBOX_256,
    );
    img
}

#[test]
fn synthetic_bluray_aes_unpack_patch_fma5_and_drop_aes_repack() {
    let ring = FrmKeyring::builtin();
    let aes_pair = &ring.aes_pairs[0];
    let rk = derive_round_key_from_custom_key_8b(&aes_pair.feistel_key_8b);

    let mut kern = vec![0x33u8; 4096];
    let cmd = b"console=ttyS0 fma4=4k:kern fma5=4k:root fma6=4k:res root=/dev/fma5\0";
    kern[..cmd.len()].copy_from_slice(cmd);
    let mut root_fs = build_synthetic_romfs("bd_root_orig", b"init");
    root_fs.resize(4096, 0);
    let mut cram_fs = build_synthetic_cramfs("bd_res_orig", 4096);
    cram_fs.resize(4096, 0);

    let mut main_plain = Vec::new();
    main_plain.extend_from_slice(&kern);
    main_plain.extend_from_slice(&root_fs);
    main_plain.extend_from_slice(&cram_fs);

    let prog = vec![0x90u8; 0x1000];
    let inner_frm = build_synthetic_bluray_feistel_frm(&rk, true, &prog, &main_plain);
    let pana_eusb = wrap_aes_frm(&[inner_frm], aes_pair).unwrap();

    let decoded = unpack_universal_frm(&pana_eusb, &ring).unwrap();
    assert_eq!(decoded.format, UniversalFrmFormat::BlurayAes);
    assert_eq!(decoded.aes_key_name.as_deref(), Some("y2018_aes"));
    assert_eq!(decoded.sub_containers[0].modules.len(), 2);
    assert!(
        decoded.sub_containers[0]
            .modules
            .iter()
            .all(|m| m.data_sum_valid && m.toc_sum_valid)
    );
    assert_eq!(
        decoded.sub_containers[0].modules[1].partitions,
        vec!["fma4", "fma5", "fma6"]
    );

    let new_root = build_synthetic_romfs("bd_root_patched", b"custom_fw_ok");
    let opts = FrmRepackOptions {
        target_mode: FrmTargetMode::Preserve,
        compression: RepackCompression::Lzss,
        drop_aes: false,
        patch_force: true,
        patch_format_hdd: false,
        patch_version_offset: Some(12),
        patch_modules: Vec::new(),
        patch_partitions: vec![("fma5".to_string(), new_root)],
        verify: true,
    };
    let (repacked_aes, rep) = repack_universal_frm(&pana_eusb, &ring, &opts).unwrap();
    assert_eq!(rep.output_format, UniversalFrmFormat::BlurayAes);
    assert!(rep.verified);
    assert_eq!(rep.partitions_patched, vec!["fma5"]);

    let re_decoded = unpack_universal_frm(&repacked_aes, &ring).unwrap();
    let main_mod = &re_decoded.sub_containers[0].modules[1];
    assert_eq!(main_mod.target_flag, 1);
    assert_eq!(main_mod.version_ascii, "1612");
    assert_eq!(main_mod.compression_algo.as_deref(), Some("lzss"));
    let parts = split_main_partitions(&main_mod.payload).unwrap();
    assert!(matches!(
        parts[1].fs_kind,
        PartitionFsKind::RomFs(RomFsHeaderInfo {
            sum_valid: true,
            ref volume_name,
            ..
        }) if volume_name == "bd_root_patched"
    ));
    assert!(matches!(
        parts[2].fs_kind,
        PartitionFsKind::CramFs(CramFsHeaderInfo {
            crc_valid: true,
            ref name,
            ..
        }) if name == "bd_res_orig"
    ));

    // Also test --drop-aes downgrade to BlurayFeistel
    let drop_opts = FrmRepackOptions {
        drop_aes: true,
        ..Default::default()
    };
    let (dropped, drop_rep) = repack_universal_frm(&pana_eusb, &ring, &drop_opts).unwrap();
    assert_eq!(drop_rep.output_format, UniversalFrmFormat::BlurayFeistel);
    assert!(dropped.starts_with(b"PANASONIC\0\0\0"));
}

#[test]
fn workspace_real_legacy_and_bluray_frm_roundtrip() {
    let ws = resolve_workspace_root();
    let ring = FrmKeyring::builtin();

    let eh55 = ws.join("firmware_and_images/PANA_DVD.FRM");
    if eh55.is_file() {
        let raw = std::fs::read(&eh55).unwrap();
        let dec = unpack_universal_frm(&raw, &ring).unwrap();
        assert_eq!(dec.format, UniversalFrmFormat::LegacyDvd48B);
        assert_eq!(dec.sub_containers[0].modules.len(), 3);
        let (_, rep) = repack_universal_frm(
            &raw,
            &ring,
            &FrmRepackOptions {
                target_mode: FrmTargetMode::Sd,
                patch_force: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rep.output_format, UniversalFrmFormat::SdPlain64B);
        assert!(rep.verified);
    }

    for (rel, expected_hdr48) in [
        (
            "firmware_and_images/dmr-xw_bd_series/bs850_bs750_xs350eg/PANA_DVD.FRM",
            false,
        ),
        (
            "firmware_and_images/dmr-xw_bd_series/bs885_bs785_xs385eg/PANA_DVD.FRM",
            true,
        ),
    ] {
        let path = ws.join(rel);
        if path.is_file() {
            let raw = std::fs::read(&path).unwrap();
            let dec = unpack_universal_frm(&raw, &ring).unwrap();
            assert_eq!(dec.format, UniversalFrmFormat::BlurayFeistel);
            assert_eq!(dec.sub_containers[0].has_panasonic_header, expected_hdr48);
            assert!(
                dec.sub_containers[0]
                    .modules
                    .iter()
                    .all(|m| m.data_sum_valid)
            );
            let (_, rep) = repack_universal_frm(
                &raw,
                &ring,
                &FrmRepackOptions {
                    patch_force: true,
                    patch_version_offset: Some(10),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(rep.output_format, UniversalFrmFormat::BlurayFeistel);
            assert!(rep.verified);
        }
    }

    let xw300 = ws.join("firmware_and_images/dmr-xw_bd_series/xw300gn/PANA_DVD.FRM");
    if xw300.is_file() {
        let raw = std::fs::read(&xw300).unwrap();
        let dec = unpack_universal_frm(&raw, &ring).unwrap();
        assert_eq!(dec.format, UniversalFrmFormat::LegacyDvd48B);
        assert_eq!(dec.sub_containers[0].modules.len(), 3);
    }
}
