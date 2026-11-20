#![deny(unsafe_code)]

//! High-level `PANAMON1>` target memory, burst write, and `J` jump client.

use std::time::Duration;

use anyhow::{Result, bail};

use super::super::rsp_codec::policy::{NorWritePolicy, is_nor_write_blocked_with_policy};
use super::protocol::{
    PANAMON1_FULL_PROMPT, PANAMON1_POST_WRITE_JUMP_DELAY_MS, PANAMON1_READ_CHUNK_BYTES,
    PANAMON1_RETRY_PACING_US, Panamon1Transport, parse_panamon1_read_bytes,
};

/// High-level `PANAMON1>` host client with retry, safety preflight, and RAII Upper NOR lock.
pub struct Panamon1Client<T: Panamon1Transport> {
    transport: T,
    needs_post_write_jump_settle: bool,
}

impl<T: Panamon1Transport> Panamon1Client<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            needs_post_write_jump_settle: false,
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    pub fn sleep(&mut self, duration: Duration) {
        self.transport.sleep(duration);
    }

    /// Safe `PANAMON1>` liveness probe sending `?\r` (never bare `\r`).
    pub fn ping(&mut self) -> Result<()> {
        let raw = self.transport.target_exchange_bytes(
            "?",
            Duration::ZERO,
            &[PANAMON1_FULL_PROMPT.as_bytes()],
            Duration::from_millis(450),
        )?;
        if String::from_utf8_lossy(&raw).contains(PANAMON1_FULL_PROMPT) {
            Ok(())
        } else {
            bail!("PANAMON1> prompt did not reply to '?\\r'")
        }
    }

    pub fn try_ping(&mut self) -> bool {
        self.ping().is_ok()
    }

    pub(super) fn exec_target_cmd(
        &mut self,
        cmd: &str,
        required_tokens: &[&str],
        timeout: Duration,
    ) -> Result<String> {
        let retry_pace = Duration::from_micros(PANAMON1_RETRY_PACING_US);
        for attempt in 0..2 {
            let pace = if attempt == 0 {
                Duration::ZERO
            } else {
                self.transport.sleep(retry_pace);
                retry_pace
            };
            let raw = self.transport.target_exchange_bytes(
                cmd,
                pace,
                &[PANAMON1_FULL_PROMPT.as_bytes()],
                timeout,
            )?;
            let text = String::from_utf8_lossy(&raw).into_owned();
            if text.contains("ERR:E05") {
                bail!("Hardware Safety Lockout (E05) from PANAMON1 after '{cmd}': {text:?}");
            }
            let has_syntax_err =
                text.contains("\r\n?\r\n") || text.lines().any(|line| line.trim() == "?");
            let missing_prompt = !text.contains(PANAMON1_FULL_PROMPT);
            let missing_req = required_tokens.iter().any(|tok| !text.contains(tok));
            if !has_syntax_err && !missing_prompt && !missing_req {
                return Ok(text);
            }
            if attempt == 1 {
                if has_syntax_err {
                    bail!("PANAMON1 syntax '?' persisted after retry for '{cmd}': {text:?}");
                }
                if missing_prompt {
                    bail!("PANAMON1 timeout waiting for prompt after '{cmd}': {text:?}");
                }
                bail!(
                    "PANAMON1 missing expected tokens {required_tokens:?} after '{cmd}': {text:?}"
                );
            }
        }
        unreachable!()
    }

    /// Reads `len` bytes from AM34 VA `addr` in 64-byte `R` chunks.
    pub fn mem_read_bytes(&mut self, addr: u32, len: usize) -> Result<Vec<u8>> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let mut out = Vec::with_capacity(len);
        let mut offset = 0usize;
        while offset < len {
            let chunk = (len - offset).min(PANAMON1_READ_CHUNK_BYTES);
            let cmd = format!("R {:08X},{chunk:X}", addr.wrapping_add(offset as u32));
            let reply = self.exec_target_cmd(&cmd, &[], Duration::from_millis(3000))?;
            let parsed = parse_panamon1_read_bytes(&reply, chunk).ok_or_else(|| {
                anyhow::anyhow!("Failed to parse {chunk} hex bytes from '{cmd}': {reply:?}")
            })?;
            out.extend_from_slice(&parsed);
            offset += chunk;
        }
        Ok(out)
    }

    /// Reads one little-endian 32-bit word from AM34 VA `addr`.
    pub fn mem_read_u32(&mut self, addr: u32) -> Result<u32> {
        let bytes = self.mem_read_bytes(addr, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(super) fn mem_write_bytes_burst_with_policy(
        &mut self,
        addr: u32,
        data: &[u8],
        policy: NorWritePolicy,
    ) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        if is_nor_write_blocked_with_policy(addr, data.len(), policy) {
            bail!(
                "Hardware Safety Lockout (E05): refusing PANAMON1 write to protected NOR 0x{addr:08X} (+{} B)",
                data.len()
            );
        }
        for (idx, &byte) in data.iter().enumerate() {
            let cmd = format!("W {:08X},{byte:02X}", addr.wrapping_add(idx as u32));
            self.exec_target_cmd(&cmd, &["OK:WRITE"], Duration::from_millis(3000))?;
        }
        self.needs_post_write_jump_settle = true;
        Ok(())
    }

    /// Burst-writes `data` to `addr` (`W <va>,<bb>` with single `300 us` retry).
    /// Blocks all IPL/Upper NOR (`0x8000_0000..0x807E_FFFF` / `0x84..`) with `E05` unless unlocked via guard.
    pub fn mem_write_bytes_burst(&mut self, addr: u32, data: &[u8]) -> Result<()> {
        self.mem_write_bytes_burst_with_policy(addr, data, NorWritePolicy::NvramOnly)
    }

    fn settle_before_jump(&mut self) {
        if self.needs_post_write_jump_settle {
            self.transport
                .sleep(Duration::from_millis(PANAMON1_POST_WRITE_JUMP_DELAY_MS));
            self.needs_post_write_jump_settle = false;
        }
    }

    /// Executes `J <va>` after settling any pending SDRAM burst write, waiting for `OK:DONE`.
    pub fn jump(&mut self, va: u32) -> Result<()> {
        self.settle_before_jump();
        let _ = self.exec_target_cmd(
            &format!("J {va:08X}"),
            &["OK:DONE"],
            Duration::from_millis(12_000),
        )?;
        Ok(())
    }

    /// Executes `J <va>` and returns raw unlossy UART bytes up to `required_tokens`.
    pub fn jump_and_capture_raw(
        &mut self,
        va: u32,
        required_tokens: &[&[u8]],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        self.settle_before_jump();
        self.transport.target_exchange_bytes(
            &format!("J {va:08X}"),
            Duration::from_millis(3),
            required_tokens,
            timeout,
        )
    }
}
