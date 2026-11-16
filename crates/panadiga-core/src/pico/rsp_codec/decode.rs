//! GDB RSP reply parsing, `E0x` error detection, `$m` hex decoding, and MN103S `$g` register extraction.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::encode::rsp_checksum;

/// Canonical MN103S `_dev_SERIAL` `$g` register frame size in bytes (28 x 32-bit LE words).
pub const G_REGISTER_BYTES: usize = 0x70;
const G_REGISTER_WORDS: usize = 28;

const E0X_ERRORS: [(&[u8], &str); 5] = [
    (b"E00", "RSP malformed/bad length"),
    (b"E01", "RSP invalid address or bad argument"),
    (b"E02", "RSP bus/alignment fault"),
    (b"E03", "RSP unsupported command"),
    (b"E04", "RSP overrun / buffer limit"),
];

/// Decoded GDB RSP reply frame extracted from a UART byte stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedRspFrame {
    pub payload_hex: String,
    pub expected_csum: u8,
    pub actual_csum: u8,
    pub checksum_ok: bool,
    pub ack_plus: bool,
    pub nak_minus: bool,
    pub raw_nak_0x15: bool,
}

/// Returns an error if the RSP reply payload matches `E00`..`E04`.
pub fn check_rsp_e0x_error(payload: &[u8]) -> Result<()> {
    if let Some((_, desc)) = E0X_ERRORS.iter().find(|&(code, _)| *code == payload) {
        let code_str = std::str::from_utf8(payload).unwrap_or("E0?");
        bail!("Target returned {code_str}: {desc}");
    }
    Ok(())
}

fn try_build_frame(
    payload: &[u8],
    hex_pair: &[u8],
    ack_plus: bool,
    nak_minus: bool,
    raw_nak_0x15: bool,
) -> Option<ParsedRspFrame> {
    if hex_pair.len() != 2 {
        return None;
    }
    let hex_str = std::str::from_utf8(hex_pair).ok()?;
    let actual_csum = u8::from_str_radix(hex_str, 16).ok()?;
    let expected_csum = rsp_checksum(payload);
    Some(ParsedRspFrame {
        payload_hex: hex::encode(payload),
        expected_csum,
        actual_csum,
        checksum_ok: expected_csum == actual_csum,
        ack_plus,
        nak_minus,
        raw_nak_0x15,
    })
}

/// Scans raw UART bytes for a `$<payload>#<hh>` RSP frame, honoring fixed length hints on binary `$x`.
pub fn parse_rsp_reply(
    stream: &[u8],
    expected_payload_len: Option<usize>,
) -> Option<ParsedRspFrame> {
    let mut search_idx = 0usize;
    let mut first_mismatch: Option<ParsedRspFrame> = None;

    while let Some(rel) = stream[search_idx..].iter().position(|&b| b == b'$') {
        let dollar = search_idx + rel;
        let prefix = &stream[..dollar];
        let ack_plus = prefix.contains(&b'+');
        let nak_minus = prefix.contains(&b'-');
        let raw_nak = prefix.contains(&0x15);
        let start = dollar + 1;

        if let Some(exp_len) = expected_payload_len {
            if start + 6 <= stream.len()
                && E0X_ERRORS
                    .iter()
                    .any(|&(c, _)| c == &stream[start..start + 3])
                && stream[start + 3] == b'#'
                && let Some(err_frame) = try_build_frame(
                    &stream[start..start + 3],
                    &stream[start + 4..start + 6],
                    ack_plus,
                    nak_minus,
                    raw_nak,
                )
                && err_frame.checksum_ok
            {
                return Some(err_frame);
            }
            let exact_hash = start + exp_len;
            if exact_hash + 3 > stream.len() {
                return first_mismatch;
            }
            if stream[exact_hash] == b'#'
                && let Some(exact_frame) = try_build_frame(
                    &stream[start..exact_hash],
                    &stream[exact_hash + 1..exact_hash + 3],
                    ack_plus,
                    nak_minus,
                    raw_nak,
                )
            {
                return Some(exact_frame);
            }
        }

        let mut pos = start;
        while let Some(h_rel) = stream[pos..].iter().position(|&b| b == b'#') {
            let hash_pos = pos + h_rel;
            if hash_pos + 2 >= stream.len() {
                break;
            }
            if let Some(cand) = try_build_frame(
                &stream[start..hash_pos],
                &stream[hash_pos + 1..hash_pos + 3],
                ack_plus,
                nak_minus,
                raw_nak,
            ) {
                if cand.checksum_ok {
                    return Some(cand);
                }
                if first_mismatch.is_none() {
                    first_mismatch = Some(cand);
                }
            }
            pos = hash_pos + 1;
        }
        search_idx = dollar + 1;
    }
    first_mismatch
}

/// Decodes ASCII hex payload returned by `$m`, checking `E00`..`E04`.
pub fn decode_rsp_hex_memory(payload: &[u8]) -> Result<Vec<u8>> {
    check_rsp_e0x_error(payload)?;
    let text = std::str::from_utf8(payload)?;
    if text.is_empty() || !text.len().is_multiple_of(2) {
        bail!("Invalid even hex payload length: {}", text.len());
    }
    Ok(hex::decode(text)?)
}

/// Decodes a 112-byte (`0x70`, `28 x u32` LE) MN103S `_dev_SERIAL` `$g` register reply (`w00`..`w27`).
pub fn decode_mn103s_g_registers(payload: &[u8]) -> Result<BTreeMap<String, u32>> {
    let raw = decode_rsp_hex_memory(payload)?;
    if raw.len() != G_REGISTER_BYTES {
        bail!(
            "Expected {G_REGISTER_BYTES}-byte MN103S $g register frame, got {} bytes",
            raw.len()
        );
    }
    let mut regs = BTreeMap::new();
    for (idx, chunk) in raw
        .as_chunks::<4>()
        .0
        .iter()
        .take(G_REGISTER_WORDS)
        .enumerate()
    {
        regs.insert(format!("w{idx:02}"), u32::from_le_bytes(*chunk));
    }
    Ok(regs)
}

/// Validates non-destructive RSP memory read bounds and chunk limits.
pub fn validate_dump_range(
    _start_addr: u32,
    total_bytes: usize,
    chunk_size: usize,
    max_chunk_size: usize,
) -> Result<()> {
    if total_bytes == 0 {
        bail!("total_bytes must be positive");
    }
    if chunk_size == 0 || chunk_size > max_chunk_size {
        bail!("chunk_size must be in 1..{max_chunk_size} for reliable UART RSP framing");
    }
    Ok(())
}
