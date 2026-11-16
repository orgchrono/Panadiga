//! Panasonic `EXTRHEAD`/`EXTRFOOT` zlib/gzip/LZSS Codec & Partial Feistel Wrapper
//!
//! Pure-Rust implementation matching `BOOT.BIN` (`FUN_4400b5e1`, `FUN_4400b61a`,
//! `FUN_4400b6c6`, `FUN_4401e5a3`) plus Go `pana-fw` / `unixtract` multi-key & compression parity.

use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use super::codec::{extract_c_ascii, pad_to_block, read_u16_le, read_u32_le, write_padded_ascii};
use super::feistel::{
    PANA_DVD_KEY_CATALOG, adler32_panasonic_strided, derive_round_key_from_custom_key_8b,
    feistel_decrypt_block, feistel_decrypt_buffer, feistel_encrypt_buffer,
};
use crate::constants::*;

/// Partial encryption profile for an `EXTRHEAD`/`EXTRFOOT` container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtrheadCryptMode {
    Auto,
    Main,
    Drv,
    Plain,
}

/// Supported inner payload compression algorithms for `EXTRHEAD`/`EXTRFOOT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum ExtrheadCompressionAlgo {
    Zlib = 0,
    Gzip = 1,
    Lzss = 2,
}

impl std::str::FromStr for ExtrheadCompressionAlgo {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "zlib" | "0" => Ok(Self::Zlib),
            "gzip" | "gz" | "1" => Ok(Self::Gzip),
            "lzss" | "2" => Ok(Self::Lzss),
            other => bail!("Unsupported compression algo '{other}': expected zlib, gzip, or lzss"),
        }
    }
}

impl std::str::FromStr for ExtrheadCryptMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "main" => Ok(Self::Main),
            "drv" => Ok(Self::Drv),
            "plain" => Ok(Self::Plain),
            other => bail!("Unsupported unpack mode '{other}': expected auto, main, drv, or plain"),
        }
    }
}

/// Parsed 64-byte (`0x40`) Panasonic `EXTRHEAD` header block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtrheadHeader {
    pub tag: String,
    pub flag: u16,
    pub algo: u16,
    pub dst_len: u32,
    pub dst_addr: u32,
    pub src_len: u32,
    pub footer_offset: u32,
    pub header_checksum: u32,
    pub adler_step: u32,
    pub build_name: String,
}

/// Result of decrypting, decompressing, and validating an `EXTRHEAD`/`EXTRFOOT` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpackedExtrhead {
    pub header: ExtrheadHeader,
    pub decompressed: Vec<u8>,
    pub calculated_checksum: u32,
    pub checksum_valid: bool,
    pub footer_present: bool,
}

