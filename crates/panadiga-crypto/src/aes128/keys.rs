#![deny(unsafe_code)]

//! Panasonic Blu-ray AES/Feistel Keyring (`keys.ukf`) and `libdownloadlib.so` Network Key Derivation.

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::constants::PANA_FEISTEL_ROUND_KEY_32B;
use crate::feistel::{PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b};
use crate::pana_fw_go::network_decipher_in_place;

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
