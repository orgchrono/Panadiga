//! `EXTRHEAD`/`EXTRFOOT` header parser, packer, and recursive unpacker.

use anyhow::{Context, Result, bail};

use super::super::codec::{
    extract_c_ascii, pad_to_block, read_u16_le, read_u32_le, write_padded_ascii,
};
use super::super::feistel::{adler32_panasonic_strided, feistel_decrypt_buffer};
use super::crypt::{
    apply_partial_crypt_with_key, compress_payload_by_algo, decompress_payload_by_algo,
    detect_extrhead_key_and_double, resolve_crypt_mode,
};
use super::types::{ExtrheadCompressionAlgo, ExtrheadCryptMode, ExtrheadHeader, UnpackedExtrhead};
use crate::constants::{PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256};

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
