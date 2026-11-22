#![deny(unsafe_code)]

//! Unified `.FRM` Repacker, Version/Flag Patcher, and `MAIN` `fmaX` Partition Rebuilder.

use std::str::FromStr;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::super::aes128::FrmKeyring;
use super::super::codec::compute_fvu_byte_sum;
use super::super::extrhead::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, pack_extrhead_with_options,
};
use super::super::feistel::{adler32_panasonic_strided, feistel_encrypt_buffer};
use super::super::frm_pack::{FrmContainerMode, FrmPackModule, pack_frm_with_key};
use super::super::pana_fw_go::{
    MAIN_LIST_MODERN_SEGMENT_TARGET_SIZE, pack_drv_module, pack_main_list,
    reassemble_main_partitions, wrap_aes_frm,
};
use super::{
    ModuleEncryptionKind, UniversalFrmContainer, UniversalFrmFormat, UniversalFrmModule,
    UniversalFrmSubContainer, unpack_universal_frm,
};
use crate::constants::{
    PANA_EXTRHEAD_DEFAULT_ADLER_STEP, PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256,
    PANA_FRM_TOC_ENTRY_SIZE, PANA_FRM_TOC_REGION_SIZE, PANA_SD_TOC_REGION_SIZE,
};

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

fn align_up(val: usize, align: usize) -> usize {
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

fn bump_ascii_digits(slice: &mut [u8], delta: u32) -> bool {
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

fn find_patch_for_module<'a>(
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

fn rebuild_modified_module_slot(
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

fn pack_bluray_subcontainer(
    sub: &UniversalFrmSubContainer,
    opts: &FrmRepackOptions,
    modified_flags: &[bool],
) -> Result<Vec<u8>> {
    let toc_start = if sub.has_panasonic_header {
        48usize
    } else {
        0usize
    };
    let first_offset = sub
        .modules
        .iter()
        .map(|m| m.module_offset as usize)
        .min()
        .unwrap_or(toc_start + PANA_FRM_TOC_REGION_SIZE)
        .max(toc_start + sub.modules.len() * PANA_FRM_TOC_ENTRY_SIZE);
    let first_offset = align_up(first_offset, 0x200).max(if sub.has_panasonic_header {
        0x2000
    } else {
        PANA_FRM_TOC_REGION_SIZE
    });

    let mut image = vec![0u8; first_offset];
    if let Some(hdr48) = sub.raw_outer_48b {
        image[..48].copy_from_slice(&hdr48);
    }

    for (idx, mod_info) in sub.modules.iter().enumerate() {
        let up = mod_info.name.trim().to_ascii_uppercase();
        let is_main = up.starts_with("MAIN") || up.starts_with("SMAI");
        let is_pad_mod = up.starts_with("PROG") || up.starts_with("DIAG");

        let mut raw_data = rebuild_modified_module_slot(
            mod_info,
            modified_flags[idx],
            &sub.feistel_round_key,
            opts,
        )?;
        if is_pad_mod && !raw_data.len().is_multiple_of(16) {
            raw_data.resize(align_up(raw_data.len(), 16), 0);
        }

        let data_sum = if sub.uses_adler32_sums {
            adler32_panasonic_strided(&raw_data, 1)
        } else {
            compute_fvu_byte_sum(&raw_data)
        };
        let plain_size = raw_data.len();

        let skip_outer = is_main
            || up == "VGD"
            || mod_info.encryption_kind == ModuleEncryptionKind::BlurayDrvEdge;
        let mut disk_bytes = raw_data;
        if !skip_outer && sub.toc_encrypted {
            let aligned = align_up(disk_bytes.len(), 16);
            disk_bytes.resize(aligned, 0);
            feistel_encrypt_buffer(
                &mut disk_bytes,
                &sub.feistel_round_key,
                &PANA_FEISTEL_SBOX_256,
            );
        }

        let mod_off = if idx == 0 {
            first_offset
        } else {
            let cur = align_up(image.len(), 0x1000);
            image.resize(cur, 0);
            cur
        };
        image.extend_from_slice(&disk_bytes);

        let mut slot = mod_info.raw_toc_entry.clone();
        if slot.len() < PANA_FRM_TOC_ENTRY_SIZE {
            slot.resize(PANA_FRM_TOC_ENTRY_SIZE, 0);
        }
        slot[0x0C..0x10].copy_from_slice(&u32::try_from(mod_off)?.to_le_bytes());
        slot[0x19] = mod_info.target_flag;
        slot[0x20..0x24].copy_from_slice(&u32::try_from(plain_size)?.to_le_bytes());
        slot[0x24..0x28].copy_from_slice(&data_sum.to_le_bytes());
        if let Some(delta) = opts.patch_version_offset
            && is_main
        {
            if !bump_ascii_digits(&mut slot[0x04..0x08], delta) {
                let ver = mod_info.version_u16;
                slot[0x0A..0x0C].copy_from_slice(&ver.to_le_bytes());
            }
            let _ = bump_ascii_digits(&mut slot[0x1C..0x20], delta);
        }

        let hdr_sum = adler32_panasonic_strided(&slot[..0x2C], 1);
        slot[0x2C..0x30].copy_from_slice(&hdr_sum.to_le_bytes());

        let slot_pos = toc_start + mod_info.index * PANA_FRM_TOC_ENTRY_SIZE;
        image[slot_pos..slot_pos + PANA_FRM_TOC_ENTRY_SIZE].copy_from_slice(&slot[..48]);
    }

    if sub.has_panasonic_header {
        let content_sz = u32::try_from(image.len().saturating_sub(48))?;
        image[36..40].copy_from_slice(&content_sz.to_le_bytes());
        image[43] = u8::from(sub.toc_encrypted);
        let hsum = adler32_panasonic_strided(&image[..44], 1);
        image[44..48].copy_from_slice(&hsum.to_le_bytes());
    }

    if sub.toc_encrypted {
        let enc_len = (first_offset - toc_start) & !7;
        feistel_encrypt_buffer(
            &mut image[toc_start..toc_start + enc_len],
            &sub.feistel_round_key,
            &PANA_FEISTEL_SBOX_256,
        );
    }

    Ok(image)
}

fn pack_legacy_subcontainer(
    sub: &UniversalFrmSubContainer,
    mode: FrmContainerMode,
    opts: &FrmRepackOptions,
    modified_flags: &[bool],
) -> Result<Vec<u8>> {
    let mut modules = Vec::with_capacity(sub.modules.len());
    for (idx, mod_info) in sub.modules.iter().enumerate() {
        let slot_bytes = rebuild_modified_module_slot(
            mod_info,
            modified_flags[idx],
            &sub.feistel_round_key,
            opts,
        )?;
        let mut m = FrmPackModule::new(mod_info.name.clone(), slot_bytes);
        m.version_u16 = mod_info.version_u16;
        m.hardware_family = mod_info.hardware_family.clone();
        m.model_prefix = mod_info.oem;
        m.target_flag = mod_info.target_flag;
        m.model_group = mod_info.model_group.clone();
        if mod_info.raw_toc_entry.len() >= 0x2E {
            m.tail_meta
                .copy_from_slice(&mod_info.raw_toc_entry[0x28..0x2E]);
        }
        modules.push(m);
    }
    let region_sz = match mode {
        FrmContainerMode::Dvd64B | FrmContainerMode::Sd => Some(PANA_SD_TOC_REGION_SIZE),
        FrmContainerMode::Dvd | FrmContainerMode::Sd48 => Some(PANA_FRM_TOC_REGION_SIZE),
    };
    let rk = if sub.feistel_round_key == [0u8; 32] {
        PANA_FEISTEL_ROUND_KEY_32B
    } else {
        sub.feistel_round_key
    };
    let (bytes, _rep) = pack_frm_with_key(&modules, mode, region_sz, &rk)?;
    Ok(bytes)
}

fn resolve_output_formats(
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

fn mutate_subcontainer(
    sub: &mut UniversalFrmSubContainer,
    opts: &FrmRepackOptions,
    seen_mod_patches: &mut [bool],
    modules_modified: &mut Vec<String>,
    partitions_patched: &mut Vec<String>,
) -> Result<Vec<bool>> {
    let mut mod_flags = vec![false; sub.modules.len()];
    for (idx, m) in sub.modules.iter_mut().enumerate() {
        let up = m.name.trim().to_ascii_uppercase();
        let is_main = up.starts_with("MAIN") || up.starts_with("SMAI");
        if opts.patch_force && is_main {
            m.target_flag = 1;
        }
        if opts.patch_format_hdd && up.starts_with("PROG") {
            m.target_flag = 1;
        }
        if let Some(delta) = opts.patch_version_offset
            && is_main
        {
            m.version_u16 = m.version_u16.wrapping_add(delta as u16);
        }
        if let Some((p_idx, replacement)) = find_patch_for_module(m, &opts.patch_modules) {
            seen_mod_patches[p_idx] = true;
            m.payload = replacement.to_vec();
            mod_flags[idx] = true;
            modules_modified.push(m.name.trim().to_string());
        }
        if is_main && !opts.patch_partitions.is_empty() {
            m.payload = reassemble_main_partitions(&m.payload, &opts.patch_partitions)
                .with_context(|| format!("Patching MAIN partitions inside '{}'", m.name))?;
            mod_flags[idx] = true;
            if !modules_modified.iter().any(|x| x == m.name.trim()) {
                modules_modified.push(m.name.trim().to_string());
            }
            for (part_name, _) in &opts.patch_partitions {
                if !partitions_patched.contains(part_name) {
                    partitions_patched.push(part_name.clone());
                }
            }
        }
    }
    Ok(mod_flags)
}

/// Unpacks, patches, repacks, and optionally roundtrip-verifies any Legacy or Blu-ray `.FRM` image.
pub fn repack_universal_frm(
    raw: &[u8],
    keyring: &FrmKeyring,
    opts: &FrmRepackOptions,
) -> Result<(Vec<u8>, UniversalFrmRepackReport)> {
    let mut container: UniversalFrmContainer = unpack_universal_frm(raw, keyring)?;
    let source_format = container.format;
    let (output_format, legacy_mode) = resolve_output_formats(source_format, opts);

    let mut seen_mod_patches = vec![false; opts.patch_modules.len()];
    let mut modules_modified = Vec::new();
    let mut partitions_patched = Vec::new();
    let mut sub_buffers = Vec::with_capacity(container.sub_containers.len());

    for sub in &mut container.sub_containers {
        let mod_flags = mutate_subcontainer(
            sub,
            opts,
            &mut seen_mod_patches,
            &mut modules_modified,
            &mut partitions_patched,
        )?;
        let encoded = if let Some(lm) = legacy_mode {
            pack_legacy_subcontainer(sub, lm, opts, &mod_flags)?
        } else {
            pack_bluray_subcontainer(sub, opts, &mod_flags)?
        };
        sub_buffers.push(encoded);
    }

    for (idx, seen) in seen_mod_patches.into_iter().enumerate() {
        if !seen {
            bail!(
                "Requested --patch-module '{}' not found in container",
                opts.patch_modules[idx].0
            );
        }
    }

    let final_bytes = if output_format == UniversalFrmFormat::BlurayAes {
        let pair = container
            .aes_key_pair
            .as_ref()
            .or_else(|| keyring.aes_pairs.first())
            .context("No AES key pair available for BlurayAes repack")?;
        wrap_aes_frm(&sub_buffers, pair)?
    } else if sub_buffers.len() == 1 {
        sub_buffers.into_iter().next().unwrap_or_default()
    } else {
        bail!(
            "Multi-subfile ({}) source requires BlurayAes output or single-subfile input",
            sub_buffers.len()
        );
    };

    if opts.verify {
        let checked = unpack_universal_frm(&final_bytes, keyring)
            .context("Post-repack roundtrip verification failed")?;
        for sub in &checked.sub_containers {
            for m in &sub.modules {
                if !m.toc_sum_valid {
                    bail!(
                        "Post-repack TOC checksum invalid on module '{}' (sub {})",
                        m.name,
                        sub.sub_index
                    );
                }
                if !m.data_sum_valid && legacy_mode.is_none() {
                    bail!(
                        "Post-repack Adler-32 data checksum invalid on module '{}' (sub {})",
                        m.name,
                        sub.sub_index
                    );
                }
            }
        }
    }

    let report = UniversalFrmRepackReport {
        source_format,
        output_format,
        output_bytes: final_bytes.len(),
        sub_containers: container.sub_containers.len(),
        modules_modified,
        partitions_patched,
        verified: opts.verify,
    };
    Ok((final_bytes, report))
}
