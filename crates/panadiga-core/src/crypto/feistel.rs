//! Matsushita MN103S (`BOOT.BIN` `FUN_4400a666`) 64-bit Feistel Block Cipher & Strided Adler-32
//!
//! Pure-Rust implementation reconstructed directly from `BOOT.BIN` (`FUN_44011d42` key schedule,
//! `FUN_44011ec0` block encrypt, `FUN_44012029` block decrypt, `FUN_44012191` F-function, and
//! `FUN_4401e5a3` strided Adler-32 checksum).

/// Reconstructs the 7-byte Feistel master key from split `.data` fragments (`0x50480278` / `0x5048027F`).
pub fn reconstruct_master_key_7b(p1: &[u8; 7], p2: &[u8; 8]) -> [u8; 7] {
    let mut master = [0u8; 7];
    for (idx, dst) in master.iter_mut().enumerate() {
        *dst = p2[idx] ^ p1[idx];
    }
    master[1] ^= p2[7];
    master[4] ^= p2[7];
    master
}

/// Evaluates the 32-bit Matsushita Feistel F-function (`FUN_44012191`).
#[allow(clippy::too_many_arguments)]
pub fn feistel_f_round(
    b0: u8,
    b1: u8,
    b2: u8,
    b3: u8,
    k0: u8,
    k1: u8,
    k2: u8,
    k3: u8,
    sbox: &[u8; 256],
) -> [u8; 4] {
    let v3 = k1 ^ b1;
    let v4 = k0 ^ b0;
    let x3 = k3 ^ b3;
    let x2 = k2 ^ b2;

    let v1 = x3 >> 2;
    let v5 = x3.rotate_right(2);
    let v2 = x2 >> 5;
    let v6 = x2.rotate_right(5);

    let o3 = (v6 >> 1) | (v1 << 7);
    let o2 = (v3 & 0x7F) | (v2 << 7);
    let o1 = (v4 >> 1) | (v3 & 0x80);
    let o0 = (v5 >> 1) | (v4 << 7);

    let s = sbox[(v4 ^ v3.rotate_left(1) ^ v6 ^ v5) as usize];
    [
        o0.wrapping_add(s.rotate_left(3)),
        o1.wrapping_add(s.rotate_left(2)),
        o2.wrapping_add(s.rotate_left(1)),
        o3.wrapping_add(s),
    ]
}

/// Expands the 7-byte master key and 4-byte IV into the 32-byte round-key table (`FUN_44011d42`).
pub fn derive_round_key_32b(key_7b: &[u8; 7], iv: &[u8; 4], sbox: &[u8; 256]) -> [u8; 32] {
    let mut rk = [0u8; 32];
    rk[..7].copy_from_slice(key_7b);
    let mut state = *iv;

    for round in 0..16usize {
        let off = (round & 7) * 4;
        let next = feistel_f_round(
            rk[off],
            rk[off + 1],
            rk[off + 2],
            rk[off + 3],
            state[0],
            state[1],
            state[2],
            state[3],
            sbox,
        );
        rk[off..off + 4].copy_from_slice(&next);
        state = next;
    }

    rk
}

fn run_feistel_rounds(
    block: &[u8; 8],
    round_key: &[u8; 32],
    sbox: &[u8; 256],
    decrypt: bool,
) -> [u8; 8] {
    let mut l = [block[0], block[1], block[2], block[3]];
    let mut r = [block[4], block[5], block[6], block[7]];

    for step in 0..16usize {
        let round = if decrypt { 15 - step } else { step };
        let k_off = (round & 7) * 4;
        let f_out = feistel_f_round(
            l[0],
            l[1],
            l[2],
            l[3],
            round_key[k_off],
            round_key[k_off + 1],
            round_key[k_off + 2],
            round_key[k_off + 3],
            sbox,
        );
        let prev_l = l;
        for idx in 0..4 {
            l[idx] = f_out[idx] ^ r[idx];
        }
        r = prev_l;
    }

    [r[0], r[1], r[2], r[3], l[0], l[1], l[2], l[3]]
}

/// Decrypts a single 64-bit ECB block (`FUN_44012029`).
pub fn feistel_decrypt_block(block: &[u8; 8], round_key: &[u8; 32], sbox: &[u8; 256]) -> [u8; 8] {
    run_feistel_rounds(block, round_key, sbox, true)
}

/// Encrypts a single 64-bit ECB block (`FUN_44011ec0`).
pub fn feistel_encrypt_block(block: &[u8; 8], round_key: &[u8; 32], sbox: &[u8; 256]) -> [u8; 8] {
    run_feistel_rounds(block, round_key, sbox, false)
}

fn transform_buffer_in_place(
    data: &mut [u8],
    round_key: &[u8; 32],
    sbox: &[u8; 256],
    decrypt: bool,
) {
    for chunk in data.as_chunks_mut::<8>().0 {
        *chunk = run_feistel_rounds(chunk, round_key, sbox, decrypt);
    }
}

/// Decrypts an aligned buffer in-place in 8-byte blocks (`FUN_4400a666`).
pub fn feistel_decrypt_buffer(data: &mut [u8], round_key: &[u8; 32], sbox: &[u8; 256]) {
    transform_buffer_in_place(data, round_key, sbox, true);
}

