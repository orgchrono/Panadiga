//! Command allowlist guard and telemetry parsers for `STATUS`, `PINCHECK`, `BUSDIAG`, `PIOLOGIC`, `PANARSP`.

use anyhow::{Result, bail};

use super::super::cdc::parse_vdd_reply;
use super::super::diag_rows::{parse_pincheck_rows, parse_voltage_cell};
use super::types::{
    BenchStatusReport, BusDiagBenchReport, FORBIDDEN_BENCH_TOKENS, PanarspBenchReport,
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

fn reject_uart_payload_writes(
    head: &str,
    upper_full: &str,
    decoded_payload: Option<&str>,
    allow_nvram_or_upper_writes: bool,
) -> Result<()> {
    if !matches!(head, "UARTSEND" | "UARTTXHEX" | "WAITTX" | "PANARSP") {
        return Ok(());
    }
    let body = upper_full
        .split_once(head)
        .map(|(_, r)| r.trim())
        .unwrap_or("");
    let inner_body = body.strip_prefix("LOW:").map(str::trim).unwrap_or(body);
    for candidate in [Some(inner_body), decoded_payload].into_iter().flatten() {
        if !allow_nvram_or_upper_writes {
            for &tok in FORBIDDEN_BENCH_TOKENS {
                if candidate.contains(tok) {
                    bail!("Forbidden bench token '{tok}' in {head} payload");
                }
            }
            if contains_lethal_am34_address(candidate) {
                bail!("Lethal AM34 MMIO address inside {head} payload is blocked");
            }
            if payload_has_write_intent(candidate) {
                bail!("Write-intent payload inside {head} is blocked in safe bench mode");
            }
        } else if payload_has_write_intent(candidate)
            && candidate
                .split(|c: char| c.is_whitespace() || c == ':' || c == ',')
                .any(is_ipl_sa0_sa10_token)
        {
            bail!("IPL SA0..SA10 write/erase payload inside {head} is permanently Read-Only (E05)");
        }
    }
    Ok(())
}

/// Validates that `command` belongs strictly to the read-only P7501 bench allowlist.
pub fn validate_safe_bench_command(command: &str) -> Result<String> {
    let normalized = normalize_single_line_command(command)?;
    let upper_full = normalized.to_ascii_uppercase();
    let head = upper_full.split_whitespace().next().unwrap_or_default();
    if !SAFE_BENCH_COMMANDS.contains(&head) {
        bail!("Command '{head}' is outside SAFE_BENCH_COMMANDS");
    }
    for &tok in FORBIDDEN_BENCH_TOKENS {
        if upper_full.contains(tok) {
            bail!("Forbidden bench token '{tok}' in bench command");
        }
    }
    if contains_lethal_am34_address(&upper_full) {
        bail!("Lethal AM34 MMIO address 0xC000100C..0xC000100F / 0xDC0F0000 is blocked");
    }
    let decoded_payload = extract_decoded_payload_text(head, &upper_full)?;
    reject_uart_payload_writes(head, &upper_full, decoded_payload.as_deref(), false)?;
    Ok(normalized.to_string())
}

fn extract_hex_addr_tokens(raw_text: &str) -> Vec<u32> {
    raw_text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter_map(|tok| {
            let no_us: String = tok.chars().filter(|&c| c != '_').collect();
            let upper = no_us.to_ascii_uppercase();
            let stripped = upper.strip_prefix("0X").unwrap_or(upper.as_str());
            let has_prefix = upper.starts_with("0X");
            if (has_prefix && !stripped.is_empty() || stripped.len() == 8)
                && stripped.len() <= 8
                && stripped.bytes().all(|b| b.is_ascii_hexdigit())
            {
                u32::from_str_radix(stripped, 16).ok()
            } else {
                None
            }
        })
        .collect()
}

fn parse_span_token_max_bytes(tok: &str) -> Option<u32> {
    let trimmed = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    let clean = trimmed.strip_prefix("0X").unwrap_or(trimmed);
    if clean.is_empty() || clean.len() > 4 {
        return None;
    }
    let dec = clean.parse::<u32>().ok().unwrap_or(0);
    let hex = u32::from_str_radix(clean, 16).ok().unwrap_or(0);
    let best = dec.max(hex).min(4096);
    (best > 0).then_some(best)
}

fn contains_lethal_am34_address(upper_text: &str) -> bool {
    let compact: String = upper_text.chars().filter(|c| *c != '_').collect();
    if compact.contains("C000100C") || compact.contains("DC0F0000") {
        return true;
    }
    let tokens: Vec<&str> = upper_text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .collect();
    for (idx, &tok) in tokens.iter().enumerate() {
        let span = tokens
            .get(idx + 1)
            .and_then(|next| parse_span_token_max_bytes(next))
            .unwrap_or(4)
            .max(4) as usize;
        if extract_hex_addr_tokens(tok)
            .into_iter()
            .any(|addr| crate::pico::rsp_codec::policy::is_am34_lethal_mmio_address(addr, span))
        {
            return true;
        }
    }
    false
}

const IPL_SA0_SA10_PREFIXES: &[&str] = &[
    "8000", "8001", "8002", "8003", "8400", "8401", "8402", "8403",
];

fn is_ipl_sa0_sa10_token(token: &str) -> bool {
    let no_us: String = token
        .trim_end_matches([',', ';'])
        .chars()
        .filter(|&c| c != '_')
        .collect();
    let stripped = no_us.trim_start_matches("0X").trim_start_matches('$');
    if stripped.len() < 4 || !stripped.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    if let Ok(addr) = u32::from_str_radix(stripped, 16)
        && crate::pico::rsp_codec::policy::is_critical_ipl_boot_address(addr, 4)
    {
        return true;
    }
    IPL_SA0_SA10_PREFIXES
        .iter()
        .any(|prefix| stripped.starts_with(prefix))
}

fn payload_has_write_intent(upper_text: &str) -> bool {
    let trimmed = upper_text.trim();
    if trimmed.starts_with('$') {
        let body = trimmed.trim_start_matches('$');
        if body.starts_with('M')
            || body.starts_with('X')
            || body.starts_with("VFLASHERASE")
            || body.starts_with("VFLASHWRITE")
        {
            return true;
        }
    }
    let words: Vec<&str> = trimmed
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .filter(|w| !w.is_empty())
        .collect();
    words.iter().enumerate().any(|(idx, &w)| {
        matches!(
            w,
            "WRITE"
                | "ERASE"
                | "PATCH"
                | "FAST_WRITE"
                | "FAST_ERASE"
                | "WRITEMEM"
                | "MEMW"
                | "MW"
                | "MB"
                | "MH"
                | "WB"
                | "WH"
                | "WW"
                | "PW"
                | "PB"
                | "PH"
                | "PO"
                | "EB"
                | "EW"
                | "SW"
                | "POKE"
                | "WMEM"
                | "JEDEC"
                | "VFLASH"
        ) || (matches!(w, "W" | "E") && idx + 1 < words.len())
            || (w.starts_with("$M") || w.starts_with("$X"))
    })
}

fn decode_hex_ascii_upper(hex_tok: &str) -> Result<String> {
    let clean = hex_tok.trim();
    if clean.is_empty()
        || !clean.len().is_multiple_of(2)
        || !clean.bytes().all(|b| b.is_ascii_hexdigit())
    {
        bail!("Invalid hex payload token '{clean}'");
    }
    let decoded = hex::decode(clean)?;
    Ok(decoded
        .into_iter()
        .map(|b| {
            if b.is_ascii() {
                (b as char).to_ascii_uppercase()
            } else {
                ' '
            }
        })
        .collect())
}

fn extract_decoded_payload_text(head: &str, normalized_upper: &str) -> Result<Option<String>> {
    match head {
        "UARTTXHEX" => {
            let Some(raw_tok) = normalized_upper.split_whitespace().nth(1) else {
                return Ok(None);
            };
            let payload_tok = raw_tok.strip_prefix("LOW:").unwrap_or(raw_tok);
            if matches!(payload_tok, "BRK" | "BREAK") {
                return Ok(None);
            }
            Ok(Some(decode_hex_ascii_upper(payload_tok)?))
        }
        "WAITTX" => {
            // Format: WAITTX <timeout_ms> <NORMAL|JIG[:ms]|NOPULL> "<trigger>" <hex|BRK> [post_ms]
            let Some(after_first_quote) = normalized_upper.split('"').nth(2) else {
                return Ok(None);
            };
            let Some(payload_tok) = after_first_quote.split_whitespace().next() else {
                return Ok(None);
            };
            if matches!(payload_tok, "BRK" | "BREAK") {
                return Ok(None);
            }
            Ok(Some(decode_hex_ascii_upper(payload_tok)?))
        }
        _ => Ok(None),
    }
}

/// Validates an interactive Pico CDC serial command against the SSOT catalog and hardware barriers.
pub fn validate_interactive_pico_command(command: &str) -> Result<String> {
    let normalized = normalize_single_line_command(command)?;
    let upper = normalized.to_ascii_uppercase();
    let head = upper.split_whitespace().next().unwrap_or_default();
    if !crate::ssot::spec::check_known_pico_command_id(head) {
        bail!("Command '{head}' is outside SSOT Pico CLI catalog");
    }

    let decoded_payload = extract_decoded_payload_text(head, &upper)?;
    if contains_lethal_am34_address(&upper)
        || decoded_payload
            .as_deref()
            .is_some_and(contains_lethal_am34_address)
    {
        bail!(
            "Lethal AM34 MMIO address 0xC000100C..0xC000100F / 0xDC0F0000 is permanently blocked"
        );
    }

    let is_sector_patch = head == "SECTOR" && upper.split_whitespace().nth(1) == Some("PATCH");
    if matches!(head, "FAST_WRITE" | "FAST_ERASE" | "ERASE" | "WRITEMEM") || is_sector_patch {
        if (matches!(head, "FAST_WRITE" | "FAST_ERASE") || is_sector_patch)
            && !upper.split_whitespace().any(|t| t == "CONFIRM_UPPER_NOR")
        {
            bail!("{head} requires explicit CONFIRM_UPPER_NOR token");
        }
        if upper.split_whitespace().skip(1).any(is_ipl_sa0_sa10_token) {
            bail!("IPL SA0..SA10 write target is permanently Read-Only (E05)");
        }
    }

    reject_uart_payload_writes(head, &upper, decoded_payload.as_deref(), true)?;
    Ok(upper)
}

fn extract_kv_token<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    text.split_whitespace().find_map(|tok| {
        tok.trim_matches(|c: char| matches!(c, ',' | ';' | '(' | ')' | '[' | ']'))
            .strip_prefix(&prefix)
    })
}

