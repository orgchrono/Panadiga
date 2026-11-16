//! Pure functional GDB Remote Serial Protocol (RSP) frame codec and AM34 read-only safety guards.
//!
//! Shared SSOT across CLI, Tauri bridge, and Python bench tooling for `$<payload>#<xx>` framing,
//! binary `$x` escaping, E05 IPL NOR write protection (`0x80000000..0x807EFFFF` / `0x84..`),
//! `$m`/`$g` response validation, and 28-word (`0x70`-byte) MN103S register decoding.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Strictly read-only IPL NOR windows (`0x80000000..0x807EFFFF` cached & `0x84000000..0x847EFFFF` uncached).
pub const IPL_READONLY_RANGES: [(u64, u64); 2] =
    [(0x8000_0000, 0x807F_0000), (0x8400_0000, 0x847F_0000)];
/// Safe maximum `$m` ASCII hex chunk payload size.
pub const MAX_M_CHUNK: usize = 512;
/// Safe maximum `$x` binary chunk payload size.
pub const MAX_X_CHUNK: usize = 0x440;
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

/// Computes standard 8-bit modulo-256 GDB RSP checksum.
pub fn rsp_checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// Returns `true` when `[addr, addr + length)` touches read-only Spansion IPL NOR flash.
pub fn is_ipl_readonly_address(addr: u32, length: usize) -> bool {
    let start = addr as u64;
    let span = length.max(1) as u64;
    let raw_end = (start + span) & 0xFFFF_FFFF;
    let end = if raw_end < start {
        0x1_0000_0000
    } else {
        raw_end
    };
    IPL_READONLY_RANGES
        .iter()
        .any(|&(lo, hi)| start < hi && end > lo)
}

/// Encodes `$<payload>#<xx>`, rejecting mutating or execution opcodes unless `allow_write` is true.
pub fn build_rsp_frame(payload: &[u8], allow_write: bool) -> Result<Vec<u8>> {
    if !allow_write
        && matches!(
            payload.first().copied(),
            Some(b'M' | b'X' | b'c' | b's' | b'k')
        )
    {
        let op = payload[0] as char;
        bail!("Mutating or execution RSP command '{op}' requires allow_write=true");
    }
    let csum = rsp_checksum(payload);
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.push(b'$');
    out.extend_from_slice(payload);
    out.extend_from_slice(format!("#{csum:02x}").as_bytes());
    Ok(out)
}

/// Builds a hardware-guarded `$M` (ASCII hex) or `$X` (escaped binary) write frame, blocking IPL NOR (`E05`).
pub fn build_rsp_write_frame(addr: u32, data: &[u8], use_binary_x: bool) -> Result<Vec<u8>> {
    if data.is_empty() {
        bail!("Write payload cannot be empty");
    }
    if is_ipl_readonly_address(addr, data.len()) {
        bail!(
            "Hardware Safety Lockout (E05): refusing RSP write to read-only IPL NOR 0x{addr:08X}"
        );
    }
    if use_binary_x {
        let mut raw = format!("X{addr:x},{:x}:", data.len()).into_bytes();
        for &b in data {
            if matches!(b, 0x23 | 0x24 | 0x2A | 0x7D) {
                raw.push(0x7D);
                raw.push(b ^ 0x20);
            } else {
                raw.push(b);
            }
        }
        return build_rsp_frame(&raw, true);
    }
    let hex_body = hex::encode(data);
    let raw = format!("M{addr:x},{:x}:{hex_body}", data.len());
    build_rsp_frame(raw.as_bytes(), true)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsp_guard_binary_x_and_register_decoding_work() {
        assert!(is_ipl_readonly_address(0x8400_0000, 4));
        assert!(!is_ipl_readonly_address(0x847F_0000, 4));
        assert!(build_rsp_frame(b"M51800000,4:01020304", false).is_err());
        assert!(build_rsp_write_frame(0x8000_0000, &[0, 1], false).is_err());
        assert!(build_rsp_write_frame(0x847E_FFF8, &[0; 16], false).is_err());

        let x_frame = build_rsp_write_frame(0x847F_0000, b"}#$*", true).unwrap();
        let parsed = parse_rsp_reply(&x_frame, None).unwrap();
        assert!(parsed.checksum_ok);
        let decoded = hex::decode(&parsed.payload_hex).unwrap();
        assert!(decoded.starts_with(b"X847f0000,4:"));

        let mut raw_g = Vec::with_capacity(G_REGISTER_BYTES);
        for i in 0..28u32 {
            let v = match i {
                25 => 0xAA00_0064,
                26 => 0xBB00_0068,
                27 => 0xCC00_006C,
                _ => 0x1000 + i,
            };
            raw_g.extend_from_slice(&v.to_le_bytes());
        }
        let regs = decode_mn103s_g_registers(hex::encode(raw_g).as_bytes()).unwrap();
        assert_eq!(regs.get("w25"), Some(&0xAA00_0064));
        assert_eq!(regs.get("w27"), Some(&0xCC00_006C));
        assert!(decode_mn103s_g_registers(b"0000").is_err());
    }
}
