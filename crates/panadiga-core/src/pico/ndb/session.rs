#![deny(unsafe_code)]

//! Transactional volatile SDRAM/MMIO session with hot-diffing, instant reverse-order rollback,
//! and live `PicoFW` `BRIDGE` transport over `STANDBY_GATE_STOP`.

use std::time::Duration;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serialport::SerialPort;

use crate::constants::DEFAULT_BAUD_RATE;
use crate::pico::bench_decoders::{evaluate_live_rail_gate, parse_status_output_report};
use crate::pico::cdc::{
    PICO_ISOLATE_STOP_TOKENS, PICO_PROMPT_STOP_TOKENS, PICO_STATUS_STOP_TOKENS, open_cdc,
    send_and_read_until,
};

use super::parser::{
    NdbAccessKind, NdbDdResult, VolatileDiffEntry, detect_ndb_bcu_fault, parse_ndb_dd_output,
    parse_ndb_write_echo, strip_ndb_ansi_prompt, validate_ndb_access,
};

/// Granularity of an `ndb>` volatile write operation (`eb` byte sequence vs `ew` halfword).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolatilePatchWidth {
    Bytes,
    U16,
}

/// Declarative volatile SDRAM/MMIO write request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolatilePatchOp {
    pub addr: u32,
    pub width: VolatilePatchWidth,
    pub bytes: Vec<u8>,
}

impl VolatilePatchOp {
    /// Creates an `eb` byte-sequence volatile patch starting at `addr`.
    #[must_use]
    pub fn write_bytes(addr: u32, bytes: Vec<u8>) -> Self {
        Self {
            addr,
            width: VolatilePatchWidth::Bytes,
            bytes,
        }
    }

    /// Creates a 16-bit aligned `ew` little-endian volatile patch at `addr`.
    #[must_use]
    pub fn write_u16(addr: u32, val: u16) -> Self {
        Self {
            addr,
            width: VolatilePatchWidth::U16,
            bytes: val.to_le_bytes().to_vec(),
        }
    }
}

/// Aggregated hot-diff report across one transactional batch of volatile patches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolatileDiffReport {
    pub changes: Vec<VolatileDiffEntry>,
    pub before_snapshots: Vec<NdbDdResult>,
    pub after_snapshots: Vec<NdbDdResult>,
}

/// Result summary emitted by [`NdbSession::rollback`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackReport {
    pub restored_ops: usize,
    pub restored_bytes: usize,
    pub verified_clean: bool,
}

#[derive(Debug, Clone)]
struct RecordedPatchUndo {
    op: VolatilePatchOp,
    before_bytes: Vec<u8>,
}

/// Abstract `ndb>` command transport (allows deterministic unit/E2E mock transports and live CDC serial).
pub trait NdbTransport {
    fn execute_command(&mut self, cmd: &str) -> Result<String>;
}

/// Fail-closed live serial `ndb>` transport operating inside PicoFW `BRIDGE` mode under `STANDBY_GATE_STOP`.
pub struct SerialNdbTransport {
    port: Box<dyn SerialPort>,
}

impl SerialNdbTransport {
    /// Checks `STATUS` first (aborts + `ISOLATE` on `STANDBY_GATE_STOP`) and enters `BRIDGE` toward AM34 `SC0`.
    pub fn connect_and_enter_bridge(port_name: &str) -> Result<Self> {
        let mut port = open_cdc(port_name, DEFAULT_BAUD_RATE, Duration::from_millis(800))?;
        let status_out = send_and_read_until(
            port.as_mut(),
            "STATUS",
            PICO_STATUS_STOP_TOKENS,
            Duration::from_millis(2500),
        );
        let parsed = parse_status_output_report(&status_out);
        if let Err(err) = evaluate_live_rail_gate(&parsed) {
            let _ = send_and_read_until(
                port.as_mut(),
                "ISOLATE",
                PICO_ISOLATE_STOP_TOKENS,
                Duration::from_millis(1200),
            );
            return Err(err);
        }

        let bridge_banner = send_and_read_until(
            port.as_mut(),
            "BRIDGE 115200",
            &["[BRIDGE]", "ndb>"],
            Duration::from_millis(2500),
        );
        if !bridge_banner.contains("[BRIDGE]") && !bridge_banner.contains("ndb>") {
            let _ = port.write_all(b"~~~\r\nISOLATE\r\n");
            let _ = port.flush();
            bail!("Failed to enter PicoFW BRIDGE mode cleanly: {bridge_banner}");
        }
        Ok(Self { port })
    }

