//! Unit tests for `pana_fw_go` (`MainList`, `DRV`, `RomFS`/`CramFS`, Feistel key recovery, stream cipher).

use flate2::Crc;

use super::super::codec::write_padded_ascii;
use super::super::extrhead::ExtrheadCompressionAlgo;
use super::super::feistel::{
    PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b, feistel_encrypt_buffer,
};
use super::*;
use crate::constants::{
    PANA_CRAMFS_FLAG_FSID_VERSION_2, PANA_CRAMFS_MAGIC, PANA_CRAMFS_SIGNATURE,
    PANA_FEISTEL_SBOX_256, PANA_ROMFS_MAGIC,
};

fn build_synthetic_romfs(vol_name: &str, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(PANA_ROMFS_MAGIC);
    let vol_padded_len = (vol_name.len() + 1 + 15) & !15;
    let total = 16 + vol_padded_len + ((payload.len() + 15) & !15);
    let aligned_total = (total + 3) & !3;
    buf.extend_from_slice(&(aligned_total as u32).to_be_bytes());
    buf.extend_from_slice(&0u32.to_be_bytes());
    let mut vol = vec![0u8; vol_padded_len];
    vol[..vol_name.len()].copy_from_slice(vol_name.as_bytes());
    buf.extend_from_slice(&vol);
    buf.extend_from_slice(payload);
    buf.resize(aligned_total, 0);
    let sum_len = aligned_total.min(512);
    let sum = checksum_affs(&buf[..sum_len]).unwrap();
    buf[12..16].copy_from_slice(&sum.to_be_bytes());
    buf
}

fn build_synthetic_cramfs(name: &str, size: u32) -> Vec<u8> {
    let mut buf = vec![0xAAu8; size as usize];
    buf[0..4].copy_from_slice(&PANA_CRAMFS_MAGIC.to_le_bytes());
    buf[4..8].copy_from_slice(&size.to_le_bytes());
    buf[8..12].copy_from_slice(&PANA_CRAMFS_FLAG_FSID_VERSION_2.to_le_bytes());
    buf[16..32].copy_from_slice(PANA_CRAMFS_SIGNATURE);
    buf[32..36].fill(0);
    write_padded_ascii(&mut buf[48..64], name, 0);
    let mut crc = Crc::new();
    crc.update(&buf);
    let sum = crc.sum();
    buf[32..36].copy_from_slice(&sum.to_le_bytes());
    buf
}

#[test]
fn test_main_list_pack_and_parse_roundtrip_with_overlap() {
    let payload = b"MATSUSHITA AM34 KERNEL AND ROOTFS STREAM ".repeat(64);
    let rk = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[1].1);
    let packed = pack_main_list(
        &payload,
        "DRV ",
        "M6930_MAIN",
        ExtrheadCompressionAlgo::Lzss,
        1024,
        Some(&rk),
        16,
        1,
    )
    .unwrap();

    let unpacked = parse_main_list(&packed, Some(&rk)).unwrap();
    assert!(unpacked.header_checksum_valid);
    assert_eq!(unpacked.entries.len(), 3);
    assert!(unpacked.entries.iter().all(|e| e.checksum_valid));
    assert_eq!(unpacked.data, payload);
}

#[test]
fn test_drv_module_pack_and_extract_roundtrip() {
    let hdr = DriveHeaderInfo {
        manufacturer: "MATSHITA".to_string(),
        model: "DVD-RAM SW-9574".to_string(),
        version: "S211   ".to_string(),
        unknown: 0x01,
    };
    let payload = b"OPTICAL DRIVE SERVO DSP IMAGE DATA ".repeat(400);
    let rk = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[2].1);

    let wrapped = pack_drv_module(
        &payload,
        Some(&hdr),
        Some(ExtrheadCompressionAlgo::Gzip),
        "DRV211",
        &rk,
    )
    .unwrap();
    let unpacked = extract_drv_module(&wrapped, None).unwrap();
    let parsed_hdr = unpacked.header.expect("expected 32B DriveHeader");
    assert_eq!(parsed_hdr.manufacturer, "MATSHITA");
    assert_eq!(parsed_hdr.model, "DVD-RAM SW-9574");
    assert_eq!(unpacked.payload, payload);
    assert!(unpacked.extrhead.unwrap().checksum_valid);
}

