//! Asynchronous UART <-> GDB Remote Serial Protocol (RSP) translator for P7501.
//!
//! Because connector `P7501` on Panasonic DMR-EH55 carries asynchronous `115200 8N1` UART
//! (no synchronous DWire32A OCD clock), Ghidra's synchronous GDB RSP packets (`$m`, `$M`,
//! `$g`, `$qSupported`) are translated on-the-fly into asynchronous `PANAMON1>` (`R`/`W`)
//! or `MAIN` `_dev_SERIAL` (`PANARSP`) exchanges over Pico CDC `BRIDGE`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use serialport::SerialPort;

use super::rsp_codec::{NorWritePolicy, build_rsp_frame, is_nor_write_blocked_with_policy};

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

fn parse_panamon1_read_words(reply: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for line in reply.lines() {
        let Some((_, hex_part)) = line.split_once(':') else {
            continue;
        };
        for tok in hex_part.split_whitespace() {
            if tok.len() == 8
                && tok.chars().all(|c| c.is_ascii_hexdigit())
                && let Ok(word) = u32::from_str_radix(tok, 16)
            {
                out.extend_from_slice(&word.to_le_bytes());
            }
        }
    }
    out
}

fn query_panamon1_uart(serial: &mut dyn SerialPort, cmd: &str, max_wait: Duration) -> String {
    let _ = serial.write_all(format!("{cmd}\r\n").as_bytes());
    let _ = serial.flush();
    let deadline = Instant::now() + max_wait;
    let mut out = String::new();
    let mut buf = [0u8; 512];
    while Instant::now() < deadline {
        if let Ok(n) = serial.read(&mut buf)
            && n > 0
        {
            out.push_str(&String::from_utf8_lossy(&buf[..n]));
            if out.contains("PANAMON1>") {
                break;
            }
        }
    }
    out
}

fn handle_panamon1_rsp_packet(cmd: &str, serial: Option<&mut dyn SerialPort>) -> Vec<u8> {
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
        let Some(port) = serial else {
            return build_rsp_frame(b"E0A", false).unwrap_or_default();
        };
        let aligned = addr & !0x3;
        let offset = (addr - aligned) as usize;
        let words = (offset + len).div_ceil(4).clamp(1, 32);
        let mut raw_bytes = Vec::with_capacity(words * 4);
        for w in 0..words {
            let va = aligned.wrapping_add((w * 4) as u32);
            let reply =
                query_panamon1_uart(port, &format!("R {va:08X}"), Duration::from_millis(350));
            let parsed = parse_panamon1_read_words(&reply);
            if parsed.is_empty() {
                return build_rsp_frame(b"E0A", false).unwrap_or_default();
            }
            raw_bytes.extend_from_slice(&parsed);
        }
        let slice = &raw_bytes[offset.min(raw_bytes.len())..(offset + len).min(raw_bytes.len())];
        slice.iter().map(|b| format!("{b:02x}")).collect()
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
        let bytes = hex::decode(hex_data.trim()).unwrap_or_default();
        if bytes.len() < 2 {
            return build_rsp_frame(b"E01", false).unwrap_or_default();
        }
        let val16 = u16::from_le_bytes([bytes[0], bytes[1]]);
        let reply = query_panamon1_uart(
            port,
            &format!("W {addr:08X},{val16:04X}"),
            Duration::from_millis(400),
        );
        if reply.contains("PANAMON1>") && !reply.contains("ERR:") {
            "OK".to_string()
        } else {
            "E05".to_string()
        }
    } else {
        String::new()
    };

    build_rsp_frame(payload.as_bytes(), false).unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_target_modes_and_translates_panamon1_rsp_safely() {
        assert_eq!(
            GdbBridgeTargetMode::parse_mode("panamon1").unwrap(),
            GdbBridgeTargetMode::Panamon1
        );
        assert_eq!(
            GdbBridgeTargetMode::parse_mode("panarsp").unwrap(),
            GdbBridgeTargetMode::Panarsp
        );
        let sup = String::from_utf8(handle_panamon1_rsp_packet("qSupported", None)).unwrap();
        assert!(sup.contains("qXfer:memory-map:read+"));

        let g = String::from_utf8(handle_panamon1_rsp_packet("g", None)).unwrap();
        assert!(g.starts_with('$') && g.len() > 220);

        // Critical IPL NOR write must be blocked with E05 even through async Ghidra proxy
        let e05 = String::from_utf8(handle_panamon1_rsp_packet("M80000000,2:1234", None)).unwrap();
        assert!(e05.starts_with("$E05#"));
    }

    #[test]
    fn parses_panamon1_word_dump_lines_cleanly() {
        let parsed = parse_panamon1_read_words("80000000: AABBCCDD 11223344\r\nPANAMON1>");
        assert_eq!(parsed, vec![0xDD, 0xCC, 0xBB, 0xAA, 0x44, 0x33, 0x22, 0x11]);
    }
}
