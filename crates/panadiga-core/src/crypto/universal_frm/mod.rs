#![deny(unsafe_code)]

//! Unified Legacy DVD/HDD + Blu-ray/UHD `.FRM` Auto-Detector, Unpacker & Repacker.
//!
//! Combines Panadiga's legacy Matsushita AM34/MN103S formats (`Dvd48B`, `Dvd64B`, `Sd48B`,
//! `Sd64B`, `STB`, `DIGADIAG`) with the Blu-ray / `PANAEUSB.FRM` stack from `pana-fw` &
//! `unixtract` (`AES-128-CBC + Feistel` `AESHeader`, `BlurayFeistel` 48B Adler-32 TOC,
//! `PANASONIC\0\0\0` 48B inner header, `VGD ` exclusion, `DRV1`/`DRV2` edge cipher, and
//! `MainList` GZIP/LZSS partition streams).

pub mod repack;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use super::aes128::{FrmKeyring, PanaAesKeyPair};
use super::codec::{compute_fvu_byte_sum, extract_c_ascii, read_u32_le, try_decode_stb_slot};
use super::extrhead::{
    ExtrheadCryptMode, decompress_pana_lzss, decompress_payload_by_algo, unpack_extrhead_with_key,
};
use super::feistel::{
    adler32_panasonic_strided, derive_round_key_from_custom_key_8b, feistel_decrypt_buffer,
};
use super::pana_fw_go::{
    AesHeader, detect_and_unwrap_aes_frm, extract_drv_module, find_feistel_key_slow_with_keyring,
    parse_main_list, split_main_partitions,
};
use super::toc::{
    TocLayout, decrypt_frm_toc_with_keyring, parse_sd_frm_toc_with_layout,
    strip_panasonic_outer_header,
};
use super::types::DecryptedFrmTocEntry;
use crate::constants::{
    PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256, PANA_FRM_TOC_ENTRY_SIZE,
    PANA_SD_TOC_ENTRY_SIZE, PANA_SD_TOC_REGION_SIZE,
};

pub use repack::{
    FrmRepackOptions, FrmTargetMode, RepackCompression, UniversalFrmRepackReport,
    repack_universal_frm,
};

/// Discriminated Panasonic `.FRM` container format covering Legacy DVD/HDD, SD, and Blu-ray lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UniversalFrmFormat {
    LegacyDvd48B,
    LegacyDvd64B,
    SdPlain48B,
    SdPlain64B,
    BlurayFeistel,
    BlurayAes,
}

/// Outer/inner encryption strategy for a single firmware module payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleEncryptionKind {
    Plain,
    FullFeistel,
    BlurayDrvEdge,
    MainListPartial,
}

/// Decoded single module slot inside a `.FRM` container or Blu-ray sub-container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalFrmModule {
    pub index: usize,
    pub name: String,
    pub toc_tag_4b: String,
    pub version_ascii: String,
    pub version_u16: u16,
    pub module_offset: u32,
    pub module_size: u32,
    pub hardware_family: String,
    pub oem: u8,
    pub target_flag: u8,
    pub model_group: String,
    pub checksum_u32: u32,
    pub toc_compression_type: u8,
    pub toc_sum_valid: bool,
    pub data_sum_valid: bool,
    pub encoding: String,
    pub compression_algo: Option<String>,
    pub build_name: Option<String>,
    pub dst_addr_hex: Option<String>,
    pub partitions: Vec<String>,
    pub main_list_meta: Option<(u32, u32, bool)>,
    pub extrhead_meta: Option<(u16, String, u32, u32, u16)>,
    pub encryption_kind: ModuleEncryptionKind,
    #[serde(skip)]
    pub raw_toc_entry: Vec<u8>,
    #[serde(skip)]
    pub pre_decode_slot_bytes: Vec<u8>,
    #[serde(skip)]
    pub payload: Vec<u8>,
}

