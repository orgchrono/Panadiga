#![deny(unsafe_code)]

//! Pure-Rust FIPS-197 AES-128 block and CBC (nopad) primitives.

use anyhow::{Result, bail};

const AES_SBOX: [u8; 256] = [
    0x63, 0x7C, 0x77, 0x7B, 0xF2, 0x6B, 0x6F, 0xC5, 0x30, 0x01, 0x67, 0x2B, 0xFE, 0xD7, 0xAB, 0x76,
    0xCA, 0x82, 0xC9, 0x7D, 0xFA, 0x59, 0x47, 0xF0, 0xAD, 0xD4, 0xA2, 0xAF, 0x9C, 0xA4, 0x72, 0xC0,
    0xB7, 0xFD, 0x93, 0x26, 0x36, 0x3F, 0xF7, 0xCC, 0x34, 0xA5, 0xE5, 0xF1, 0x71, 0xD8, 0x31, 0x15,
    0x04, 0xC7, 0x23, 0xC3, 0x18, 0x96, 0x05, 0x9A, 0x07, 0x12, 0x80, 0xE2, 0xEB, 0x27, 0xB2, 0x75,
    0x09, 0x83, 0x2C, 0x1A, 0x1B, 0x6E, 0x5A, 0xA0, 0x52, 0x3B, 0xD6, 0xB3, 0x29, 0xE3, 0x2F, 0x84,
    0x53, 0xD1, 0x00, 0xED, 0x20, 0xFC, 0xB1, 0x5B, 0x6A, 0xCB, 0xBE, 0x39, 0x4A, 0x4C, 0x58, 0xCF,
    0xD0, 0xEF, 0xAA, 0xFB, 0x43, 0x4D, 0x33, 0x85, 0x45, 0xF9, 0x02, 0x7F, 0x50, 0x3C, 0x9F, 0xA8,
    0x51, 0xA3, 0x40, 0x8F, 0x92, 0x9D, 0x38, 0xF5, 0xBC, 0xB6, 0xDA, 0x21, 0x10, 0xFF, 0xF3, 0xD2,
    0xCD, 0x0C, 0x13, 0xEC, 0x5F, 0x97, 0x44, 0x17, 0xC4, 0xA7, 0x7E, 0x3D, 0x64, 0x5D, 0x19, 0x73,
    0x60, 0x81, 0x4F, 0xDC, 0x22, 0x2A, 0x90, 0x88, 0x46, 0xEE, 0xB8, 0x14, 0xDE, 0x5E, 0x0B, 0xDB,
    0xE0, 0x32, 0x3A, 0x0A, 0x49, 0x06, 0x24, 0x5C, 0xC2, 0xD3, 0xAC, 0x62, 0x91, 0x95, 0xE4, 0x79,
    0xE7, 0xC8, 0x37, 0x6D, 0x8D, 0xD5, 0x4E, 0xA9, 0x6C, 0x56, 0xF4, 0xEA, 0x65, 0x7A, 0xAE, 0x08,
    0xBA, 0x78, 0x25, 0x2E, 0x1C, 0xA6, 0xB4, 0xC6, 0xE8, 0xDD, 0x74, 0x1F, 0x4B, 0xBD, 0x8B, 0x8A,
    0x70, 0x3E, 0xB5, 0x66, 0x48, 0x03, 0xF6, 0x0E, 0x61, 0x35, 0x57, 0xB9, 0x86, 0xC1, 0x1D, 0x9E,
    0xE1, 0xF8, 0x98, 0x11, 0x69, 0xD9, 0x8E, 0x94, 0x9B, 0x1E, 0x87, 0xE9, 0xCE, 0x55, 0x28, 0xDF,
    0x8C, 0xA1, 0x89, 0x0D, 0xBF, 0xE6, 0x42, 0x68, 0x41, 0x99, 0x2D, 0x0F, 0xB0, 0x54, 0xBB, 0x16,
];

const fn build_inv_sbox() -> [u8; 256] {
    let mut inv = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        inv[AES_SBOX[i] as usize] = i as u8;
        i += 1;
    }
    inv
}

const AES_INV_SBOX: [u8; 256] = build_inv_sbox();

const RCON: [u8; 11] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36,
];

#[inline]
fn xtime(b: u8) -> u8 {
    (b << 1) ^ (((b >> 7) & 1) * 0x1B)
}

#[inline]
fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut res = 0u8;
    for _ in 0..8 {
        if (b & 1) != 0 {
            res ^= a;
        }
        a = xtime(a);
        b >>= 1;
    }
    res
}

