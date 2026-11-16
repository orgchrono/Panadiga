//! Statistical A/B cold-boot harness command synthesis, classification, and CSV formatting.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

use super::super::log_decoder::parse_bench_log_text;
use super::types::{ALLOWED_AB_MODES, AbBootRun, AbHarnessSummary, AbModeStats};

/// Builds the read-only Pico command (`BOOTCAP` or `JIGBOOT`) for one A/B trial arm.
pub fn build_ab_boot_command(mode: &str, window_ms: u32) -> Result<String> {
    let norm = mode.trim().to_ascii_uppercase();
    if !ALLOWED_AB_MODES.contains(&norm.as_str()) {
        bail!("Unsupported A/B boot mode '{mode}'; expected NORMAL or JIG");
    }
    let clamped = window_ms.clamp(500, 60_000);
    Ok(if norm == "NORMAL" {
        format!("BOOTCAP {clamped}")
    } else {
        format!("JIGBOOT {clamped}")
    })
}

/// Classifies one raw Pico boot capture log deterministically into A/B error flags via `parse_bench_log_text`.
pub fn classify_ab_boot_log(
    run_index: usize,
    mode: &str,
    command: &str,
    raw_log: &str,
    duration_ms: f64,
) -> AbBootRun {
    let report = parse_bench_log_text(raw_log);
    let upper = raw_log.to_ascii_uppercase();
    let ddr_check_ok = report.boot_stages.iter().any(|s| s == "DDR_OK");
    let odd_83a7 = report
        .sense_packets
        .iter()
        .any(|s| s.mecha_error_code.as_deref() == Some("0x83A7"))
        || upper.contains("83 A7")
        || upper.contains("83A7");
    let boot_mode = report.boot_mode.map(|m| match m.as_str() {
        "normal" => "normal mode".to_string(),
        "serial_boot" => "serial boot mode".to_string(),
        other => other.to_string(),
    });
    AbBootRun {
        run_index,
        mode: mode.to_string(),
        command: command.to_string(),
        ddr_check_ok,
        boot_mode,
        bgo_failed_code: report.bgo_failed_code,
        sd_91006: report.sdcd_errors.contains(&-91006),
        sd_90002: report.sdcd_errors.contains(&-90002),
        sd_90001: report.sdcd_errors.contains(&-90001),
        odd_83a7,
        duration_ms: (duration_ms * 100.0).round() / 100.0,
        sd_hw_ok: report.sd_hw_init_ok,
        sd_diag_load_fail: report.sd_diag_load_failed.is_some(),
        sd_uncompress_fail: report.sd_uncompress_failed,
        odd_tray_fail: report.odd_tray_failed,
        diagnosis: report.diagnosis,
    }
}

/// Aggregates per-mode A/B statistics across recorded trials.
pub fn summarize_ab_runs(runs: &[AbBootRun]) -> AbHarnessSummary {
    let mut by_mode = BTreeMap::new();
    for mode in ALLOWED_AB_MODES {
        let group: Vec<&AbBootRun> = runs.iter().filter(|r| r.mode == mode).collect();
        by_mode.insert(
            mode.to_string(),
            AbModeStats {
                mode: mode.to_string(),
                runs: group.len(),
                ddr_ok_count: group.iter().filter(|r| r.ddr_check_ok).count(),
                sd_91006_count: group.iter().filter(|r| r.sd_91006).count(),
                sd_90002_count: group.iter().filter(|r| r.sd_90002).count(),
                sd_90001_count: group.iter().filter(|r| r.sd_90001).count(),
                odd_83a7_count: group.iter().filter(|r| r.odd_83a7).count(),
                bgo_fail_count: group.iter().filter(|r| r.bgo_failed_code.is_some()).count(),
                sd_hw_ok_count: group.iter().filter(|r| r.sd_hw_ok).count(),
                sd_diag_load_fail_count: group.iter().filter(|r| r.sd_diag_load_fail).count(),
            },
        );
    }
    AbHarnessSummary {
        runs: runs.to_vec(),
        by_mode,
    }
}

fn csv_escape(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn py_bool(val: bool) -> &'static str {
    if val { "True" } else { "False" }
}

/// Renders structured CSV output for A/B cold-boot statistical runs.
pub fn format_ab_csv(runs: &[AbBootRun]) -> String {
    let mut out = String::from(
        "run_index,mode,command,ddr_check_ok,boot_mode,bgo_failed_code,sd_91006,sd_90002,sd_90001,odd_83a7,duration_ms,sd_hw_ok,sd_diag_load_fail,sd_uncompress_fail,odd_tray_fail,diagnosis\n",
    );
    for r in runs {
        let bm = r.boot_mode.as_deref().unwrap_or("");
        let bgo = r.bgo_failed_code.map(|c| c.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.run_index,
            csv_escape(&r.mode),
            csv_escape(&r.command),
            py_bool(r.ddr_check_ok),
            csv_escape(bm),
            bgo,
            py_bool(r.sd_91006),
            py_bool(r.sd_90002),
            py_bool(r.sd_90001),
            py_bool(r.odd_83a7),
            r.duration_ms,
            py_bool(r.sd_hw_ok),
            py_bool(r.sd_diag_load_fail),
            py_bool(r.sd_uncompress_fail),
            py_bool(r.odd_tray_fail),
            csv_escape(&r.diagnosis),
        ));
    }
    out
}
