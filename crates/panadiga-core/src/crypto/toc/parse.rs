#![deny(unsafe_code)]

//! Deterministic `.FRM` TOC entry decoding, Feistel auto-detection, and flag interpretation.

use super::super::codec::{extract_c_ascii, read_u16_le, read_u32_le};
use super::super::feistel::{detect_frm_feistel_key, feistel_decrypt_buffer};
use super::super::types::DecryptedFrmTocEntry;
use super::layout::{
    DVD_TOC_LAYOUT, DVD64_TOC_LAYOUT, SD_TOC_LAYOUT, SD48_TOC_LAYOUT, TocLayout, TocLayoutOffsets,
};
use crate::constants::{
    PANA_FEISTEL_SBOX_256, PANA_FRM_TOC_ENTRY_SIZE, PANA_SD_TOC_ENTRY_SIZE,
    PANA_SD_TOC_MAX_ENTRIES, PANA_SD_TOC_REGION_SIZE,
};

fn parse_toc_entries_with_layout(
    plain: &[u8],
    count: usize,
    layout: &TocLayoutOffsets,
) -> Vec<DecryptedFrmTocEntry> {
    let mut entries = Vec::with_capacity(count);
    for (idx, entry) in plain.chunks_exact(layout.stride).take(count).enumerate() {
        let ver_u16 = read_u16_le(entry, layout.ver_off);
        let module_offset = read_u32_le(entry, layout.off_pos);
        let module_size = read_u32_le(entry, 0x20);
        let fvu_checksum_u32 = read_u32_le(entry, 0x24);
        let list_sum_u16 = read_u16_le(entry, layout.sum_off);
        let calculated_list_sum_u16 = entry[..layout.sum_off]
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));

        if layout.skip_empty && module_size == 0 && entry[..layout.name_end].iter().all(|&b| b == 0)
        {
            continue;
        }

        entries.push(DecryptedFrmTocEntry {
            index: idx,
            entry_offset_hex: format!("0x{:04X}", idx * layout.stride),
            module_name: extract_c_ascii(&entry[0x00..layout.name_end]),
            version_u16_hex: format!("0x{:04X}", ver_u16),
            module_offset,
            module_size,
            hardware_family: extract_c_ascii(&entry[0x10..layout.hw_end]),
            target_flag: entry[layout.flag_pos],
            model_group: extract_c_ascii(&entry[layout.grp_start..layout.grp_end]),
            fvu_checksum_u32,
            list_sum_u16,
            calculated_list_sum_u16,
            list_sum_valid: list_sum_u16 == calculated_list_sum_u16,
        });
    }
    entries
}

fn entry_list_sum_valid(decrypted_slot: &[u8], stride: usize) -> bool {
    if decrypted_slot.len() < stride || stride < 2 {
        return false;
    }
    let sum_off = stride - 2;
    let expected = read_u16_le(decrypted_slot, sum_off);
    let computed = decrypted_slot[..sum_off]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
    expected == computed
}

fn detect_toc_feistel_key(bytes: &[u8], toc_span: usize) -> Option<(&'static str, [u8; 32])> {
    let head_len = toc_span.min(0x80);
    if head_len >= 8
        && let Some(found) = detect_frm_feistel_key(&bytes[..head_len])
    {
        return Some(found);
    }
    let mut off = 0x40usize;
    while off + 8 <= toc_span {
        let end = toc_span.min(off + 0x80);
        if let Some((name, rk)) = detect_frm_feistel_key(&bytes[off..end]) {
            let mut probe = bytes[off..end].to_vec();
            feistel_decrypt_buffer(&mut probe, &rk, &PANA_FEISTEL_SBOX_256);
            if entry_list_sum_valid(&probe, PANA_SD_TOC_ENTRY_SIZE)
                || entry_list_sum_valid(&probe, PANA_FRM_TOC_ENTRY_SIZE)
            {
                return Some((name, rk));
            }
        }
        off += 0x40;
    }
    None
}