#[test]
fn test_split_main_partitions_with_romfs_cramfs_and_swup_addon() {
    let mut p_kern = vec![0x55u8; 4096];
    let cmdline = b"console=ttyS0 fma4=4k:kernel fma5=4k:root fma6=4k:res root=/dev/fma5\0";
    p_kern[16..16 + cmdline.len()].copy_from_slice(cmdline);

    let mut p_root = build_synthetic_romfs("panaroot", b"hello");
    p_root.resize(4096, 0);
    let mut p_cram = build_synthetic_cramfs("res_fs", 4096);
    p_cram.resize(4096, 0);

    let addon_offset = 12_288u32;
    let mut full = Vec::new();
    full.extend_from_slice(b"SWUP_ADDON\0\0");
    full.extend_from_slice(&addon_offset.to_le_bytes());
    full.extend_from_slice(&p_kern[16..]);
    full.extend_from_slice(&p_root);
    full.extend_from_slice(&p_cram);
    full.extend_from_slice(b"ADDON_TAIL_PAYLOAD");

    let slices = split_main_partitions(&full).unwrap();
    assert_eq!(slices.len(), 4);
    assert_eq!(slices[0].name, "fma4");
    assert_eq!(slices[0].fs_kind, PartitionFsKind::Raw);
    assert_eq!(slices[1].name, "fma5");
    assert!(matches!(
        slices[1].fs_kind,
        PartitionFsKind::RomFs(RomFsHeaderInfo {
            sum_valid: true,
            ..
        })
    ));
    assert_eq!(slices[2].name, "fma6");
    assert!(matches!(
        slices[2].fs_kind,
        PartitionFsKind::CramFs(CramFsHeaderInfo {
            crc_valid: true,
            ..
        })
    ));
    assert_eq!(slices[3].name, "SWUP_ADDON");
}

#[test]
fn test_find_feistel_key_slow_and_network_decipher() {
    let mut sample = vec![0xFFu8; 256];
    sample[..12].copy_from_slice(b"PANASONIC\x00\x00\x00");
    sample[64..96].fill(0x00);
    let target_key = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[3].1);
    feistel_encrypt_buffer(&mut sample, &target_key, &PANA_FEISTEL_SBOX_256);

    let (matched_name, matched_rk, score) = find_feistel_key_slow(&sample).unwrap();
    assert_eq!(matched_name, PANA_DVD_KEY_CATALOG[3].0);
    assert_eq!(matched_rk, target_key);
    assert!(score >= 10);

    let orig =
        b"GET /firmware/panasonic/version.xml HTTP/1.1\r\nHost: bdautovchk.svrpf.jp\r\n".repeat(6);
    let mut buf = orig.clone();
    network_encipher_in_place(&mut buf);
    assert_ne!(buf, orig);
    network_decipher_in_place(&mut buf);
    assert_eq!(buf, orig);
}

#[test]
fn test_flipper_kaseikyo_decode_matches_panasonic_dvd_tv_capture() {
    assert_eq!(
        build_kaseikyo_wire_frame(
            0xB0,
            0x00,
            decode_flipper_kaseikyo_command(0x00, 0x30, 0x03)
        ),
        [0x02, 0x20, 0xB0, 0x00, 0x33, 0x83]
    );
    assert_eq!(
        build_kaseikyo_wire_frame(
            0xB0,
            0x00,
            decode_flipper_kaseikyo_command(0x02, 0x20, 0x01)
        ),
        [0x02, 0x20, 0xB0, 0x00, 0x92, 0x22]
    );
    assert_eq!(
        build_kaseikyo_wire_frame(
            0xB0,
            0x00,
            decode_flipper_kaseikyo_command(0x02, 0x20, 0x00)
        ),
        [0x02, 0x20, 0xB0, 0x00, 0x82, 0x32]
    );
    assert_eq!(
        build_kaseikyo_wire_frame(
            0xB0,
            0x00,
            decode_flipper_kaseikyo_command(0x01, 0x70, 0x01)
        ),
        [0x02, 0x20, 0xB0, 0x00, 0x57, 0xE7]
    );

    let ir_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../firmware_and_images/Panasonic_DVD_TV.ir");
    if ir_path.is_file() {
        let raw = std::fs::read_to_string(ir_path).unwrap();
        let count = raw.lines().filter(|l| l.starts_with("name:")).count();
        assert_eq!(count, 50);
    }
}
