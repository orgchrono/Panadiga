//! Recompiler, Demuxer, Filesystem Validator & Stream Cipher Primitives Ported from `pana-fw` (Go)
//!
//! Brings key discoveries from `WeebDataHoarder/pana-fw` and `theubusu/unixtract` into pure Rust:
//! - Multi-segment `MainList` (`20 + 8*N` header + partial Feistel `0x1400`/`0x2800` overlap ordering)
//! - Optical drive `DRV` `0x20`-byte `DriveHeader` + tail `-48` cipher wrapper
//! - `MAIN` partition splitter (`fmaX` command line + `SWUP_ADDON`) with `RomFS` & `CramFS` verification
//! - Statistical key recovery (`find_feistel_key_slow`) over `PANA_DVD_KEY_CATALOG`
//! - `libdownloadlib.so` stream cipher (`network_decipher_in_place`) & Kaseikyo IR service table

use anyhow::{Context, Result, bail};
use flate2::Crc;
use serde::{Deserialize, Serialize};

use super::codec::{extract_c_ascii, pad_to_block, read_u32_be, read_u32_le, write_padded_ascii};
use super::extrhead::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, UnpackedExtrhead, pack_extrhead_with_options,
    unpack_extrhead_with_key,
};
use super::feistel::{
    PANA_DVD_KEY_CATALOG, adler32_panasonic_strided, derive_round_key_from_custom_key_8b,
    feistel_decrypt_buffer, feistel_encrypt_buffer,
};
use crate::constants::{
    PANA_CRAMFS_FLAG_FSID_VERSION_2, PANA_CRAMFS_MAGIC, PANA_CRAMFS_SIGNATURE,
    PANA_DRV_HEADER_SIZE, PANA_DRV_TAIL_CLEAR_BYTES, PANA_EXTRHEAD_DEFAULT_ADLER_STEP,
    PANA_EXTRHEAD_DRV_CRYPT_SPAN, PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256,
    PANA_LIBDOWNLOADLIB_CIPHER_IV, PANA_LIBDOWNLOADLIB_CIPHER_STEP, PANA_MAIN_LIST_ENTRY_SIZE,
    PANA_MAIN_LIST_HEADER_SIZE, PANA_MAIN_LIST_LEGACY_SEG_TARGET, PANA_MAIN_LIST_MODERN_CRYPT_SPAN,
    PANA_MAIN_LIST_MODERN_SEG_TARGET, PANA_ROMFS_MAGIC,
};

pub const MAIN_LIST_LEGACY_SEGMENT_TARGET_SIZE: u32 = PANA_MAIN_LIST_LEGACY_SEG_TARGET;
pub const MAIN_LIST_MODERN_SEGMENT_TARGET_SIZE: u32 = PANA_MAIN_LIST_MODERN_SEG_TARGET;
pub const MAIN_LIST_LEGACY_CRYPT_SPAN: usize = PANA_EXTRHEAD_DRV_CRYPT_SPAN; // 0x2800 (10 KiB)
pub const MAIN_LIST_MODERN_CRYPT_SPAN: usize = PANA_MAIN_LIST_MODERN_CRYPT_SPAN; // 0x1400 (5 KiB)
pub const DRV_HEADER_SIZE: usize = PANA_DRV_HEADER_SIZE;
pub const DRV_TAIL_CLEAR_BYTES: usize = PANA_DRV_TAIL_CLEAR_BYTES;

