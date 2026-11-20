#![deny(unsafe_code)]

//! Pico CLI safety preflight (`PING`/`STATUS`/`PINCHECK`) and `BRIDGE 115200 PASSIVE` helpers.

use std::time::Duration;

use anyhow::{Result, bail};

use super::super::bench_decoders::{BenchStatusReport, parse_status_output_report};
use super::client::Panamon1Client;
use super::protocol::{BridgeExitSummary, Panamon1Transport, parse_bridge_exit_summary};

impl<T: Panamon1Transport> Panamon1Client<T> {
    /// Escapes bridge (`~~~`), checks `PING`, `STATUS` (VDD OK & GP26 not stuck), and `PINCHECK: OK`.
    pub fn ensure_pico_cli(&mut self) -> Result<BenchStatusReport> {
        let _ = self
            .transport_mut()
            .escape_bridge(Duration::from_millis(2000))?;
        let ping_reply = self.transport_mut().pico_cli_exchange(
            "PING",
            &["PONG"],
            Duration::from_millis(2000),
        )?;
        if !ping_reply.contains("PONG") {
            bail!("Pico CLI did not answer PING with PONG: {ping_reply:?}");
        }

        let status_reply = self.transport_mut().pico_cli_exchange(
            "STATUS",
            &["VDD:"],
            Duration::from_millis(2000),
        )?;
        let status = parse_status_output_report(&status_reply);
        if status.gp26_sticky_low_suspected
            || status_reply.to_ascii_uppercase().contains("GP26 STICKY")
        {
            bail!(
                "Unsafe Pico STATUS: GP26 sticky-LOW suspected (VDD={:?}, raw={:?})",
                status.vdd_volts,
                status.raw_text.trim()
            );
        }
        if status.vdd_status != "OK" || status.vdd_volts.is_none_or(|v| v < 2.50) {
            bail!(
                "Unsafe target VDD in Pico STATUS (expected OK >= 2.50V): {:?}",
                status.raw_text.trim()
            );
        }

        let pincheck = self.transport_mut().pico_cli_exchange(
            "PINCHECK",
            &["PINCHECK: OK"],
            Duration::from_millis(2500),
        )?;
        if !pincheck.contains("PINCHECK: OK") {
            bail!(
                "PINCHECK failed (GP3/NSW5V Hi-Z safety check did not pass): {:?}",
                pincheck.trim()
            );
        }
        Ok(status)
    }

    fn enter_bridge_and_ping(&mut self) -> Result<bool> {
        let _ = self.transport_mut().pico_cli_exchange(
            "BRIDGE 115200 PASSIVE",
            &["BRIDGE"],
            Duration::from_millis(2000),
        )?;
        Ok(self.try_ping())
    }

    /// Ensures `PANAMON1>` is active over `BRIDGE 115200 PASSIVE`, optionally running `BOOTTRACE`.
    /// Returns `Ok(true)` if `BOOTTRACE` rebooted the target, or `Ok(false)` if re-attached live.
    pub fn ensure_panamon1_bridge(
        &mut self,
        boot_mode: Option<&str>,
        timeout_ms: u32,
    ) -> Result<bool> {
        if self.try_ping() {
            return Ok(false);
        }
        let _status = self.ensure_pico_cli()?;
        if self.enter_bridge_and_ping()? {
            return Ok(false);
        }

        let _ = self
            .transport_mut()
            .escape_bridge(Duration::from_millis(1500))?;
        let Some(mode) = boot_mode else {
            bail!(
                "Target did not respond at PANAMON1> over BRIDGE 115200 PASSIVE (no boot_mode configured)"
            );
        };
        let bt_cmd = format!("BOOTTRACE {timeout_ms} {mode}");
        let bt_wait = Duration::from_millis(u64::from(timeout_ms).saturating_add(4000));
        let bt_out =
            self.transport_mut()
                .pico_cli_exchange(&bt_cmd, &["BOOTTRACE: OK"], bt_wait)?;
        if !bt_out.contains("BOOTTRACE: OK") {
            bail!("BOOTTRACE command '{bt_cmd}' did not complete cleanly: {bt_out:?}");
        }
        if !self.enter_bridge_and_ping()? {
            let _ = self
                .transport_mut()
                .escape_bridge(Duration::from_millis(1500));
            bail!("Entered BRIDGE after '{bt_cmd}', but PANAMON1> did not respond to '?\\r'");
        }
        Ok(true)
    }

    /// Exits `BRIDGE` via `~~~` and fails if `BRIDGE: OK` reports framing/overflow/ctrl glitches.
    pub fn exit_bridge_checked(&mut self) -> Result<BridgeExitSummary> {
        let raw = self.exit_bridge_raw()?;
        let summary = parse_bridge_exit_summary(&raw)?;
        summary.validate_healthy()?;
        Ok(summary)
    }

    /// Exits `BRIDGE` via `~~~` returning raw CLI reply.
    pub fn exit_bridge_raw(&mut self) -> Result<String> {
        self.transport_mut()
            .escape_bridge(Duration::from_millis(2500))
    }
}
