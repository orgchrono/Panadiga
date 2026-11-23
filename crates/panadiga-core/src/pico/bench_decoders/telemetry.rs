//! Command allowlist guard and telemetry parsers for `STATUS`, `PINCHECK`, `BUSDIAG`, `PIOLOGIC`, `PANARSP`.

use anyhow::{Result, bail};

use super::super::cdc::parse_vdd_reply;
use super::super::diag_rows::{parse_pincheck_rows, parse_voltage_cell};
use super::types::{
    BenchStatusReport, BusDiagBenchReport, FORBIDDEN_WRITE_TOKENS, PanarspBenchReport,
    PinCheckBenchRow, PiologicBenchReport, SAFE_BENCH_COMMANDS,
};

fn normalize_single_line_command(command: &str) -> Result<&str> {
    let normalized = command.trim();
    if normalized.is_empty() {
        bail!("Empty bench command is not permitted");
    }
    if normalized.len() > crate::ssot::pico_header::P7501_CLI_LINE_MAX_SSOT as usize {
        bail!(
            "Command length {} exceeds P7501 CLI limit ({})",
            normalized.len(),
            crate::ssot::pico_header::P7501_CLI_LINE_MAX_SSOT
        );
    }
    if normalized.bytes().any(|b| !(0x20..=0x7E).contains(&b)) {
        bail!("Only single-line printable ASCII commands are permitted on P7501 serial CDC");
    }
    Ok(normalized)
}

/// Validates that `command` belongs strictly to the read-only P7501 bench allowlist.
pub fn validate_safe_bench_command(command: &str) -> Result<String> {
    let normalized = normalize_single_line_command(command)?;
    let head = normalized
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if !SAFE_BENCH_COMMANDS.contains(&head.as_str()) {
        bail!("Command '{head}' is outside SAFE_BENCH_COMMANDS");
    }
    let upper_full = normalized.to_ascii_uppercase();
    for tok in FORBIDDEN_WRITE_TOKENS {
        if upper_full.contains(tok) {
            bail!("Forbidden mutating token '{tok}' in bench command");
        }
    }
    Ok(normalized.to_string())
}

fn is_ipl_sa0_sa10_token(token: &str) -> bool {
    let stripped = token
        .trim_end_matches([',', ';'])
        .trim_start_matches("0X")
        .trim_start_matches('$');
    if stripped.len() < 4 || !stripped.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    if let Ok(addr) = u32::from_str_radix(stripped, 16)
        && ((0x8000_0000..0x8004_0000).contains(&addr)
            || (0x8400_0000..0x8404_0000).contains(&addr))
    {
        return true;
    }
    stripped.starts_with("8000")
        || stripped.starts_with("8001")
        || stripped.starts_with("8002")
        || stripped.starts_with("8003")
        || stripped.starts_with("8400")
        || stripped.starts_with("8401")
        || stripped.starts_with("8402")
        || stripped.starts_with("8403")
}

/// Validates an interactive Pico CDC serial command against the SSOT catalog and hardware barriers.
pub fn validate_interactive_pico_command(command: &str) -> Result<String> {
    let normalized = normalize_single_line_command(command)?;
    let upper = normalized.to_ascii_uppercase();
    let head = upper.split_whitespace().next().unwrap_or_default();
    if !crate::ssot::spec::check_known_pico_command_id(head) {
        bail!("Command '{head}' is outside SSOT Pico CLI catalog");
    }
    let compact: String = upper.chars().filter(|c| *c != '_').collect();
    if compact.contains("C000100C") {
        bail!("Lethal AM34 MMIO address 0xC000100C is permanently blocked");
    }
    if matches!(head, "FAST_WRITE" | "ERASE" | "WRITEMEM") {
        if head == "FAST_WRITE" && !upper.split_whitespace().any(|t| t == "CONFIRM_UPPER_NOR") {
            bail!("FAST_WRITE requires explicit CONFIRM_UPPER_NOR token");
        }
        if upper.split_whitespace().skip(1).any(is_ipl_sa0_sa10_token) {
            bail!("IPL SA0..SA10 write target is permanently Read-Only (E05)");
        }
    }
    Ok(upper)
}

/// Parses `STATUS` or `VDD` output into voltage, rail state, and sticky-LOW detection.
pub fn parse_status_output_report(text: &str) -> BenchStatusReport {
    let vdd_volts = parse_vdd_reply(text);
    let vdd_status = text
        .split("VDD:")
        .nth(1)
        .and_then(|after| after.split_once('('))
        .and_then(|(_, rhs)| rhs.split_once(')'))
        .map(|(st, _)| st.trim().to_ascii_uppercase())
        .filter(|st| !st.is_empty())
        .unwrap_or_else(|| "UNKNOWN".to_string());
    let gp26_sticky_low_suspected = vdd_volts.is_some_and(|v| v <= 0.10);
    BenchStatusReport {
        vdd_volts,
        vdd_status,
        gp26_sticky_low_suspected,
        raw_text: text.to_string(),
    }
}

