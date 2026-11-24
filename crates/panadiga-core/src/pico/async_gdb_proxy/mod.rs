#![deny(unsafe_code)]

//! Asynchronous UART <-> GDB Remote Serial Protocol (RSP) translator for P7501.
//!
//! Because connector `P7501` on Panasonic DMR-EH55 carries asynchronous `115200 8N1` UART
//! (no synchronous DWire32A OCD clock), Ghidra's synchronous GDB RSP packets (`$m`, `$M`,
//! `$g`, `$qSupported`) are translated on-the-fly into asynchronous `PANAMON1>` (`R`/`W`)
//! or `MAIN` `_dev_SERIAL` (`PANARSP`) exchanges over Pico CDC `BRIDGE`.

use std::io::{Read, Write};
use std::time::Duration;

#[cfg(test)]
use serialport::SerialPort;

use super::panamon1::{
    PANAMON1_FULL_PROMPT, PANAMON1_READ_CHUNK_BYTES, PANAMON1_RETRY_PACING_US,
    parse_panamon1_read_bytes, query_panamon1_uart,
};
use super::rsp_codec::{
    NorWritePolicy, build_am34_gdb_memory_map_xml, build_rsp_frame,
    is_nor_write_blocked_with_policy,
};

mod tcp_server;

#[cfg(test)]
mod tests;

pub use tcp_server::serve_panamon1_tcp_client;

/// Target monitor mode used behind TCP `127.0.0.1:3333` GDB RSP bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GdbBridgeTargetMode {
    /// Legacy RP2040 C11 `GDB` command handler (hardware DWire stub; returns `0xFFFFFFFF` on P7501).
    #[default]
    OcdRaw,
    /// Translates GDB RSP requests into asynchronous `PANAMON1>` commands over Pico `BRIDGE 115200 PASSIVE`.
    Panamon1,
    /// Proxies GDB RSP packets directly to `MAIN` `_dev_SERIAL` stub over Pico `BRIDGE 115200 PASSIVE`.
    Panarsp,
}

impl GdbBridgeTargetMode {
    /// Parses CLI `--target-mode` string (`ocd`, `panamon1`, `panarsp`).
    pub fn parse_mode(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ocd" | "ocd-raw" | "raw" | "synchronous" => Ok(Self::OcdRaw),
            "panamon1" | "digadiag" | "async-panamon1" => Ok(Self::Panamon1),
            "panarsp" | "main" | "async-panarsp" => Ok(Self::Panarsp),
            other => Err(format!(
                "Unsupported GDB bridge target mode '{other}' (expected ocd, panamon1, or panarsp)"
            )),
        }
    }

    /// Human-readable description shown on bridge startup banner.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OcdRaw => "ocd (RP2040 native GDB handler — passive/no DWire clock on P7501)",
            Self::Panamon1 => {
                "panamon1 (Async UART RSP proxy -> DIGADIAG PANAMON1> R/W over BRIDGE)"
            }
            Self::Panarsp => "panarsp (Async UART RSP passthrough -> MAIN _dev_SERIAL over BRIDGE)",
        }
    }
}

/// Returns `true` when `[addr, addr + length)` touches the lethal AM34 watchdog/bus register `0xC000100C..0xC000100F`.
#[must_use]
pub fn is_lethal_am34_register(addr: u32, length: usize) -> bool {
    let start = u64::from(addr);
    let end = start.saturating_add(length.max(1) as u64);
    let lo = u64::from(crate::constants::AM34_FORBIDDEN_LETHAL_REG);
    start < lo + 4 && end > lo
}

/// Parses `addr,len` hexadecimal arguments used by GDB RSP `$m`, `$M`, and `$X`.
#[must_use]
pub fn parse_rsp_addr_len(arg: &str) -> Option<(u32, usize)> {
    let (lhs, rhs) = arg.split_once(',')?;
    let addr = u32::from_str_radix(lhs.trim().trim_start_matches("0x"), 16).ok()?;
    let len = usize::from_str_radix(rhs.trim().trim_start_matches("0x"), 16).ok()?;
    Some((addr, len))
}