fn parse_bool_bit_kv(text: &str, key: &str) -> Option<bool> {
    match extract_kv_token(text, key)? {
        "1" | "YES" | "TRUE" | "yes" | "true" => Some(true),
        "0" | "NO" | "FALSE" | "no" | "false" => Some(false),
        _ => None,
    }
}

/// Parses `STATUS` or `VDD` output into voltage, rail state, NSW5V, fault latch, and sticky-LOW detection.
pub fn parse_status_output_report(text: &str) -> BenchStatusReport {
    let vdd_volts = parse_vdd_reply(text);
    let nsw5v_high = parse_bool_bit_kv(text, "NSW5V");
    let fault_latch = parse_bool_bit_kv(text, "fault_latch");
    let dropouts = extract_kv_token(text, "dropouts")
        .or_else(|| extract_kv_token(text, "NSW5V_DROPS"))
        .and_then(|v| v.parse::<u32>().ok());
    let explicit_status = text
        .split("VDD:")
        .nth(1)
        .and_then(|after| after.split_once('('))
        .and_then(|(_, rhs)| rhs.split_once(')'))
        .map(|(st, _)| st.trim().to_ascii_uppercase())
        .filter(|st| !st.is_empty());
    let gp26_sticky_low_suspected = vdd_volts.is_some_and(|v| v <= 0.10);
    let vdd_status = explicit_status.unwrap_or_else(|| {
        if fault_latch == Some(true) || nsw5v_high == Some(false) {
            "STANDBY_OR_FAULT".to_string()
        } else if vdd_volts.is_some_and(|v| v >= 2.50 && !gp26_sticky_low_suspected) {
            "OK".to_string()
        } else if vdd_volts.is_some() {
            "LOW".to_string()
        } else {
            "UNKNOWN".to_string()
        }
    });
    BenchStatusReport {
        vdd_volts,
        vdd_status,
        gp26_sticky_low_suspected,
        nsw5v_high,
        fault_latch,
        dropouts,
        raw_text: text.to_string(),
    }
}

