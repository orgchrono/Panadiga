#![deny(unsafe_code)]

//! Asynchronous UART <-> GDB Remote Serial Protocol (RSP) translator for P7501.
//!
//! Because connector `P7501` on Panasonic DMR-EH55 carries asynchronous `115200 8N1` UART
//! (no synchronous DWire32A OCD clock), Ghidra's synchronous GDB RSP packets (`$m`, `$M`,
//! `$g`, `$qSupported`) are translated on-the-fly into asynchronous `PANAMON1>` (`R`/`W`)
//! or `MAIN` `_dev_SERIAL` (`PANARSP`) exchanges over Pico CDC `BRIDGE`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serialport::SerialPort;

use super::panamon1::{
    PANAMON1_FULL_PROMPT, PANAMON1_READ_CHUNK_BYTES, parse_panamon1_read_bytes, query_panamon1_uart,
};
use super::rsp_codec::{NorWritePolicy, build_rsp_frame, is_nor_write_blocked_with_policy};

#[cfg(test)]
mod tests;

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

const AM34_MEMORY_MAP_XML: &str = concat!(
    "<memory-map>",
    "<memory type=\"ram\" start=\"0x00000000\" length=\"0x00040000\"/>",
    "<memory type=\"ram\" start=\"0x40000000\" length=\"0x04000000\"/>",
    "<memory type=\"rom\" start=\"0x80000000\" length=\"0x007F0000\"/>",
    "<memory type=\"flash\" start=\"0x807F0000\" length=\"0x00010000\">",
    "<property name=\"blocksize\">0x2000</property></memory>",
    "</memory-map>"
);

fn parse_two_hex_args(arg: &str) -> Option<(u32, usize)> {
    let (lhs, rhs) = arg.split_once(',')?;
    let addr = u32::from_str_radix(lhs.trim().trim_start_matches("0x"), 16).ok()?;
    let len = usize::from_str_radix(rhs.trim().trim_start_matches("0x"), 16).ok()?;
    Some((addr, len))
}

pub(super) fn handle_panamon1_rsp_packet_io<T: Read + Write + ?Sized>(
    cmd: &str,
    serial: Option<&mut T>,
) -> Vec<u8> {
    let payload: String = if cmd.starts_with("qSupported") {
        "PacketSize=400;qXfer:memory-map:read+;qAttached+".to_string()
    } else if cmd == "qAttached" {
        "1".to_string()
    } else if cmd.starts_with("qXfer:memory-map:read::") {
        format!("l{AM34_MEMORY_MAP_XML}")
    } else if cmd == "?" {
        "S05".to_string()
    } else if cmd.starts_with('H') || cmd == "k" {
        "OK".to_string()
    } else if cmd == "g" {
        // Return 28x32-bit zeroed MN103S snapshot (pc=0x50400000 stage-1.5 entry) cleanly
        let mut regs = "0".repeat(224);
        regs.replace_range(80..88, "00004050");
        regs
    } else if let Some(rest) = cmd.strip_prefix('m') {
        let Some((addr, len)) = parse_two_hex_args(rest) else {
            return build_rsp_frame(b"E01", false).unwrap_or_default();
        };
        if len == 0 {
            return build_rsp_frame(b"", false).unwrap_or_default();
        }
        let Some(port) = serial else {
            return build_rsp_frame(b"E0A", false).unwrap_or_default();
        };
        let mut raw_bytes = Vec::with_capacity(len);
        let mut offset = 0usize;
        while offset < len {
            let chunk = (len - offset).min(PANAMON1_READ_CHUNK_BYTES);
            let va = addr.wrapping_add(offset as u32);
            let reply = query_panamon1_uart(
                port,
                &format!("R {va:08X},{chunk:X}"),
                Duration::from_millis(500),
            );
            let Some(parsed) = parse_panamon1_read_bytes(&reply, chunk) else {
                return build_rsp_frame(b"E0A", false).unwrap_or_default();
            };
            raw_bytes.extend_from_slice(&parsed);
            offset += chunk;
        }
        raw_bytes.iter().map(|b| format!("{b:02x}")).collect()
    } else if let Some(rest) = cmd.strip_prefix('M') {
        let Some((hdr, hex_data)) = rest.split_once(':') else {
            return build_rsp_frame(b"E01", false).unwrap_or_default();
        };
        let Some((addr, len)) = parse_two_hex_args(hdr) else {
            return build_rsp_frame(b"E01", false).unwrap_or_default();
        };
        if is_nor_write_blocked_with_policy(addr, len, NorWritePolicy::NvramOnly) {
            return build_rsp_frame(b"E05", false).unwrap_or_default();
        }
        let Some(port) = serial else {
            return build_rsp_frame(b"E0A", false).unwrap_or_default();
        };
        let Ok(bytes) = hex::decode(hex_data.trim()) else {
            return build_rsp_frame(b"E01", false).unwrap_or_default();
        };
        if len == 0 || bytes.len() != len {
            return build_rsp_frame(b"E01", false).unwrap_or_default();
        }
        let mut all_ok = true;
        for (idx, &byte) in bytes.iter().enumerate() {
            let va = addr.wrapping_add(idx as u32);
            let reply = query_panamon1_uart(
                port,
                &format!("W {va:08X},{byte:02X}"),
                Duration::from_millis(400),
            );
            if !reply.contains("OK:WRITE")
                || !reply.contains(PANAMON1_FULL_PROMPT)
                || reply.contains("ERR:")
            {
                all_ok = false;
                break;
            }
        }
        if all_ok {
            "OK".to_string()
        } else {
            "E05".to_string()
        }
    } else {
        String::new()
    };

    build_rsp_frame(payload.as_bytes(), false).unwrap_or_default()
}

pub(super) fn handle_panamon1_rsp_packet(
    cmd: &str,
    serial: Option<&mut dyn SerialPort>,
) -> Vec<u8> {
    handle_panamon1_rsp_packet_io(cmd, serial)
}

/// Serves one synchronous TCP GDB/Ghidra client by translating RSP packets asynchronously
/// into `PANAMON1>` UART transactions on Pico `BRIDGE`.
pub fn serve_panamon1_tcp_client(
    mut tcp_stream: TcpStream,
    serial_port: &mut dyn SerialPort,
) -> std::io::Result<()> {
    let _ = tcp_stream.set_read_timeout(Some(Duration::from_millis(80)));
    let mut buf = [0u8; 2048];
    let mut rx_acc = String::new();

    loop {
        match tcp_stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                rx_acc.push_str(&String::from_utf8_lossy(&buf[..n]));
                if rx_acc.contains('\x03') {
                    rx_acc = rx_acc.replace('\x03', "");
                    let _ =
                        tcp_stream.write_all(&build_rsp_frame(b"S05", false).unwrap_or_default());
                }
                while let Some(dollar) = rx_acc.find('$') {
                    let after_dollar = &rx_acc[dollar + 1..];
                    let Some(hash) = after_dollar.find('#') else {
                        break;
                    };
                    if after_dollar.len() < hash + 3 {
                        break;
                    }
                    let cmd = after_dollar[..hash].to_string();
                    rx_acc.drain(..dollar + 1 + hash + 3);
                    let _ = tcp_stream.write_all(b"+");
                    let reply = handle_panamon1_rsp_packet(&cmd, Some(serial_port));
                    tcp_stream.write_all(&reply)?;
                    tcp_stream.flush()?;
                    if cmd == "k" {
                        return Ok(());
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