fn query_panamon1_with_retry<T: Read + Write + ?Sized>(
    serial: &mut T,
    cmd: &str,
    max_wait: Duration,
) -> String {
    let pacing = Duration::from_micros(PANAMON1_RETRY_PACING_US);
    for attempt in 0..2 {
        if attempt > 0 {
            std::thread::sleep(pacing);
        }
        let reply = query_panamon1_uart(serial, cmd, max_wait);
        let has_syntax_retry = reply.lines().any(|line| line.trim() == "?");
        let missing_prompt = !reply.contains(PANAMON1_FULL_PROMPT);
        if attempt == 0 && (has_syntax_retry || missing_prompt) {
            continue;
        }
        std::thread::sleep(pacing);
        return reply;
    }
    String::new()
}

fn decode_rsp_binary_x(raw: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(raw.len());
    let mut idx = 0usize;
    while idx < raw.len() {
        if raw[idx] == 0x7D {
            idx += 1;
            if idx >= raw.len() {
                return None;
            }
            out.push(raw[idx] ^ 0x20);
        } else {
            out.push(raw[idx]);
        }
        idx += 1;
    }
    Some(out)
}

fn write_bytes_via_panamon1<T: Read + Write + ?Sized>(
    port: &mut T,
    addr: u32,
    bytes: &[u8],
) -> &'static str {
    for (idx, &byte) in bytes.iter().enumerate() {
        let va = addr.wrapping_add(idx as u32);
        let reply = query_panamon1_with_retry(
            port,
            &format!("W {va:08X},{byte:02X}"),
            Duration::from_millis(400),
        );
        if !reply.contains("OK:WRITE")
            || !reply.contains(PANAMON1_FULL_PROMPT)
            || reply.contains("ERR:")
        {
            return "E05";
        }
    }
    "OK"
}

