#![deny(unsafe_code)]

//! Wire protocol constants, parsers, and [`Panamon1Transport`] trait for `PANAMON1>` / Pico `BRIDGE`.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Exact interactive prompt emitted by `PROG061` `PANAMON1` over `SC0` UART0.
pub const PANAMON1_FULL_PROMPT: &str = "PANAMON1>[B/R/W/D/F/J/E/U/Q]";
/// Safe default read burst chunk size for `R <va>,<len>` exchanges.
pub const PANAMON1_READ_CHUNK_BYTES: usize = 64;
/// Deterministic single-retry pacing delay (`300 us`) on `?` syntax echo or partial timeout.
pub const PANAMON1_RETRY_PACING_US: u64 = 300;
/// Mandatory SDRAM settle delay (`3 ms`) after burst `W` before issuing `J <va>`.
pub const PANAMON1_POST_WRITE_JUMP_DELAY_MS: u64 = 3;

pub(super) const NOR_MAGIC_HEADER: &[u8; 8] = b"PANADMP1";
pub(super) const NOR_MAGIC_FOOTER_DONE: &[u8; 15] = b"PANADONEOK:DONE";

/// Telemetry summary printed by Pico firmware upon `~~~` escape out of `BRIDGE` mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeExitSummary {
    pub rx_bytes: u32,
    pub tx_bytes: u32,
    pub ferr: u32,
    pub ctrl_edges: u32,
    pub overflow: bool,
    pub raw_line: String,
}

impl BridgeExitSummary {
    /// Verifies zero framing errors, zero FIFO overflows, and zero control-line glitches.
    pub fn validate_healthy(&self) -> Result<()> {
        if self.ctrl_edges != 0 || self.overflow || self.ferr != 0 {
            bail!(
                "Pico BRIDGE telemetry anomaly: ctrl_edges={}, overflow={}, ferr={} ({})",
                self.ctrl_edges,
                self.overflow,
                self.ferr,
                self.raw_line
            );
        }
        Ok(())
    }
}

fn find_kv_field<'a>(line: &'a str, key_eq: &str) -> Option<&'a str> {
    line.split_whitespace().find_map(|tok| {
        tok.trim_matches(|c: char| matches!(c, '(' | ')' | ','))
            .strip_prefix(key_eq)
            .map(|v| v.trim_end_matches(')'))
    })
}

/// Parses `BRIDGE: OK (rx=... tx=... ferr=... ctrl_edges=... overflow=NO|YES)`.
pub fn parse_bridge_exit_summary(text: &str) -> Result<BridgeExitSummary> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.contains("BRIDGE: OK"))
        .ok_or_else(|| anyhow::anyhow!("Missing 'BRIDGE: OK' summary in reply: {text:?}"))?;
    let parse_u32 = |key| {
        find_kv_field(line, key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    };
    let overflow = find_kv_field(line, "overflow=").is_none_or(|v| v.eq_ignore_ascii_case("YES"));
    Ok(BridgeExitSummary {
        rx_bytes: parse_u32("rx="),
        tx_bytes: parse_u32("tx="),
        ferr: parse_u32("ferr="),
        ctrl_edges: parse_u32("ctrl_edges="),
        overflow,
        raw_line: line.to_string(),
    })
}

/// Parses exact raw hex line from a `PANAMON1>` `R <va>,<len>` reply (rejecting echos/`ERR:`).
pub fn parse_panamon1_read_bytes(reply: &str, expected_len: usize) -> Option<Vec<u8>> {
    if reply.contains("ERR:") {
        return None;
    }
    if expected_len == 0 {
        return Some(Vec::new());
    }
    let hex_len = expected_len.checked_mul(2)?;
    for trimmed in reply.lines().map(str::trim) {
        let is_meta = trimmed.is_empty()
            || trimmed.starts_with("PANAMON1")
            || trimmed.starts_with("R ")
            || trimmed.starts_with("W ")
            || trimmed.starts_with("OK:")
            || trimmed.starts_with("ERR:");
        if !is_meta && trimmed.len() == hex_len && trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
            return hex::decode(trimmed).ok();
        }
    }
    None
}

/// Sends one `PANAMON1>` command terminated by `\r` over any `Read + Write` byte stream.
pub fn query_panamon1_uart<T: Read + Write + ?Sized>(
    serial: &mut T,
    cmd: &str,
    max_wait: Duration,
) -> String {
    let _ = serial.write_all(format!("{cmd}\r").as_bytes());
    let _ = serial.flush();
    let deadline = Instant::now() + max_wait;
    let mut out = String::new();
    let mut buf = [0u8; 512];
    while Instant::now() < deadline {
        if let Ok(n) = serial.read(&mut buf)
            && n > 0
        {
            out.push_str(&String::from_utf8_lossy(&buf[..n]));
            if out.contains(PANAMON1_FULL_PROMPT) {
                break;
            }
        }
    }
    out
}

/// Abstract host-to-Pico `BRIDGE` / `PANAMON1>` transport for live CDC hardware and offline mocks.
pub trait Panamon1Transport {
    fn port_label(&self) -> &str;
    fn pico_cli_exchange(
        &mut self,
        cmd: &str,
        stop_tokens: &[&str],
        timeout: Duration,
    ) -> Result<String>;
    fn escape_bridge(&mut self, timeout: Duration) -> Result<String>;
    fn target_exchange_bytes(
        &mut self,
        cmd: &str,
        pace: Duration,
        required_tokens: &[&[u8]],
        timeout: Duration,
    ) -> Result<Vec<u8>>;
    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
