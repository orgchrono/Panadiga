//! Pure deterministic parser, validator, and formatter for Panasonic `.FRM` TOC tables.
//!
//! Zero filesystem, mmap, or terminal I/O belongs in this module; file loading lives in `toc_io`.

mod report;

pub use report::{analyze_frm_toc_report, analyze_toc, format_toc_dump_lines};

use serde::{Deserialize, Serialize};

use super::codec::{extract_c_ascii, read_u16_le, read_u32_le};
use super::feistel::{detect_frm_feistel_key, feistel_decrypt_buffer};
use super::types::DecryptedFrmTocEntry;
use crate::constants::*;

/// Discriminated TOC layout supported by Panasonic DIGA `.FRM` containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TocLayout {
    /// Optical update `PANA_DVD.FRM`: Feistel-encrypted 48-byte (`0x30`) TOC.
    Dvd48B,
    /// Optical update `PANA_DVD.FRM`: Feistel-encrypted 64-byte (`0x40`) TOC (`EH49`, `EH59`, `EH69`).
    Dvd64B,
    /// Obsolete early/custom 48-byte (`0x30`) plaintext SD TOC (`OPCAO_3`, rejected by `BOOT.BIN`).
    Sd48B,
    /// Verified factory `PANA__SD.FRM`: plaintext 64-byte (`0x40`) TOC (`0x2000` header).
    Sd64B,
}

impl TocLayout {
    pub const fn stride_bytes(self) -> usize {
        match self {
            Self::Dvd48B | Self::Sd48B => PANA_FRM_TOC_ENTRY_SIZE,
            Self::Dvd64B | Self::Sd64B => PANA_SD_TOC_ENTRY_SIZE,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Dvd48B => "dvd-feistel",
            Self::Dvd64B => "dvd-64b-feistel",
            Self::Sd48B => "sd-48b-plaintext-obsolete",
            Self::Sd64B => "sd-plaintext",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Dvd48B => "PANA_DVD.FRM (48B Feistel TOC)",
            Self::Dvd64B => "PANA_DVD.FRM (64B Feistel TOC)",
            Self::Sd48B => "PANA__SD.FRM (48B Obsolete SD TOC — rejected by BOOT.BIN)",
            Self::Sd64B => "PANA__SD.FRM (64B Plaintext SD TOC)",
        }
    }
}

pub(crate) struct TocLayoutOffsets {
    pub stride: usize,
    pub name_end: usize,
    pub ver_off: usize,
    pub off_pos: usize,
    pub hw_end: usize,
    pub flag_pos: usize,
    pub grp_start: usize,
    pub grp_end: usize,
    pub sum_off: usize,
    pub skip_empty: bool,
}

const DVD_TOC_LAYOUT: TocLayoutOffsets = TocLayoutOffsets {
    stride: PANA_FRM_TOC_ENTRY_SIZE,
    name_end: 0x0A,
    ver_off: 0x0A,
    off_pos: 0x0C,
    hw_end: 0x18,
    flag_pos: 0x19,
    grp_start: 0x1A,
    grp_end: 0x20,
    sum_off: 0x2E,
    skip_empty: false,
};

const SD48_TOC_LAYOUT: TocLayoutOffsets = TocLayoutOffsets {
    skip_empty: true,
    ..DVD_TOC_LAYOUT
};

const SD_TOC_LAYOUT: TocLayoutOffsets = TocLayoutOffsets {
    stride: PANA_SD_TOC_ENTRY_SIZE,
    name_end: 0x0A,
    ver_off: 0x0A,
    off_pos: 0x0C,
    hw_end: 0x18,
    flag_pos: 0x18,
    grp_start: 0x30,
    grp_end: 0x3C,
    sum_off: 0x3E,
    skip_empty: true,
};