    /// Leaves `BRIDGE` via `~~~` and returns target signals cleanly to High-Z `ISOLATE`.
    pub fn close_safely(&mut self) -> Result<()> {
        let _ = send_and_read_until(
            self.port.as_mut(),
            "~~~",
            PICO_PROMPT_STOP_TOKENS,
            Duration::from_millis(1500),
        );
        let _ = send_and_read_until(
            self.port.as_mut(),
            "ISOLATE",
            PICO_ISOLATE_STOP_TOKENS,
            Duration::from_millis(1500),
        );
        Ok(())
    }
}

impl Drop for SerialNdbTransport {
    fn drop(&mut self) {
        let _ = self.close_safely();
    }
}

fn read_ndb_until_tokens(
    port: &mut dyn SerialPort,
    tokens: &[&str],
    max_wait: Duration,
) -> Result<String> {
    Ok(crate::pico::cdc::read_port_until(port, max_wait, |acc| {
        tokens.iter().any(|tok| acc.contains(tok))
    }))
}

impl NdbTransport for SerialNdbTransport {
    fn execute_command(&mut self, cmd: &str) -> Result<String> {
        self.port.write_all(format!("{cmd}\r").as_bytes())?;
        self.port.flush()?;
        read_ndb_until_tokens(
            self.port.as_mut(),
            &["ndb>", "Bus error inf"],
            Duration::from_millis(2500),
        )
    }
}

/// Stateful `ndb>` Silicon Explorer session with fail-closed address barriers and transactional rollback.
pub struct NdbSession<T: NdbTransport> {
    transport: T,
    history: Vec<RecordedPatchUndo>,
}

