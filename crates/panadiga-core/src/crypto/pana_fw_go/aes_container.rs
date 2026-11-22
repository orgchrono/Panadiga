#![deny(unsafe_code)]

//! Blu-ray / UHD Blu-ray `PANAEUSB.FRM` 48-byte `AESHeader` wrapper & unwrapper (`AES-128-CBC + Feistel`).

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::super::aes128::{
    FrmKeyring, PanaAesKeyPair, aes128_cbc_decrypt_in_place, aes128_cbc_encrypt_in_place,
};
use super::super::codec::read_u32_le;
use super::super::feistel::{
    adler32_panasonic_strided, derive_round_key_from_custom_key_8b, feistel_decrypt_buffer,
    feistel_encrypt_buffer,
};
use crate::constants::PANA_FEISTEL_SBOX_256;

/// 48-byte outer container magic inside `PANAEUSB.FRM` (`entries[0..32] + PANASONIC\0\0\0 + sum`).
pub const PANA_AES_HEADER_SIZE: usize = 48;
pub const PANA_AES_HEADER_MAGIC: &[u8; 12] = b"PANASONIC\x00\x00\x00";

/// Single sub-firmware entry inside a 48-byte Panasonic `AESHeader`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AesHeaderFileEntry {
    pub offset: u32,
    pub size: u32,
}

/// Decoded 48-byte `PANAEUSB.FRM` `AESHeader` structure (`pana-fw/utils/firmware/module.go`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AesHeader {
    pub entries: [AesHeaderFileEntry; 4],
    pub magic: String,
    pub sum: u32,
}

/// Unwrapped `PANAEUSB.FRM` container holding the matched key pair and decrypted inner `.FRM` sub-containers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwrappedAesFrm {
    pub key_pair: PanaAesKeyPair,
    pub header: AesHeader,
    pub sub_containers: Vec<(AesHeaderFileEntry, Vec<u8>)>,
}

fn aes_decrypt_custom_header(
    buf48: &[u8],
    pair: &PanaAesKeyPair,
) -> Result<[u8; PANA_AES_HEADER_SIZE]> {
    let mut hdr = [0u8; PANA_AES_HEADER_SIZE];
    hdr.copy_from_slice(&buf48[..PANA_AES_HEADER_SIZE]);
    aes128_cbc_decrypt_in_place(&pair.aes_key, &pair.aes_iv, &mut hdr)?;
    let rk = derive_round_key_from_custom_key_8b(&pair.feistel_key_8b);
    feistel_decrypt_buffer(&mut hdr, &rk, &PANA_FEISTEL_SBOX_256);
    Ok(hdr)
}

fn aes_encrypt_custom_header(
    plain48: &[u8; PANA_AES_HEADER_SIZE],
    pair: &PanaAesKeyPair,
) -> Result<[u8; PANA_AES_HEADER_SIZE]> {
    let mut hdr = *plain48;
    let rk = derive_round_key_from_custom_key_8b(&pair.feistel_key_8b);
    feistel_encrypt_buffer(&mut hdr, &rk, &PANA_FEISTEL_SBOX_256);
    aes128_cbc_encrypt_in_place(&pair.aes_key, &pair.aes_iv, &mut hdr)?;
    Ok(hdr)
}

/// Attempts to unwrap a `PANAEUSB.FRM` double-encrypted container (`AES-128-CBC + Feistel`) using `keyring`.
///
/// Returns `Ok(None)` when the buffer does not match any AES key pair cleanly.
pub fn detect_and_unwrap_aes_frm(
    raw: &[u8],
    keyring: &FrmKeyring,
) -> Result<Option<UnwrappedAesFrm>> {
    if raw.len() < PANA_AES_HEADER_SIZE + 16 {
        return Ok(None);
    }

    let mut matched: Option<(PanaAesKeyPair, [u8; PANA_AES_HEADER_SIZE])> = None;
    for pair in &keyring.aes_pairs {
        let dec = aes_decrypt_custom_header(&raw[..PANA_AES_HEADER_SIZE], pair)?;
        if &dec[32..44] == PANA_AES_HEADER_MAGIC {
            let expected_sum = read_u32_le(&dec, 44);
            let actual_sum = adler32_panasonic_strided(&dec[..44], 1);
            if expected_sum == actual_sum {
                matched = Some((pair.clone(), dec));
                break;
            }
        }
    }

    let Some((key_pair, dec_hdr)) = matched else {
        return Ok(None);
    };

    let mut entries = [AesHeaderFileEntry { offset: 0, size: 0 }; 4];
    let mut sub_containers = Vec::new();
    let mut seen_offsets = Vec::new();

    for (idx, slot) in entries.iter_mut().enumerate() {
        let off = read_u32_le(&dec_hdr, idx * 8);
        let sz = read_u32_le(&dec_hdr, idx * 8 + 4);
        *slot = AesHeaderFileEntry {
            offset: off,
            size: sz,
        };
        if off == 0 && sz == 0 {
            continue;
        }
        if seen_offsets.contains(&off) {
            continue;
        }
        seen_offsets.push(off);

        let start = off as usize;
        let len = sz as usize;
        let end = start.checked_add(len).ok_or_else(|| {
            anyhow::anyhow!("AESHeader slot {idx} offset overflow (0x{off:X} + 0x{sz:X})")
        })?;
        if end > raw.len() {
            bail!(
                "AESHeader slot {idx} (0x{off:X}..0x{end:X}) exceeds container length 0x{:X}",
                raw.len()
            );
        }
        let aligned_len = len & !15;
        let mut sub_buf = raw[start..start + aligned_len].to_vec();
        aes128_cbc_decrypt_in_place(&key_pair.aes_key, &key_pair.aes_iv, &mut sub_buf)?;
        if len > aligned_len {
            sub_buf.extend_from_slice(&raw[start + aligned_len..end]);
        }
        sub_containers.push((*slot, sub_buf));
    }

    if sub_containers.is_empty() {
        bail!("AESHeader contains PANASONIC magic but zero valid sub-container entries");
    }

    Ok(Some(UnwrappedAesFrm {
        key_pair,
        header: AesHeader {
            entries,
            magic: String::from_utf8_lossy(&dec_hdr[32..41]).into_owned(),
            sum: read_u32_le(&dec_hdr, 44),
        },
        sub_containers,
    }))
}

