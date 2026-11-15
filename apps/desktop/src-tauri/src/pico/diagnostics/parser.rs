//! Pure structural parsers for Pico P7501 CLI diagnostic replies.

use panadiga_core::pico::cdc::{parse_pico_dump_lines, parse_vdd_reply};

use super::command::{
    BusTransitionRowDto, DumpWordRowDto, PicoDiagnosticResultDto, PincheckRowDto,
};

fn parse_pincheck_rows(raw_output: &str) -> (Vec<PincheckRowDto>, Option<String>) {
    let mut rows = Vec::new();
    let mut rise_time = None;

    for line in raw_output.lines() {
        let trimmed = line.trim();
        if trimmed.contains('|')
            && (trimmed.starts_with("GP") || trimmed.contains("Pin"))
            && !trimmed.starts_with("Pin ")
            && !trimmed.contains("---+")
        {
            let parts: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            if parts.len() >= 5 {
                rows.push(PincheckRowDto {
                    pin_label: parts[0].to_string(),
                    hiz: parts[1].to_string(),
                    pull_down: parts[2].to_string(),
                    pull_up: parts[3].to_string(),
                    state: parts[4].to_string(),
                });
            }
        } else if let Some((_, rhs)) = trimmed.split_once("XRESET Release Rise Time")
            && let Some((_, val)) = rhs.split_once(':')
        {
            rise_time = Some(val.trim().to_string());
        }
    }
    (rows, rise_time)
}

fn parse_transition_rows(raw_output: &str) -> Vec<BusTransitionRowDto> {
    let mut rows = Vec::new();
    for line in raw_output.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Final=")
            && trimmed.contains("Transitions=")
            && let Some((left, right)) = trimmed.split_once(':')
        {
            let final_level = right
                .split("Final=")
                .nth(1)
                .and_then(|s| s.split([',', ' ']).next())
                .and_then(|n| n.parse::<u8>().ok());
            let transitions = right
                .split("Transitions=")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0);
            let parts: Vec<&str> = left.split_whitespace().collect();
            let signal = parts.first().copied().unwrap_or("SIG").to_string();
            let gpio = parts
                .get(1)
                .map(|g| g.trim_matches(['(', ')']).to_string())
                .unwrap_or_default();
            rows.push(BusTransitionRowDto {
                signal,
                gpio,
                final_level,
                transitions,
            });
        }
    }
    rows
}

fn parse_dump_rows(raw_output: &str) -> Vec<DumpWordRowDto> {
    parse_pico_dump_lines(raw_output)
        .into_iter()
        .map(|row| {
            let ascii = row
                .raw_bytes
                .iter()
                .map(|&b| {
                    if (0x20..=0x7E).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            DumpWordRowDto {
                address: row.address,
                words: row.words,
                ascii,
            }
        })
        .collect()
}

fn parse_captured_bytes(raw_output: &str) -> Option<u32> {
    for line in raw_output.lines() {
        if let Some(pos) = line.find(" bytes") {
            let prefix = &line[..pos];
            if let Some(token) = prefix.split(['(', ' ', ':']).next_back()
                && let Ok(val) = token.trim().parse::<u32>()
            {
                return Some(val);
            }
        }
    }
    None
}

fn parse_flash_mb(raw_output: &str) -> Option<u32> {
    for mb in [16u32, 8, 4, 2] {
        let needle = format!("{} MB", mb);
        if raw_output.contains(&needle) {
            return Some(mb);
        }
    }
    None
}

fn parse_voltage_cell(cell: &str) -> Option<f32> {
    cell.trim()
        .trim_end_matches(['V', 'v'])
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
}

/// Extracts P7501 Pin 1 (`SW_UART` / `GP26`) target rail voltage from structured `PINCHECK` rows,
/// prioritizing the Pull-Down test (`< 1.50 V`) so floating/unplugged wires never report false HIGH.
pub fn extract_pincheck_vdd(rows: &[PincheckRowDto]) -> Option<f32> {
    let vdd_row = rows.iter().find(|r| {
        r.pin_label.contains("SWUART")
            || r.pin_label.contains("GP26")
            || r.pin_label.contains("VDD")
    })?;
    let pd = parse_voltage_cell(&vdd_row.pull_down);
    let hiz = parse_voltage_cell(&vdd_row.hiz);
    if let Some(pd_val) = pd
        && pd_val < 1.50
    {
        return Some(pd_val);
    }
    hiz.or(pd)
}

/// Pure structured parser converting raw Pico CLI output into typed diagnostic metrics.
pub fn parse_diagnostic_output(
    command_id: &str,
    raw_command: &str,
    raw_output: &str,
    elapsed_ms: u64,
) -> PicoDiagnosticResultDto {
    let norm = command_id.trim().to_ascii_uppercase();
    let norm_lower = norm.to_ascii_lowercase();
    let lower = raw_output.to_ascii_lowercase();
    let cmd_ok_prefix = format!("{}: ok", norm_lower);
    let cmd_err_prefix = format!("{}: error", norm_lower);
    let bracket_err_prefix = format!("[{}] error:", norm_lower);

    let has_explicit_ok = lower.lines().map(str::trim).any(|line| {
        line.starts_with(&cmd_ok_prefix) || (norm_lower == "scan" && line == "scan: completed")
    });
    let has_pico_fatal = lower.lines().map(str::trim).any(|line| {
        line.starts_with(&cmd_err_prefix)
            || line.starts_with(&bracket_err_prefix)
            || line.starts_with("[safety interlock]")
            || line.starts_with("[handshake] failed")
            || line.starts_with("unknown command:")
    });

    let failed = raw_output.trim().is_empty()
        || has_pico_fatal
        || (!has_explicit_ok
            && (lower.contains("failed")
                || lower.contains("error")
                || lower.contains("abort")
                || lower.contains("safety interlock")));

    let (pincheck_rows, xreset_rise_us) = if norm == "PINCHECK" {
        parse_pincheck_rows(raw_output)
    } else {
        (Vec::new(), None)
    };
    let transition_rows = parse_transition_rows(raw_output);
    let dump_rows = if norm == "DUMP" {
        parse_dump_rows(raw_output)
    } else {
        Vec::new()
    };
    let captured_bytes = parse_captured_bytes(raw_output);
    let detected_flash_mb = parse_flash_mb(raw_output);
    let vdd_volts = parse_vdd_reply(raw_output).or_else(|| extract_pincheck_vdd(&pincheck_rows));

    let summary = if let Some(last_meaningful) = raw_output
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("PANADIGA>") && !l.starts_with("P7501>"))
    {
        last_meaningful.to_string()
    } else if raw_output.trim().is_empty() {
        format!("Sem resposta no barramento para '{}'", raw_command)
    } else {
        raw_output.trim().to_string()
    };

    PicoDiagnosticResultDto {
        command_id: norm,
        raw_command: raw_command.to_string(),
        success: !failed,
        elapsed_ms,
        summary,
        raw_output: raw_output.to_string(),
        vdd_volts,
        detected_flash_mb,
        xreset_rise_us,
        pincheck_rows,
        transition_rows,
        dump_rows,
        captured_bytes,
    }
}
