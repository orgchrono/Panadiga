//! Telemetry parsers, diagnostic correlator, and Markdown report renderer for `bench_wizard`.

use crate::re::error_dict::correlate_firmware_occurrences;

use super::types::{BaudScanSummary, GuidedBenchReport, VfdObservation, WakeTraceSummary};

fn extract_u32_kv(text: &str, key: &str) -> Option<u32> {
    let (_, rhs) = text.split_once(key)?;
    let digits: String = rhs
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}

fn extract_f32_kv(text: &str, key: &str) -> Option<f32> {
    let (_, rhs) = text.split_once(key)?;
    let num: String = rhs
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num.parse::<f32>().ok()
}

/// Parses Pico `BAUDSCAN` output (`edges=... min_low_us=... snapped_baud=... framing_errs=...`).
#[must_use]
pub fn parse_baudscan_output(raw: &str) -> BaudScanSummary {
    BaudScanSummary {
        edges: extract_u32_kv(raw, "edges=").unwrap_or(0),
        min_low_us: extract_u32_kv(raw, "min_low_us=").unwrap_or(0),
        min_high_us: extract_u32_kv(raw, "min_high_us=").unwrap_or(0),
        raw_bps: extract_u32_kv(raw, "raw_bps=").unwrap_or(0),
        snapped_baud: extract_u32_kv(raw, "snapped_baud=").unwrap_or(0),
        cluster_hits: extract_u32_kv(raw, "cluster_hits=").unwrap_or(0),
        confidence_pct: extract_u32_kv(raw, "confidence=").unwrap_or(0),
        glitches: extract_u32_kv(raw, "glitches=").unwrap_or(0),
        rx_bytes: extract_u32_kv(raw, "rx_bytes=").unwrap_or(0),
        ascii_bytes: extract_u32_kv(raw, "ascii=").unwrap_or(0),
        framing_errs: extract_u32_kv(raw, "framing_errs=").unwrap_or(0),
        no_signal: raw.contains("no_signal"),
        raw_output: raw.trim().to_string(),
    }
}

/// Parses edge counts (`nsw5v_edges`, `rxd_edges`, `txd_edges`) and `VDD` min/max from `PWRTRACE`/`BOOTTRACE`.
#[must_use]
pub fn parse_wake_trace_output(raw: &str) -> WakeTraceSummary {
    let nsw5v_edges = extract_u32_kv(raw, "nsw5v_edges=").unwrap_or(0);
    let rxd_edges = extract_u32_kv(raw, "rxd_edges=").unwrap_or(0);
    let txd_edges = extract_u32_kv(raw, "txd_edges=").unwrap_or(0);
    let vdd_min_volts = extract_f32_kv(raw, "adc0_min=");
    let vdd_max_volts = extract_f32_kv(raw, "adc0_max=").or_else(|| extract_f32_kv(raw, "VDD:"));
    WakeTraceSummary {
        nsw5v_edges,
        rxd_edges,
        txd_edges,
        vdd_min_volts,
        vdd_max_volts,
        sw_uart_released: raw.contains("sw_rel=1") || raw.contains("RELEASED"),
        raw_output: raw.trim().to_string(),
    }
}