const CRAMFS_MAGIC: u32 = PANA_CRAMFS_MAGIC;
const CRAMFS_SIGNATURE: &[u8; 16] = PANA_CRAMFS_SIGNATURE;
const CRAMFS_FLAG_FSID_VERSION_2: u32 = PANA_CRAMFS_FLAG_FSID_VERSION_2;
const ROMFS_MAGIC: &[u8; 8] = PANA_ROMFS_MAGIC;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainListHeader {
    pub checksum: u32,
    pub unknown1: u32,
    pub list_size: u32,
    pub decompressed_segment_size: u32,
    pub compression_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainListEntry {
    pub size: u32,
    pub checksum: u32,
    pub checksum_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpackedMainList {
    pub header: MainListHeader,
    pub header_checksum_valid: bool,
    pub entries: Vec<MainListEntry>,
    pub segments: Vec<UnpackedExtrhead>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriveHeaderInfo {
    pub manufacturer: String,
    pub model: String,
    pub version: String,
    pub unknown: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpackedDrvModule {
    pub header: Option<DriveHeaderInfo>,
    pub extrhead: Option<UnpackedExtrhead>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RomFsHeaderInfo {
    pub size: u32,
    pub checksum_be: u32,
    pub computed_checksum_be: u32,
    pub sum_valid: bool,
    pub volume_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CramFsHeaderInfo {
    pub size: u32,
    pub flags: u32,
    pub fsid_crc: u32,
    pub computed_crc: Option<u32>,
    pub crc_valid: bool,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PartitionFsKind {
    RomFs(RomFsHeaderInfo),
    CramFs(CramFsHeaderInfo),
    SwupAddon,
    Raw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainPartitionSlice {
    pub name: String,
    pub offset: usize,
    pub fs_kind: PartitionFsKind,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct KaseikyoServiceCommand {
    pub name: &'static str,
    pub fpc_bytes: [u8; 4],
    pub device: u8,
    pub subdevice: u8,
    pub command: u8,
    pub flipper_hint: &'static str,
    pub notes: &'static str,
}

/// Verified Panasonic Kaseikyo (`VendorID 0x2002`) service & front-panel commands from `pana-fw/docs/IR.md`.
pub const PANA_KASEIKYO_SERVICE_COMMANDS: &[KaseikyoServiceCommand] = &[
    KaseikyoServiceCommand {
        name: "Service Key",
        fpc_bytes: [0x41, 0xB0, 0x00, 0xF2],
        device: 0xB,
        subdevice: 0x0,
        command: 0xF2,
        flipper_hint: "A:0x32002B0, C:0x320",
        notes: "Direct service remote trigger",
    },
    KaseikyoServiceCommand {
        name: "Panel INIT",
        fpc_bytes: [0x41, 0xB0, 0x00, 0xF6],
        device: 0xB,
        subdevice: 0x0,
        command: 0xF6,
        flipper_hint: "A:0x32002B0, C:0x360",
        notes: "Front panel initialization",
    },
    KaseikyoServiceCommand {
        name: "Panel ACCEPT",
        fpc_bytes: [0x41, 0xB0, 0x00, 0xFD],
        device: 0xB,
        subdevice: 0x0,
        command: 0xFD,
        flipper_hint: "A:0x32002B0, C:0x3D0",
        notes: "LINE 0x0fd",
    },
    KaseikyoServiceCommand {
        name: "AGING-RMC",
        fpc_bytes: [0x41, 0xB0, 0x01, 0x40],
        device: 0xB,
        subdevice: 0x1,
        command: 0x3F,
        flipper_hint: "A:0x12002B1, C:0x000",
        notes: "AGING-RMC 0x13f",
    },
    KaseikyoServiceCommand {
        name: "Service Disclosure mode",
        fpc_bytes: [0x40, 0xB0, 0x01, 0x80],
        device: 0xB,
        subdevice: 0x1,
        command: 0x80,
        flipper_hint: "A:0x22002B1, C:0x000",
        notes: "OK + Blue + Yellow",
    },
    KaseikyoServiceCommand {
        name: "Service Non-Disclosure mode 1",
        fpc_bytes: [0x40, 0xB0, 0x01, 0x81],
        device: 0xB,
        subdevice: 0x1,
        command: 0x81,
        flipper_hint: "A:0x22002B1, C:0x010",
        notes: "6 + 7 + Yellow",
    },
    KaseikyoServiceCommand {
        name: "Service Non-Disclosure mode 2",
        fpc_bytes: [0x40, 0xB0, 0x01, 0x82],
        device: 0xB,
        subdevice: 0x1,
        command: 0x82,
        flipper_hint: "A:0x22002B1, C:0x020",
        notes: "5 + 9 + Red (70 RET -> 80 SRV)",
    },
    KaseikyoServiceCommand {
        name: "Audio / Subtitles [A]",
        fpc_bytes: [0x41, 0xB0, 0x00, 0x33],
        device: 0xB,
        subdevice: 0x0,
        command: 0x33,
        flipper_hint: "A:B0 02 20 00, C:30 03 (wire 02 20 B0 00 33 83)",
        notes: "NA Combo remote AUDIO [A] != Teletext RED (0x141)",
    },
    KaseikyoServiceCommand {
        name: "Display [B]",
        fpc_bytes: [0x41, 0xB0, 0x00, 0x92],
        device: 0xB,
        subdevice: 0x0,
        command: 0x92,
        flipper_hint: "wire 02 20 B0 00 92 22 (DMR-E50/EX77)",
        notes: "NA Combo remote DISPLAY [B]",
    },
    KaseikyoServiceCommand {
        name: "Teletext Red [RED]",
        fpc_bytes: [0x41, 0xB0, 0x01, 0x41],
        device: 0xB,
        subdevice: 0x1,
        command: 0x41,
        flipper_hint: "wire 02 20 B0 01 41 F0 (DMR-BCT73/pana-fw)",
        notes: "EU/JDM Teletext Red button (absent on NA remotes)",
    },
    KaseikyoServiceCommand {
        name: "Power On/Off",
        fpc_bytes: [0x41, 0xB0, 0x00, 0x3D],
        device: 0xB,
        subdevice: 0x0,
        command: 0x3D,
        flipper_hint: "wire 02 20 B0 00 3D 8D",
        notes: "DVD/DIGA power toggle",
    },
];

fn main_list_crypt_span(decompressed_segment_size: u32) -> usize {
    if decompressed_segment_size == MAIN_LIST_LEGACY_SEGMENT_TARGET_SIZE {
        MAIN_LIST_LEGACY_CRYPT_SPAN
    } else {
        MAIN_LIST_MODERN_CRYPT_SPAN
    }
}

/// Applies head+tail Feistel transform preserving `pana-fw` overlap ordering.
///
/// Decrypts `[..span]` then `[len-span..]`; encrypts `[len-span..]` then `[..span]`.
fn apply_edge_crypt_order(buf: &mut [u8], span: usize, round_key: &[u8; 32], encrypt: bool) {
    let aligned_span = span & !7;
    let aligned_len = buf.len() & !7;
    if aligned_span == 0 || aligned_len < aligned_span {
        return;
    }
    let tail_start = aligned_len - aligned_span;
    if encrypt {
        feistel_encrypt_buffer(
            &mut buf[tail_start..aligned_len],
            round_key,
            &PANA_FEISTEL_SBOX_256,
        );
        feistel_encrypt_buffer(&mut buf[..aligned_span], round_key, &PANA_FEISTEL_SBOX_256);
    } else {
        feistel_decrypt_buffer(&mut buf[..aligned_span], round_key, &PANA_FEISTEL_SBOX_256);
        feistel_decrypt_buffer(
            &mut buf[tail_start..aligned_len],
            round_key,
            &PANA_FEISTEL_SBOX_256,
        );
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

fn looks_like_drv_plain(decrypted: &[u8]) -> bool {
    if decrypted.starts_with(b"EXTRHEAD") {
        return true;
    }
    if decrypted.len() >= DRV_HEADER_SIZE + 8
        && &decrypted[DRV_HEADER_SIZE..DRV_HEADER_SIZE + 8] == b"EXTRHEAD"
    {
        return true;
    }
    decrypted.len() >= DRV_HEADER_SIZE
        && decrypted[..8]
            .iter()
            .all(|&b| b == 0 || (0x20..0x7F).contains(&b))
        && decrypted[8..24]
            .iter()
            .all(|&b| b == 0 || (0x20..0x7F).contains(&b))
        && decrypted[..4].iter().any(|&b| b.is_ascii_alphanumeric())
}

fn candidate_round_keys(explicit: Option<&[u8; 32]>) -> Vec<[u8; 32]> {
    if let Some(rk) = explicit {
        return vec![*rk];
    }
    let mut keys = Vec::with_capacity(1 + PANA_DVD_KEY_CATALOG.len());
    keys.push(PANA_FEISTEL_ROUND_KEY_32B);
    for &(_, k8) in PANA_DVD_KEY_CATALOG {
        let derived = derive_round_key_from_custom_key_8b(&k8);
        if derived != PANA_FEISTEL_ROUND_KEY_32B {
            keys.push(derived);
        }
    }
    keys
}

/// Unpacks an optical-drive `DRV` module protected by head+tail (`len - 48`) partial Feistel encryption.
pub fn extract_drv_module(raw: &[u8], round_key: Option<&[u8; 32]>) -> Result<UnpackedDrvModule> {
    let min_len = MAIN_LIST_LEGACY_CRYPT_SPAN + DRV_TAIL_CLEAR_BYTES;
    if raw.len() < min_len {
        bail!(
            "DRV module too short ({} B < minimum {min_len} B)",
            raw.len()
        );
    }
    let aligned_total = raw.len() & !7;
    let body_end = aligned_total - DRV_TAIL_CLEAR_BYTES;

    let mut decrypted = None;
    if looks_like_drv_plain(raw) {
        decrypted = Some(raw.to_vec());
    } else {
        for rk in candidate_round_keys(round_key) {
            let mut probe = raw.to_vec();
            apply_edge_crypt_order(
                &mut probe[..body_end],
                MAIN_LIST_LEGACY_CRYPT_SPAN,
                &rk,
                false,
            );
            if looks_like_drv_plain(&probe) {
                decrypted = Some(probe);
                break;
            }
        }
    }

    let plain = decrypted.unwrap_or_else(|| {
        let rk = round_key.copied().unwrap_or(PANA_FEISTEL_ROUND_KEY_32B);
        let mut fallback = raw.to_vec();
        apply_edge_crypt_order(
            &mut fallback[..body_end],
            MAIN_LIST_LEGACY_CRYPT_SPAN,
            &rk,
            false,
        );
        fallback
    });

    if plain.starts_with(b"EXTRHEAD") {
        let unpacked = unpack_extrhead_with_key(&plain, ExtrheadCryptMode::Plain, None)?;
        let payload = unpacked.decompressed.clone();
        return Ok(UnpackedDrvModule {
            header: None,
            extrhead: Some(unpacked),
            payload,
        });
    }

    let header = DriveHeaderInfo {
        manufacturer: extract_c_ascii(&plain[0..8]),
        model: extract_c_ascii(&plain[8..24]),
        version: extract_c_ascii(&plain[24..31]),
        unknown: plain[31],
    };
    let rest = &plain[DRV_HEADER_SIZE..];
    if rest.starts_with(b"EXTRHEAD") {
        let unpacked = unpack_extrhead_with_key(rest, ExtrheadCryptMode::Plain, None)?;
        let payload = unpacked.decompressed.clone();
        return Ok(UnpackedDrvModule {
            header: Some(header),
            extrhead: Some(unpacked),
            payload,
        });
    }

    Ok(UnpackedDrvModule {
        header: Some(header),
        extrhead: None,
        payload: rest.to_vec(),
    })
}

/// Packs an optical-drive `DRV` payload with optional 32B `DriveHeader`, optional `EXTRHEAD`, and edge Feistel.
pub fn pack_drv_module(
    payload: &[u8],
    drive_header: Option<&DriveHeaderInfo>,
    compress: Option<ExtrheadCompressionAlgo>,
    build_name: &str,
    round_key: &[u8; 32],
) -> Result<Vec<u8>> {
    let mut out =
        Vec::with_capacity(MAIN_LIST_LEGACY_CRYPT_SPAN + DRV_TAIL_CLEAR_BYTES + payload.len());
    if let Some(hdr) = drive_header {
        let mut hdr_buf = [0u8; DRV_HEADER_SIZE];
        write_padded_ascii(&mut hdr_buf[0..8], &hdr.manufacturer, 0);
        write_padded_ascii(&mut hdr_buf[8..24], &hdr.model, 0);
        write_padded_ascii(&mut hdr_buf[24..31], &hdr.version, 0);
        hdr_buf[31] = hdr.unknown;
        out.extend_from_slice(&hdr_buf);
    }

    if let Some(algo) = compress {
        let container = pack_extrhead_with_options(
            payload,
            "DRV ",
            0,
            build_name,
            ExtrheadCryptMode::Plain,
            algo,
            &PANA_FEISTEL_ROUND_KEY_32B,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
            1,
        )?;
        out.extend_from_slice(&container);
    } else {
        out.extend_from_slice(payload);
    }

    let min_total = MAIN_LIST_LEGACY_CRYPT_SPAN + DRV_TAIL_CLEAR_BYTES;
    if out.len() < min_total {
        out.resize(min_total, 0);
    }
    pad_to_block(&mut out);

    let body_end = out.len() - DRV_TAIL_CLEAR_BYTES;
    apply_edge_crypt_order(
        &mut out[..body_end],
        MAIN_LIST_LEGACY_CRYPT_SPAN,
        round_key,
        true,
    );
    Ok(out)
}

/// Computes `RomFS` negated big-endian 32-bit word sum (`ChecksumAFFS` from `pana-fw/utils/sum.go`).
pub fn checksum_affs(data: &[u8]) -> Result<u32> {
    if !data.len().is_multiple_of(4) {
        bail!(
            "checksum_affs requires 4-byte aligned slice, got {} B",
            data.len()
        );
    }
    let (chunks, _) = data.as_chunks::<4>();
    let sum = chunks
        .iter()
        .fold(0u32, |acc, &c| acc.wrapping_add(u32::from_be_bytes(c)));
    Ok(sum.wrapping_neg())
}

/// Validates a Linux `RomFS` superblock (`-rom1fs-` + `checksum_affs` over the first `min(size, 512)` bytes).
pub fn verify_romfs_header(data: &[u8]) -> Result<RomFsHeaderInfo> {
    if data.len() < 32 {
        bail!("RomFS header too short ({} B < 32)", data.len());
    }
    if &data[0..8] != ROMFS_MAGIC {
        bail!("Invalid RomFS magic");
    }
    let size = read_u32_be(data, 8);
    let checksum_be = read_u32_be(data, 12);
    let vol_end = data[16..]
        .iter()
        .position(|&b| b == 0)
        .context("Missing NUL-terminated RomFS volume name")?;
    let volume_name = extract_c_ascii(&data[16..16 + vol_end]);

    let sum_len = (size as usize).min(512).min(data.len()) & !3;
    if sum_len < 16 {
        bail!("RomFS image too small for checksum validation");
    }
    let mut sum_buf = data[..sum_len].to_vec();
    sum_buf[12..16].fill(0);
    let computed_checksum_be = checksum_affs(&sum_buf)?;

    Ok(RomFsHeaderInfo {
        size,
        checksum_be,
        computed_checksum_be,
        sum_valid: computed_checksum_be == checksum_be,
        volume_name,
    })
}

/// Validates a Linux `CramFS` superblock (`0x28CD3D45` / `"Compressed ROMFS"` + IEEE CRC32 when FSID v2 is set).
pub fn verify_cramfs_header(data: &[u8]) -> Result<CramFsHeaderInfo> {
    if data.len() < 64 {
        bail!("CramFS header too short ({} B < 64)", data.len());
    }
    if read_u32_le(data, 0) != CRAMFS_MAGIC || &data[16..32] != CRAMFS_SIGNATURE {
        bail!("Invalid CramFS magic or signature");
    }
    let size = read_u32_le(data, 4);
    let flags = read_u32_le(data, 8);
    let fsid_crc = read_u32_le(data, 32);
    let name = extract_c_ascii(&data[48..64]);

    let (computed_crc, crc_valid) = if (flags & CRAMFS_FLAG_FSID_VERSION_2) != 0 {
        let img_len = size as usize;
        if img_len < 64 || img_len > data.len() {
            (None, false)
        } else {
            let mut crc = Crc::new();
            crc.update(&data[..32]);
            crc.update(&[0u8; 4]);
            crc.update(&data[36..img_len]);
            let sum = crc.sum();
            (Some(sum), sum == fsid_crc)
        }
    } else {
        (None, true)
    };

    Ok(CramFsHeaderInfo {
        size,
        flags,
        fsid_crc,
        computed_crc,
        crc_valid,
        name,
    })
}

fn parse_fma_size(raw_val: &str) -> Option<usize> {
    let token = raw_val.split(':').next()?.trim();
    if let Some(num) = token.strip_suffix('k').or_else(|| token.strip_suffix('K')) {
        num.parse::<usize>().ok().map(|v| v.saturating_mul(1024))
    } else if let Some(num) = token.strip_suffix('m').or_else(|| token.strip_suffix('M')) {
        num.parse::<usize>()
            .ok()
            .map(|v| v.saturating_mul(1024 * 1024))
    } else {
        None
    }
}

fn classify_partition_slice(data: &[u8]) -> PartitionFsKind {
    if let Ok(info) = verify_romfs_header(data) {
        return PartitionFsKind::RomFs(info);
    }
    if let Ok(info) = verify_cramfs_header(data) {
        return PartitionFsKind::CramFs(info);
    }
    PartitionFsKind::Raw
}

/// Splits an unpacked `MAIN` kernel/rootfs image by parsing `fmaX=` kernel cmdline args (with `SWUP_ADDON` support).
pub fn split_main_partitions(buf: &[u8]) -> Result<Vec<MainPartitionSlice>> {
    let (cmd_start, main_limit, addon_offset) = if buf.len() >= 16 && buf.starts_with(b"SWUP_ADDON")
    {
        let off = read_u32_le(buf, 12) as usize;
        if off < 16 || off > buf.len() {
            bail!(
                "Invalid SWUP_ADDON offset 0x{off:X} for {} B buffer",
                buf.len()
            );
        }
        (16usize, off, Some(off))
    } else {
        (0usize, buf.len(), None)
    };

    let nul_rel = buf[cmd_start..main_limit]
        .iter()
        .position(|&b| b == 0)
        .context("Missing NUL terminator in MAIN partition cmdline")?;
    let cmdline = String::from_utf8_lossy(&buf[cmd_start..cmd_start + nul_rel]);

    let mut fma_defs: Vec<(String, usize)> = Vec::new();
    let mut root_name: Option<String> = None;
    for token in cmdline.split(['\n', '\r', ' ', '\t']) {
        let arg = token.trim();
        if let Some(dev) = arg.strip_prefix("root=/dev/") {
            root_name = Some(dev.trim().to_string());
        } else if let Some((k, v)) = arg.split_once('=')
            && k.starts_with("fma")
            && let Some(sz) = parse_fma_size(v)
        {
            fma_defs.push((k.to_string(), sz));
        }
    }

    let root = root_name.context("Missing root=/dev/fmaX entry in MAIN cmdline")?;
    let root_idx = fma_defs
        .iter()
        .position(|(n, _)| n == &root)
        .with_context(|| format!("Root partition '{root}' missing from fma definitions"))?;

    let mut out = Vec::new();
    let mut offset = 0usize;
    for (name, sz) in fma_defs.iter().skip(root_idx.saturating_sub(1)) {
        if offset >= main_limit {
            break;
        }
        if *sz == 0 {
            continue;
        }
        let end = offset.saturating_add(*sz).min(main_limit);
        let data = buf[offset..end].to_vec();
        let fs_kind = classify_partition_slice(&data);
        out.push(MainPartitionSlice {
            name: name.clone(),
            offset,
            fs_kind,
            data,
        });
        offset = end;
    }

    if let Some(addon_off) = addon_offset {
        out.push(MainPartitionSlice {
            name: "SWUP_ADDON".to_string(),
            offset: addon_off,
            fs_kind: PartitionFsKind::SwupAddon,
            data: buf[addon_off..].to_vec(),
        });
    }

    Ok(out)
}

fn count_non_overlapping(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    let mut count = 0usize;
    let mut i = 0usize;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            count += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count
}

/// Statistical Feistel key recovery (`findKeySlow` from `pana-fw`): scores `0x00000000`, `0xFFFFFFFF`, and `PANASONIC\0\0\0`.
pub fn find_feistel_key_slow(block: &[u8]) -> Option<(&'static str, [u8; 32], usize)> {
    let aligned = block.len() & !7;
    if aligned == 0 {
        return None;
    }

    let mut candidates: Vec<(&'static str, [u8; 32])> =
        Vec::with_capacity(1 + PANA_DVD_KEY_CATALOG.len());
    candidates.push(("old_df72", PANA_FEISTEL_ROUND_KEY_32B));
    for &(name, k8) in PANA_DVD_KEY_CATALOG {
        candidates.push((name, derive_round_key_from_custom_key_8b(&k8)));
    }

    let mut best: Option<(&'static str, [u8; 32], usize)> = None;
    for (name, rk) in candidates {
        let mut decrypted = block[..aligned].to_vec();
        feistel_decrypt_buffer(&mut decrypted, &rk, &PANA_FEISTEL_SBOX_256);
        let score = count_non_overlapping(&decrypted, &[0x00; 4])
            + count_non_overlapping(&decrypted, &[0xFF; 4])
            + count_non_overlapping(&decrypted, b"PANASONIC\x00\x00\x00") * 10;
        if score > best.map_or(0, |b| b.2) {
            best = Some((name, rk, score));
        }
    }
    best
}

fn network_stream_transform(data: &mut [u8], decrypt: bool) {
    let mut counter = 0u32;
    for (idx, byte) in data.iter_mut().enumerate() {
        if (idx & 0xFF) == 0 {
            counter = PANA_LIBDOWNLOADLIB_CIPHER_IV;
        }
        let key_byte = (counter >> 8) as u8;
        let out_byte = *byte ^ key_byte;
        let cipher_byte = if decrypt { *byte } else { out_byte };
        *byte = out_byte;
        counter = counter.wrapping_add(u32::from(cipher_byte) + PANA_LIBDOWNLOADLIB_CIPHER_STEP);
    }
}

/// Decrypts network payload using Panasonic's `libdownloadlib.so` stream decipher (`pana-fw/crypto/decipher.go`).
pub fn network_decipher_in_place(data: &mut [u8]) {
    network_stream_transform(data, true);
}

/// Inverse stream cipher for `network_decipher_in_place` (for roundtrip validation and test synthesis).
pub fn network_encipher_in_place(data: &mut [u8]) {
    network_stream_transform(data, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_synthetic_romfs(vol_name: &str, payload: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(ROMFS_MAGIC);
        let vol_padded_len = (vol_name.len() + 1 + 15) & !15;
        let total = 16 + vol_padded_len + ((payload.len() + 15) & !15);
        let aligned_total = (total + 3) & !3;
        buf.extend_from_slice(&(aligned_total as u32).to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes());
        let mut vol = vec![0u8; vol_padded_len];
        vol[..vol_name.len()].copy_from_slice(vol_name.as_bytes());
        buf.extend_from_slice(&vol);
        buf.extend_from_slice(payload);
        buf.resize(aligned_total, 0);
        let sum_len = aligned_total.min(512);
        let sum = checksum_affs(&buf[..sum_len]).unwrap();
        buf[12..16].copy_from_slice(&sum.to_be_bytes());
        buf
    }

    fn build_synthetic_cramfs(name: &str, size: u32) -> Vec<u8> {
        let mut buf = vec![0xAAu8; size as usize];
        buf[0..4].copy_from_slice(&CRAMFS_MAGIC.to_le_bytes());
        buf[4..8].copy_from_slice(&size.to_le_bytes());
        buf[8..12].copy_from_slice(&CRAMFS_FLAG_FSID_VERSION_2.to_le_bytes());
        buf[16..32].copy_from_slice(CRAMFS_SIGNATURE);
        buf[32..36].fill(0);
        write_padded_ascii(&mut buf[48..64], name, 0);
        let mut crc = Crc::new();
        crc.update(&buf);
        let sum = crc.sum();
        buf[32..36].copy_from_slice(&sum.to_le_bytes());
        buf
    }

    #[test]
    fn test_main_list_pack_and_parse_roundtrip_with_overlap() {
        let payload = b"MATSUSHITA AM34 KERNEL AND ROOTFS STREAM ".repeat(64);
        let rk = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[1].1);
        let packed = pack_main_list(
            &payload,
            "DRV ",
            "M6930_MAIN",
            ExtrheadCompressionAlgo::Lzss,
            1024,
            Some(&rk),
            16,
            1,
        )
        .unwrap();

        let unpacked = parse_main_list(&packed, Some(&rk)).unwrap();
        assert!(unpacked.header_checksum_valid);
        assert_eq!(unpacked.entries.len(), 3);
        assert!(unpacked.entries.iter().all(|e| e.checksum_valid));
        assert_eq!(unpacked.data, payload);
    }

    #[test]
    fn test_drv_module_pack_and_extract_roundtrip() {
        let hdr = DriveHeaderInfo {
            manufacturer: "MATSHITA".to_string(),
            model: "DVD-RAM SW-9574".to_string(),
            version: "S211   ".to_string(),
            unknown: 0x01,
        };
        let payload = b"OPTICAL DRIVE SERVO DSP IMAGE DATA ".repeat(400);
        let rk = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[2].1);

        let wrapped = pack_drv_module(
            &payload,
            Some(&hdr),
            Some(ExtrheadCompressionAlgo::Gzip),
            "DRV211",
            &rk,
        )
        .unwrap();
        let unpacked = extract_drv_module(&wrapped, None).unwrap();
        let parsed_hdr = unpacked.header.expect("expected 32B DriveHeader");
        assert_eq!(parsed_hdr.manufacturer, "MATSHITA");
        assert_eq!(parsed_hdr.model, "DVD-RAM SW-9574");
        assert_eq!(unpacked.payload, payload);
        assert!(unpacked.extrhead.unwrap().checksum_valid);
    }

    #[test]
    fn test_split_main_partitions_with_romfs_cramfs_and_swup_addon() {
        let mut p_kern = vec![0x55u8; 4096];
        let cmdline = b"console=ttyS0 fma4=4k:kernel fma5=4k:root fma6=4k:res root=/dev/fma5\0";
        p_kern[16..16 + cmdline.len()].copy_from_slice(cmdline);

        let mut p_root = build_synthetic_romfs("panaroot", b"hello");
        p_root.resize(4096, 0);
        let mut p_cram = build_synthetic_cramfs("res_fs", 4096);
        p_cram.resize(4096, 0);

        let addon_offset = 12_288u32;
        let mut full = Vec::new();
        full.extend_from_slice(b"SWUP_ADDON\0\0");
        full.extend_from_slice(&addon_offset.to_le_bytes());
        full.extend_from_slice(&p_kern[16..]);
        full.extend_from_slice(&p_root);
        full.extend_from_slice(&p_cram);
        full.extend_from_slice(b"ADDON_TAIL_PAYLOAD");

        let slices = split_main_partitions(&full).unwrap();
        assert_eq!(slices.len(), 4);
        assert_eq!(slices[0].name, "fma4");
        assert_eq!(slices[0].fs_kind, PartitionFsKind::Raw);
        assert_eq!(slices[1].name, "fma5");
        assert!(matches!(
            slices[1].fs_kind,
            PartitionFsKind::RomFs(RomFsHeaderInfo {
                sum_valid: true,
                ..
            })
        ));
        assert_eq!(slices[2].name, "fma6");
        assert!(matches!(
            slices[2].fs_kind,
            PartitionFsKind::CramFs(CramFsHeaderInfo {
                crc_valid: true,
                ..
            })
        ));
        assert_eq!(slices[3].name, "SWUP_ADDON");
    }

    #[test]
    fn test_find_feistel_key_slow_and_network_decipher() {
        let mut sample = vec![0xFFu8; 256];
        sample[..12].copy_from_slice(b"PANASONIC\x00\x00\x00");
        sample[64..96].fill(0x00);
        let target_key = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[3].1);
        feistel_encrypt_buffer(&mut sample, &target_key, &PANA_FEISTEL_SBOX_256);

        let (matched_name, matched_rk, score) = find_feistel_key_slow(&sample).unwrap();
        assert_eq!(matched_name, PANA_DVD_KEY_CATALOG[3].0);
        assert_eq!(matched_rk, target_key);
        assert!(score >= 10);

        let orig = b"GET /firmware/panasonic/version.xml HTTP/1.1\r\nHost: bdautovchk.svrpf.jp\r\n"
            .repeat(6);
        let mut buf = orig.clone();
        network_encipher_in_place(&mut buf);
        assert_ne!(buf, orig);
        network_decipher_in_place(&mut buf);
        assert_eq!(buf, orig);
    }
}
