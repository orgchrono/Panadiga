#![deny(unsafe_code)]

//! Pure-Rust FIPS-197 AES-128-CBC (nopad), Panasonic Blu-ray Keyring (`2016`/`2018` + `keys.ukf`),
//! and `libdownloadlib.so` Network Key Derivation.

use std::path::Path;

use anyhow::{Context, Result, bail};

use super::feistel::{PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b};
use super::pana_fw_go::network_decipher_in_place;
use crate::constants::PANA_FEISTEL_ROUND_KEY_32B;

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

/// Catalog entry `(name, aes_key_16b, aes_iv_16b, feistel_key_8b)`.
pub type AesCatalogEntry = (&'static str, [u8; 16], [u8; 16], [u8; 8]);

/// Built-in 2016/2018 Panasonic Blu-ray AES-128-CBC + Feistel pairs (`pana-fw` & `unixtract`).
pub const PANA_DVD_AES_CATALOG: &[AesCatalogEntry] = &[
    (
        "y2018_aes",
        [
            0x62, 0xA3, 0x9E, 0x1C, 0x55, 0x94, 0xAE, 0x09, 0x24, 0x4E, 0xB3, 0x26, 0xEF, 0x79,
            0x38, 0xFA,
        ],
        [
            0x46, 0xD0, 0x3C, 0x52, 0xB1, 0xA3, 0x98, 0xFA, 0x03, 0x27, 0x6F, 0xDE, 0x7C, 0xC4,
            0xEA, 0x26,
        ],
        [0x06, 0xC9, 0x43, 0xF3, 0xB9, 0x97, 0xF7, 0xE0],
    ),
    (
        "y2016_aes",
        [
            0xA6, 0x92, 0x60, 0x51, 0x94, 0xCF, 0x7F, 0x49, 0x17, 0xE5, 0x05, 0x21, 0xC1, 0xDF,
            0x0B, 0xBA,
        ],
        [
            0xF8, 0x0A, 0x6B, 0xCA, 0xD5, 0x7A, 0xA6, 0x7C, 0x3C, 0x60, 0xE3, 0xBE, 0x75, 0x5E,
            0x05, 0xDF,
        ],
        [0xFD, 0xFE, 0x0B, 0xEE, 0x4C, 0x90, 0xFA, 0x2C],
    ),
];

/// Derives Panasonic's `libdownloadlib.so` OTA network `(AES Key, AES IV)` pair.
pub fn derive_network_key_iv() -> ([u8; 16], [u8; 16]) {
    let mut raw: [u8; 32] = [
        0xD4, 0xAC, 0x76, 0x2B, 0xE8, 0x60, 0xD0, 0xF3, 0x5F, 0x15, 0xFC, 0x26, 0x6D, 0xD8, 0x91,
        0x1C, 0xDE, 0xFF, 0xE6, 0x39, 0x97, 0x75, 0xBB, 0x9E, 0xFF, 0x69, 0x3B, 0x89, 0xAA, 0xD4,
        0x12, 0x82,
    ];
    network_decipher_in_place(&mut raw);
    let mut key = [0u8; 16];
    let mut iv = [0u8; 16];
    key.copy_from_slice(&raw[..16]);
    iv.copy_from_slice(&raw[16..]);
    (key, iv)
}

/// Named 64-bit Feistel key entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanaFeistelKey {
    pub name: String,
    pub key_8b: [u8; 8],
}

/// Named AES-128-CBC key + IV + paired 64-bit Feistel key (`PANAEUSB.FRM`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanaAesKeyPair {
    pub name: String,
    pub aes_key: [u8; 16],
    pub aes_iv: [u8; 16],
    pub feistel_key_8b: [u8; 8],
}

/// Combined Legacy + Blu-ray keyring supporting built-in keys and `unixtract` `keys.ukf` files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrmKeyring {
    pub feistel_keys: Vec<PanaFeistelKey>,
    pub aes_pairs: Vec<PanaAesKeyPair>,
}

impl Default for FrmKeyring {
    fn default() -> Self {
        Self::builtin()
    }
}