/// Decoded inner `.FRM` container (directly the file for Legacy/BD-Feistel, or `1..=4` children for `BlurayAes`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalFrmSubContainer {
    pub sub_index: usize,
    pub file_offset: u32,
    pub format: UniversalFrmFormat,
    pub feistel_key_name: Option<String>,
    pub toc_encrypted: bool,
    pub has_panasonic_header: bool,
    pub toc_stride_bytes: usize,
    pub toc_region_size: usize,
    pub uses_adler32_sums: bool,
    pub modules: Vec<UniversalFrmModule>,
    #[serde(skip)]
    pub feistel_round_key: [u8; 32],
    #[serde(skip)]
    pub raw_outer_48b: Option<[u8; 48]>,
}

/// Complete decoded representation of any Panasonic `.FRM` firmware update file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalFrmContainer {
    pub format: UniversalFrmFormat,
    pub aes_key_name: Option<String>,
    pub aes_header: Option<AesHeader>,
    pub sub_containers: Vec<UniversalFrmSubContainer>,
    #[serde(skip)]
    pub aes_key_pair: Option<PanaAesKeyPair>,
}

fn check_adler_or_bytesum(bytes: &[u8], expected: u32) -> bool {
    adler32_panasonic_strided(bytes, 1) == expected || compute_fvu_byte_sum(bytes) == expected
}

fn algo_label(algo: u16) -> String {
    match algo {
        0 => "zlib".to_string(),
        1 => "gzip".to_string(),
        2 => "lzss".to_string(),
        3 => "lzma".to_string(),
        other => format!("algo_{other}"),
    }
}

struct DecodedSlot {
    payload: Vec<u8>,
    encoding: String,
    compression_algo: Option<String>,
    build_name: Option<String>,
    dst_addr_hex: Option<String>,
    extrhead_meta: Option<(u16, String, u32, u32, u16)>,
}

fn decode_extrhead_or_raw(
    raw: &[u8],
    rk_opt: Option<&[u8; 32]>,
    fallback_algo_byte: u8,
) -> DecodedSlot {
    if let Ok(ext) = unpack_extrhead_with_key(raw, ExtrheadCryptMode::Auto, rk_opt) {
        return DecodedSlot {
            payload: ext.decompressed,
            encoding: "EXTRHEAD".to_string(),
            compression_algo: Some(algo_label(ext.header.algo)),
            build_name: Some(ext.header.build_name.clone()),
            dst_addr_hex: Some(format!("0x{:08X}", ext.header.dst_addr)),
            extrhead_meta: Some((
                ext.header.algo,
                ext.header.build_name,
                ext.header.dst_addr,
                ext.header.adler_step,
                ext.header.flag,
            )),
        };
    }
    if let Some(stb) = try_decode_stb_slot(raw) {
        return DecodedSlot {
            payload: stb.bytes,
            encoding: if stb.is_zlib {
                "STB_ZLIB".to_string()
            } else {
                "STB_RAW".to_string()
            },
            compression_algo: stb.is_zlib.then(|| "zlib".to_string()),
            build_name: stb.model,
            dst_addr_hex: Some("0x80000400".to_string()),
            extrhead_meta: None,
        };
    }
    if (fallback_algo_byte & 1) != 0
        && raw.starts_with(&[0x1F, 0x8B])
        && let Ok(dec) = decompress_payload_by_algo(raw, 1, 0)
    {
        return DecodedSlot {
            payload: dec,
            encoding: "GZIP".to_string(),
            compression_algo: Some("gzip".to_string()),
            build_name: None,
            dst_addr_hex: None,
            extrhead_meta: None,
        };
    }
    if fallback_algo_byte == 2
        && !raw.is_empty()
        && let dec = decompress_pana_lzss(raw, 0)
        && !dec.is_empty()
    {
        return DecodedSlot {
            payload: dec,
            encoding: "LZSS".to_string(),
            compression_algo: Some("lzss".to_string()),
            build_name: None,
            dst_addr_hex: None,
            extrhead_meta: None,
        };
    }
    DecodedSlot {
        payload: raw.to_vec(),
        encoding: "RAW".to_string(),
        compression_algo: None,
        build_name: None,
        dst_addr_hex: None,
        extrhead_meta: None,
    }
}