impl<T: NdbTransport> NdbSession<T> {
    /// Wraps an `ndb>` transport into a fail-closed Silicon Explorer session.
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            history: Vec::new(),
        }
    }

    /// Returns the underlying transport (consuming the session).
    #[must_use]
    pub fn into_transport(self) -> T {
        self.transport
    }

    /// Returns the number of recorded reversible patch operations in the session journal.
    #[must_use]
    pub fn pending_rollback_count(&self) -> usize {
        self.history.len()
    }

    /// Reads `byte_len` bytes via `ndb> dd <addr> <byte_len>`, enforcing barriers and BCU fault abort.
    pub fn read_words(&mut self, addr: u32, byte_len: usize) -> Result<NdbDdResult> {
        let eff_len = byte_len.max(4);
        validate_ndb_access(addr, eff_len, NdbAccessKind::Read)?;
        let cmd = format!("dd {addr:08x} {eff_len}");
        let raw = self.transport.execute_command(&cmd)?;
        if let Some(fault) = detect_ndb_bcu_fault(&raw) {
            bail!(
                "BCU_FAULT_ABORT: BCBERR={} at 0x{:08X} ({} PiE-OS tasks dumped)",
                fault.bcberr_hex,
                fault.fault_address,
                fault.task_dump_count
            );
        }
        parse_ndb_dd_output(&raw)
    }

    fn read_exact_span(&mut self, addr: u32, byte_len: usize) -> Result<(NdbDdResult, Vec<u8>)> {
        let aligned_base = addr & !3u32;
        let head_offset = (addr - aligned_base) as usize;
        let read_len = (head_offset + byte_len).div_ceil(4) * 4;
        let dd = self.read_words(aligned_base, read_len.max(4))?;
        let slice = dd
            .bytes
            .get(head_offset..head_offset + byte_len)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "ndb> dd returned {} bytes at 0x{:08X}, needed offset +{} len {}",
                    dd.bytes.len(),
                    aligned_base,
                    head_offset,
                    byte_len
                )
            })?
            .to_vec();
        Ok((dd, slice))
    }

    /// Applies volatile SDRAM/MMIO patches transactionally, verifying pre/post `dd` readback and echoes.
    pub fn apply_patches_with_diff(
        &mut self,
        ops: &[VolatilePatchOp],
    ) -> Result<VolatileDiffReport> {
        if ops.is_empty() {
            bail!("No volatile patch operations specified");
        }
        // Fail closed upfront on EVERY address before touching the wire.
        for op in ops {
            if op.width == VolatilePatchWidth::U16 && op.bytes.len() != 2 {
                bail!(
                    "U16 volatile patch at 0x{:08X} requires exactly 2 bytes",
                    op.addr
                );
            }
            validate_ndb_access(op.addr, op.bytes.len(), NdbAccessKind::Write)?;
        }

        let mut changes = Vec::new();
        let mut before_snapshots = Vec::new();
        let mut after_snapshots = Vec::new();

        for op in ops {
            let (before_dd, before_exact) = self.read_exact_span(op.addr, op.bytes.len())?;
            before_snapshots.push(before_dd);

            match op.width {
                VolatilePatchWidth::U16 => {
                    let new_u16 = u16::from_le_bytes([op.bytes[0], op.bytes[1]]);
                    let cmd = format!("ew {:08x} {:04x}", op.addr, new_u16);
                    let raw = self.transport.execute_command(&cmd)?;
                    if let Some(fault) = detect_ndb_bcu_fault(&raw) {
                        bail!("BCU_FAULT_ABORT during ew: {}", fault.bcberr_hex);
                    }
                    let echo = parse_ndb_write_echo(&strip_ndb_ansi_prompt(&raw))?;
                    if echo.addr != op.addr || echo.new_value != u32::from(new_u16) {
                        bail!(
                            "ew echo mismatch at 0x{:08X}: got 0x{:08X} -> 0x{:04X}",
                            echo.addr,
                            echo.old_value,
                            echo.new_value
                        );
                    }
                }
                VolatilePatchWidth::Bytes => {
                    for (idx, &byte_val) in op.bytes.iter().enumerate() {
                        let target_addr = op.addr + (idx as u32);
                        let cmd = format!("eb {target_addr:08x} {byte_val:02x}");
                        let raw = self.transport.execute_command(&cmd)?;
                        if let Some(fault) = detect_ndb_bcu_fault(&raw) {
                            bail!("BCU_FAULT_ABORT during eb: {}", fault.bcberr_hex);
                        }
                        let echo = parse_ndb_write_echo(&strip_ndb_ansi_prompt(&raw))?;
                        if echo.addr != target_addr || echo.new_value != u32::from(byte_val) {
                            bail!(
                                "eb echo mismatch at 0x{:08X}: got 0x{:02X} -> 0x{:02X}",
                                echo.addr,
                                echo.old_value,
                                echo.new_value
                            );
                        }
                    }
                }
            }

            let (after_dd, after_exact) = self.read_exact_span(op.addr, op.bytes.len())?;
            if after_exact != op.bytes {
                bail!(
                    "Post-write verification mismatch at 0x{:08X}: expected {:02X?}, read {:02X?}",
                    op.addr,
                    op.bytes,
                    after_exact
                );
            }
            after_snapshots.push(after_dd);

            for (i, (&b, &a)) in before_exact.iter().zip(after_exact.iter()).enumerate() {
                if b != a {
                    changes.push(VolatileDiffEntry {
                        addr: op.addr + (i as u32),
                        before: b,
                        after: a,
                    });
                }
            }

            self.history.push(RecordedPatchUndo {
                op: op.clone(),
                before_bytes: before_exact,
            });
        }

        Ok(VolatileDiffReport {
            changes,
            before_snapshots,
            after_snapshots,
        })
    }

    /// Reverts every recorded volatile mutation in strict reverse operation & reverse byte order,
    /// then verifies clean pre-patch readback via `dd`.
    pub fn rollback(&mut self) -> Result<RollbackReport> {
        let recorded = std::mem::take(&mut self.history);
        if recorded.is_empty() {
            bail!("No volatile patches recorded to rollback");
        }

        let mut restored_bytes = 0usize;
        for entry in recorded.iter().rev() {
            validate_ndb_access(
                entry.op.addr,
                entry.before_bytes.len(),
                NdbAccessKind::Write,
            )?;
            match entry.op.width {
                VolatilePatchWidth::U16 => {
                    let old_u16 =
                        u16::from_le_bytes([entry.before_bytes[0], entry.before_bytes[1]]);
                    let cmd = format!("ew {:08x} {:04x}", entry.op.addr, old_u16);
                    let raw = self.transport.execute_command(&cmd)?;
                    let echo = parse_ndb_write_echo(&raw)?;
                    if echo.addr != entry.op.addr || echo.new_value != u32::from(old_u16) {
                        bail!("Rollback ew mismatch at 0x{:08X}", entry.op.addr);
                    }
                    restored_bytes += 2;
                }
                VolatilePatchWidth::Bytes => {
                    for idx in (0..entry.before_bytes.len()).rev() {
                        let target_addr = entry.op.addr + (idx as u32);
                        let orig = entry.before_bytes[idx];
                        let cmd = format!("eb {target_addr:08x} {orig:02x}");
                        let raw = self.transport.execute_command(&cmd)?;
                        let echo = parse_ndb_write_echo(&raw)?;
                        if echo.addr != target_addr || echo.new_value != u32::from(orig) {
                            bail!("Rollback eb mismatch at 0x{target_addr:08X}");
                        }
                        restored_bytes += 1;
                    }
                }
            }
            let (_dd, post_rollback) =
                self.read_exact_span(entry.op.addr, entry.before_bytes.len())?;
            if post_rollback != entry.before_bytes {
                bail!(
                    "Post-rollback verification failed at 0x{:08X}: expected {:02X?}, got {:02X?}",
                    entry.op.addr,
                    entry.before_bytes,
                    post_rollback
                );
            }
        }

        Ok(RollbackReport {
            restored_ops: recorded.len(),
            restored_bytes,
            verified_clean: true,
        })
    }
}
