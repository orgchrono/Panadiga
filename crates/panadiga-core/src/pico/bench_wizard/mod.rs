//! Interactive Guided Bench Suite (`bench-wizard`) combining live Pico P7501 probes,
//! glitch-immune `BAUDSCAN`, Standby wakeup tracing, and Firmware Error Dictionary correlation.

pub mod correlate;
#[cfg(test)]
mod tests;
pub mod types;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serialport::SerialPort;

use super::bench_decoders::{parse_pincheck_report, parse_status_output_report};
use super::cdc::send_and_read_until;
pub use correlate::{
    evaluate_guided_bench, format_guided_bench_markdown, parse_baudscan_output,
    parse_wake_trace_output,
};
pub use types::{
    BaudScanSummary, GuidedBenchReport, GuidedStepEvidence, VfdObservation, WakeTraceSummary,
};

/// Executes read-only Step 1 (`PING`, `STATUS`, `PINCHECK`) to validate standby rail & Hi-Z safety.
pub fn execute_baseline_safety_step(
    port: &mut dyn SerialPort,
) -> (Option<f32>, GuidedStepEvidence) {
    let ping = send_and_read_until(
        port,
        "PING",
        &["PANADIGA>", "P7501>"],
        Duration::from_millis(900),
    );
    let status = send_and_read_until(
        port,
        "STATUS",
        &["PANADIGA>", "P7501>"],
        Duration::from_millis(900),
    );
    let pincheck = send_and_read_until(
        port,
        "PINCHECK",
        &["PANADIGA>", "P7501>"],
        Duration::from_millis(2500),
    );
    let st = parse_status_output_report(&status);
    let rows = parse_pincheck_report(&pincheck);
    let summary = format!(
        "PING={}, VDD={} ({}), PINCHECK rows={}",
        if ping.contains("PONG") {
            "OK"
        } else {
            "NO_PONG"
        },
        st.vdd_volts
            .map_or_else(|| "n/a".to_string(), |v| format!("{v:.2} V")),
        st.vdd_status,
        rows.len()
    );
    let raw =
        format!("=== PING ===\n{ping}\n\n=== STATUS ===\n{status}\n\n=== PINCHECK ===\n{pincheck}");
    (
        st.vdd_volts,
        GuidedStepEvidence {
            step_id: "STEP_1_BASELINE".to_string(),
            title: "Electrical Safety & Standby Baseline (PINCHECK + STATUS)".to_string(),
            command_executed: "PING / STATUS / PINCHECK".to_string(),
            operator_note: "Passive check before energizing target".to_string(),
            status: if ping.contains("PONG") {
                "PASS"
            } else {
                "WARN"
            }
            .to_string(),
            summary,
            raw_output: raw,
        },
    )
}

/// Arms `BOOTTRACE` (passive or active `JIG`) for `window_ms` while operator wakes DVD from Standby.
pub fn execute_wake_trace_step(
    port: &mut dyn SerialPort,
    window_ms: u32,
    active_jig: bool,
    note: &str,
) -> (WakeTraceSummary, GuidedStepEvidence) {
    let mode_tok = if active_jig { "JIG" } else { "PASSIVE" };
    let cmd = format!("BOOTTRACE {window_ms} {mode_tok}");
    let raw = send_and_read_until(
        port,
        &cmd,
        &["BOOTTRACE: OK", "BOOTTRACE: ERR", "PANADIGA>", "P7501>"],
        Duration::from_millis(u64::from(window_ms) + 2500),
    );
    let parsed = parse_wake_trace_output(&raw);
    let ev = GuidedStepEvidence {
        step_id: if active_jig {
            "STEP_4_JIG_BOOT".to_string()
        } else {
            "STEP_2_WAKE_CAPTURE".to_string()
        },
        title: if active_jig {
            "Active SW_UART Serial Boot Probe (BOOTTRACE JIG)".to_string()
        } else {
            "Passive Wakeup Capture from Standby (BOOTTRACE PASSIVE)".to_string()
        },
        command_executed: cmd,
        operator_note: note.to_string(),
        status: if parsed.rxd_edges > 0 { "PASS" } else { "WARN" }.to_string(),
        summary: format!(
            "nsw5v_edges={}, rxd_edges={}, txd_edges={}",
            parsed.nsw5v_edges, parsed.rxd_edges, parsed.txd_edges
        ),
        raw_output: raw,
    };
    (parsed, ev)
}

/// Runs `BAUDSCAN` over `window_ms` (optionally triggering `NORMAL` boot) to classify baud & framing.
pub fn execute_baudscan_step(
    port: &mut dyn SerialPort,
    window_ms: u32,
    apply_snapped: bool,
    note: &str,
) -> (BaudScanSummary, GuidedStepEvidence) {
    let cmd = if apply_snapped {
        format!("BAUDSCAN {window_ms} PASSIVE APPLY")
    } else {
        format!("BAUDSCAN {window_ms} PASSIVE")
    };
    let raw = send_and_read_until(
        port,
        &cmd,
        &["BAUDSCAN: OK", "PANADIGA>", "P7501>"],
        Duration::from_millis(u64::from(window_ms) + 2000),
    );
    let parsed = parse_baudscan_output(&raw);
    let ev = GuidedStepEvidence {
        step_id: "STEP_3_BAUDSCAN".to_string(),
        title: "Dual-Polarity Cluster Baud & PIO1 Framing Check (BAUDSCAN)".to_string(),
        command_executed: cmd,
        operator_note: note.to_string(),
        status: if !parsed.no_signal && parsed.framing_errs == 0 {
            "PASS"
        } else {
            "WARN"
        }
        .to_string(),
        summary: format!(
            "edges={}, snapped={} bps (conf={}%, hits={}, framing_errs={}, glitches={})",
            parsed.edges,
            parsed.snapped_baud,
            parsed.confidence_pct,
            parsed.cluster_hits,
            parsed.framing_errs,
            parsed.glitches
        ),
        raw_output: raw,
    };
    (parsed, ev)
}

/// Saves Markdown and JSON reports using a non-destructive Copy-on-Write timestamped filename.
pub fn write_guided_bench_reports(
    out_dir: &Path,
    report: &GuidedBenchReport,
) -> Result<(PathBuf, PathBuf)> {
    fs::create_dir_all(out_dir)?;
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let md_path = out_dir.join(format!("bench_wizard_{epoch}.md"));
    let json_path = out_dir.join(format!("bench_wizard_{epoch}.json"));
    fs::write(&md_path, format_guided_bench_markdown(report))?;
    fs::write(&json_path, serde_json::to_string_pretty(report)?)?;
    Ok((md_path, json_path))
}