fn apply_partial_crypt_with_key(
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

fn resolve_crypt_mode(
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

fn detect_extrhead_key_and_double(
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

fn compress_payload_by_algo(payload: &[u8], algo: ExtrheadCompressionAlgo) -> Result<Vec<u8>> {
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

fn decompress_payload_by_algo(slice: &[u8], algo: u16, expected_size: usize) -> Result<Vec<u8>> {
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

/// Parses a decrypted 64-byte (`0x40`) `EXTRHEAD` header block.
pub fn parse_extrhead_header(header_bytes: &[u8]) -> Result<ExtrheadHeader> {
    if header_bytes.len() < 0x40 || &header_bytes[..8] != b"EXTRHEAD" {
        bail!("Invalid EXTRHEAD container header");
    }
    Ok(ExtrheadHeader {
        tag: String::from_utf8_lossy(&header_bytes[0x08..0x0C]).into_owned(),
        flag: read_u16_le(header_bytes, 0x0C),
        algo: read_u16_le(header_bytes, 0x0E),
        dst_len: read_u32_le(header_bytes, 0x10),
        dst_addr: read_u32_le(header_bytes, 0x14),
        src_len: read_u32_le(header_bytes, 0x18),
        footer_offset: read_u32_le(header_bytes, 0x20),
        header_checksum: read_u32_le(header_bytes, 0x28),
        adler_step: read_u32_le(header_bytes, 0x2C),
        build_name: extract_c_ascii(&header_bytes[0x30..0x40]),
    })
}

/// Compresses and wraps `payload` into an `EXTRHEAD`/`EXTRFOOT` container with explicit algo and key.
#[allow(clippy::too_many_arguments)]
pub fn pack_extrhead_with_options(
    payload: &[u8],
    tag: &str,
    dst_addr: u32,
    build_name: &str,
    mode: ExtrheadCryptMode,
    algo: ExtrheadCompressionAlgo,
    round_key: &[u8; 32],
    adler_step: u32,
    flag: u16,
) -> Result<Vec<u8>> {
    let compressed = compress_payload_by_algo(payload, algo)?;

    let footer_offset = (0x40 + compressed.len()) as u32;
    let checksum = adler32_panasonic_strided(payload, adler_step as usize);

    let mut raw = Vec::with_capacity(0x40 + compressed.len() + 16);
    raw.extend_from_slice(b"EXTRHEAD");
    let mut tag_buf = [b' '; 4];
    write_padded_ascii(&mut tag_buf, tag, b' ');
    raw.extend_from_slice(&tag_buf);
    raw.extend_from_slice(&flag.to_le_bytes());
    raw.extend_from_slice(&(algo as u16).to_le_bytes());
    raw.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    raw.extend_from_slice(&dst_addr.to_le_bytes());
    raw.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    raw.extend_from_slice(&0u32.to_le_bytes());
    raw.extend_from_slice(&footer_offset.to_le_bytes());
    raw.extend_from_slice(&dst_addr.to_le_bytes());
    raw.extend_from_slice(&checksum.to_le_bytes());
    raw.extend_from_slice(&adler_step.to_le_bytes());
    let mut build_buf = [0u8; 16];
    write_padded_ascii(&mut build_buf, build_name, 0);
    raw.extend_from_slice(&build_buf);
    raw.extend_from_slice(&compressed);
    raw.extend_from_slice(b"EXTRFOOT");
    pad_to_block(&mut raw);

    let eff_mode = resolve_crypt_mode(mode, tag, mode == ExtrheadCryptMode::Plain);
    let len = raw.len();
    apply_partial_crypt_with_key(&mut raw, eff_mode, len, true, round_key);
    Ok(raw)
}

/// Compresses and wraps `payload` into a Panasonic `EXTRHEAD`/`EXTRFOOT` container (Zlib + default EH55 key).
pub fn pack_extrhead(
    payload: &[u8],
    tag: &str,
    dst_addr: u32,
    build_name: &str,
    mode: ExtrheadCryptMode,
    adler_step: u32,
    flag: u16,
) -> Result<Vec<u8>> {
    pack_extrhead_with_options(
        payload,
        tag,
        dst_addr,
        build_name,
        mode,
        ExtrheadCompressionAlgo::Zlib,
        &PANA_FEISTEL_ROUND_KEY_32B,
        adler_step,
        flag,
    )
}

fn unpack_extrhead_inner(
    raw_container: &[u8],
    mode: ExtrheadCryptMode,
    round_key: Option<&[u8; 32]>,
    depth: u8,
) -> Result<UnpackedExtrhead> {
    if raw_container.len() < 0x40 {
        bail!("Container shorter than 64-byte EXTRHEAD header");
    }

    let is_plain_header = mode == ExtrheadCryptMode::Plain || &raw_container[..8] == b"EXTRHEAD";
    let (active_key, double_encrypted) = if is_plain_header {
        (
            round_key.copied().unwrap_or(PANA_FEISTEL_ROUND_KEY_32B),
            false,
        )
    } else {
        detect_extrhead_key_and_double(&raw_container[..8], round_key)
            .context("Invalid EXTRHEAD container header (no matching Feistel key)")?
    };

    let mut hdr_blk = [0u8; 0x40];
    hdr_blk.copy_from_slice(&raw_container[..0x40]);
    if !is_plain_header {
        feistel_decrypt_buffer(&mut hdr_blk, &active_key, &PANA_FEISTEL_SBOX_256);
        if double_encrypted {
            feistel_decrypt_buffer(&mut hdr_blk, &active_key, &PANA_FEISTEL_SBOX_256);
        }
    }

    let header = parse_extrhead_header(&hdr_blk)?;
    if header.algo > 2 {
        bail!("Unsupported EXTRHEAD compression algo: {}", header.algo);
    }

    let eff_mode = resolve_crypt_mode(mode, &header.tag, is_plain_header);
    let footer_span = if header.footer_offset >= 0x40 {
        header.footer_offset as usize + 8
    } else {
        0x40 + header.src_len as usize + 8
    };
    let aligned_end = (raw_container.len() & !7).min((footer_span + 7) & !7);

    let mut decrypted = raw_container.to_vec();
    if double_encrypted {
        feistel_decrypt_buffer(
            &mut decrypted[..aligned_end],
            &active_key,
            &PANA_FEISTEL_SBOX_256,
        );
    }
    apply_partial_crypt_with_key(&mut decrypted, eff_mode, aligned_end, false, &active_key);

    let payload_end = 0x40usize + header.src_len as usize;
    if payload_end > decrypted.len() {
        bail!(
            "EXTRHEAD container truncated: expected {} bytes, got {}",
            payload_end,
            decrypted.len()
        );
    }

    let decompressed = decompress_payload_by_algo(
        &decrypted[0x40..payload_end],
        header.algo,
        header.dst_len as usize,
    )?;

    let stride = header.adler_step.max(1) as usize;
    let calculated_checksum = adler32_panasonic_strided(&decompressed, stride);
    let outer_checksum_valid = calculated_checksum == header.header_checksum;
    let foff = header.footer_offset as usize;
    let footer_present =
        foff >= 0x40 && foff + 8 <= decrypted.len() && &decrypted[foff..foff + 8] == b"EXTRFOOT";

    if depth < 4
        && decompressed.len() >= 0x40
        && &decompressed[..8] == b"EXTRHEAD"
        && let Ok(nested_hdr) = parse_extrhead_header(&decompressed[..0x40])
        && 0x40usize + nested_hdr.src_len as usize <= decompressed.len()
        && let Ok(mut inner) =
            unpack_extrhead_inner(&decompressed, ExtrheadCryptMode::Plain, None, depth + 1)
    {
        inner.checksum_valid = outer_checksum_valid && inner.checksum_valid;
        inner.footer_present = footer_present && inner.footer_present;
        return Ok(inner);
    }

    Ok(UnpackedExtrhead {
        checksum_valid: outer_checksum_valid,
        header,
        decompressed,
        calculated_checksum,
        footer_present,
    })
}

/// Unpacks an `EXTRHEAD`/`EXTRFOOT` container using an explicit optional Feistel round key.
pub fn unpack_extrhead_with_key(
    raw_container: &[u8],
    mode: ExtrheadCryptMode,
    round_key: Option<&[u8; 32]>,
) -> Result<UnpackedExtrhead> {
    unpack_extrhead_inner(raw_container, mode, round_key, 0)
}

/// Unpacks an `EXTRHEAD`/`EXTRFOOT` container using Panasonic partial Feistel encryption (multi-key auto-detect).
pub fn unpack_extrhead(raw_container: &[u8], mode: ExtrheadCryptMode) -> Result<UnpackedExtrhead> {
    unpack_extrhead_inner(raw_container, mode, None, 0)
}

/// Decompresses an Okumura LZSS stream used by Panasonic DIGA firmware (`algo == 2`).
///
/// Uses a 4096-byte sliding window, 0-initialized, with initial window position `0x0FEE`.
/// Flag byte: bit 0 indicates literal (1) or 2-byte backreference (0).
pub fn decompress_pana_lzss(data: &[u8], expected_size: usize) -> Vec<u8> {
    let mut window = [0u8; 0x1000];
    let mut dst = if expected_size > 0 {
        Vec::with_capacity(expected_size)
    } else {
        Vec::new()
    };
    let mut src_i = 0;
    let mut win_pos = 0x0FEE;
    let mut flags = 0u16;

    while src_i < data.len() {
        flags >>= 1;
        if (flags & 0x100) == 0 {
            if src_i >= data.len() {
                break;
            }
            flags = (data[src_i] as u16) | 0xFF00;
            src_i += 1;
        }

        if (flags & 1) == 0 {
            // 2-byte Backreference: offset (12 bits) and length (4 bits + 3)
            if src_i + 1 >= data.len() {
                break;
            }
            let b1 = data[src_i];
            let b2 = data[src_i + 1];
            src_i += 2;

            let mut offset = (b1 as usize) | (((b2 & 0xF0) as usize) << 4);
            let length = ((b2 & 0x0F) as usize) + 3;

            for _ in 0..length {
                let c = window[offset];
                dst.push(c);
                window[win_pos] = c;
                win_pos = (win_pos + 1) & 0x0FFF;
                offset = (offset + 1) & 0x0FFF;
                if expected_size > 0 && dst.len() >= expected_size {
                    return dst;
                }
            }
        } else {
            // 1-byte Literal
            if src_i >= data.len() {
                break;
            }
            let c = data[src_i];
            src_i += 1;
            dst.push(c);
            window[win_pos] = c;
            win_pos = (win_pos + 1) & 0x0FFF;
            if expected_size > 0 && dst.len() >= expected_size {
                return dst;
            }
        }
    }

    dst
}

/// Compresses data with Panasonic's Okumura LZSS variant (`WindowInitialIndex = 0x0FEE`, `algo == 2`).
///
/// Mirrors `pana-fw/utils/lzss/compress.go` using 3-byte hash-chain matching and lazy match selection.
pub fn compress_pana_lzss(data: &[u8]) -> Vec<u8> {
    const WINDOW_SIZE: usize = 0x1000;
    const WINDOW_INIT: usize = 0x0FEE;
    const MAX_UNCODED: usize = 3;
    const MAX_CODED: usize = MAX_UNCODED + 16;
    const HASH_BITS: usize = 13;
    const HASH_SIZE: usize = 1 << HASH_BITS;

    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    let hash3 = |pos: usize| -> usize {
        let v = (u32::from(data[pos]) << 16)
            | (u32::from(data[pos + 1]) << 8)
            | u32::from(data[pos + 2]);
        ((v.wrapping_mul(2_654_435_761)) >> (32 - HASH_BITS)) as usize
    };

    let mut head = [-1i32; HASH_SIZE];
    let mut prev = [-1i32; WINDOW_SIZE];

    let insert_hash = |pos: usize, head: &mut [i32; HASH_SIZE], prev: &mut [i32; WINDOW_SIZE]| {
        if pos + MAX_UNCODED <= n {
            let h = hash3(pos);
            prev[pos & (WINDOW_SIZE - 1)] = head[h];
            head[h] = pos as i32;
        }
    };

    let find_match = |i: usize,
                      min_len: usize,
                      head: &[i32; HASH_SIZE],
                      prev: &[i32; WINDOW_SIZE]|
     -> (usize, usize) {
        let mut best_len = min_len;
        let mut best_pos = 0usize;
        if i + MAX_UNCODED > n {
            return (best_pos, best_len);
        }
        let max_len = (MAX_CODED - 1).min(n - i);
        if max_len <= best_len {
            return (best_pos, best_len);
        }
        let h = hash3(i);
        let min_pos = (i + 1).saturating_sub(WINDOW_SIZE - MAX_CODED);
        let mut cur = head[h];
        let mut chain = 64usize;
        while cur >= min_pos as i32 && chain > 0 {
            let j = cur as usize;
            if best_len >= max_len {
                break;
            }
            if !(best_len >= MAX_UNCODED && data[j + best_len] != data[i + best_len])
                && data[j] == data[i]
                && data[j + 1] == data[i + 1]
                && data[j + 2] == data[i + 2]
            {
                let mut k = 3usize;
                while k < max_len && data[j + k] == data[i + k] {
                    k += 1;
                }
                if k > best_len {
                    best_len = k;
                    best_pos = j;
                    if k >= max_len {
                        break;
                    }
                }
            }
            cur = prev[j & (WINDOW_SIZE - 1)];
            chain -= 1;
        }
        (best_pos, best_len)
    };

    let mut output = Vec::with_capacity(n / 2 + 16);
    let mut flags_offset = 0usize;
    let mut flags_bit = 0u8;
    let mut i = 0usize;

    while i < n {
        if flags_bit == 0 {
            flags_offset = output.len();
            output.push(0);
        }
        let (match_pos, match_len) = find_match(i, MAX_UNCODED - 1, &head, &prev);
        let lazy_skip = if (MAX_UNCODED..MAX_CODED - 1).contains(&match_len) && i + 1 < n {
            let (_, next_len) = find_match(i + 1, match_len, &head, &prev);
            next_len > match_len
        } else {
            false
        };

        if lazy_skip || match_len < MAX_UNCODED {
            output[flags_offset] |= 1u8 << flags_bit;
            output.push(data[i]);
            insert_hash(i, &mut head, &mut prev);
            i += 1;
        } else {
            let wp = (WINDOW_INIT + match_pos) & (WINDOW_SIZE - 1);
            output.push(wp as u8);
            output.push((((wp >> 4) & 0xF0) as u8) | (((match_len - MAX_UNCODED) & 0x0F) as u8));
            for k in 0..match_len {
                insert_hash(i + k, &mut head, &mut prev);
            }
            i += match_len;
        }
        flags_bit = (flags_bit + 1) & 7;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompress_pana_lzss_vector() {
        // Encodes b"PANASONIC PANASONIC"
        let compressed = [
            0xFF, b'P', b'A', b'N', b'A', b'S', b'O', b'N', b'I', 0xFB, b'C', b' ', 0xEE, 0xF6,
        ];
        let decomp = decompress_pana_lzss(&compressed, 19);
        assert_eq!(&decomp, b"PANASONIC PANASONIC");
    }

    #[test]
    fn test_compress_and_decompress_pana_lzss_roundtrip() {
        let mut payload = b"PANASONIC DIGA DMR-EH55 AM34 FIRMWARE TEST PATTERN! ".repeat(120);
        payload.extend_from_slice(&vec![0xAAu8; 512]);
        payload.extend_from_slice(&(0u8..=255).collect::<Vec<u8>>());

        let compressed = compress_pana_lzss(&payload);
        assert!(compressed.len() < payload.len());
        let decomp = decompress_pana_lzss(&compressed, payload.len());
        assert_eq!(decomp, payload);

        let decomp_auto = decompress_pana_lzss(&compressed, 0);
        assert_eq!(decomp_auto, payload);
    }

    #[test]
    fn test_pack_and_unpack_extrhead_algos_multikey_and_nested() {
        let payload = b"PANASONIC PANASONIC ".repeat(32);
        let custom_key = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[1].1);

        for algo in [
            ExtrheadCompressionAlgo::Zlib,
            ExtrheadCompressionAlgo::Gzip,
            ExtrheadCompressionAlgo::Lzss,
        ] {
            let packed = pack_extrhead_with_options(
                &payload,
                "MAIN",
                0x5040_0000,
                "TEST01",
                ExtrheadCryptMode::Main,
                algo,
                &custom_key,
                16,
                1,
            )
            .unwrap();
            let unpacked = unpack_extrhead(&packed, ExtrheadCryptMode::Auto).unwrap();
            assert_eq!(unpacked.header.algo, algo as u16);
            assert_eq!(unpacked.decompressed, payload);
            assert!(unpacked.checksum_valid);
            assert!(unpacked.footer_present);
        }

        // Test recursive EXTRHEAD (inner LZSS wrapped inside outer GZIP) + OldMain double-encrypt.
        let inner = pack_extrhead_with_options(
            &payload,
            "MAIN",
            0,
            "INNER",
            ExtrheadCryptMode::Plain,
            ExtrheadCompressionAlgo::Lzss,
            &PANA_FEISTEL_ROUND_KEY_32B,
            16,
            1,
        )
        .unwrap();
        let mut outer = pack_extrhead_with_options(
            &inner,
            "MAIN",
            0,
            "OUTER",
            ExtrheadCryptMode::Main,
            ExtrheadCompressionAlgo::Gzip,
            &PANA_FEISTEL_ROUND_KEY_32B,
            16,
            1,
        )
        .unwrap();
        // Add outer full-buffer encryption pass (OldMain double encryption)
        feistel_encrypt_buffer(
            &mut outer,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );

        let unpacked_nested = unpack_extrhead(&outer, ExtrheadCryptMode::Main).unwrap();
        assert_eq!(unpacked_nested.decompressed, payload);
        assert!(unpacked_nested.checksum_valid);
        assert!(unpacked_nested.footer_present);
    }
}