fn assess_baud_verdict(baud: Option<&BaudScanSummary>, rxd_edges: u32) -> String {
    match baud {
        Some(b) if b.no_signal || b.edges == 0 => {
            if rxd_edges == 0 {
                "SILENT GP2 (0 UARTP2M_T edges observed). Changing PuTTY/RealTerm baud has no effect until AM34 TX modulates (or verify Pin 6 vs Pin 3 wiring).".to_string()
            } else {
                format!(
                    "Wake trace saw {rxd_edges} GP2 edge(s), but follow-up BAUDSCAN window had no traffic."
                )
            }
        }
        Some(b)
            if b.framing_errs > 0
                || (b.raw_output.contains("confidence=") && b.confidence_pct < 50) =>
        {
            format!(
                "Unstable UART baud/framing on GP2: snapped={} bps (confidence {}%, framing_errs={}, glitches={}). Run `BAUD {}` + check ground reference.",
                b.snapped_baud,
                b.confidence_pct,
                b.framing_errs,
                b.glitches,
                b.snapped_baud.max(115200)
            )
        }
        Some(b) => {
            if b.raw_output.contains("confidence=") {
                format!(
                    "Locked cleanly on GP2: {} bps 8N1 (clusters={}, confidence={}%, framing_errs={}, rx_bytes={}/ascii={}).",
                    b.snapped_baud,
                    b.cluster_hits,
                    b.confidence_pct,
                    b.framing_errs,
                    b.rx_bytes,
                    b.ascii_bytes
                )
            } else {
                format!(
                    "Locked cleanly on GP2: {} bps 8N1 (edges={}, min_low={}us, raw_bps={}; legacy UF2 firmware — run `panadiga pico flash` for cluster confidence metrics).",
                    b.snapped_baud, b.edges, b.min_low_us, b.raw_bps
                )
            }
        }
        None => "BAUDSCAN was not executed in this session.".to_string(),
    }
}

