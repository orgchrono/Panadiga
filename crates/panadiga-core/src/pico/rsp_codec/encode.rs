//! GDB RSP frame encoding (`$<payload>#<xx>`), `$M`/`$X` write builders, and Upper NOR unlock packets.

use anyhow::{Result, bail};

use super::policy::{NorWritePolicy, is_nor_write_blocked_with_policy};

/// Safe maximum `$m` ASCII hex chunk payload size.
pub const MAX_M_CHUNK: usize = 512;
/// Safe maximum `$x` binary chunk payload size.
pub const MAX_X_CHUNK: usize = 0x440;

/// Computes standard 8-bit modulo-256 GDB RSP checksum.
pub fn rsp_checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

pub(super) fn encode_rsp_frame_raw(payload: &[u8]) -> Vec<u8> {
    let csum = rsp_checksum(payload);
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.push(b'$');
    out.extend_from_slice(payload);
    out.extend_from_slice(format!("#{csum:02x}").as_bytes());
    out
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
    Ok(encode_rsp_frame_raw(payload))
}

/// Builds the explicit Pico GDB RSP query frame to unlock Upper NOR App/BGO (`SA4..SA133`).
pub fn build_nor_unlock_frame() -> Vec<u8> {
    encode_rsp_frame_raw(b"qPanadigaNorUnlock:CONFIRM_UPPER_NOR")
}

/// Builds the Pico GDB RSP query frame that re-locks Upper NOR back to `NvramOnly` policy.
pub fn build_nor_lock_frame() -> Vec<u8> {
    encode_rsp_frame_raw(b"qPanadigaNorLock")
}

/// Builds a hardware-guarded `$M` or `$X` write frame using default `NvramOnly` policy (blocks IPL NOR with `E05`).
pub fn build_rsp_write_frame(addr: u32, data: &[u8], use_binary_x: bool) -> Result<Vec<u8>> {
    build_rsp_write_frame_with_policy(addr, data, use_binary_x, NorWritePolicy::NvramOnly)
}

/// Builds a hardware-guarded `$M` (ASCII hex) or `$X` (escaped binary) write frame under [`NorWritePolicy`].
pub fn build_rsp_write_frame_with_policy(
    addr: u32,
    data: &[u8],
    use_binary_x: bool,
    policy: NorWritePolicy,
) -> Result<Vec<u8>> {
    if data.is_empty() {
        bail!("Write payload cannot be empty");
    }
    if is_nor_write_blocked_with_policy(addr, data.len(), policy) {
        let scope = match policy {
            NorWritePolicy::NvramOnly => "read-only IPL NOR",
            NorWritePolicy::UpperNorAppUnlocked => "critical boot IPL NOR (SA0..SA3)",
        };
        bail!("Hardware Safety Lockout (E05): refusing RSP write to {scope} 0x{addr:08X}");
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
