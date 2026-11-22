//! Panasonic DMR-EH55 DIGA Suite - Pure GDB RSP Protocol Core
//!
//! Pure functional core (Zero I/O, Zero Mutex) for GDB Remote Serial Protocol (RSP)
//! packet framing, checksum verification, AM34 register serialization, and
//! deterministic mock RSP emulation with IPL hardware write-protection invariants.

use panadiga_core::pico::async_gdb_proxy::{is_lethal_am34_register, parse_rsp_addr_len};
use panadiga_core::pico::rsp_codec::{NorWritePolicy, is_nor_write_blocked_with_policy};

use super::types::Am34RegisterState;

/// Pure helper to verify GDB RSP packet checksum and classify the command.
pub fn verify_and_classify_rsp_packet(packet: &str) -> (String, bool) {
    let raw = packet.trim();
    if raw == "+" || raw == "-" {
        return ("AckNak".to_string(), true);
    }

    let trimmed = raw.trim_start_matches('+').trim();
    if !trimmed.starts_with('$') {
        return ("RawBytes".to_string(), false);
    }

    let without_dollar = &trimmed[1..];
    let parts: Vec<&str> = without_dollar.split('#').collect();
    if parts.len() != 2 {
        return ("Malformed".to_string(), false);
    }

    let payload = parts[0];
    let expected_csum_str = parts[1];
    let expected_csum = u8::from_str_radix(expected_csum_str, 16).unwrap_or(0);

    let actual_csum = panadiga_core::pico::rsp_codec::rsp_checksum(payload.as_bytes());
    let checksum_valid = actual_csum == expected_csum;

    let cmd_type = if payload.starts_with("qSupported") {
        "qSupported"
    } else if payload.starts_with('?') {
        "HaltReason"
    } else if payload == "g" {
        "ReadRegs"
    } else if payload.starts_with('G') {
        "WriteRegs"
    } else if payload.starts_with('m') {
        "ReadMem"
    } else if payload.starts_with('M') || payload.starts_with('X') {
        "WriteMem"
    } else if payload.starts_with('c') {
        "Continue"
    } else if payload.starts_with('s') {
        "Step"
    } else if payload.starts_with('k') || payload.starts_with('D') {
        "Detach/Kill"
    } else if payload.starts_with('Z') || payload.starts_with('z') {
        "Breakpoint"
    } else {
        "RspPayload"
    };

    (cmd_type.to_string(), checksum_valid)
}

/// Pure helper to frame a GDB RSP payload with `+$<payload>#<checksum>`.
pub fn build_rsp_packet(payload: &str) -> Vec<u8> {
    let csum = panadiga_core::pico::rsp_codec::rsp_checksum(payload.as_bytes());
    format!("+${}#{:02x}", payload, csum).into_bytes()
}

/// Purely encodes the AM34 register file into 48 little-endian bytes (96 hex chars) for GDB `g` reply.
pub fn encode_am34_registers_hex(regs: &Am34RegisterState) -> String {
    let mut out = String::with_capacity(96);
    for &d in &regs.d {
        for b in d.to_le_bytes() {
            out.push_str(&format!("{:02x}", b));
        }
    }
    for &a in &regs.a {
        for b in a.to_le_bytes() {
            out.push_str(&format!("{:02x}", b));
        }
    }
    for &val in &[regs.mdr, regs.psw, regs.pc, regs.sp] {
        for b in val.to_le_bytes() {
            out.push_str(&format!("{:02x}", b));
        }
    }
    out
}

/// Purely parses a 96-char (`PANAMON1>`) or 224-char (`0x70`-byte `_dev_SERIAL`) hex string from a GDB `g` reply or `G` command into an updated `Am34RegisterState`.
pub fn decode_am34_registers_hex(hex_str: &str, is_halted: bool) -> Option<Am34RegisterState> {
    let clean = hex_str
        .trim()
        .trim_start_matches('+')
        .trim_start_matches('$')
        .split('#')
        .next()
        .unwrap_or("")
        .trim_start_matches('G');
    if !matches!(clean.len(), 96 | 224) || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let parse_u32_le = |slice: &str| -> u32 {
        let mut bytes = [0u8; 4];
        for i in 0..4 {
            bytes[i] = u8::from_str_radix(&slice[i * 2..i * 2 + 2], 16).unwrap_or(0);
        }
        u32::from_le_bytes(bytes)
    };
    let mut d = [0u32; 4];
    let mut a = [0u32; 4];
    for i in 0..4 {
        d[i] = parse_u32_le(&clean[i * 8..(i + 1) * 8]);
        a[i] = parse_u32_le(&clean[32 + i * 8..32 + (i + 1) * 8]);
    }
    Some(Am34RegisterState {
        d,
        a,
        mdr: parse_u32_le(&clean[64..72]),
        psw: parse_u32_le(&clean[72..80]),
        pc: parse_u32_le(&clean[80..88]),
        sp: parse_u32_le(&clean[88..96]),
        is_halted,
    })
}