fn extract_ukf_hex_tokens(line: &str) -> Result<Vec<Vec<u8>>> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find("x\"") {
        let after = &rest[pos + 2..];
        let Some(end) = after.find('"') else {
            bail!("Unterminated x\"...\" hex token in keys.ukf: {line}");
        };
        let bytes = hex::decode(after[..end].trim())
            .with_context(|| format!("Decoding keys.ukf hex literal '{}'", &after[..end]))?;
        out.push(bytes);
        rest = &after[end + 1..];
    }
    Ok(out)
}

fn extract_first_quoted(line: &str) -> Option<&str> {
    let start = line.find('"')?;
    let tail = &line[start + 1..];
    let end = tail.find('"')?;
    Some(&tail[..end])
}

impl FrmKeyring {
    /// Returns the built-in keyring containing all 12 Feistel keys and `2016`/`2018` AES pairs.
    pub fn builtin() -> Self {
        let feistel_keys = PANA_DVD_KEY_CATALOG
            .iter()
            .map(|&(name, key_8b)| PanaFeistelKey {
                name: name.to_string(),
                key_8b,
            })
            .collect();
        let aes_pairs = PANA_DVD_AES_CATALOG
            .iter()
            .map(|&(name, aes_key, aes_iv, feistel_key_8b)| PanaAesKeyPair {
                name: name.to_string(),
                aes_key,
                aes_iv,
                feistel_key_8b,
            })
            .collect();
        Self {
            feistel_keys,
            aes_pairs,
        }
    }

    /// Creates a keyring initialized with built-in keys plus entries loaded from `path` (`keys.ukf`).
    pub fn with_ukf_file(path: &Path) -> Result<Self> {
        let mut ring = Self::builtin();
        ring.merge_ukf_file(path)?;
        Ok(ring)
    }

