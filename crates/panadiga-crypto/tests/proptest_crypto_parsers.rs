#![forbid(unsafe_code)]

use proptest::prelude::*;

use panadiga_crypto::constants::{PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256};
use panadiga_crypto::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, compress_pana_lzss, decompress_pana_lzss,
    derive_round_key_32b, feistel_decrypt_buffer, feistel_encrypt_buffer,
    pack_extrhead_with_options, parse_extrhead_header, unpack_extrhead,
};

fn prop_cfg() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(prop_cfg())]

    #[test]
    fn feistel_encrypt_decrypt_roundtrip_is_exact(
        key_7b in any::<[u8; 7]>(),
        iv in any::<[u8; 4]>(),
        blocks in proptest::collection::vec(any::<[u8; 8]>(), 0..=32),
    ) {
        let original: Vec<u8> = blocks.into_iter().flatten().collect();
        let round_key = derive_round_key_32b(&key_7b, &iv, &PANA_FEISTEL_SBOX_256);
        let mut work = original.clone();
        feistel_encrypt_buffer(&mut work, &round_key, &PANA_FEISTEL_SBOX_256);
        feistel_decrypt_buffer(&mut work, &round_key, &PANA_FEISTEL_SBOX_256);
        prop_assert_eq!(work, original);
    }

    #[test]
    fn pana_lzss_compress_decompress_roundtrip(
        data in proptest::collection::vec(any::<u8>(), 0..=1024),
    ) {
        let compressed = compress_pana_lzss(&data);
        let decompressed = decompress_pana_lzss(&compressed, data.len());
        prop_assert_eq!(decompressed, data);
    }

    #[test]
    fn pana_lzss_hostile_stream_never_panics_or_ooms(
        raw in proptest::collection::vec(any::<u8>(), 0..=512),
        expected_size in prop_oneof![Just(0usize), 1usize..=2048, Just(usize::MAX / 2)],
    ) {
        let out = decompress_pana_lzss(&raw, expected_size);
        prop_assert!(expected_size == 0 || expected_size > 2048 || out.len() <= expected_size);
    }

    #[test]
    fn extrhead_roundtrip_and_hostile_bytes_safe(
        payload in proptest::collection::vec(any::<u8>(), 1..=512),
        mode_idx in 0usize..3,
        algo_idx in 0usize..3,
        hostile in proptest::collection::vec(any::<u8>(), 0..=512),
    ) {
        let (mode, tag) = match mode_idx {
            0 => (ExtrheadCryptMode::Plain, "MAIN"),
            1 => (ExtrheadCryptMode::Main, "MAIN"),
            _ => (ExtrheadCryptMode::Drv, "DRV"),
        };
        let algo = match algo_idx {
            0 => ExtrheadCompressionAlgo::Zlib,
            1 => ExtrheadCompressionAlgo::Gzip,
            _ => ExtrheadCompressionAlgo::Lzss,
        };
        let packed = pack_extrhead_with_options(
            &payload,
            tag,
            0x5700_0000,
            "PROP",
            mode,
            algo,
            &PANA_FEISTEL_ROUND_KEY_32B,
            16,
            1,
        )
        .expect("pack_extrhead_with_options must succeed");
        let unpacked = unpack_extrhead(&packed, ExtrheadCryptMode::Auto)
            .expect("unpack_extrhead Auto must decode packed test container");
        prop_assert_eq!(unpacked.decompressed, payload);
        prop_assert!(unpacked.checksum_valid);
        prop_assert!(unpacked.footer_present);

        let _ = parse_extrhead_header(&hostile);
        let _ = unpack_extrhead(&hostile, ExtrheadCryptMode::Auto);
    }
}
