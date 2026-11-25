#![deny(unsafe_code)]

//! Shared `PANAMON1>` UART0 (`SC0`) host protocol over Pico `BRIDGE 115200 PASSIVE`.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serialport::{ClearBuffer, SerialPort};

use super::cdc::{open_cdc, send_and_read_until, strip_leading_prompts};

pub mod client;
pub mod mock;
mod mock_nor;
pub mod nor_guard;
mod preflight;
pub mod protocol;

#[cfg(test)]
mod tests;

pub use client::Panamon1Client;
pub use mock::MockPanamon1Transport;
pub use nor_guard::Panamon1NorUnlockGuard;
pub use protocol::{
    BridgeExitSummary, PANAMON1_FULL_PROMPT, PANAMON1_POST_WRITE_JUMP_DELAY_MS,
    PANAMON1_READ_CHUNK_BYTES, PANAMON1_RETRY_PACING_US, Panamon1Transport,
    parse_bridge_exit_summary, parse_panamon1_read_bytes, query_panamon1_uart,
};

/// Production USB-CDC serial transport reusing [`crate::pico::cdc`].
pub struct SerialPanamon1Transport {
    port_name: String,
    port: Box<dyn SerialPort>,
}

impl SerialPanamon1Transport {
    /// Opens `port_name` via [`open_cdc`] asserting DTR/RTS and flushing stale boot lines.
    pub fn open(port_name: &str, baud_rate: u32, timeout: Duration) -> Result<Self> {
        let port = open_cdc(port_name, baud_rate, timeout)
            .with_context(|| format!("Failed to open Pico CDC port '{port_name}'"))?;
        Ok(Self::from_port(port_name, port))
    }

    /// Wraps an already-opened [`SerialPort`] handle without touching OS handles.
    pub fn from_port(port_name: impl Into<String>, port: Box<dyn SerialPort>) -> Self {
        Self {
            port_name: port_name.into(),
            port,
        }
    }
}

fn update_found_tokens(buf: &[u8], tokens: &[&[u8]], found: &mut [bool], prev_len: usize) -> bool {
    for (idx, tok) in tokens.iter().enumerate() {
        if found[idx] {
            continue;
        }
        if tok.is_empty()
            || buf[prev_len.saturating_sub(tok.len())..]
                .windows(tok.len())
                .any(|w| w == *tok)
        {
            found[idx] = true;
        }
    }
    found.iter().all(|&ok| ok)
}

impl Panamon1Transport for SerialPanamon1Transport {
    fn port_label(&self) -> &str {
        &self.port_name
    }

    fn pico_cli_exchange(
        &mut self,
        cmd: &str,
        stop_tokens: &[&str],
        timeout: Duration,
    ) -> Result<String> {
        Ok(send_and_read_until(
            &mut *self.port,
            cmd,
            stop_tokens,
            timeout,
        ))
    }

    fn escape_bridge(&mut self, timeout: Duration) -> Result<String> {
        std::thread::sleep(Duration::from_millis(60));
        let _ = self.port.clear(ClearBuffer::Input);
        self.port.write_all(b"~~~")?;
        self.port.flush()?;
        let _ = self.port.set_timeout(Duration::from_millis(80));

        let total_deadline = Instant::now() + timeout;
        let tilde_deadline = total_deadline.min(Instant::now() + Duration::from_millis(350));
        let mut out = String::new();
        let mut buf = [0u8; 1024];

        while Instant::now() < total_deadline {
            if super::cdc::contains_pico_prompt(&out) {
                break;
            }
            if !out.contains("BRIDGE: OK") && Instant::now() >= tilde_deadline {
                break;
            }
            if let Ok(n) = self.port.read(&mut buf)
                && n > 0
            {
                out.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        }

        if !out.contains("BRIDGE: OK") && !super::cdc::contains_pico_prompt(&out) {
            self.port.write_all(b"\x03")?;
            self.port.flush()?;
            let ctrl_c_deadline = total_deadline.max(Instant::now() + Duration::from_millis(400));
            while Instant::now() < ctrl_c_deadline {
                if let Ok(n) = self.port.read(&mut buf)
                    && n > 0
                {
                    out.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if super::cdc::contains_pico_prompt(&out) {
                        break;
                    }
                }
            }
        }

        Ok(strip_leading_prompts(&out).trim().to_string())
    }

    fn target_exchange_bytes(
        &mut self,
        cmd: &str,
        pace: Duration,
        required_tokens: &[&[u8]],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let _ = self.port.clear(ClearBuffer::Input);
        let payload = format!("{cmd}\r");
        if pace.is_zero() {
            self.port.write_all(payload.as_bytes())?;
            self.port.flush()?;
        } else {
            for b in payload.as_bytes() {
                self.port.write_all(std::slice::from_ref(b))?;
                self.port.flush()?;
                std::thread::sleep(pace);
            }
        }
        let default_tok = [PANAMON1_FULL_PROMPT.as_bytes()];
        let tokens = if required_tokens.is_empty() {
            &default_tok[..]
        } else {
            required_tokens
        };
        let deadline = Instant::now() + timeout;
        let mut out = Vec::with_capacity(256);
        let mut found = vec![false; tokens.len()];
        let mut buf = [0u8; 4096];
        while Instant::now() < deadline {
            if let Ok(n) = self.port.read(&mut buf)
                && n > 0
            {
                let prev = out.len();
                out.extend_from_slice(&buf[..n]);
                if update_found_tokens(&out, tokens, &mut found, prev) {
                    break;
                }
            }
        }
        Ok(out)
    }
}
