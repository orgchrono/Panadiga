//! Multi-segment `MainList` (`20 + 8*N` header + `EXTRHEAD` segments) parser and packer.

use anyhow::{Context, Result, bail};

use super::super::codec::read_u32_le;
use super::super::extrhead::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, pack_extrhead_with_options,
    unpack_extrhead_with_key,
};
use super::super::feistel::adler32_panasonic_strided;
use super::crypto_utils::apply_edge_crypt_order;
use super::types::{
    MAIN_LIST_LEGACY_CRYPT_SPAN, MAIN_LIST_LEGACY_SEGMENT_TARGET_SIZE, MAIN_LIST_MODERN_CRYPT_SPAN,
    MainListEntry, MainListHeader, UnpackedMainList,
};
use crate::constants::{
    PANA_FEISTEL_ROUND_KEY_32B, PANA_MAIN_LIST_ENTRY_SIZE, PANA_MAIN_LIST_HEADER_SIZE,
};

fn main_list_crypt_span(decompressed_segment_size: u32) -> usize {
    if decompressed_segment_size == MAIN_LIST_LEGACY_SEGMENT_TARGET_SIZE {
        MAIN_LIST_LEGACY_CRYPT_SPAN
    } else {
        MAIN_LIST_MODERN_CRYPT_SPAN
    }
}

/// Parses and decrypts a multi-segment `MainList` (`MAIN` module payload in modern/split `.FRM` containers).
pub fn parse_main_list(raw: &[u8], round_key: Option<&[u8; 32]>) -> Result<UnpackedMainList> {
    if raw.len() < PANA_MAIN_LIST_HEADER_SIZE {
        bail!(
            "MainList header too short ({} bytes < {PANA_MAIN_LIST_HEADER_SIZE})",
            raw.len()
        );
    }
    let checksum = read_u32_le(raw, 0x00);
    let unknown1 = read_u32_le(raw, 0x04);
    let list_size = read_u32_le(raw, 0x08);
    let decompressed_segment_size = read_u32_le(raw, 0x0C);
    let compression_type = read_u32_le(raw, 0x10);

    if (list_size as usize) < PANA_MAIN_LIST_HEADER_SIZE
        || !(list_size as usize - PANA_MAIN_LIST_HEADER_SIZE)
            .is_multiple_of(PANA_MAIN_LIST_ENTRY_SIZE)
    {
        bail!("Invalid MainList list_size {list_size}");
    }
    let list_size_us = list_size as usize;
    if list_size_us > raw.len() {
        bail!(
            "MainList header truncated: needs {list_size_us} bytes, got {}",
            raw.len()
        );
    }
    let header_checksum_valid = adler32_panasonic_strided(&raw[4..list_size_us], 1) == checksum;

    let count = (list_size_us - PANA_MAIN_LIST_HEADER_SIZE) / PANA_MAIN_LIST_ENTRY_SIZE;
    let crypt_span = main_list_crypt_span(decompressed_segment_size);
    let mut entries = Vec::with_capacity(count);
    let mut segments = Vec::with_capacity(count);
    let mut data = Vec::new();
    let mut cursor = list_size_us;

    for idx in 0..count {
        let entry_off = PANA_MAIN_LIST_HEADER_SIZE + idx * PANA_MAIN_LIST_ENTRY_SIZE;
        let seg_size = read_u32_le(raw, entry_off);
        let seg_sum = read_u32_le(raw, entry_off + 4);
        let seg_end = cursor
            .checked_add(seg_size as usize)
            .context("MainList segment offset overflow")?;
        if seg_end > raw.len() {
            bail!(
                "MainList segment {idx} ({cursor}..{seg_end}) exceeds input length {}",
                raw.len()
            );
        }

        let mut seg_buf = raw[cursor..seg_end].to_vec();
        let checksum_valid = adler32_panasonic_strided(&seg_buf, 1) == seg_sum;
        if let Some(rk) = round_key {
            if seg_buf.len() < crypt_span {
                bail!(
                    "MainList segment {idx} ({} B) smaller than crypt span {crypt_span} B",
                    seg_buf.len()
                );
            }
            apply_edge_crypt_order(&mut seg_buf, crypt_span, rk, false);
        }

        let unpacked = unpack_extrhead_with_key(&seg_buf, ExtrheadCryptMode::Plain, None)
            .with_context(|| format!("Unpacking MainList segment {idx}"))?;
        data.extend_from_slice(&unpacked.decompressed);
        segments.push(unpacked);
        entries.push(MainListEntry {
            size: seg_size,
            checksum: seg_sum,
            checksum_valid,
        });
        cursor = seg_end;
    }

    Ok(UnpackedMainList {
        header: MainListHeader {
            checksum,
            unknown1,
            list_size,
            decompressed_segment_size,
            compression_type,
        },
        header_checksum_valid,
        entries,
        segments,
        data,
    })
}

/// Packs `payload` into a multi-segment `MainList` (`20 + 8*N` header + partially encrypted `EXTRHEAD` segments).
#[allow(clippy::too_many_arguments)]
pub fn pack_main_list(
    payload: &[u8],
    tag: &str,
    build_name: &str,
    algo: ExtrheadCompressionAlgo,
    target_segment_size: u32,
    round_key: Option<&[u8; 32]>,
    adler_step: u32,
    flag: u16,
) -> Result<Vec<u8>> {
    if target_segment_size == 0 {
        bail!("target_segment_size must be non-zero");
    }
    let chunk_size = target_segment_size as usize;
    let crypt_span = main_list_crypt_span(target_segment_size);
    let slices: Vec<&[u8]> = if payload.is_empty() {
        vec![&[][..]]
    } else {
        payload.chunks(chunk_size).collect()
    };

    let list_size = (20 + slices.len() * 8) as u32;
    let mut table = vec![0u8; list_size as usize];
    table[0x04..0x08].copy_from_slice(&1u32.to_le_bytes());
    table[0x08..0x0C].copy_from_slice(&list_size.to_le_bytes());
    table[0x0C..0x10].copy_from_slice(&target_segment_size.to_le_bytes());
    table[0x10..0x14].copy_from_slice(&u32::from(algo as u16).to_le_bytes());

    let mut body = Vec::new();
    for (idx, part) in slices.iter().enumerate() {
        let mut seg = pack_extrhead_with_options(
            part,
            tag,
            0,
            build_name,
            ExtrheadCryptMode::Plain,
            algo,
            &PANA_FEISTEL_ROUND_KEY_32B,
            adler_step,
            flag,
        )?;
        if let Some(rk) = round_key {
            if seg.len() < crypt_span {
                seg.resize((crypt_span + 7) & !7, 0);
            }
            apply_edge_crypt_order(&mut seg, crypt_span, rk, true);
        }
        let seg_sum = adler32_panasonic_strided(&seg, 1);
        let entry_off = 20 + idx * 8;
        table[entry_off..entry_off + 4].copy_from_slice(&(seg.len() as u32).to_le_bytes());
        table[entry_off + 4..entry_off + 8].copy_from_slice(&seg_sum.to_le_bytes());
        body.extend_from_slice(&seg);
    }

    let hdr_sum = adler32_panasonic_strided(&table[4..], 1);
    table[0..4].copy_from_slice(&hdr_sum.to_le_bytes());
    table.extend_from_slice(&body);
    Ok(table)
}
