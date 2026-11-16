//! Edge Feistel overlap transform, statistical key recovery, and `libdownloadlib.so` stream cipher.

use super::super::feistel::{
    PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b, feistel_decrypt_buffer,
    feistel_encrypt_buffer,
};
use crate::constants::{
    PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256, PANA_LIBDOWNLOADLIB_CIPHER_IV,
    PANA_LIBDOWNLOADLIB_CIPHER_STEP,
};

/// Applies head+tail Feistel transform preserving `pana-fw` overlap ordering.
///
/// Decrypts `[..span]` then `[len-span..]`; encrypts `[len-span..]` then `[..span]`.
pub(super) fn apply_edge_crypt_order(
    buf: &mut [u8],
    span: usize,
    round_key: &[u8; 32],
    encrypt: bool,
) {
    let aligned_span = span & !7;
    let aligned_len = buf.len() & !7;
    if aligned_span == 0 || aligned_len < aligned_span {
        return;
    }
    let tail_start = aligned_len - aligned_span;
    if encrypt {
        feistel_encrypt_buffer(
            &mut buf[tail_start..aligned_len],
            round_key,
            &PANA_FEISTEL_SBOX_256,
        );
        feistel_encrypt_buffer(&mut buf[..aligned_span], round_key, &PANA_FEISTEL_SBOX_256);
    } else {
        feistel_decrypt_buffer(&mut buf[..aligned_span], round_key, &PANA_FEISTEL_SBOX_256);
        feistel_decrypt_buffer(
            &mut buf[tail_start..aligned_len],
            round_key,
            &PANA_FEISTEL_SBOX_256,
        );
    }
}

pub(super) fn candidate_round_keys(explicit: Option<&[u8; 32]>) -> Vec<[u8; 32]> {
    if let Some(rk) = explicit {
        return vec![*rk];
    }
    let mut keys = Vec::with_capacity(1 + PANA_DVD_KEY_CATALOG.len());
    keys.push(PANA_FEISTEL_ROUND_KEY_32B);
    for &(_, k8) in PANA_DVD_KEY_CATALOG {
        let derived = derive_round_key_from_custom_key_8b(&k8);
        if derived != PANA_FEISTEL_ROUND_KEY_32B {
            keys.push(derived);
        }
    }
    keys
}

fn count_non_overlapping(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    let mut count = 0usize;
    let mut i = 0usize;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            count += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count
}

/// Statistical Feistel key recovery (`findKeySlow` from `pana-fw`): scores `0x00000000`, `0xFFFFFFFF`, and `PANASONIC\0\0\0`.
pub fn find_feistel_key_slow(block: &[u8]) -> Option<(&'static str, [u8; 32], usize)> {
    let aligned = block.len() & !7;
    if aligned == 0 {
        return None;
    }

    let mut candidates: Vec<(&'static str, [u8; 32])> =
        Vec::with_capacity(1 + PANA_DVD_KEY_CATALOG.len());
    candidates.push(("old_df72", PANA_FEISTEL_ROUND_KEY_32B));
    for &(name, k8) in PANA_DVD_KEY_CATALOG {
        candidates.push((name, derive_round_key_from_custom_key_8b(&k8)));
    }

    let mut best: Option<(&'static str, [u8; 32], usize)> = None;
    for (name, rk) in candidates {
        let mut decrypted = block[..aligned].to_vec();
        feistel_decrypt_buffer(&mut decrypted, &rk, &PANA_FEISTEL_SBOX_256);
        let score = count_non_overlapping(&decrypted, &[0x00; 4])
            + count_non_overlapping(&decrypted, &[0xFF; 4])
            + count_non_overlapping(&decrypted, b"PANASONIC\x00\x00\x00") * 10;
        if score > best.map_or(0, |b| b.2) {
            best = Some((name, rk, score));
        }
    }
    best
}

fn network_stream_transform(data: &mut [u8], decrypt: bool) {
    let mut counter = 0u32;
    for (idx, byte) in data.iter_mut().enumerate() {
        if (idx & 0xFF) == 0 {
            counter = PANA_LIBDOWNLOADLIB_CIPHER_IV;
        }
        let key_byte = (counter >> 8) as u8;
        let out_byte = *byte ^ key_byte;
        let cipher_byte = if decrypt { *byte } else { out_byte };
        *byte = out_byte;
        counter = counter.wrapping_add(u32::from(cipher_byte) + PANA_LIBDOWNLOADLIB_CIPHER_STEP);
    }
}

/// Decrypts network payload using Panasonic's `libdownloadlib.so` stream decipher (`pana-fw/crypto/decipher.go`).
pub fn network_decipher_in_place(data: &mut [u8]) {
    network_stream_transform(data, true);
}

/// Inverse stream cipher for `network_decipher_in_place` (for roundtrip validation and test synthesis).
pub fn network_encipher_in_place(data: &mut [u8]) {
    network_stream_transform(data, false);
}