/// Combines all guided step captures with `re::error_dict` into a structured diagnostic report.
#[must_use]
pub fn evaluate_guided_bench(
    port: &str,
    initial_vdd_volts: Option<f32>,
    wake_trace: Option<WakeTraceSummary>,
    baud_scan: Option<BaudScanSummary>,
    vfd_observation: VfdObservation,
    steps: Vec<super::types::GuidedStepEvidence>,
) -> GuidedBenchReport {
    let combined_uart: String = steps
        .iter()
        .map(|s| s.raw_output.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let nsw_edges = wake_trace.as_ref().map(|w| w.nsw5v_edges);
    let rxd_edges = wake_trace
        .as_ref()
        .map(|w| w.rxd_edges)
        .or_else(|| baud_scan.as_ref().map(|b| b.edges));

    let corr = correlate_firmware_occurrences(
        &combined_uart,
        Some(vfd_observation.as_cue_text()),
        initial_vdd_volts,
        nsw_edges,
        rxd_edges,
    );

    let upper_uart = combined_uart.to_ascii_uppercase();
    let has_panamon1 = upper_uart.contains("PANAMON1>");
    let has_panarsp = upper_uart.contains("_DEV_SERIAL") || upper_uart.contains("$T05");

    let (can_attach, rec_mode, gdb_verdict) = if has_panamon1 {
        (
            true,
            "panamon1".to_string(),
            format!(
                "PANAMON1> detected over async UART: launch `panadiga pico gdb-bridge --port {port} --target-mode panamon1` \
                 so Ghidra (`target remote 127.0.0.1:3333`) reads NOR/SRAM asynchronously without DWire32A clock."
            ),
        )
    } else if has_panarsp {
        (
            true,
            "panarsp".to_string(),
            format!(
                "MAIN _dev_SERIAL RSP stub online: launch `panadiga pico gdb-bridge --port {port} --target-mode panarsp` for live Ghidra RSP attach."
            ),
        )
    } else {
        (
            false,
            "none_pre_uart_or_u99".to_string(),
            "Not ready for live Ghidra RSP attach yet: P7501 has no synchronous DWire32A clock, and neither PANAMON1> nor MAIN _dev_SERIAL responded on UART.".to_string(),
        )
    };

    let root_cause_verdict = if !corr.matches.is_empty() {
        format!(
            "{} — Primary hit: {} ({})",
            vfd_observation.as_cue_text(),
            corr.matches[0].title,
            corr.matches[0].address
        )
    } else if rxd_edges.unwrap_or(0) == 0 && nsw_edges.unwrap_or(0) > 0 {
        "Digital PCB energized (NSW5V toggled), but 0 UART edges emitted before U99/dropout — check X_SW rails, PFAIL[L], SA0..SA3, SDRAM, or TBUS.".to_string()
    } else {
        corr.summary.clone()
    };

    let mut next_actions = corr.prioritized_actions.clone();
    if can_attach {
        next_actions.insert(
            0,
            format!(
                "Attach Ghidra Debugger via async proxy: panadiga pico gdb-bridge --port {port} --target-mode {rec_mode}"
            ),
        );
    } else if nsw_edges.unwrap_or(0) == 0 && initial_vdd_volts.is_some_and(|v| v <= 0.20) {
        next_actions.push(
            "Target remained in Standby P-OFF during trace window: re-run wakeup capture pressing [POWER] while BOOTTRACE/PWRTRACE is armed."
                .to_string(),
        );
    }

    let baud_verdict = assess_baud_verdict(baud_scan.as_ref(), rxd_edges.unwrap_or(0));

    GuidedBenchReport {
        port: port.to_string(),
        initial_vdd_volts,
        wake_trace,
        baud_scan,
        vfd_observation,
        root_cause_verdict,
        baud_verdict,
        ghidra_gdb_verdict: gdb_verdict,
        can_attach_ghidra_now: can_attach,
        recommended_gdb_mode: rec_mode,
        firmware_correlation: corr,
        steps,
        next_actions,
    }
}

/// Formats a human-readable Markdown bench triage report suitable for archiving in `docs/debug_logs/`.
#[must_use]
pub fn format_guided_bench_markdown(report: &GuidedBenchReport) -> String {
    let mut md = String::new();
    md.push_str("# Panadiga Guided Live Bench Report (P7501)\n\n");
    md.push_str(&format!("- **Serial Port:** `{}`\n", report.port));
    md.push_str(&format!(
        "- **Initial VDD (GP26 ADC0):** {}\n",
        report
            .initial_vdd_volts
            .map_or_else(|| "n/a".to_string(), |v| format!("{v:.2} V"))
    ));
    md.push_str(&format!(
        "- **Front Panel VFD Observation:** `{}`\n",
        report.vfd_observation.as_cue_text()
    ));
    md.push_str(&format!(
        "- **Root-Cause Verdict:** {}\n",
        report.root_cause_verdict
    ));
    md.push_str(&format!(
        "- **Baud/Framing Verdict:** {}\n",
        report.baud_verdict
    ));
    md.push_str(&format!(
        "- **Async GDB / Ghidra Verdict (`attach={} | mode={}`):** {}\n\n",
        report.can_attach_ghidra_now, report.recommended_gdb_mode, report.ghidra_gdb_verdict
    ));

    md.push_str("## Executed Steps\n\n");
    for s in &report.steps {
        md.push_str(&format!(
            "### {} — {} [{}]\n- Command: `{}`\n- Operator Note: {}\n- Summary: {}\n\n```text\n{}\n```\n\n",
            s.step_id, s.title, s.status, s.command_executed, s.operator_note, s.summary, s.raw_output
        ));
    }

    md.push_str(&format!(
        "## Firmware Dictionary Matches ({} entries indexed)\n\n",
        report.firmware_correlation.dictionary_total_entries
    ));
    if report.firmware_correlation.matches.is_empty() {
        md.push_str("_No textual error signatures matched in captured serial/VFD stream._\n\n");
    } else {
        for m in &report.firmware_correlation.matches {
            md.push_str(&format!(
                "- **`{}`** (`{}` @ `{}`) — {}\n  - Evidence: `{}`\n  - Translation: {}\n",
                m.code, m.evidence_source, m.address, m.title, m.native_text, m.translation
            ));
        }
        md.push('\n');
    }

    md.push_str("## Recommended Next Actions\n\n");
    for (idx, act) in report.next_actions.iter().enumerate() {
        md.push_str(&format!("{}. {}\n", idx + 1, act));
    }
    md
}