/// Wraps `1..=4` inner `.FRM` buffers into a double-encrypted `PANAEUSB.FRM` `AESHeader` image.
pub fn wrap_aes_frm(sub_containers: &[Vec<u8>], pair: &PanaAesKeyPair) -> Result<Vec<u8>> {
    if sub_containers.is_empty() || sub_containers.len() > 4 {
        bail!(
            "AESHeader supports 1..=4 sub-containers, got {}",
            sub_containers.len()
        );
    }

    let mut plain_hdr = [0u8; PANA_AES_HEADER_SIZE];
    let mut body = Vec::new();
    let mut cursor = PANA_AES_HEADER_SIZE;

    for (idx, sub) in sub_containers.iter().enumerate() {
        let mut enc_sub = sub.clone();
        let rem = enc_sub.len() % 16;
        if rem != 0 {
            enc_sub.resize(enc_sub.len() + (16 - rem), 0);
        }
        let sz = u32::try_from(enc_sub.len())?;
        let off = u32::try_from(cursor)?;
        plain_hdr[idx * 8..idx * 8 + 4].copy_from_slice(&off.to_le_bytes());
        plain_hdr[idx * 8 + 4..idx * 8 + 8].copy_from_slice(&sz.to_le_bytes());
        aes128_cbc_encrypt_in_place(&pair.aes_key, &pair.aes_iv, &mut enc_sub)?;
        body.extend_from_slice(&enc_sub);
        cursor += enc_sub.len();
    }

    plain_hdr[32..44].copy_from_slice(PANA_AES_HEADER_MAGIC);
    let sum = adler32_panasonic_strided(&plain_hdr[..44], 1);
    plain_hdr[44..48].copy_from_slice(&sum.to_le_bytes());

    let enc_hdr = aes_encrypt_custom_header(&plain_hdr, pair)?;
    let mut out = Vec::with_capacity(PANA_AES_HEADER_SIZE + body.len());
    out.extend_from_slice(&enc_hdr);
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_container_single_and_multi_entry_roundtrip() {
        let ring = FrmKeyring::builtin();
        let pair = &ring.aes_pairs[0];

        let sub1 = vec![0x42u8; 0x100];
        let mut sub2 = vec![0x7Eu8; 0x83]; // intentionally unaligned before wrap padding
        sub2[..12].copy_from_slice(b"PANASONIC\0\0\0");

        let wrapped = wrap_aes_frm(&[sub1.clone(), sub2.clone()], pair).unwrap();
        let unwrapped = detect_and_unwrap_aes_frm(&wrapped, &ring)
            .unwrap()
            .expect("should match built-in y2018_aes pair");
        assert_eq!(unwrapped.key_pair.name, pair.name);
        assert_eq!(unwrapped.sub_containers.len(), 2);
        assert_eq!(unwrapped.sub_containers[0].0.offset, 48);
        assert_eq!(unwrapped.sub_containers[0].1, sub1);
        assert!(unwrapped.sub_containers[1].1.starts_with(&sub2));

        // Corrupt header byte -> checksum failure / non-match
        let mut bad = wrapped;
        bad[45] ^= 0xAA;
        assert!(detect_and_unwrap_aes_frm(&bad, &ring).unwrap().is_none());
    }
}