    /// Reads and merges an external `keys.ukf` file using name-precedence upsert.
    pub fn merge_ukf_file(&mut self, path: &Path) -> Result<usize> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("Reading keys.ukf from {}", path.display()))?;
        self.merge_ukf_str(&raw)
    }

    /// Loads deduplicated union of built-in -> env `PANADIGA_KEY_FILE` -> `keys.local.ukf` -> CLI `--key-file`.
    /// Higher-precedence entries override matching key names without dropping built-in or older keys.
    pub fn load_cascading(
        env_key_file: Option<&Path>,
        local_key_file: Option<&Path>,
        cli_key_file: Option<&Path>,
    ) -> Result<Self> {
        let mut ring = Self::builtin();
        if let Some(p) = env_key_file {
            ring.merge_ukf_file(p)
                .with_context(|| format!("Loading PANADIGA_KEY_FILE {}", p.display()))?;
        }
        if let Some(p) = local_key_file
            && p.is_file()
        {
            ring.merge_ukf_file(p)?;
        }
        if let Some(p) = cli_key_file {
            ring.merge_ukf_file(p)?;
        }
        Ok(ring)
    }

    /// Loads default cascading keyring from `PANADIGA_KEY_FILE`, `./keys.local.ukf`, and CLI `--key-file`.
    pub fn load_default_with_cli(cli_key_file: Option<&Path>) -> Result<Self> {
        let env_raw = std::env::var_os("PANADIGA_KEY_FILE")
            .map(|v| v.to_string_lossy().trim().to_string())
            .filter(|s| !s.is_empty());
        let env_path = env_raw.as_deref().map(Path::new);
        let local_path = Path::new("keys.local.ukf");
        Self::load_cascading(env_path, Some(local_path), cli_key_file)
    }

    fn push_feistel_unique(&mut self, name: String, key_8b: [u8; 8]) -> bool {
        if let Some(existing) = self.feistel_keys.iter_mut().find(|k| k.name == name) {
            if existing.key_8b == key_8b {
                return false;
            }
            existing.key_8b = key_8b;
            return true;
        }
        if self.feistel_keys.iter().any(|k| k.key_8b == key_8b) {
            return false;
        }
        self.feistel_keys.push(PanaFeistelKey { name, key_8b });
        true
    }

    fn push_aes_unique(&mut self, pair: PanaAesKeyPair) -> bool {
        self.push_feistel_unique(format!("{}_feistel", pair.name), pair.feistel_key_8b);
        if let Some(existing) = self.aes_pairs.iter_mut().find(|p| p.name == pair.name) {
            if existing.aes_key == pair.aes_key
                && existing.aes_iv == pair.aes_iv
                && existing.feistel_key_8b == pair.feistel_key_8b
            {
                return false;
            }
            *existing = pair;
            return true;
        }
        if self.aes_pairs.iter().any(|p| {
            p.aes_key == pair.aes_key
                && p.aes_iv == pair.aes_iv
                && p.feistel_key_8b == pair.feistel_key_8b
        }) {
            return false;
        }
        self.aes_pairs.push(pair);
        true
    }

    /// Parses `PANA_DVD_KEYONLY` and `PANA_DVD_AESPAIR` collections from a `keys.ukf` string.
    pub fn merge_ukf_str(&mut self, text: &str) -> Result<usize> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum ActiveCollection {
            None,
            KeyOnly,
            AesPair,
        }

        let mut active = ActiveCollection::None;
        let mut added = 0usize;

        for raw_line in text.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with("collection ") {
                if let Some(col_name) = extract_first_quoted(line) {
                    active = match col_name {
                        "PANA_DVD_KEYONLY" => ActiveCollection::KeyOnly,
                        "PANA_DVD_AESPAIR" => ActiveCollection::AesPair,
                        _ => ActiveCollection::None,
                    };
                }
                continue;
            }
            if active != ActiveCollection::None && line.starts_with('}') {
                active = ActiveCollection::None;
                continue;
            }
            if active == ActiveCollection::None {
                continue;
            }
            let Some(entry_name) = extract_first_quoted(line) else {
                continue;
            };
            let tokens = extract_ukf_hex_tokens(line)?;
            match active {
                ActiveCollection::KeyOnly if !tokens.is_empty() && tokens[0].len() == 8 => {
                    let mut key_8b = [0u8; 8];
                    key_8b.copy_from_slice(&tokens[0]);
                    if self.push_feistel_unique(entry_name.to_string(), key_8b) {
                        added += 1;
                    }
                }
                ActiveCollection::AesPair
                    if tokens.len() >= 3
                        && tokens[0].len() == 16
                        && tokens[1].len() == 16
                        && tokens[2].len() == 8 =>
                {
                    let mut aes_key = [0u8; 16];
                    let mut aes_iv = [0u8; 16];
                    let mut feistel_key_8b = [0u8; 8];
                    aes_key.copy_from_slice(&tokens[0]);
                    aes_iv.copy_from_slice(&tokens[1]);
                    feistel_key_8b.copy_from_slice(&tokens[2]);
                    if self.push_aes_unique(PanaAesKeyPair {
                        name: entry_name.to_string(),
                        aes_key,
                        aes_iv,
                        feistel_key_8b,
                    }) {
                        added += 1;
                    }
                }
                _ => {}
            }
        }

        Ok(added)
    }

    /// Expands all unique 32-byte Feistel round keys with `"old_df72"` prioritized first.
    pub fn feistel_round_key_candidates(&self) -> Vec<(String, [u8; 32])> {
        let mut candidates = Vec::with_capacity(1 + self.feistel_keys.len());
        candidates.push(("old_df72".to_string(), PANA_FEISTEL_ROUND_KEY_32B));
        for k in &self.feistel_keys {
            let rk = derive_round_key_from_custom_key_8b(&k.key_8b);
            if rk != PANA_FEISTEL_ROUND_KEY_32B {
                candidates.push((k.name.clone(), rk));
            }
        }
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes128_fips197_and_sp800_38a_vectors() {
        // FIPS-197 Appendix B
        let key = hex::decode("2b7e151628aed2a6abf7158809cf4f3c").unwrap();
        let pt = hex::decode("3243f6a8885a308d313198a2e0370734").unwrap();
        let ct = hex::decode("3925841d02dc09fbdc118597196a0b32").unwrap();
        let k: [u8; 16] = key.try_into().unwrap();
        let p: [u8; 16] = pt.try_into().unwrap();
        let c: [u8; 16] = ct.try_into().unwrap();
        assert_eq!(aes128_encrypt_block(&k, &p), c);
        assert_eq!(aes128_decrypt_block(&k, &c), p);

        // NIST SP 800-38A F.2.1 CBC-AES128.Encrypt (2 blocks)
        let iv: [u8; 16] = hex::decode("000102030405060708090a0b0c0d0e0f")
            .unwrap()
            .try_into()
            .unwrap();
        let cbc_pt =
            hex::decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51")
                .unwrap();
        let cbc_ct =
            hex::decode("7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2")
                .unwrap();
        let mut buf = cbc_pt.clone();
        aes128_cbc_encrypt_in_place(&k, &iv, &mut buf).unwrap();
        assert_eq!(buf, cbc_ct);
        aes128_cbc_decrypt_in_place(&k, &iv, &mut buf).unwrap();
        assert_eq!(buf, cbc_pt);
    }

    #[test]
    fn network_key_derivation_and_ukf_merge() {
        let (nkey, niv) = derive_network_key_iv();
        assert_eq!(hex::encode(nkey), "d73644e288975ed5e24117a57468d9c3");
        assert_eq!(hex::encode(niv), "a8f24304441eb907cfa1647f27f0aed1");

        let ukf = r#"
# sample keys.ukf
collection "PANA_DVD_KEYONLY": {
    "Old": {x"DF7215C7427FD010"}, # duplicate of built-in
    "custom_test": {x"1122334455667788"},
}
collection "PANA_DVD_AESPAIR": {
    "custom_2025": {x"00112233445566778899AABBCCDDEEFF", x"FFEEDDCCBBAA99887766554433221100", x"AABBCCDDEEFF0011"},
}
"#;
        let mut ring = FrmKeyring::builtin();
        let base_feistel = ring.feistel_keys.len();
        let base_aes = ring.aes_pairs.len();
        let added = ring.merge_ukf_str(ukf).unwrap();
        assert_eq!(added, 2);
        assert_eq!(ring.feistel_keys.len(), base_feistel + 2);
        assert_eq!(ring.aes_pairs.len(), base_aes + 1);
    }

    #[test]
    fn cascading_keyring_unions_distinct_keys_and_overrides_on_name_collision() {
        let dir = std::env::temp_dir().join(format!("panadiga_ukf_cascade_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let env_p = dir.join("env.ukf");
        let local_p = dir.join("local.ukf");
        let cli_p = dir.join("cli.ukf");
        let missing_local = dir.join("absent_local.ukf");

        std::fs::write(
            &env_p,
            r#"collection "PANA_DVD_KEYONLY": {
                "env_k": {x"0102030405060708"},
                "shared_id": {x"AAAAAAAAAAAAAAAA"},
            }"#,
        )
        .unwrap();
        std::fs::write(
            &local_p,
            r#"collection "PANA_DVD_KEYONLY": {
                "local_k": {x"1112131415161718"},
            }"#,
        )
        .unwrap();
        std::fs::write(
            &cli_p,
            r#"collection "PANA_DVD_KEYONLY": {
                "shared_id": {x"BBBBBBBBBBBBBBBB"},
            }"#,
        )
        .unwrap();

        let ring = FrmKeyring::load_cascading(Some(&env_p), Some(&local_p), Some(&cli_p)).unwrap();
        assert!(ring.feistel_keys.iter().any(|k| k.name == "env_k"));
        assert!(ring.feistel_keys.iter().any(|k| k.name == "local_k"));
        let shared = ring
            .feistel_keys
            .iter()
            .find(|k| k.name == "shared_id")
            .unwrap();
        assert_eq!(shared.key_8b, [0xBB; 8]);

        let ring2 = FrmKeyring::load_cascading(Some(&env_p), Some(&missing_local), None).unwrap();
        assert!(ring2.feistel_keys.iter().any(|k| k.name == "env_k"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