fn feistel_decrypt_slot(raw: &[u8], start: usize, size: usize, rk: &[u8; 32]) -> Vec<u8> {
    let aligned_end = (start + ((size + 7) & !7)).min(raw.len());
    let mut buf = raw[start..aligned_end].to_vec();
    feistel_decrypt_buffer(&mut buf, rk, &PANA_FEISTEL_SBOX_256);
    buf.truncate(size);
    buf
}

struct ParsedMainListSlot {
    payload: Vec<u8>,
    compression_algo: Option<String>,
    partitions: Vec<String>,
    main_list_meta: Option<(u32, u32, bool)>,
    data_sum_valid: bool,
    enc_kind: ModuleEncryptionKind,
}

fn parse_main_list_slot(
    slot_bytes: &[u8],
    rk: &[u8; 32],
    expected_sum: u32,
    uses_adler32: bool,
    enc_kind: ModuleEncryptionKind,
) -> Option<ParsedMainListSlot> {
    let ml = parse_main_list(slot_bytes, Some(rk)).ok()?;
    let data_sum_valid = check_adler_or_bytesum(slot_bytes, expected_sum);
    let compression_algo = ml.segments.first().map(|s| algo_label(s.header.algo));
    let partitions = split_main_partitions(&ml.data)
        .map(|ps| ps.into_iter().map(|p| p.name).collect())
        .unwrap_or_default();
    let main_list_meta = Some((
        ml.header.decompressed_segment_size,
        ml.header.compression_type,
        !uses_adler32,
    ));
    Some(ParsedMainListSlot {
        payload: ml.data,
        compression_algo,
        partitions,
        main_list_meta,
        data_sum_valid,
        enc_kind,
    })
}

