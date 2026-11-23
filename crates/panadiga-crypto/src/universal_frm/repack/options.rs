#![deny(unsafe_code)]

//! Declarative options, module rebuild dispatch, and target-format resolution for `.FRM` repacking.

use std::str::FromStr;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::super::types::{ModuleEncryptionKind, UniversalFrmFormat, UniversalFrmModule};
use crate::constants::PANA_EXTRHEAD_DEFAULT_ADLER_STEP;
use crate::extrhead::{ExtrheadCompressionAlgo, ExtrheadCryptMode, pack_extrhead_with_options};
use crate::frm_pack::FrmContainerMode;
use crate::pana_fw_go::{MAIN_LIST_MODERN_SEGMENT_TARGET_SIZE, pack_drv_module, pack_main_list};

/// Requested target format when repacking a `.FRM` update image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrmTargetMode {
    Preserve,
    Dvd,
    Dvd64b,
    Sd,
    Sd48,
    Bluray,
    BlurayAes,
}

fn parse_token_from_table<T: Copy>(
    raw: &str,
    table: &[(&str, T)],
    flag: &str,
    expected: &str,
) -> Result<T> {
    let norm = raw.trim().to_ascii_lowercase();
    table
        .iter()
        .find(|&&(k, _)| k == norm)
        .map(|&(_, v)| v)
        .ok_or_else(|| anyhow::anyhow!("Unsupported {flag} '{raw}': expected {expected}"))
}

impl FromStr for FrmTargetMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        parse_token_from_table(
            raw,
            &[
                ("preserve", Self::Preserve),
                ("auto", Self::Preserve),
                ("dvd", Self::Dvd),
                ("dvd48", Self::Dvd),
                ("dvd64", Self::Dvd64b),
                ("dvd64b", Self::Dvd64b),
                ("sd", Self::Sd),
                ("sd64", Self::Sd),
                ("sd48", Self::Sd48),
                ("bluray", Self::Bluray),
                ("bd", Self::Bluray),
                ("bluray-aes", Self::BlurayAes),
                ("bluray_aes", Self::BlurayAes),
                ("aes", Self::BlurayAes),
            ],
            "--target-mode",
            "preserve, dvd, dvd64b, sd, sd48, bluray, or bluray-aes",
        )
    }
}

/// Requested inner compression method when rebuilding modified `MAIN` payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepackCompression {
    Preserve,
    Gzip,
    Lzss,
    Zlib,
}

impl FromStr for RepackCompression {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        parse_token_from_table(
            raw,
            &[
                ("preserve", Self::Preserve),
                ("auto", Self::Preserve),
                ("gzip", Self::Gzip),
                ("gz", Self::Gzip),
                ("lzss", Self::Lzss),
                ("zlib", Self::Zlib),
            ],
            "--compression",
            "preserve, gzip, lzss, or zlib",
        )
    }
}

/// Declarative mutation and container options for `repack_universal_frm`.
#[derive(Debug, Clone)]
pub struct FrmRepackOptions {
    pub target_mode: FrmTargetMode,
    pub compression: RepackCompression,
    pub drop_aes: bool,
    pub patch_force: bool,
    pub patch_format_hdd: bool,
    pub patch_version_offset: Option<u32>,
    pub patch_modules: Vec<(String, Vec<u8>)>,
    pub patch_partitions: Vec<(String, Vec<u8>)>,
    pub verify: bool,
}

impl Default for FrmRepackOptions {
    fn default() -> Self {
        Self {
            target_mode: FrmTargetMode::Preserve,
            compression: RepackCompression::Preserve,
            drop_aes: false,
            patch_force: false,
            patch_format_hdd: false,
            patch_version_offset: None,
            patch_modules: Vec::new(),
            patch_partitions: Vec::new(),
            verify: true,
        }
    }
}

/// Structured audit report produced after repacking a Panasonic `.FRM` firmware update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalFrmRepackReport {
    pub source_format: UniversalFrmFormat,
    pub output_format: UniversalFrmFormat,
    pub output_bytes: usize,
    pub sub_containers: usize,
    pub modules_modified: Vec<String>,
    pub partitions_patched: Vec<String>,
    pub verified: bool,
}

pub(super) fn align_up(val: usize, align: usize) -> usize {
    if align == 0 || val.is_multiple_of(align) {
        val
    } else {
        val + (align - (val % align))
    }
}

fn select_compression_algo(
    req: RepackCompression,
    mod_info: &UniversalFrmModule,
) -> ExtrheadCompressionAlgo {
    match req {
        RepackCompression::Gzip => ExtrheadCompressionAlgo::Gzip,
        RepackCompression::Lzss => ExtrheadCompressionAlgo::Lzss,
        RepackCompression::Zlib => ExtrheadCompressionAlgo::Zlib,
        RepackCompression::Preserve => match mod_info.compression_algo.as_deref() {
            Some("lzss") => ExtrheadCompressionAlgo::Lzss,
            Some("zlib") => ExtrheadCompressionAlgo::Zlib,
            _ => ExtrheadCompressionAlgo::Gzip,
        },
    }
}

