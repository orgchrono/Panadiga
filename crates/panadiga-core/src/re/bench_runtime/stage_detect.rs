#![deny(unsafe_code)]

//! Low-level firmware stage fingerprinting, base-VA inference, and STB/LZMA unpacking helpers.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};

use super::slot_unpack::UnpackedSlotBytes;
use crate::constants::{AM34_BOOT_BIN_BASE, AM34_FLASH_BASE, AM34_MAIN_BASE_VA};
use crate::crypto::codec::read_u32_le;
use crate::crypto::{adler32_panasonic_strided, try_decode_stb_slot};
use crate::re::bgo_unbrick::PROG061_BASE_VA;
use crate::re::mn10300_traps::identify_firmware_revision;
use crate::ssot::{ResolvedModelProfile, is_valid_nor_flash_size};

pub(super) fn looks_like_valid_main_list(raw: &[u8]) -> bool {
    if raw.len() < 20 {
        return false;
    }
    let chk = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let lsz = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]) as usize;
    if lsz < 20 || lsz > raw.len() || !(lsz - 20).is_multiple_of(8) {
        return false;
    }
    let hdr_tail = &raw[4..lsz];
    let byte_sum = hdr_tail
        .iter()
        .fold(0u32, |acc, &b| acc.wrapping_add(u32::from(b)));
    adler32_panasonic_strided(hdr_tail, 1) == chk || byte_sum == chk
}

fn infer_prog_base_va(data: &[u8]) -> u32 {
    // Check early MOV imm32 instructions (`0xFC 0xDC..0xDF <u32>`) for load VA hints.
    let probe_end = data.len().min(0x400);
    for idx in 0..probe_end.saturating_sub(6) {
        if data[idx] == 0xFC && (0xDC..=0xDF).contains(&data[idx + 1]) {
            let imm = read_u32_le(data, idx + 2);
            if (0x5080_0000..0x5084_0000).contains(&imm) {
                return 0x5080_0000;
            }
            if (0x5003_0400..0x5006_0000).contains(&imm) {
                return 0x5003_0400;
            }
        }
    }
    PROG061_BASE_VA
}

pub(super) fn try_unpack_stb_zlib(stage_buf: &[u8]) -> Option<UnpackedSlotBytes> {
    let decoded = try_decode_stb_slot(stage_buf)?;
    Some(UnpackedSlotBytes {
        bytes: decoded.bytes,
        source_kind: if decoded.is_zlib {
            "FRM_FEISTEL+STB_ZLIB".to_string()
        } else {
            "FRM_FEISTEL+STB_RAW".to_string()
        },
        build_name: decoded.model,
        algo: decoded.is_zlib.then(|| "zlib".to_string()),
        dst_addr: Some(0x8000_0400),
    })
}

pub(super) fn try_extract_embedded_lzma(raw: &[u8]) -> Option<(usize, Vec<u8>)> {
    for idx in 0..raw.len().saturating_sub(32) {
        if matches!(raw[idx], 0x5D | 0x5C | 0x6D)
            && raw[idx + 1] == 0x00
            && raw[idx + 2] == 0x00
            && (raw[idx + 3] <= 0x80 || raw[idx + 3] == 0xFF)
        {
            let mut cursor = Cursor::new(&raw[idx..]);
            let mut out = Vec::new();
            if lzma_rs::lzma_decompress(&mut cursor, &mut out).is_ok() && out.len() >= 4096 {
                return Some((idx, out));
            }
        }
    }
    None
}

pub(super) fn infer_stage_kind_and_base(
    stage_name: &str,
    data: &[u8],
    resolved: Option<&ResolvedModelProfile>,
    fallback_dst: Option<u32>,
) -> (&'static str, u32) {
    let up = stage_name.trim().to_ascii_uppercase();
    let train = resolved.and_then(|r| r.firmware_train.as_ref());
    if up.contains("PROG") || up.contains("DIGADIAG") {
        let inferred = infer_prog_base_va(data);
        let base = if inferred != PROG061_BASE_VA {
            inferred
        } else {
            fallback_dst
                .filter(|&a| a != 0 && a != 0x9040_0000)
                .or_else(|| {
                    train
                        .and_then(|t| t.prog_load_va)
                        .filter(|&a| a != 0 && a != 0x9040_0000)
                })
                .unwrap_or(inferred)
        };
        return ("PROG", base);
    }
    if up.starts_with("STB") {
        return ("STB", fallback_dst.unwrap_or(0x8000_0400));
    }
    if data.starts_with(b"\n root=/dev/fma") {
        return ("MAIN_LINUX_NAND", 0x9010_0000);
    }
    if up.contains("DRV") {
        let base = fallback_dst
            .filter(|&a| a != 0 && a != 0x9020_2000)
            .or_else(|| {
                train
                    .and_then(|t| t.drv_load_va)
                    .filter(|&a| a != 0 && a != 0x9020_2000)
            })
            .unwrap_or(0x4000_0000);
        return ("DRV", base);
    }
    if up.contains("MAIN") {
        let base = fallback_dst
            .filter(|&a| a != 0)
            .or_else(|| train.and_then(|t| t.main_load_va))
            .unwrap_or(AM34_MAIN_BASE_VA);
        return ("MAIN", base);
    }
    if up.contains("BOOT") || up.contains("LOADER") || up.contains("IPL") {
        return ("IPL", AM34_BOOT_BIN_BASE);
    }
    if is_valid_nor_flash_size(data.len() as u64) {
        let non_mn103 = resolved
            .and_then(|r| r.soc_core.as_ref())
            .is_some_and(|c| !c.isa.to_ascii_uppercase().contains("MN103"));
        return (
            "NOR_FULL",
            if non_mn103 {
                0x0000_0000
            } else {
                AM34_FLASH_BASE
            },
        );
    }
    let rev = identify_firmware_revision(data);
    match rev.stage.as_str() {
        "DRV" => (
            "DRV",
            train
                .and_then(|t| t.drv_load_va)
                .filter(|&a| a != 0 && a != 0x9020_2000)
                .unwrap_or(0x4000_0000),
        ),
        "BOOT_OR_PROG061" => {
            if data.len() == 0x4_0000 && !data.windows(16).any(|w| w.starts_with(b"Loader ver")) {
                ("PROG", PROG061_BASE_VA)
            } else {
                ("IPL", AM34_BOOT_BIN_BASE)
            }
        }
        _ if data.len() > 0x40_0000 => ("MAIN", AM34_MAIN_BASE_VA),
        _ => ("DRV", 0x4000_0000),
    }
}

fn sanitize_slug(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "stage".to_string()
    } else {
        trimmed.to_string()
    }
}

pub(super) fn write_stage_if_dir(
    out_dir: Option<&Path>,
    idx: usize,
    stage_key: &str,
    bytes: &[u8],
) -> Result<Option<String>> {
    let Some(dir) = out_dir else {
        return Ok(None);
    };
    fs::create_dir_all(dir)
        .with_context(|| format!("Creating bench runtime output dir {}", dir.display()))?;
    let file_path = dir.join(format!("{idx:02}_{}.bin", sanitize_slug(stage_key)));
    fs::write(&file_path, bytes)
        .with_context(|| format!("Writing unpacked stage {}", file_path.display()))?;
    Ok(Some(file_path.to_string_lossy().into_owned()))
}