#[allow(clippy::too_many_arguments)]
fn decode_module_slot(
    raw: &[u8],
    raw_toc_entry: &[u8],
    entry: &DecryptedFrmTocEntry,
    stride: usize,
    uses_adler32: bool,
    outer_encrypted: bool,
    rk: &[u8; 32],
    is_bluray_rules: bool,
) -> anyhow::Result<UniversalFrmModule> {
    let start = entry.module_offset as usize;
    let size = entry.module_size as usize;
    let end = start
        .checked_add(size)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' offset+size overflow", entry.module_name))?;
    if end > raw.len() {
        anyhow::bail!(
            "Module '{}' range 0x{:X}..0x{:X} exceeds buffer ({} B)",
            entry.module_name,
            start,
            end,
            raw.len()
        );
    }

    let raw_slice = &raw[start..end];
    let up = entry.module_name.trim().to_ascii_uppercase();
    let tag_4b = extract_c_ascii(&raw_toc_entry[0..4]);
    let ver_ascii = if stride == PANA_FRM_TOC_ENTRY_SIZE
        && raw_toc_entry[4..8]
            .iter()
            .all(|&b| b == 0 || b.is_ascii_graphic())
    {
        extract_c_ascii(&raw_toc_entry[4..8])
    } else {
        String::new()
    };
    let ver_u16 = parse_version_u16_hex(&entry.version_u16_hex);
    let toc_comp = if stride == PANA_FRM_TOC_ENTRY_SIZE {
        raw_toc_entry[0x2B]
    } else {
        0
    };

    // 1. MAIN / SMAI (unencrypted MainList OR Legacy Feistel+EXTRHEAD)
    let mut enc_kind = if outer_encrypted {
        ModuleEncryptionKind::FullFeistel
    } else {
        ModuleEncryptionKind::Plain
    };
    let mut pre_decode = Vec::new();
    let mut payload = Vec::new();
    let mut encoding = "RAW".to_string();
    let mut compression_algo = None;
    let mut build_name = None;
    let mut dst_addr_hex = None;
    let mut partitions = Vec::new();
    let mut main_list_meta = None;
    let mut extrhead_meta = None;
    let mut data_sum_valid = false;
    let mut apply_ds = false;

    if up.starts_with("MAIN") || up.starts_with("SMAI") {
        if let Some(parsed) = parse_main_list_slot(
            raw_slice,
            rk,
            entry.fvu_checksum_u32,
            uses_adler32,
            ModuleEncryptionKind::MainListPartial,
        ) {
            pre_decode = raw_slice.to_vec();
            payload = parsed.payload;
            encoding = "MainList".to_string();
            compression_algo = parsed.compression_algo;
            partitions = parsed.partitions;
            main_list_meta = parsed.main_list_meta;
            data_sum_valid = parsed.data_sum_valid;
            enc_kind = parsed.enc_kind;
        } else if outer_encrypted {
            let dec = feistel_decrypt_slot(raw, start, size, rk);
            if let Some(parsed) = parse_main_list_slot(
                &dec,
                rk,
                entry.fvu_checksum_u32,
                uses_adler32,
                ModuleEncryptionKind::FullFeistel,
            ) {
                pre_decode = dec;
                payload = parsed.payload;
                encoding = "MainList".to_string();
                compression_algo = parsed.compression_algo;
                partitions = parsed.partitions;
                main_list_meta = parsed.main_list_meta;
                data_sum_valid = parsed.data_sum_valid;
                enc_kind = parsed.enc_kind;
            } else {
                pre_decode = dec;
                apply_ds = true;
            }
        }
    }

    if pre_decode.is_empty() {
        let skip_outer = is_bluray_rules && up == "VGD";
        if skip_outer || !outer_encrypted {
            enc_kind = ModuleEncryptionKind::Plain;
            pre_decode = raw_slice.to_vec();
            apply_ds = true;
        } else if is_bluray_rules
            && up.starts_with("DRV")
            && adler32_panasonic_strided(raw_slice, 1) == entry.fvu_checksum_u32
            && let Ok(drv) = extract_drv_module(raw_slice, Some(rk))
        {
            enc_kind = ModuleEncryptionKind::BlurayDrvEdge;
            data_sum_valid = true;
            pre_decode = raw_slice.to_vec();
            payload = drv.payload;
            encoding = "DRV_EDGE".to_string();
            build_name = drv.header.map(|h| h.model);
            compression_algo = drv.extrhead.map(|u| algo_label(u.header.algo));
        } else {
            pre_decode = feistel_decrypt_slot(raw, start, size, rk);
            if up.starts_with("PROG") {
                data_sum_valid = check_adler_or_bytesum(&pre_decode, entry.fvu_checksum_u32);
                payload = pre_decode.clone();
                encoding = "RAW_BOOT".to_string();
            } else {
                apply_ds = true;
            }
        }
    }

    if apply_ds {
        data_sum_valid = check_adler_or_bytesum(&pre_decode, entry.fvu_checksum_u32);
        let ds = decode_extrhead_or_raw(&pre_decode, Some(rk), toc_comp);
        payload = ds.payload;
        encoding = ds.encoding;
        compression_algo = ds.compression_algo;
        build_name = ds.build_name;
        dst_addr_hex = ds.dst_addr_hex;
        extrhead_meta = ds.extrhead_meta;
    }

    Ok(UniversalFrmModule {
        index: entry.index,
        name: entry.module_name.clone(),
        toc_tag_4b: tag_4b,
        version_ascii: ver_ascii,
        version_u16: ver_u16,
        module_offset: entry.module_offset,
        module_size: entry.module_size,
        hardware_family: entry.hardware_family.clone(),
        oem: raw_toc_entry[0x18],
        target_flag: entry.target_flag,
        model_group: entry.model_group.clone(),
        checksum_u32: entry.fvu_checksum_u32,
        toc_compression_type: toc_comp,
        toc_sum_valid: entry.list_sum_valid,
        data_sum_valid,
        encoding,
        compression_algo,
        build_name,
        dst_addr_hex,
        partitions,
        main_list_meta,
        extrhead_meta,
        encryption_kind: enc_kind,
        raw_toc_entry: raw_toc_entry.to_vec(),
        pre_decode_slot_bytes: pre_decode,
        payload,
    })
}

fn parse_version_u16_hex(raw: &str) -> u16 {
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
