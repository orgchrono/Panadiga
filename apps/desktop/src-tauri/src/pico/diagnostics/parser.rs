//! Adapter delegating Pico P7501 diagnostic parsing to canonical SSOT parsers in `panadiga-core`.

use panadiga_core::pico::diag_rows::{
    extract_diagnostic_vdd, parse_captured_bytes, parse_dump_rows, parse_flash_mb,
    parse_pincheck_rows, parse_transition_rows, parse_wdt_status,
};
use panadiga_core::pico::log_decoder::parse_bench_log_text;

use super::command::{DiagnosticFindingDto, PicoDiagnosticResultDto};

pub use panadiga_core::pico::diag_rows::extract_pincheck_vdd;

fn stage_finding_meta(stage: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match stage {
        "SD_HW_OK" => Some(("SD_HW_OK", "info", "Stage-1 SD hardware 4-bit init OK")),
        "SD_DIAG_LOAD_FAIL" => Some((
            "SD_DIAG_LOAD_FAIL",
            "error",
            "Stage-1 SD DIAG load failed (`DIGADIAG.FRM`)",
        )),
        "SD_UNCOMPRESS_FAIL" => Some((
            "SD_UNCOMPRESS_FAIL",
            "error",
            "Stage-1 DIGADIAG.FRM EXTRHEAD/zlib unpack failed",
        )),
        "DIAG_EXEC" => Some((
            "DIAG_EXEC",
            "info",
            "Stage-1 DIGADIAG.FRM decompressed and jumped",
        )),
        "PANAMON_ACTIVE" => Some((
            "PANAMON_ACTIVE",
            "info",
            "PANAMON1> debug monitor active on SC0",
        )),
        "BGO_FAIL" => Some((
            "BGO_INIT_HALT",
            "warning",
            "BGO driver init failed -> BOOT halted in NORMAL mode",
        )),
        "ODD_NO_TRAY_80045300" => Some((
            "ODD_NO_TRAY_80045300",
            "warning",
            "ODD drive cannot close tray / no disc (80045300)",
        )),
        "ODD_APC_80040904" => Some((
            "LASER_APC_05MW_83A7",
            "error",
            "Drive laser APC fault 0x83A7 / 80040904",
        )),
        "SD_ERR_-91006" => Some((
            "SD_STATUS_MINUS_91006",
            "warning",
            "GetSD_Status ACMD13 rejected SD card (-91006 / -0x1637e)",
        )),
        _ => None,
    }
}

fn build_diagnostic_findings(raw_output: &str) -> Vec<DiagnosticFindingDto> {
    let mut findings = Vec::new();
    if let Some(wdt) = parse_wdt_status(raw_output) {
        let is_recovery = wdt.recoveries > 0 && wdt.reboot_src == "WDT_RECOVERY";
        findings.push(DiagnosticFindingDto {
            code: "WDT_ACTIVE".to_string(),
            severity: if is_recovery { "warning" } else { "info" }.to_string(),
            summary: format!(
                "Hardware Watchdog ACTIVE ({}ms, non-overridable)",
                wdt.timeout_ms
            ),
            detail: format!(
                "recoveries={} | reboot_src={} | fault_stage={} | last_cmd={}",
                wdt.recoveries, wdt.reboot_src, wdt.fault_stage, wdt.last_cmd
            ),
        });
    }

    let report = parse_bench_log_text(raw_output);
    for stage in &report.boot_stages {
        if let Some((code, severity, summary)) = stage_finding_meta(stage) {
            findings.push(DiagnosticFindingDto {
                code: code.to_string(),
                severity: severity.to_string(),
                summary: summary.to_string(),
                detail: report.diagnosis.clone(),
            });
        }
    }
    findings
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
        line.starts_with(&cmd_ok_prefix)
            || (norm_lower == "scan" && line == "scan: completed")
            || (norm_lower == "ping" && line == "pong")
            || ((norm_lower == "status" || norm_lower == "vdd") && line.starts_with("vdd:"))
            || (norm_lower == "wdt" && line.starts_with("wdt: ok"))
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
    let findings = build_diagnostic_findings(raw_output);

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
        findings,
    }
}