/// Enforces the mandatory Standby / NSW5V / Fault-Latch Gate before any active `P7501` probe.
pub fn evaluate_live_rail_gate(status: &BenchStatusReport) -> Result<()> {
    if status.nsw5v_high == Some(false) {
        bail!(
            "STANDBY_GATE_STOP: NSW5V=0 (target unpowered or in STANDBY P-OFF; raw={:?})",
            status.raw_text.trim()
        );
    }
    if status.fault_latch == Some(true) {
        bail!(
            "STANDBY_GATE_STOP: fault_latch=1 (dropouts={:?}; run FAULTACK after stabilizing power; raw={:?})",
            status.dropouts,
            status.raw_text.trim()
        );
    }
    if status.gp26_sticky_low_suspected
        || status.raw_text.to_ascii_uppercase().contains("GP26 STICKY")
    {
        bail!(
            "STANDBY_GATE_STOP: GP26 sticky-LOW suspected (VDD={:?}, raw={:?})",
            status.vdd_volts,
            status.raw_text.trim()
        );
    }
    if status.vdd_status != "OK" || status.vdd_volts.is_none_or(|v| v < 2.50) {
        bail!(
            "STANDBY_GATE_STOP: Unsafe target VDD in Pico STATUS (expected OK >= 2.50V, got {:?} {}): {:?}",
            status.vdd_volts,
            status.vdd_status,
            status.raw_text.trim()
        );
    }
    Ok(())
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