const DVD64_TOC_LAYOUT: TocLayoutOffsets = TocLayoutOffsets {
    stride: PANA_SD_TOC_ENTRY_SIZE,
    name_end: 0x0A,
    ver_off: 0x0A,
    off_pos: 0x0C,
    hw_end: 0x18,
    flag_pos: 0x19,
    grp_start: 0x1A,
    grp_end: 0x20,
    sum_off: 0x3E,
    skip_empty: true,
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

/// Decrypts the `.FRM` TOC using the detected Feistel key and auto-discriminates `Dvd48B` vs `Dvd64B`.
pub fn decrypt_frm_toc_with_detected_key(
    bytes: &[u8],
) -> (
    Option<&'static str>,
    [u8; 32],
    TocLayout,
    Vec<DecryptedFrmTocEntry>,
) {
    let probe_len = bytes.len().min(0x80);
    let (key_name, round_key) = if probe_len >= 8 {
        detect_frm_feistel_key(&bytes[..probe_len])
            .unwrap_or(("old_df72", PANA_FEISTEL_ROUND_KEY_32B))
    } else {
        ("old_df72", PANA_FEISTEL_ROUND_KEY_32B)
    };

    let toc_span = bytes.len().min(PANA_FRM_TOC_REGION_SIZE) & !7;
    if toc_span == 0 {
        return (Some(key_name), round_key, TocLayout::Dvd48B, Vec::new());
    }

    let mut decrypted = bytes[..toc_span].to_vec();
    feistel_decrypt_buffer(&mut decrypted, &round_key, &PANA_FEISTEL_SBOX_256);

    let count64 = decrypted.len() / PANA_SD_TOC_ENTRY_SIZE;
    if count64 > 0 {
        let entries64 = parse_toc_entries_with_layout(&decrypted, count64, &DVD64_TOC_LAYOUT);
        let valid_named_64 = entries64
            .iter()
            .filter(|e| e.list_sum_valid && !e.module_name.trim().is_empty())
            .count();

        let is_64b = if valid_named_64 > 0 {
            let entry0_ok = entries64[0].module_offset >= 0x2000;
            let pfx40_ok = if decrypted.len() >= 0x44 {
                const PREFIXES: [&[u8; 4]; 4] = [b"PROG", b"DRV ", b"MAIN", b"STB "];
                PREFIXES.iter().any(|pfx| &decrypted[0x40..0x44] == *pfx)
            } else {
                false
            };
            entry0_ok && (pfx40_ok || valid_named_64 >= 2)
        } else {
            false
        };

        if is_64b {
            let valid_entries64: Vec<DecryptedFrmTocEntry> = entries64
                .into_iter()
                .filter(|e| e.list_sum_valid && !e.module_name.trim().is_empty())
                .collect();
            return (
                Some(key_name),
                round_key,
                TocLayout::Dvd64B,
                valid_entries64,
            );
        }
    }

    let count48 = decrypted.len() / PANA_FRM_TOC_ENTRY_SIZE;
    let entries48 = parse_toc_entries_with_layout(&decrypted, count48, &DVD_TOC_LAYOUT);
    let valid_entries48: Vec<DecryptedFrmTocEntry> = entries48
        .into_iter()
        .filter(|e| e.list_sum_valid && !e.module_name.trim().is_empty())
        .collect();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpret_frm_target_flag() {
        assert_eq!(interpret_frm_target_flag("MAIN3410PQ", 1), "FORCE_UPDATE");
        assert_eq!(
            interpret_frm_target_flag("MAIN3060NP", 0),
            "NORMAL_VERSION_CHECK"
        );
        assert_eq!(interpret_frm_target_flag("PROG061 ", 1), "FORMAT_HDD");
        assert_eq!(
            interpret_frm_target_flag("PROG061 ", 0),
            "NORMAL_PRESERVE_HDD"
        );
        assert_eq!(interpret_frm_target_flag("DRV S211", 0), "NORMAL");
        assert_eq!(interpret_frm_target_flag("DRV S211", 1), "FLAG_SET");
    }

    #[test]
    fn parse_sd_frm_toc_with_layout_discriminates_64b_vs_48b() {
        let mut sd64 = vec![0u8; 0x2000];
        sd64[0..8].copy_from_slice(b"DRV S211");
        sd64[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
        let sum64: u16 = sd64[..0x3E]
            .iter()
            .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
        sd64[0x3E..0x40].copy_from_slice(&sum64.to_le_bytes());
        let (layout64, entries64) = parse_sd_frm_toc_with_layout(&sd64);
        assert_eq!(layout64, Some(TocLayout::Sd64B));
        assert_eq!(entries64.len(), 1);

        let mut sd48 = vec![0u8; 0x1800];
        sd48[0..8].copy_from_slice(b"DRV S211");
        sd48[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
        let sum48: u16 = sd48[..0x2E]
            .iter()
            .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
        sd48[0x2E..0x30].copy_from_slice(&sum48.to_le_bytes());
        let (layout48, entries48) = parse_sd_frm_toc_with_layout(&sd48);
        assert_eq!(layout48, Some(TocLayout::Sd48B));
        assert_eq!(entries48.len(), 1);
    }

    #[test]
    fn test_decrypt_frm_toc_with_detected_key_synthetic_dvd64() {
        use super::super::feistel::feistel_encrypt_buffer;

        let mut plain = vec![0u8; 0x1800];
        // Entry 0 (64 bytes)
        plain[0..7].copy_from_slice(b"PROG071");
        plain[0x0C..0x10].copy_from_slice(&0x00002000u32.to_le_bytes()); // offset >= 0x2000
        plain[0x10..0x16].copy_from_slice(b"EH49EG");
        plain[0x18] = b'M';
        plain[0x19] = 1; // target_flag
        plain[0x1A..0x20].copy_from_slice(b"000000");
        plain[0x20..0x24].copy_from_slice(&0x00020000u32.to_le_bytes()); // size
        plain[0x24..0x28].copy_from_slice(&0x0102D54Fu32.to_le_bytes());
        let sum0: u16 = plain[..0x3E]
            .iter()
            .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
        plain[0x3E..0x40].copy_from_slice(&sum0.to_le_bytes());

        // Entry 1 (64 bytes)
        plain[0x40..0x47].copy_from_slice(b"PROG071");
        plain[0x4C..0x50].copy_from_slice(&0x00022000u32.to_le_bytes());
        plain[0x50..0x56].copy_from_slice(b"EH49EC");
        plain[0x58] = b'M';
        plain[0x59] = 1;
        plain[0x5A..0x60].copy_from_slice(b"000000");
        plain[0x60..0x64].copy_from_slice(&0x00020000u32.to_le_bytes());
        let sum1: u16 = plain[0x40..0x7E]
            .iter()
            .fold(0u16, |a, &b| a.wrapping_add(u16::from(b)));
        plain[0x7E..0x80].copy_from_slice(&sum1.to_le_bytes());

        // Encrypt with old_df72
        let mut encrypted = plain.clone();
        feistel_encrypt_buffer(
            &mut encrypted,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );

        let (key_name, _, layout, entries) = decrypt_frm_toc_with_detected_key(&encrypted);
        assert_eq!(key_name, Some("old_df72"));
        assert_eq!(layout, TocLayout::Dvd64B);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].module_name, "PROG071");
        assert_eq!(entries[0].hardware_family, "EH49EG");
        assert_eq!(entries[0].target_flag, 1);
        assert!(entries[0].list_sum_valid);
        assert_eq!(entries[1].module_name, "PROG071");
        assert_eq!(entries[1].hardware_family, "EH49EC");
        assert!(entries[1].list_sum_valid);
    }

    #[test]
    fn test_decrypt_frm_toc_with_detected_key_real_firmwares() {
        let cases = [
            (
                "firmware_and_images/PANA_DVD.FRM",
                "../../firmware_and_images/PANA_DVD.FRM",
                "old_df72",
                TocLayout::Dvd48B,
            ),
            (
                "firmware_and_images/dmr-eh50/PANA_DVD.FRM",
                "../../firmware_and_images/dmr-eh50/PANA_DVD.FRM",
                "le2011_08e0",
                TocLayout::Dvd48B,
            ),
            (
                "firmware_and_images/dmr-eh49_59_69/eh49/PANA_DVD.FRM",
                "../../firmware_and_images/dmr-eh49_59_69/eh49/PANA_DVD.FRM",
                "old_df72",
                TocLayout::Dvd64B,
            ),
        ];

        for (p1, p2, exp_key, exp_layout) in cases {
            if let Ok(data) = std::fs::read(p1).or_else(|_| std::fs::read(p2)) {
                let (key_name, _, layout, entries) = decrypt_frm_toc_with_detected_key(&data);
                assert_eq!(key_name, Some(exp_key), "Checking key for {p1}");
                assert_eq!(layout, exp_layout, "Checking layout for {p1}");
                assert!(!entries.is_empty(), "Entries must not be empty for {p1}");
                for entry in &entries {
                    assert!(
                        entry.list_sum_valid,
                        "Entry {} list_sum must be valid in {p1}",
                        entry.index
                    );
                    assert!(!entry.module_name.trim().is_empty());
                }
            }
        }
    }
}