pub(super) fn bump_ascii_digits(slice: &mut [u8], delta: u32) -> bool {
    let Ok(s) = std::str::from_utf8(slice) else {
        return false;
    };
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let Ok(val) = s.parse::<u32>() else {
        return false;
    };
    let width = slice.len();
    let bumped = format!("{:0width$}", val.wrapping_add(delta));
    let bytes = bumped.as_bytes();
    if bytes.len() == width {
        slice.copy_from_slice(bytes);
    } else {
        slice.copy_from_slice(&bytes[bytes.len() - width..]);
    }
    true
}

pub(super) fn find_patch_for_module<'a>(
    mod_info: &UniversalFrmModule,
    patches: &'a [(String, Vec<u8>)],
) -> Option<(usize, &'a [u8])> {
    let trimmed = mod_info.name.trim();
    patches.iter().enumerate().find_map(|(i, (target, bytes))| {
        let t = target.trim();
        let hit = t.eq_ignore_ascii_case(trimmed) || t.eq_ignore_ascii_case(&mod_info.toc_tag_4b);
        hit.then_some((i, bytes.as_slice()))
    })
}

pub(super) fn rebuild_modified_module_slot(
    mod_info: &UniversalFrmModule,
    modified: bool,
    rk: &[u8; 32],
    opts: &FrmRepackOptions,
) -> Result<Vec<u8>> {
    let up = mod_info.name.trim().to_ascii_uppercase();
    let is_main = up.starts_with("MAIN") || up.starts_with("SMAI");
    let force_recompress = is_main && opts.compression != RepackCompression::Preserve;
    if !modified && !force_recompress {
        return Ok(mod_info.pre_decode_slot_bytes.clone());
    }

    if is_main {
        let algo = select_compression_algo(opts.compression, mod_info);
        if let Some((seg_sz, _ctype, _)) = mod_info.main_list_meta {
            let target_seg = if seg_sz == 0 {
                MAIN_LIST_MODERN_SEGMENT_TARGET_SIZE
            } else {
                seg_sz
            };
            return pack_main_list(
                &mod_info.payload,
                "DRV ",
                mod_info.build_name.as_deref().unwrap_or("PANADIGA"),
                algo,
                target_seg,
                Some(rk),
                PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
                1,
            );
        }
        if let Some((_, ref bld, dst_addr, adler_step, flag)) = mod_info.extrhead_meta {
            return pack_extrhead_with_options(
                &mod_info.payload,
                "MAIN",
                dst_addr,
                bld,
                ExtrheadCryptMode::Main,
                algo,
                rk,
                adler_step.max(1),
                flag,
            );
        }
    }

    if up.starts_with("DRV") && mod_info.encryption_kind == ModuleEncryptionKind::BlurayDrvEdge {
        return pack_drv_module(
            &mod_info.payload,
            None,
            Some(ExtrheadCompressionAlgo::Gzip),
            "DRV ",
            rk,
        );
    }

    Ok(mod_info.payload.clone())
}

pub(super) fn resolve_output_formats(
    src_format: UniversalFrmFormat,
    opts: &FrmRepackOptions,
) -> (UniversalFrmFormat, Option<FrmContainerMode>) {
    let eff = match opts.target_mode {
        FrmTargetMode::Preserve => {
            if src_format == UniversalFrmFormat::BlurayAes && opts.drop_aes {
                UniversalFrmFormat::BlurayFeistel
            } else {
                src_format
            }
        }
        FrmTargetMode::Dvd => UniversalFrmFormat::LegacyDvd48B,
        FrmTargetMode::Dvd64b => UniversalFrmFormat::LegacyDvd64B,
        FrmTargetMode::Sd => UniversalFrmFormat::SdPlain64B,
        FrmTargetMode::Sd48 => UniversalFrmFormat::SdPlain48B,
        FrmTargetMode::Bluray => UniversalFrmFormat::BlurayFeistel,
        FrmTargetMode::BlurayAes => {
            if opts.drop_aes {
                UniversalFrmFormat::BlurayFeistel
            } else {
                UniversalFrmFormat::BlurayAes
            }
        }
    };
    let legacy_mode = match eff {
        UniversalFrmFormat::LegacyDvd48B => Some(FrmContainerMode::Dvd),
        UniversalFrmFormat::LegacyDvd64B => Some(FrmContainerMode::Dvd64B),
        UniversalFrmFormat::SdPlain64B => Some(FrmContainerMode::Sd),
        UniversalFrmFormat::SdPlain48B => Some(FrmContainerMode::Sd48),
        UniversalFrmFormat::BlurayFeistel | UniversalFrmFormat::BlurayAes => None,
    };
    (eff, legacy_mode)
}
