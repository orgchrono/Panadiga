#![deny(unsafe_code)]

//! Autonomous multi-generation container fingerprinting & cryptographic artifact harvesting.

pub mod auto_fingerprint;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::constants::{
    PANA_FEISTEL_KEYSCHED_IV, PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256,
};
use crate::crypto::{PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b};

pub use auto_fingerprint::{fingerprint_binary, harvest_crypto_artifacts};

/// Detected firmware or storage container format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerFormat {
    Sd48B,
    Sd64B,
    Dvd48B,
    Dvd64B,
    Digadiag,
    Extrhead,
    RawNor,
    MeihfsHdd,
    Unknown,
}

/// Primary cipher family detected in container wrapping or embedded key tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CipherFamily {
    MatsushitaFeistel64,
    Des,
    Blowfish,
    Plaintext,
    Unknown,
}

/// Discovered 256-byte bijective non-linear cryptographic S-box.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarvestedSbox {
    pub offset: usize,
    pub label: String,
    pub sha256_prefix: String,
    pub matches_known_matsushita: bool,
}

/// Discovered cipher key schedule or constant table inside a firmware image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarvestedKeySchedule {
    pub offset: usize,
    pub family: CipherFamily,
    pub label: String,
    pub preview_hex: String,
}

/// Full deterministic ingestion fingerprint report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestFingerprintReport {
    pub label: String,
    pub byte_len: usize,
    pub container: ContainerFormat,
    pub cipher_family: CipherFamily,
    pub detected_key_label: Option<String>,
    pub double_encrypted_extrhead: bool,
    pub detected_model: String,
    pub sboxes: Vec<HarvestedSbox>,
    pub key_schedules: Vec<HarvestedKeySchedule>,
    pub handshake_tokens: Vec<String>,
    pub toc_entry_count: usize,
}

pub(crate) const BLOWFISH_P_INIT_BE: [u8; 16] = [
    0x24, 0x3F, 0x6A, 0x88, 0x85, 0xA3, 0x08, 0xD3, 0x13, 0x19, 0x8A, 0x2E, 0x03, 0x70, 0x73, 0x44,
];
pub(crate) const BLOWFISH_P_INIT_LE: [u8; 16] = [
    0x88, 0x6A, 0x3F, 0x24, 0xD3, 0x08, 0xA3, 0x85, 0x2E, 0x8A, 0x19, 0x13, 0x44, 0x73, 0x70, 0x03,
];
pub(crate) const DES_SBOX1_UNPACKED: [u8; 16] =
    [14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7];
pub(crate) const DES_IP_HEAD: [u8; 8] = [58, 50, 42, 34, 26, 18, 10, 2];

fn find_exact(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    (!needle.is_empty() && haystack.len() >= needle.len())
        .then(|| haystack.windows(needle.len()).position(|w| w == needle))
        .flatten()
}

pub(crate) fn scan_256b_sboxes(data: &[u8], base_off: usize, out: &mut Vec<HarvestedSbox>) {
    if data.len() < 256 {
        return;
    }
    let mut counts = [0u16; 256];
    let mut unique = 0usize;
    for &b in &data[..256] {
        let idx = b as usize;
        if counts[idx] == 0 {
            unique += 1;
        }
        counts[idx] += 1;
    }
    let (mut i, mut next_allowed) = (0usize, 0usize);
    loop {
        if i >= next_allowed && unique == 256 {
            let win = &data[i..i + 256];
            let step = win[1].wrapping_sub(win[0]);
            let affine = win.windows(2).all(|p| p[1].wrapping_sub(p[0]) == step);
            if !affine {
                let known = win == PANA_FEISTEL_SBOX_256;
                out.push(HarvestedSbox {
                    offset: base_off + i,
                    label: if known {
                        "matsushita_feistel_sbox_256".into()
                    } else {
                        format!("bijective_sbox_0x{:X}", base_off + i)
                    },
                    sha256_prefix: hex::encode(&Sha256::digest(win)[..8]),
                    matches_known_matsushita: known,
                });
                next_allowed = i + 256;
            }
        }
        if i + 256 >= data.len() {
            break;
        }
        counts[data[i] as usize] -= 1;
        if counts[data[i] as usize] == 0 {
            unique -= 1;
        }
        if counts[data[i + 256] as usize] == 0 {
            unique += 1;
        }
        counts[data[i + 256] as usize] += 1;
        i += 1;
    }
}

pub(crate) fn scan_key_schedules(
    data: &[u8],
    base_off: usize,
    out: &mut Vec<HarvestedKeySchedule>,
) {
    let mut push = |off: usize, fam: CipherFamily, lbl: String, pat: &[u8]| {
        out.push(HarvestedKeySchedule {
            offset: base_off + off,
            family: fam,
            label: lbl,
            preview_hex: hex::encode(&pat[..pat.len().min(8)]),
        });
    };
    if let Some(pos) = find_exact(data, &PANA_FEISTEL_ROUND_KEY_32B) {
        push(
            pos,
            CipherFamily::MatsushitaFeistel64,
            "old_df72_round_key_32b".into(),
            &PANA_FEISTEL_ROUND_KEY_32B,
        );
    }
    for &(name, k8) in PANA_DVD_KEY_CATALOG {
        let rk = derive_round_key_from_custom_key_8b(&k8);
        if rk != PANA_FEISTEL_ROUND_KEY_32B
            && let Some(pos) = find_exact(data, &rk)
        {
            push(
                pos,
                CipherFamily::MatsushitaFeistel64,
                format!("{name}_round_key_32b"),
                &rk,
            );
        }
    }
    if let Some(pos) = find_exact(data, &PANA_FEISTEL_KEYSCHED_IV) {
        push(
            pos,
            CipherFamily::MatsushitaFeistel64,
            "feistel_keysched_iv".into(),
            &PANA_FEISTEL_KEYSCHED_IV,
        );
    }
    for (pat, label, fam) in [
        (
            &BLOWFISH_P_INIT_BE[..],
            "blowfish_pi_p_array_be",
            CipherFamily::Blowfish,
        ),
        (
            &BLOWFISH_P_INIT_LE[..],
            "blowfish_pi_p_array_le",
            CipherFamily::Blowfish,
        ),
        (
            &DES_SBOX1_UNPACKED[..],
            "des_sbox1_table",
            CipherFamily::Des,
        ),
        (&DES_IP_HEAD[..], "des_ip_table", CipherFamily::Des),
    ] {
        if let Some(pos) = find_exact(data, pat) {
            push(pos, fam, label.into(), pat);
        }
    }
}

pub(crate) fn extract_handshake_tokens(data: &[u8], out: &mut Vec<String>) {
    const PFX: [&str; 12] = [
        "DMR-",
        "MN103",
        "MN2WS",
        "PANA_DVD",
        "PANA__SD",
        "DIGADIAG",
        "SDDIAG",
        "CDDIAG",
        "EXTRHEAD",
        "UNIPHIER",
        "VER0.",
        "PANASONIC",
    ];
    let mut start: Option<usize> = None;
    for (idx, &b) in data.iter().enumerate() {
        if (0x20..=0x7E).contains(&b) {
            start.get_or_insert(idx);
        } else if let Some(s) = start.take()
            && idx - s >= 5
            && let Ok(text) = std::str::from_utf8(&data[s..idx])
        {
            let trimmed = text.trim();
            let upper = trimmed.to_ascii_uppercase();
            if PFX.iter().any(|p| upper.contains(p)) && !out.iter().any(|t| t == trimmed) {
                out.push(trimmed.chars().take(48).collect());
                if out.len() >= 32 {
                    return;
                }
            }
        }
    }
}
