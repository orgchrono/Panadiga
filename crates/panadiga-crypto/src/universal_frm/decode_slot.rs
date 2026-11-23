#![deny(unsafe_code)]

//! Per-module slot decoding helpers (`EXTRHEAD`, `MainList`, `DRV1`/`DRV2`, STB/plain payloads).

use super::decode::parse_version_u16_hex;
use super::types::{ModuleEncryptionKind, UniversalFrmModule};
use crate::codec::{compute_fvu_byte_sum, extract_c_ascii, try_decode_stb_slot};
use crate::constants::{PANA_FEISTEL_SBOX_256, PANA_FRM_TOC_ENTRY_SIZE};
use crate::extrhead::{
    ExtrheadCryptMode, decompress_pana_lzss, decompress_payload_by_algo, unpack_extrhead_with_key,
};
use crate::feistel::{adler32_panasonic_strided, feistel_decrypt_buffer};
use crate::pana_fw_go::{extract_drv_module, parse_main_list, split_main_partitions};
use crate::types::DecryptedFrmTocEntry;

pub(super) fn check_adler_or_bytesum(bytes: &[u8], expected: u32) -> bool {
    adler32_panasonic_strided(bytes, 1) == expected || compute_fvu_byte_sum(bytes) == expected
}

pub(super) fn algo_label(algo: u16) -> String {
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
pub(super) fn decode_module_slot(
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
