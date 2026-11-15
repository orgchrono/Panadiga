//! Pure terminal report formatting and thin JSON file emission for CLI `unbrick`.

use anyhow::Result;
use panadiga_core::re::matshita_emu::{
    BootSequenceReport, FrmUnlockVerificationSummary, MatshitaEmulatorSnapshot,
    WriteInjectionRecord,
};
use panadiga_core::re::{
    FrmModuleExtractionReport, MainPassthroughReport, MainPassthroughRspPlan, Prog061BgoInspection,
    SdMediaVerificationReport, SdUnbrickKitReport, UnbrickWorkspaceReport,
};
use serde::Serialize;
use std::fs;
use std::path::Path;

pub fn emit_json_if_requested<T: Serialize>(val: &T, json_path: Option<&Path>) -> Result<()> {
    if let Some(p) = json_path {
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(p, serde_json::to_string_pretty(val)?)?;
        println!("JSON report written to {}", p.display());
    }
    Ok(())
}

pub fn format_unbrick_status_lines(report: &UnbrickWorkspaceReport) -> Vec<String> {
    let mut lines = vec![
        "=== PANADIGA UNBRICK WORKBENCH STATUS ===".to_string(),
        format!("Workspace   : {}", report.workspace_root),
        format!(
            "PANA_DVD    : {}",
            report.discovered_pana_dvd.as_deref().unwrap_or("not found")
        ),
        format!(
            "PROG061     : {}",
            report.discovered_prog061.as_deref().unwrap_or("not found")
        ),
        format!(
            "MAIN FW     : {}",
            report
                .discovered_main_3410pq
                .as_deref()
                .unwrap_or("not found")
        ),
        format!(
            "BGO Status  : {}",
            report
                .prog061_bgo
                .as_ref()
                .map(|b| b.detected_mode.as_str())
                .unwrap_or("unavailable")
        ),
        format!(
            "TBC Mode    : {}",
            report
                .main_passthrough
                .as_ref()
                .map(|m| m.detected_mode.as_str())
                .unwrap_or("unavailable")
        ),
    ];
    if let Some(sd) = &report.sd_verification {
        lines.push(format!(
            "SD Kit ({}) : {}",
            sd.sd_root_dir, sd.overall_status
        ));
    }
    for fact in &report.hardware_facts {
        lines.push(format!(
            "[{}] {} -> {}",
            fact.verdict, fact.topic, fact.evidence
        ));
    }
    lines
}

pub fn format_frm_extraction_lines(report: &FrmModuleExtractionReport) -> Vec<String> {
    let mut lines = vec![format!(
        "Extracted {} modules ({}) into {}",
        report.modules.len(),
        report.container_format,
        report.output_dir
    )];
    for m in &report.modules {
        lines.push(format!(
            "  - {:<10} {} ({} B) -> {}",
            m.name, m.offset_hex, m.encrypted_size, m.decrypted_path
        ));
    }
    lines
}

pub fn format_rsp_plan_lines(plan: &MainPassthroughRspPlan) -> Vec<String> {
    let mut lines = vec![format!("_dev_SERIAL Handshake Hex: {}", plan.handshake_hex)];
    for f in &plan.passthrough_frames {
        lines.push(format!("  RSP $M: {f}"));
    }
    for s in &plan.pico_cli_shortcuts {
        lines.push(format!("  Pico  : {s}"));
    }
    lines
}

pub fn format_passthrough_inspection_lines(rep: &MainPassthroughReport) -> Vec<String> {
    let mut lines = vec![format!(
        "MAIN Passthrough Status: {} (size={} B, match={})",
        rep.detected_mode, rep.file_size, rep.expected_size_match
    )];
    for s in &rep.sites {
        lines.push(format!(
            "  * {} @ {} [{}]: {}",
            s.name, s.va_hex, s.state, s.rsp_passthrough_frame
        ));
    }
    lines
}

pub fn format_sd_kit_lines(report: &SdUnbrickKitReport) -> Vec<String> {
    let mut lines = vec![
        format!(
            "SD Unbrick Kit Ready ({}) in {}",
            report.verification.overall_status, report.output_dir
        ),
        format!(
            "  DIGADIAG.FRM : {} [{}]",
            report.digadiag_path,
            &report.digadiag_sha256[..report.digadiag_sha256.len().min(16)]
        ),
    ];
    if let Some(ref sd) = report.pana_sd_path {
        lines.push(format!("  PANA__SD.FRM : {sd}"));
    }
    lines
}

pub fn format_sd_verification_lines(report: &SdMediaVerificationReport) -> Vec<String> {
    let mut lines = vec![format!(
        "SD Verification ({}): {}",
        report.sd_root_dir, report.overall_status
    )];
    for chk in &report.checks {
        lines.push(format!(
            "  [{}] {}: {}",
            chk.severity.to_uppercase(),
            chk.title,
            chk.detail
        ));
    }
    lines
}

pub fn format_bgo_inspection_line(rep: &Prog061BgoInspection) -> String {
    format!(
        "PROG061 BGO Inspection: mode={} ({})",
        rep.detected_mode, rep.raw_sha256
    )
}

pub fn format_matshita_boot_lines(report: &BootSequenceReport) -> Vec<String> {
    let mut lines = vec![format!(
        "Matshita R6-MV4 Boot [{:?}]: inquiry='{}' startup={} unlock_verified={} final={}",
        report.scenario,
        report.inquiry_id,
        report.startup_succeeded,
        report.unlock_media_verified,
        report.final_boot_code_hex
    )];
    if let Some(ref va) = report.jump_target_va_hex {
        lines.push(format!("  Boot Jump VA: {va}"));
    }
    for step in &report.steps {
        lines.push(format!(
            "  [{}/{}] {} -> CDB [{}] -> {} ({})",
            step.step_index,
            if step.passed { "OK" } else { "ERR" },
            step.boot_fn_name,
            step.cdb_hex,
            step.sense_hex,
            step.boot_log_line
        ));
    }
    lines
}

pub fn format_matshita_injection_lines(
    rec: &WriteInjectionRecord,
    snap: &MatshitaEmulatorSnapshot,
) -> Vec<String> {
    vec![
        format!(
            "Matshita ATAPI Injection #{} ({} / mode={}): {} B @ 0x{:08X} -> {}",
            rec.seq, rec.cdb0_or_cmd, rec.mode, rec.len_bytes, rec.target_va_or_lba, rec.status
        ),
        format!("  Note      : {}", rec.note),
        format!("  BufferSHA : {}", snap.buffer_sha256),
    ]
}

pub fn format_frm_unlock_verification_lines(summary: &FrmUnlockVerificationSummary) -> Vec<String> {
    let mut lines = vec![format!(
        "FRM Unlock Verify [{}]: {} B, entries={}, verified={}, jmp={}",
        summary.container_kind,
        summary.container_bytes,
        summary.entry_count,
        summary.verified,
        summary.bootloader_jump_va_hex
    )];
    for note in &summary.notes {
        lines.push(format!("  * {note}"));
    }
    lines
}

pub fn print_lines(lines: &[String]) {
    for line in lines {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use panadiga_core::re::build_passthrough_rsp_plan;

    #[test]
    fn pure_cli_formatters_emit_expected_headers() {
        let plan = build_passthrough_rsp_plan(Some(0x5040_1F3C));
        let lines = format_rsp_plan_lines(&plan);
        assert!(lines.iter().any(|l| l.contains("0e0402")));
        assert!(lines.iter().any(|l| l.contains("RSP $M:")));
    }
}