/// Pure responder for GDB RSP packets (used in Local AM34 Stub mode and unit tests).
pub fn compute_mock_rsp_response_pure(cmd: &str, current_regs: &Am34RegisterState) -> Vec<u8> {
    let mut default_policy = NorWritePolicy::NvramOnly;
    compute_mock_rsp_response_with_policy(cmd, current_regs, &mut default_policy)
}

/// Stateful responder for GDB RSP packets tracking session `NorWritePolicy`.
pub fn compute_mock_rsp_response_with_policy(
    cmd: &str,
    current_regs: &Am34RegisterState,
    policy: &mut NorWritePolicy,
) -> Vec<u8> {
    if cmd.starts_with("qSupported") {
        build_rsp_packet("PacketSize=800;qXfer:memory-map:read+")
    } else if cmd.starts_with("qXfer:memory-map:read::") {
        let xml = concat!(
            "l<memory-map>",
            "<memory type=\"ram\" start=\"0x00000000\" length=\"0x00040000\"/>",
            "<memory type=\"flash\" start=\"0x80000000\" length=\"0x007f0000\">",
            "<property name=\"blocksize\">0x10000</property></memory>",
            "<memory type=\"flash\" start=\"0x807f0000\" length=\"0x00010000\">",
            "<property name=\"blocksize\">0x2000</property></memory>",
            "</memory-map>"
        );
        build_rsp_packet(xml)
    } else if cmd == "qPanadigaNorUnlock:CONFIRM_UPPER_NOR" {
        *policy = NorWritePolicy::UpperNorAppUnlocked;
        build_rsp_packet("OK")
    } else if cmd.starts_with("qPanadigaNorUnlock") {
        build_rsp_packet("E01")
    } else if cmd == "qPanadigaNorLock" {
        *policy = NorWritePolicy::NvramOnly;
        build_rsp_packet("OK")
    } else if cmd.starts_with('?') {
        build_rsp_packet("S05")
    } else if cmd == "g" {
        build_rsp_packet(&encode_am34_registers_hex(current_regs))
    } else if cmd.starts_with('G') {
        build_rsp_packet("OK")
    } else if let Some(rest) = cmd.strip_prefix('m') {
        let mut parts = rest.split(',');
        let addr = u32::from_str_radix(parts.next().unwrap_or("0"), 16).unwrap_or(0);
        let len = usize::from_str_radix(parts.next().unwrap_or("4"), 16)
            .unwrap_or(4)
            .min(512);
        if is_lethal_am34_register(addr, len) {
            return build_rsp_packet("E05");
        }
        let mut hex_out = String::with_capacity(len * 2);
        for i in 0..len {
            let cur = addr.wrapping_add(i as u32);
            let canonical = if (0x8400_0000..=0x847F_FFFF).contains(&cur) {
                cur - 0x0400_0000
            } else {
                cur
            };
            let byte_val: u8 = if (0x807F_0000..=0x807F_FFFF).contains(&canonical) {
                0xFF // Virgin / erased NVRAM sector state
            } else if (0x8000_0000..=0x8000_000F).contains(&canonical) {
                panadiga_core::constants::PANA_FRM_GLOBAL_HEADER[(canonical - 0x8000_0000) as usize]
            } else if (0x8000_0000..=0x807E_FFFF).contains(&canonical) {
                ((canonical & 0xFF) as u8) ^ 0xA5
            } else {
                0x00
            };
            hex_out.push_str(&format!("{:02x}", byte_val));
        }
        build_rsp_packet(&hex_out)
    } else if cmd.starts_with('M') || cmd.starts_with('X') {
        let rest = &cmd[1..];
        let Some((hdr, _data)) = rest.split_once(':') else {
            return build_rsp_packet("E01");
        };
        let Some((addr, len)) = parse_rsp_addr_len(hdr) else {
            return build_rsp_packet("E01");
        };
        // Hardware safety invariant: IPL/Upper NOR (`0x80000000..0x807EFFFF` and uncached mirror `0x84000000..0x847EFFFF`),
        // permanent `SA0..SA10` IPL boot, and lethal register `0xC000100C..0xC000100F` return `E05` per active policy.
        if is_lethal_am34_register(addr, len)
            || is_nor_write_blocked_with_policy(addr, len, *policy)
        {
            build_rsp_packet("E05")
        } else {
            build_rsp_packet("OK")
        }
    } else if cmd.starts_with("Z0")
        || cmd.starts_with("z0")
        || cmd.starts_with("Z1")
        || cmd.starts_with("z1")
    {
        build_rsp_packet("OK")
    } else if cmd.starts_with('c') || cmd.starts_with('s') {
        build_rsp_packet("S05")
    } else if cmd.starts_with('k') || cmd.starts_with('D') {
        build_rsp_packet("OK")
    } else {
        build_rsp_packet("")
    }
}

#[cfg(test)]
#[path = "rsp_protocol_tests.rs"]
mod tests;