fn handle_read_mem_cmd<T: Read + Write + ?Sized>(rest: &str, serial: Option<&mut T>) -> String {
    let Some((addr, len)) = parse_rsp_addr_len(rest) else {
        return "E01".to_string();
    };
    if len == 0 {
        return String::new();
    }
    if is_lethal_am34_register(addr, len) {
        return "E05".to_string();
    }
    let Some(port) = serial else {
        return "E0A".to_string();
    };
    let mut raw_bytes = Vec::with_capacity(len);
    let mut offset = 0usize;
    while offset < len {
        let chunk = (len - offset).min(PANAMON1_READ_CHUNK_BYTES);
        let va = addr.wrapping_add(offset as u32);
        let reply = query_panamon1_with_retry(
            port,
            &format!("R {va:08X},{chunk:X}"),
            Duration::from_millis(500),
        );
        if reply.contains("ERR:E05") {
            return "E05".to_string();
        }
        let Some(parsed) = parse_panamon1_read_bytes(&reply, chunk) else {
            return "E0A".to_string();
        };
        raw_bytes.extend_from_slice(&parsed);
        offset += chunk;
    }
    raw_bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn handle_write_mem_cmd<T: Read + Write + ?Sized>(
    is_binary_x: bool,
    rest: &[u8],
    serial: Option<&mut T>,
    policy: NorWritePolicy,
) -> String {
    let Some(colon) = rest.iter().position(|&b| b == b':') else {
        return "E01".to_string();
    };
    let Ok(hdr) = std::str::from_utf8(&rest[..colon]) else {
        return "E01".to_string();
    };
    let Some((addr, len)) = parse_rsp_addr_len(hdr) else {
        return "E01".to_string();
    };
    if is_lethal_am34_register(addr, len) || is_nor_write_blocked_with_policy(addr, len, policy) {
        return "E05".to_string();
    }
    let payload = &rest[colon + 1..];
    if is_binary_x && len == 0 && payload.is_empty() {
        return "OK".to_string();
    }
    let bytes = if is_binary_x {
        let Some(decoded) = decode_rsp_binary_x(payload) else {
            return "E01".to_string();
        };
        decoded
    } else {
        let Ok(hex_str) = std::str::from_utf8(payload) else {
            return "E01".to_string();
        };
        let Ok(decoded) = hex::decode(hex_str.trim()) else {
            return "E01".to_string();
        };
        decoded
    };
    if len == 0 || bytes.len() != len {
        return "E01".to_string();
    }
    let Some(port) = serial else {
        return "E0A".to_string();
    };
    write_bytes_via_panamon1(port, addr, &bytes).to_string()
}

pub(super) fn handle_panamon1_rsp_bytes_io<T: Read + Write + ?Sized>(
    cmd_bytes: &[u8],
    serial: Option<&mut T>,
    policy: &mut NorWritePolicy,
) -> Vec<u8> {
    if let Some((&op, rest)) = cmd_bytes.split_first()
        && matches!(op, b'M' | b'X')
    {
        let payload = handle_write_mem_cmd(op == b'X', rest, serial, *policy);
        return build_rsp_frame(payload.as_bytes(), false).unwrap_or_default();
    }

    let Ok(cmd) = std::str::from_utf8(cmd_bytes) else {
        return build_rsp_frame(b"E01", false).unwrap_or_default();
    };

    let payload: String = if cmd.starts_with("qSupported") {
        "PacketSize=400;qXfer:memory-map:read+;qAttached+".to_string()
    } else if cmd == "qAttached" {
        "1".to_string()
    } else if cmd.starts_with("qXfer:memory-map:read::") {
        format!("l{}", build_am34_gdb_memory_map_xml(true, false))
    } else if cmd == "qPanadigaNorUnlock:CONFIRM_UPPER_NOR" {
        if let Some(port) = serial {
            let reply =
                query_panamon1_with_retry(port, "U CONFIRM_UPPER_NOR", Duration::from_millis(1000));
            if reply.contains("OK:NOR_UNLOCK")
                && reply.contains(PANAMON1_FULL_PROMPT)
                && !reply.contains("ERR:")
            {
                *policy = NorWritePolicy::UpperNorAppUnlocked;
                "OK".to_string()
            } else {
                "E05".to_string()
            }
        } else {
            "E0A".to_string()
        }
    } else if cmd.starts_with("qPanadigaNorUnlock") {
        "E01".to_string()
    } else if cmd == "qPanadigaNorLock" {
        *policy = NorWritePolicy::NvramOnly;
        if let Some(port) = serial {
            let reply = query_panamon1_with_retry(port, "U LOCK", Duration::from_millis(1000));
            if reply.contains("OK:NOR_LOCK")
                && reply.contains(PANAMON1_FULL_PROMPT)
                && !reply.contains("ERR:")
            {
                "OK".to_string()
            } else {
                "E0A".to_string()
            }
        } else {
            "OK".to_string()
        }
    } else if cmd == "?" {
        "S05".to_string()
    } else if cmd.starts_with('H') || cmd == "k" {
        "OK".to_string()
    } else if cmd == "g" {
        // Return 28x32-bit zeroed MN103S snapshot (pc=0x50400000 DIAG_PROG_RAM entry) cleanly
        let mut regs = "0".repeat(224);
        regs.replace_range(80..88, "00004050");
        regs
    } else if let Some(rest) = cmd.strip_prefix('m') {
        handle_read_mem_cmd(rest, serial)
    } else {
        String::new()
    };

    build_rsp_frame(payload.as_bytes(), false).unwrap_or_default()
}

#[cfg(test)]
pub(super) fn handle_panamon1_rsp_packet_io<T: Read + Write + ?Sized>(
    cmd: &str,
    serial: Option<&mut T>,
) -> Vec<u8> {
    let mut policy = NorWritePolicy::NvramOnly;
    handle_panamon1_rsp_bytes_io(cmd.as_bytes(), serial, &mut policy)
}

#[cfg(test)]
pub(super) fn handle_panamon1_rsp_packet(
    cmd: &str,
    serial: Option<&mut dyn SerialPort>,
) -> Vec<u8> {
    handle_panamon1_rsp_packet_io(cmd, serial)
}
