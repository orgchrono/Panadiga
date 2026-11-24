#![deny(unsafe_code)]

//! Pico CLI safety preflight (`PING`/`STATUS`/`PINCHECK`) and `BRIDGE 115200 PASSIVE` helpers.

use std::time::Duration;

use anyhow::{Result, bail};

use super::super::bench_decoders::{
    BenchStatusReport, evaluate_live_rail_gate, parse_status_output_report,
};
use super::client::Panamon1Client;
use super::protocol::{BridgeExitSummary, Panamon1Transport, parse_bridge_exit_summary};

impl<T: Panamon1Transport> Panamon1Client<T> {
    fn send_isolate_best_effort(&mut self) {
        let _ = self.transport_mut().pico_cli_exchange(
            "ISOLATE",
            &["ISOLATE: OK", "@P7501_RES cmd=ISOLATE"],
            Duration::from_millis(1000),
        );
    }

    /// Escapes bridge (`~~~`), checks `PING`, `STATUS` (`NSW5V=1`, `fault_latch=0`, `VDD >= 2.50V`),
    /// and `PINCHECK: OK`. On any rail/standby/safety failure, immediately issues `ISOLATE`.
    pub fn ensure_pico_cli(&mut self) -> Result<BenchStatusReport> {
        let _ = self
            .transport_mut()
            .escape_bridge(Duration::from_millis(2000))?;
        self.bridge_entered = false;
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
            &["@P7501_RES cmd=STATUS", "VDD:"],
            Duration::from_millis(2000),
        )?;
        let status = parse_status_output_report(&status_reply);
        if let Err(err) = evaluate_live_rail_gate(&status) {
            self.send_isolate_best_effort();
            return Err(err);
        }

        let pincheck = self.transport_mut().pico_cli_exchange(
            "PINCHECK PASSIVE",
            &["PINCHECK: OK"],
            Duration::from_millis(2500),
        )?;
        if !pincheck.contains("PINCHECK: OK") {
            self.send_isolate_best_effort();
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
        self.bridge_entered = true;
        Ok(self.try_ping())
    }

    /// Ensures `PANAMON1>` is active over `BRIDGE 115200 PASSIVE` AFTER mandatory `STATUS` gate,
    /// optionally running `BOOTTRACE`. Returns `Ok(true)` if `BOOTTRACE` rebooted, `Ok(false)` otherwise.
    pub fn ensure_panamon1_bridge(
        &mut self,
        boot_mode: Option<&str>,
        timeout_ms: u32,
    ) -> Result<bool> {
        let _status = self.ensure_pico_cli()?;
        if self.enter_bridge_and_ping()? {
            return Ok(false);
        }

        let _ = self.exit_bridge_raw();
        let Some(mode) = boot_mode else {
            self.send_isolate_best_effort();
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
            self.send_isolate_best_effort();
            bail!("BOOTTRACE command '{bt_cmd}' did not complete cleanly: {bt_out:?}");
        }
        if !self.enter_bridge_and_ping()? {
            let _ = self.exit_bridge_raw();
            self.send_isolate_best_effort();
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
        self.bridge_entered = false;
        self.transport_mut()
            .escape_bridge(Duration::from_millis(2500))
    }

    /// Cleanly closes any active `BRIDGE` session (validating telemetry) and isolates `P7501` in Hi-Z.
    pub fn close_bridge_and_isolate(&mut self) -> Result<Option<BridgeExitSummary>> {
        let summary = if self.bridge_entered {
            Some(self.exit_bridge_checked()?)
        } else {
            None
        };
        let iso = self.transport_mut().pico_cli_exchange(
            "ISOLATE",
            &["ISOLATE: OK", "@P7501_RES cmd=ISOLATE"],
            Duration::from_millis(1500),
        )?;
        if !iso.contains("ISOLATE: OK") && !iso.contains("@P7501_RES cmd=ISOLATE status=OK") {
            bail!("Pico ISOLATE did not confirm cleanly: {iso:?}");
        }
        Ok(summary)
    }
}