/// Converts `diag_rows::parse_pincheck_rows` into numeric voltage matrix rows.
pub fn parse_pincheck_report(text: &str) -> Vec<PinCheckBenchRow> {
    let (rows, _) = parse_pincheck_rows(text);
    rows.into_iter()
        .map(|r| PinCheckBenchRow {
            pin_label: r.pin_label,
            hiz_volts: parse_voltage_cell(&r.hiz),
            pull_down_volts: parse_voltage_cell(&r.pull_down),
            pull_up_volts: parse_voltage_cell(&r.pull_up),
            electrical_state: r.state,
        })
        .collect()
}

fn extract_leading_u32(slice: &str) -> Option<u32> {
    let digits: String = slice
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}

fn extract_gpio_transitions(text: &str, gpio: &str) -> Option<u32> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(gpio)
            && let Some((_, rhs)) = trimmed.split_once(':')
            && let Some((left, _)) = rhs.split_once("trans")
        {
            return extract_leading_u32(left);
        }
    }
    None
}

/// Extracts GP2/GP3/GP4 transition counts and `XRESET` rise time from `BUSDIAG` output.
pub fn parse_busdiag_report(text: &str) -> BusDiagBenchReport {
    let gp2 = extract_gpio_transitions(text, "GP2");
    let gp3 = extract_gpio_transitions(text, "GP3");
    let gp4 = extract_gpio_transitions(text, "GP4");
    let (_, rise_str) = parse_pincheck_rows(text);
    let xreset_rise_us = rise_str.as_deref().and_then(extract_leading_u32);
    let gp2_sole_active_tx =
        gp2.is_some_and(|n| n > 0) && gp3.unwrap_or(0) == 0 && gp4.unwrap_or(0) == 0;
    BusDiagBenchReport {
        gp2_transitions: gp2,
        gp3_transitions: gp3,
        gp4_transitions: gp4,
        xreset_rise_us,
        gp2_sole_active_tx,
    }
}

fn find_marker_u32(text: &str, marker: &str) -> Option<u32> {
    text.split(marker).nth(1).and_then(extract_leading_u32)
}

/// Decodes 4-channel `PIOLOGIC` timing and `NSW5V` safety edge counts.
pub fn parse_piologic_report(text: &str) -> PiologicBenchReport {
    let upper = text.to_ascii_uppercase();
    let nsw5v_transitions = text
        .lines()
        .find(|l| l.contains("NSW5V") && l.contains("Transitions="))
        .and_then(|l| find_marker_u32(l, "Transitions="));
    let min_tx_to_rx_us = find_marker_u32(text, "min=");
    let max_tx_to_rx_us = text
        .split("turnaround:")
        .nth(1)
        .and_then(|rhs| find_marker_u32(rhs, "max="));
    PiologicBenchReport {
        fifo_stall: upper.contains("FIFO_STALL=YES"),
        timing_skew: upper.contains("TIMING_SKEW"),
        edges_count: find_marker_u32(text, "edges=").unwrap_or(0),
        edges_truncated: text.contains("(TRUNCATED)"),
        span_us: find_marker_u32(text, "span="),
        nsw5v_transitions,
        reset_to_first_rxd_us: find_marker_u32(text, "first_RXD_fall:"),
        min_tx_to_rx_us,
        max_tx_to_rx_us,
    }
}

fn extract_token_after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    text.split(marker).nth(1)?.split_whitespace().next()
}

/// Decodes `PANARSP` RX byte count, frame verification flags, and `PayloadHex`.
pub fn parse_panarsp_report(text: &str) -> PanarspBenchReport {
    let rx_bytes = find_marker_u32(text, "rx=");
    let rsp_status = extract_token_after(text, "rsp=")
        .map(|s| s.trim_end_matches([',', ')']).to_ascii_uppercase())
        .unwrap_or_else(|| "UNKNOWN".to_string());
    let fifo_overflow = text.to_ascii_uppercase().contains("FIFO_OVF=YES");
    let frame_line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("[PANARSP]") && l.contains("Frame:"));
    let ack_plus = frame_line.is_some_and(|l| l.contains("ack=+"));
    let nak_minus = frame_line.is_some_and(|l| l.contains("nak=-"));
    let nak_0x15 = frame_line.is_some_and(|l| l.to_ascii_lowercase().contains("nak0x15=yes"));
    let payload = frame_line.and_then(|l| {
        let (_, rhs) = l.split_once("payload=\"")?;
        let (inner, _) = rhs.split_once('"')?;
        Some(inner.to_string())
    });
    let checksum_ok = payload.as_ref().map(|_| text.contains("(VALID)"));
    let payload_hex = text
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("[PANARSP] PayloadHex:"))
        .map(|s| s.trim().to_string());
    PanarspBenchReport {
        rx_bytes,
        ack_plus,
        nak_minus,
        nak_0x15,
        payload,
        payload_hex,
        checksum_ok,
        rsp_status,
        fifo_overflow,
    }
}