/// Encrypts an aligned buffer in-place in 8-byte blocks.
pub fn feistel_encrypt_buffer(data: &mut [u8], round_key: &[u8; 32], sbox: &[u8; 256]) {
    transform_buffer_in_place(data, round_key, sbox, false);
}

/// Computes Panasonic's strided Adler-32 checksum (`FUN_4401e5a3`, `s1 | (s2 << 16)`).
pub fn adler32_panasonic_strided(data: &[u8], step: usize) -> u32 {
    const MOD_ADLER: u32 = 65_521;
    let stride = step.max(1);
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;

    for &byte in data.iter().step_by(stride) {
        s1 = (s1 + u32::from(byte)) % MOD_ADLER;
        s2 = (s2 + s1) % MOD_ADLER;
    }

    s1 | (s2 << 16)
}

/// 12 verified 8-byte PANA_DVD Feistel keys catalog.
pub const PANA_DVD_KEY_CATALOG: &[(&str, [u8; 8])] = &[
    ("old_df72", [0xDF, 0x72, 0x15, 0xC7, 0x42, 0x7F, 0xD0, 0x10]),
    (
        "le2011_08e0",
        [0x08, 0xE0, 0x3D, 0x85, 0x9A, 0xF9, 0xF3, 0xEE],
    ),
    (
        "y2011_af4d",
        [0xAF, 0x4D, 0x5B, 0x16, 0xC2, 0x98, 0xC1, 0x6D],
    ),
    (
        "y2012_9407",
        [0x94, 0x07, 0x31, 0x1B, 0x75, 0x61, 0xF9, 0x7E],
    ),
    (
        "y2012_5d88",
        [0x5D, 0x88, 0x71, 0x97, 0x45, 0xA4, 0xEF, 0xFF],
    ),
    (
        "y2013_38bb",
        [0x38, 0xBB, 0x0C, 0x17, 0xD3, 0x62, 0x70, 0x1F],
    ),
    (
        "y2013_ec23",
        [0xEC, 0x23, 0x8C, 0xD7, 0x91, 0x45, 0x66, 0x46],
    ),
    (
        "hualu_0a1e",
        [0x0A, 0x1E, 0xE6, 0xC5, 0xBD, 0xC4, 0x93, 0x3F],
    ),
    (
        "y2014_8f8c",
        [0x8F, 0x8C, 0xA8, 0x84, 0x82, 0xE4, 0x3D, 0x9B],
    ),
    (
        "y2014_21fc",
        [0x21, 0xFC, 0xA1, 0x73, 0x61, 0xE6, 0x6B, 0x4E],
    ),
    (
        "y2016_fdfe",
        [0xFD, 0xFE, 0x0B, 0xEE, 0x4C, 0x90, 0xFA, 0x2C],
    ),
    (
        "y2018_06c9",
        [0x06, 0xC9, 0x43, 0xF3, 0xB9, 0x97, 0xF7, 0xE0],
    ),
];

/// Derives the 32-byte Feistel round key from an 8-byte XORed key (`FUN_29f055dc`).
pub fn derive_round_key_from_custom_key_8b(xored_key: &[u8; 8]) -> [u8; 32] {
    let mut key_7b = [0u8; 7];
    key_7b.copy_from_slice(&xored_key[..7]);
    key_7b[1] ^= xored_key[7];
    key_7b[4] ^= xored_key[7];
    derive_round_key_32b(
        &key_7b,
        &crate::constants::PANA_FEISTEL_KEYSCHED_IV,
        &crate::constants::PANA_FEISTEL_SBOX_256,
    )
}

/// Detects the Feistel round key and label matching the provided header buffer.
///
/// Tests `PANA_FEISTEL_ROUND_KEY_32B` first, then each key in `PANA_DVD_KEY_CATALOG`.
/// Checks prefixes `b"PROG"`, `b"DRV "`, `b"MAIN"`, and `b"STB "` at offset `0` or `48`.
pub fn detect_frm_feistel_key(header_64b: &[u8]) -> Option<(&'static str, [u8; 32])> {
    const PREFIXES: [&[u8; 4]; 4] = [b"PROG", b"DRV ", b"MAIN", b"STB "];
    let check_key = |rk: &[u8; 32]| -> bool {
        if header_64b.len() >= 8 {
            let mut blk = [0u8; 8];
            blk.copy_from_slice(&header_64b[..8]);
            let dec = feistel_decrypt_block(&blk, rk, &crate::constants::PANA_FEISTEL_SBOX_256);
            if PREFIXES.iter().any(|pfx| dec.starts_with(*pfx)) {
                return true;
            }
        }
        if header_64b.len() >= 56 {
            let mut blk = [0u8; 8];
            blk.copy_from_slice(&header_64b[48..56]);
            let dec = feistel_decrypt_block(&blk, rk, &crate::constants::PANA_FEISTEL_SBOX_256);
            if PREFIXES.iter().any(|pfx| dec.starts_with(*pfx)) {
                return true;
            }
        }
        false
    };

    if check_key(&crate::constants::PANA_FEISTEL_ROUND_KEY_32B) {
        return Some(("old_df72", crate::constants::PANA_FEISTEL_ROUND_KEY_32B));
    }

    for &(name, key_8b) in PANA_DVD_KEY_CATALOG {
        let rk = derive_round_key_from_custom_key_8b(&key_8b);
        if check_key(&rk) {
            return Some((name, rk));
        }
    }

    None
}

#[cfg(test)]
mod tests {
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
}
