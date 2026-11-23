#![deny(unsafe_code)]

//! Top-level container auto-detection and sub-container unpacking for Panasonic `.FRM` images.

use super::decode_slot::decode_module_slot;
use super::types::{UniversalFrmContainer, UniversalFrmFormat, UniversalFrmSubContainer};
use crate::aes128::{FrmKeyring, PanaAesKeyPair};
use crate::codec::read_u32_le;
use crate::constants::{
    PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256, PANA_FRM_TOC_ENTRY_SIZE,
    PANA_SD_TOC_ENTRY_SIZE, PANA_SD_TOC_REGION_SIZE,
};
use crate::feistel::{
    adler32_panasonic_strided, derive_round_key_from_custom_key_8b, feistel_decrypt_buffer,
};
use crate::pana_fw_go::{detect_and_unwrap_aes_frm, find_feistel_key_slow_with_keyring};
use crate::toc::{
    TocLayout, decrypt_frm_toc_with_keyring, parse_sd_frm_toc_with_layout,
    strip_panasonic_outer_header,
};

pub(super) fn parse_version_u16_hex(raw: &str) -> u16 {
    u16::from_str_radix(raw.trim_start_matches("0x"), 16).unwrap_or(1)
}

fn unpack_single_subcontainer(
    raw: &[u8],
    sub_index: usize,
    file_offset: u32,
    keyring: &FrmKeyring,
    preferred_aes_pair: Option<&PanaAesKeyPair>,
) -> anyhow::Result<UniversalFrmSubContainer> {
    let (after_hdr, has_outer_48b) = strip_panasonic_outer_header(raw);
    let (raw_outer_48b, outer_toc_enc_flag) = if has_outer_48b {
        let mut h = [0u8; 48];
        h.copy_from_slice(&raw[..48]);
        let enc_flag = if adler32_panasonic_strided(&h[..44], 1) == read_u32_le(&h, 44) {
            h[43] != 0
        } else {
            true
        };
        (Some(h), enc_flag)
    } else {
        (None, true)
    };

    let (mut key_name, mut rk, layout, mut entries) = decrypt_frm_toc_with_keyring(raw, keyring);
    let mut toc_encrypted = true;
    let mut sd_plain_layout = None;

    if entries.is_empty() {
        let (sd_lay, sd_ents) = parse_sd_frm_toc_with_layout(after_hdr);
        if !sd_ents.is_empty() {
            sd_plain_layout = sd_lay;
            entries = sd_ents;
            toc_encrypted = false;
            key_name = preferred_aes_pair.map(|p| format!("{}_feistel", p.name));
            if let Some(pair) = preferred_aes_pair {
                rk = derive_round_key_from_custom_key_8b(&pair.feistel_key_8b);
            } else {
                rk = PANA_FEISTEL_ROUND_KEY_32B;
            }
        } else if outer_toc_enc_flag
            && let Some((slow_name, slow_rk, _score)) = find_feistel_key_slow_with_keyring(
                &after_hdr[..after_hdr.len().min(0x1800)],
                keyring,
            )
        {
            key_name = Some(slow_name);
            rk = slow_rk;
        }
    }

    if entries.is_empty() {
        anyhow::bail!(
            "Unable to decode Panasonic .FRM TOC (unrecognized container or missing key)"
        );
    }

    let stride = match sd_plain_layout.unwrap_or(layout) {
        TocLayout::Dvd48B | TocLayout::Sd48B => PANA_FRM_TOC_ENTRY_SIZE,
        TocLayout::Dvd64B | TocLayout::Sd64B => PANA_SD_TOC_ENTRY_SIZE,
    };

    let toc_span = after_hdr.len().min(PANA_SD_TOC_REGION_SIZE) & !7;
    let mut decrypted_toc = after_hdr[..toc_span].to_vec();
    if toc_encrypted {
        feistel_decrypt_buffer(&mut decrypted_toc, &rk, &PANA_FEISTEL_SBOX_256);
    }

    let first_toc_slot = entries
        .first()
        .map(|e| &decrypted_toc[e.index * stride..(e.index + 1) * stride]);
    let uses_adler32 = stride == PANA_FRM_TOC_ENTRY_SIZE
        && first_toc_slot
            .is_some_and(|s| adler32_panasonic_strided(&s[..0x2C], 1) == read_u32_le(s, 0x2C));

    let format = if uses_adler32 {
        UniversalFrmFormat::BlurayFeistel
    } else if !toc_encrypted && sd_plain_layout == Some(TocLayout::Sd64B) {
        UniversalFrmFormat::SdPlain64B
    } else if !toc_encrypted && sd_plain_layout == Some(TocLayout::Sd48B) {
        UniversalFrmFormat::SdPlain48B
    } else if stride == PANA_SD_TOC_ENTRY_SIZE {
        UniversalFrmFormat::LegacyDvd64B
    } else {
        UniversalFrmFormat::LegacyDvd48B
    };

    let is_bluray_rules = matches!(format, UniversalFrmFormat::BlurayFeistel);
    let mut modules = Vec::with_capacity(entries.len());
    for entry in &entries {
        let slot_off = entry.index * stride;
        let slot_raw = &decrypted_toc[slot_off..slot_off + stride];
        modules.push(decode_module_slot(
            raw,
            slot_raw,
            entry,
            stride,
            uses_adler32,
            toc_encrypted,
            &rk,
            is_bluray_rules,
        )?);
    }

    let toc_region_size = modules
        .iter()
        .map(|m| m.module_offset as usize)
        .min()
        .unwrap_or(0x1800)
        .max(if has_outer_48b { 0x1800 } else { stride * 128 });

    Ok(UniversalFrmSubContainer {
        sub_index,
        file_offset,
        format,
        feistel_key_name: key_name,
        toc_encrypted,
        has_panasonic_header: has_outer_48b,
        toc_stride_bytes: stride,
        toc_region_size,
        uses_adler32_sums: uses_adler32,
        modules,
        feistel_round_key: rk,
        raw_outer_48b,
    })
}

/// Automatically detects and unpacks ANY Panasonic `.FRM` firmware update (Legacy DVD/HDD, SD, or Blu-ray/AES).
pub fn unpack_universal_frm(
    raw: &[u8],
    keyring: &FrmKeyring,
) -> anyhow::Result<UniversalFrmContainer> {
    if let Some(unwrapped) = detect_and_unwrap_aes_frm(raw, keyring)? {
        let mut subs = Vec::with_capacity(unwrapped.sub_containers.len());
        for (idx, (entry, sub_bytes)) in unwrapped.sub_containers.iter().enumerate() {
            subs.push(unpack_single_subcontainer(
                sub_bytes,
                idx,
                entry.offset,
                keyring,
                Some(&unwrapped.key_pair),
            )?);
        }
        return Ok(UniversalFrmContainer {
            format: UniversalFrmFormat::BlurayAes,
            aes_key_name: Some(unwrapped.key_pair.name.clone()),
            aes_header: Some(unwrapped.header),
            sub_containers: subs,
            aes_key_pair: Some(unwrapped.key_pair),
        });
    }

    let sub = unpack_single_subcontainer(raw, 0, 0, keyring, None)?;
    Ok(UniversalFrmContainer {
        format: sub.format,
        aes_key_name: None,
        aes_header: None,
        sub_containers: vec![sub],
        aes_key_pair: None,
    })
}
