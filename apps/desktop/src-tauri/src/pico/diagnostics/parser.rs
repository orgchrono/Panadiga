//! Adapter delegating Pico P7501 diagnostic parsing to canonical SSOT parsers in `panadiga-core`.

use panadiga_core::pico::diag_rows::{
    extract_diagnostic_vdd, parse_captured_bytes, parse_dump_rows, parse_flash_mb,
    parse_pincheck_rows, parse_transition_rows,
};

use super::command::PicoDiagnosticResultDto;

pub use panadiga_core::pico::diag_rows::extract_pincheck_vdd;

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
    let vdd_volts = extract_diagnostic_vdd(raw_output, &pincheck_rows);

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