/// Decrypts the `.FRM` TOC using the detected Feistel key and auto-discriminates `Dvd48B` vs `Dvd64B`.
pub fn decrypt_frm_toc_with_detected_key(
    bytes: &[u8],
) -> (
    Option<&'static str>,
    [u8; 32],
    TocLayout,
    Vec<DecryptedFrmTocEntry>,
) {
    let toc_span = bytes.len().min(PANA_SD_TOC_REGION_SIZE) & !7;
    if toc_span < 8 {
        return (None, [0u8; 32], TocLayout::Dvd48B, Vec::new());
    }
    let Some((key_name, round_key)) = detect_toc_feistel_key(bytes, toc_span) else {
        return (None, [0u8; 32], TocLayout::Dvd48B, Vec::new());
    };

    let mut decrypted = bytes[..toc_span].to_vec();
    feistel_decrypt_buffer(&mut decrypted, &round_key, &PANA_FEISTEL_SBOX_256);

    let count48 = decrypted.len() / PANA_FRM_TOC_ENTRY_SIZE;
    let valid_entries48: Vec<DecryptedFrmTocEntry> = {
        let mut entries = parse_toc_entries_with_layout(&decrypted, count48, &DVD_TOC_LAYOUT)
            .into_iter()
            .filter(|e| e.list_sum_valid && !e.module_name.trim().is_empty())
            .collect::<Vec<_>>();
        // EH50 `DRV J147` stores slot-0's payload offset at byte +0x08 (0x1800) instead of +0x0C.
        if let Some(first) = entries.first_mut()
            && first.index == 0
            && (first.module_offset as usize) >= bytes.len()
            && decrypted.len() >= PANA_FRM_TOC_ENTRY_SIZE
        {
            let alt_off = read_u32_le(&decrypted[..PANA_FRM_TOC_ENTRY_SIZE], 0x08);
            if alt_off == 0x1800
                && (alt_off as usize).saturating_add(first.module_size as usize) <= bytes.len()
            {
                first.module_offset = alt_off;
            }
        }
        entries
    };

    let count64 = decrypted.len() / PANA_SD_TOC_ENTRY_SIZE;
    if count64 > 0 {
        let valid_entries64: Vec<DecryptedFrmTocEntry> =
            parse_toc_entries_with_layout(&decrypted, count64, &DVD64_TOC_LAYOUT)
                .into_iter()
                .filter(|e| e.list_sum_valid && !e.module_name.trim().is_empty())
                .collect();

        let is_64b = if let Some(first64) = valid_entries64.first() {
            let entry0_ok = first64.module_offset >= 0x2000;
            let next_off = (first64.index + 1) * PANA_SD_TOC_ENTRY_SIZE;
            let pfx_next_ok = if next_off + 4 <= decrypted.len() {
                const PREFIXES: [&[u8; 4]; 4] = [b"PROG", b"DRV ", b"MAIN", b"STB "];
                PREFIXES
                    .iter()
                    .any(|pfx| &decrypted[next_off..next_off + 4] == *pfx)
            } else {
                false
            };
            entry0_ok
                && (pfx_next_ok
                    || valid_entries64.len() >= 2
                    || valid_entries48.is_empty()
                    || first64.index * PANA_SD_TOC_ENTRY_SIZE >= 0x1000)
        } else {
            false
        };

        if is_64b {
            return (
                Some(key_name),
                round_key,
                TocLayout::Dvd64B,
                valid_entries64,
            );
        }
    }

    (
        Some(key_name),
        round_key,
        TocLayout::Dvd48B,
        valid_entries48,
    )
}

/// Decrypts the Table of Contents of `PANA_DVD.FRM` using auto-detected key and layout.
pub fn decrypt_and_parse_frm_toc(data: &[u8]) -> Vec<DecryptedFrmTocEntry> {
    decrypt_frm_toc_with_detected_key(data).3
}

fn has_valid_named_entry(entries: &[DecryptedFrmTocEntry]) -> bool {
    entries
        .iter()
        .any(|e| e.list_sum_valid && !e.module_name.trim().is_empty())
}

/// Parses plaintext `PANA__SD.FRM` TOC and returns the matched `TocLayout` (`Sd64B` vs obsolete `Sd48B`).
pub fn parse_sd_frm_toc_with_layout(data: &[u8]) -> (Option<TocLayout>, Vec<DecryptedFrmTocEntry>) {
    let count64 = (data.len() / PANA_SD_TOC_ENTRY_SIZE).min(PANA_SD_TOC_MAX_ENTRIES);
    if count64 > 0 {
        let entries64 = parse_toc_entries_with_layout(data, count64, &SD_TOC_LAYOUT);
        if has_valid_named_entry(&entries64) {
            return (Some(TocLayout::Sd64B), entries64);
        }
    }
    let count48 = (data.len() / PANA_FRM_TOC_ENTRY_SIZE).min(PANA_SD_TOC_MAX_ENTRIES);
    if count48 > 0 {
        let entries48 = parse_toc_entries_with_layout(data, count48, &SD48_TOC_LAYOUT);
        if has_valid_named_entry(&entries48) {
            return (Some(TocLayout::Sd48B), entries48);
        }
    }
    (None, Vec::new())
}

/// Parses plaintext 64-byte (`0x40`, `BOOT.BIN`) or 48-byte (`0x30`, `PROG061`) TOC entries of `PANA__SD.FRM`.
pub fn parse_sd_frm_toc(data: &[u8]) -> Vec<DecryptedFrmTocEntry> {
    parse_sd_frm_toc_with_layout(data).1
}

/// Interprets the TOC `FormatFlag` / `target_flag` byte using verified `pana-fw` / DIGA loader rules.
pub fn interpret_frm_target_flag(module_name: &str, target_flag: u8) -> &'static str {
    let upper = module_name.trim().to_ascii_uppercase();
    if upper.starts_with("MAIN") {
        if target_flag != 0 {
            "FORCE_UPDATE"
        } else {
            "NORMAL_VERSION_CHECK"
        }
    } else if upper.starts_with("PROG") {
        if target_flag != 0 {
            "FORMAT_HDD"
        } else {
            "NORMAL_PRESERVE_HDD"
        }
    } else if target_flag != 0 {
        "FLAG_SET"
    } else {
        "NORMAL"
    }
}
