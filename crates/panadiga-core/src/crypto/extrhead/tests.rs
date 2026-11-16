//! Unit tests for `extrhead` (`LZSS`, zlib/gzip, multi-key auto-detection, nested containers).

use super::super::feistel::{
    PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b, feistel_encrypt_buffer,
};
use super::*;
use crate::constants::{PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256};

#[test]
fn test_decompress_pana_lzss_vector() {
    // Encodes b"PANASONIC PANASONIC"
    let compressed = [
        0xFF, b'P', b'A', b'N', b'A', b'S', b'O', b'N', b'I', 0xFB, b'C', b' ', 0xEE, 0xF6,
    ];
    let decomp = decompress_pana_lzss(&compressed, 19);
    assert_eq!(&decomp, b"PANASONIC PANASONIC");
}

#[test]
fn test_compress_and_decompress_pana_lzss_roundtrip() {
    let mut payload = b"PANASONIC DIGA DMR-EH55 AM34 FIRMWARE TEST PATTERN! ".repeat(120);
    payload.extend_from_slice(&vec![0xAAu8; 512]);
    payload.extend_from_slice(&(0u8..=255).collect::<Vec<u8>>());

    let compressed = compress_pana_lzss(&payload);
    assert!(compressed.len() < payload.len());
    let decomp = decompress_pana_lzss(&compressed, payload.len());
    assert_eq!(decomp, payload);

    let decomp_auto = decompress_pana_lzss(&compressed, 0);
    assert_eq!(decomp_auto, payload);
}

#[test]
fn test_pack_and_unpack_extrhead_algos_multikey_and_nested() {
    let payload = b"PANASONIC PANASONIC ".repeat(32);
    let custom_key = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[1].1);

    for algo in [
        ExtrheadCompressionAlgo::Zlib,
        ExtrheadCompressionAlgo::Gzip,
        ExtrheadCompressionAlgo::Lzss,
    ] {
        let packed = pack_extrhead_with_options(
            &payload,
            "MAIN",
            0x5040_0000,
            "TEST01",
            ExtrheadCryptMode::Main,
            algo,
            &custom_key,
            16,
            1,
        )
        .unwrap();
        let unpacked = unpack_extrhead(&packed, ExtrheadCryptMode::Auto).unwrap();
        assert_eq!(unpacked.header.algo, algo as u16);
        assert_eq!(unpacked.decompressed, payload);
        assert!(unpacked.checksum_valid);
        assert!(unpacked.footer_present);
    }

    // Test recursive EXTRHEAD (inner LZSS wrapped inside outer GZIP) + OldMain double-encrypt.
    let inner = pack_extrhead_with_options(
        &payload,
        "MAIN",
        0,
        "INNER",
        ExtrheadCryptMode::Plain,
        ExtrheadCompressionAlgo::Lzss,
        &PANA_FEISTEL_ROUND_KEY_32B,
        16,
        1,
    )
    .unwrap();
    let mut outer = pack_extrhead_with_options(
        &inner,
        "MAIN",
        0,
        "OUTER",
        ExtrheadCryptMode::Main,
        ExtrheadCompressionAlgo::Gzip,
        &PANA_FEISTEL_ROUND_KEY_32B,
        16,
        1,
    )
    .unwrap();
    // Add outer full-buffer encryption pass (OldMain double encryption)
    feistel_encrypt_buffer(
        &mut outer,
        &PANA_FEISTEL_ROUND_KEY_32B,
        &PANA_FEISTEL_SBOX_256,
    );

    let unpacked_nested = unpack_extrhead(&outer, ExtrheadCryptMode::Main).unwrap();
    assert_eq!(unpacked_nested.decompressed, payload);
    assert!(unpacked_nested.checksum_valid);
    assert!(unpacked_nested.footer_present);
}