fn expand_aes128_key(key: &[u8; 16]) -> [[u8; 16]; 11] {
    let mut rk = [0u8; 176];
    rk[..16].copy_from_slice(key);
    for w_idx in 4..44usize {
        let prev = (w_idx - 1) * 4;
        let mut t = [rk[prev], rk[prev + 1], rk[prev + 2], rk[prev + 3]];
        if w_idx % 4 == 0 {
            t = [
                AES_SBOX[t[1] as usize] ^ RCON[w_idx / 4],
                AES_SBOX[t[2] as usize],
                AES_SBOX[t[3] as usize],
                AES_SBOX[t[0] as usize],
            ];
        }
        let back = (w_idx - 4) * 4;
        let dst = w_idx * 4;
        for j in 0..4 {
            rk[dst + j] = rk[back + j] ^ t[j];
        }
    }
    let mut rounds = [[0u8; 16]; 11];
    for (r, out) in rounds.iter_mut().enumerate() {
        out.copy_from_slice(&rk[r * 16..(r + 1) * 16]);
    }
    rounds
}

fn add_round_key(state: &mut [u8; 16], rk: &[u8; 16]) {
    for (s, &k) in state.iter_mut().zip(rk.iter()) {
        *s ^= k;
    }
}

fn shift_rows_dir(s: &mut [u8; 16], inverse: bool) {
    // Column-major 4x4 layout: byte index = col * 4 + row.
    for r in 1..4usize {
        let mut row = [s[r], s[4 + r], s[8 + r], s[12 + r]];
        if inverse {
            row.rotate_right(r);
        } else {
            row.rotate_left(r);
        }
        s[r] = row[0];
        s[4 + r] = row[1];
        s[8 + r] = row[2];
        s[12 + r] = row[3];
    }
}

fn mix_columns_with(state: &mut [u8; 16], matrix: [u8; 4]) {
    for col in 0..4usize {
        let base = col * 4;
        let a = [
            state[base],
            state[base + 1],
            state[base + 2],
            state[base + 3],
        ];
        for r in 0..4usize {
            state[base + r] = gf_mul(a[0], matrix[(4 - r) & 3])
                ^ gf_mul(a[1], matrix[(5 - r) & 3])
                ^ gf_mul(a[2], matrix[(6 - r) & 3])
                ^ gf_mul(a[3], matrix[(7 - r) & 3]);
        }
    }
}

/// Encrypts a single 16-byte block with AES-128 (FIPS-197).
pub fn aes128_encrypt_block(key: &[u8; 16], block: &[u8; 16]) -> [u8; 16] {
    let rks = expand_aes128_key(key);
    let mut state = *block;
    add_round_key(&mut state, &rks[0]);
    for rk in &rks[1..10] {
        for b in &mut state {
            *b = AES_SBOX[*b as usize];
        }
        shift_rows_dir(&mut state, false);
        mix_columns_with(&mut state, [0x02, 0x03, 0x01, 0x01]);
        add_round_key(&mut state, rk);
    }
    for b in &mut state {
        *b = AES_SBOX[*b as usize];
    }
    shift_rows_dir(&mut state, false);
    add_round_key(&mut state, &rks[10]);
    state
}

/// Decrypts a single 16-byte block with AES-128 (FIPS-197).
pub fn aes128_decrypt_block(key: &[u8; 16], block: &[u8; 16]) -> [u8; 16] {
    let rks = expand_aes128_key(key);
    let mut state = *block;
    add_round_key(&mut state, &rks[10]);
    for rk in rks[1..10].iter().rev() {
        shift_rows_dir(&mut state, true);
        for b in &mut state {
            *b = AES_INV_SBOX[*b as usize];
        }
        add_round_key(&mut state, rk);
        mix_columns_with(&mut state, [0x0E, 0x0B, 0x0D, 0x09]);
    }
    shift_rows_dir(&mut state, true);
    for b in &mut state {
        *b = AES_INV_SBOX[*b as usize];
    }
    add_round_key(&mut state, &rks[0]);
    state
}

fn xor_block_16(dst: &mut [u8; 16], mask: &[u8; 16]) {
    for i in 0..16 {
        dst[i] ^= mask[i];
    }
}

fn ensure_cbc_aligned(len: usize) -> Result<()> {
    if !len.is_multiple_of(16) {
        bail!("AES-128-CBC requires 16-byte aligned buffer, got {len} B");
    }
    Ok(())
}

/// Encrypts a 16-byte-aligned slice in-place using AES-128-CBC (nopad).
pub fn aes128_cbc_encrypt_in_place(key: &[u8; 16], iv: &[u8; 16], data: &mut [u8]) -> Result<()> {
    ensure_cbc_aligned(data.len())?;
    let mut prev = *iv;
    for chunk in data.as_chunks_mut::<16>().0 {
        xor_block_16(chunk, &prev);
        let enc = aes128_encrypt_block(key, chunk);
        *chunk = enc;
        prev = enc;
    }
    Ok(())
}

/// Decrypts a 16-byte-aligned slice in-place using AES-128-CBC (nopad).
pub fn aes128_cbc_decrypt_in_place(key: &[u8; 16], iv: &[u8; 16], data: &mut [u8]) -> Result<()> {
    ensure_cbc_aligned(data.len())?;
    let mut prev = *iv;
    for chunk in data.as_chunks_mut::<16>().0 {
        let cipher_blk = *chunk;
        *chunk = aes128_decrypt_block(key, &cipher_blk);
        xor_block_16(chunk, &prev);
        prev = cipher_blk;
    }
    Ok(())
}
