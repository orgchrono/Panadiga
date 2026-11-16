//! Partial Feistel key auto-detection and compression stream dispatch for `EXTRHEAD`.

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};

use super::super::feistel::{
    PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b, feistel_decrypt_block,
    feistel_decrypt_buffer, feistel_encrypt_buffer,
};
use super::lzss::{compress_pana_lzss, decompress_pana_lzss};
use super::types::{ExtrheadCompressionAlgo, ExtrheadCryptMode};
use crate::constants::{
    PANA_EXTRHEAD_DRV_CRYPT_SPAN, PANA_EXTRHEAD_MAIN_CRYPT_SPAN, PANA_FEISTEL_ROUND_KEY_32B,
    PANA_FEISTEL_SBOX_256,
};

pub(super) fn apply_partial_crypt_with_key(
    buf: &mut [u8],
    mode: ExtrheadCryptMode,
    aligned_end: usize,
    encrypt: bool,
    round_key: &[u8; 32],
) {
    let transform = |slice: &mut [u8]| {
        if encrypt {
            feistel_encrypt_buffer(slice, round_key, &PANA_FEISTEL_SBOX_256);
        } else {
            feistel_decrypt_buffer(slice, round_key, &PANA_FEISTEL_SBOX_256);
        }
    };
    let end = aligned_end.min(buf.len()) & !7;
    if end == 0 {
        return;
    }
    match mode {
        ExtrheadCryptMode::Plain | ExtrheadCryptMode::Auto => {}
        ExtrheadCryptMode::Drv => {
            let span = PANA_EXTRHEAD_DRV_CRYPT_SPAN.min(end) & !7;
            transform(&mut buf[..span]);
        }
        ExtrheadCryptMode::Main => {
            let span = PANA_EXTRHEAD_MAIN_CRYPT_SPAN;
            if end <= 2 * span {
                transform(&mut buf[..end]);
            } else {
                transform(&mut buf[..span]);
                transform(&mut buf[(end - span)..end]);
            }
        }
    }
}

pub(super) fn resolve_crypt_mode(
    mode: ExtrheadCryptMode,
    tag: &str,
    is_plain_header: bool,
) -> ExtrheadCryptMode {
    if mode != ExtrheadCryptMode::Auto {
        return mode;
    }
    if is_plain_header {
        return ExtrheadCryptMode::Plain;
    }
    match tag.trim() {
        "DRV" => ExtrheadCryptMode::Drv,
        "MAIN" => ExtrheadCryptMode::Main,
        _ => ExtrheadCryptMode::Plain,
    }
}

fn match_extrhead_magic(blk8: &[u8; 8], rk: &[u8; 32]) -> Option<bool> {
    let dec1 = feistel_decrypt_block(blk8, rk, &PANA_FEISTEL_SBOX_256);
    if &dec1 == b"EXTRHEAD" {
        return Some(false);
    }
    let dec2 = feistel_decrypt_block(&dec1, rk, &PANA_FEISTEL_SBOX_256);
    if &dec2 == b"EXTRHEAD" {
        return Some(true);
    }
    None
}

pub(super) fn detect_extrhead_key_and_double(
    header_8b: &[u8],
    preferred_key: Option<&[u8; 32]>,
) -> Option<([u8; 32], bool)> {
    let mut blk = [0u8; 8];
    blk.copy_from_slice(&header_8b[..8]);
    let primary = preferred_key.unwrap_or(&PANA_FEISTEL_ROUND_KEY_32B);
    if let Some(is_double) = match_extrhead_magic(&blk, primary) {
        return Some((*primary, is_double));
    }
    if preferred_key.is_some() {
        return None;
    }
    for &(_, key_8b) in PANA_DVD_KEY_CATALOG {
        let rk = derive_round_key_from_custom_key_8b(&key_8b);
        if let Some(is_double) = match_extrhead_magic(&blk, &rk) {
            return Some((rk, is_double));
        }
    }
    None
}

pub(super) fn compress_payload_by_algo(
    payload: &[u8],
    algo: ExtrheadCompressionAlgo,
) -> Result<Vec<u8>> {
    match algo {
        ExtrheadCompressionAlgo::Zlib => {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(payload)
                .context("Compressing EXTRHEAD zlib payload")?;
            encoder.finish().context("Finalizing zlib EXTRHEAD stream")
        }
        ExtrheadCompressionAlgo::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(payload)
                .context("Compressing EXTRHEAD gzip payload")?;
            encoder.finish().context("Finalizing gzip EXTRHEAD stream")
        }
        ExtrheadCompressionAlgo::Lzss => Ok(compress_pana_lzss(payload)),
    }
}

pub(super) fn decompress_payload_by_algo(
    slice: &[u8],
    algo: u16,
    expected_size: usize,
) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(expected_size);
    match algo {
        0 => {
            ZlibDecoder::new(slice)
                .read_to_end(&mut buf)
                .context("Decompressing EXTRHEAD zlib payload")?;
        }
        1 => {
            GzDecoder::new(slice)
                .read_to_end(&mut buf)
                .context("Decompressing EXTRHEAD gzip payload")?;
        }
        2 => return Ok(decompress_pana_lzss(slice, expected_size)),
        other => bail!("Unsupported EXTRHEAD compression algo: {other}"),
    }
    Ok(buf)
}
