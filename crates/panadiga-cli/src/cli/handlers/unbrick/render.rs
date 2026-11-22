//! Pure terminal report formatting and thin JSON file emission for CLI `unbrick`.

use anyhow::Result;
use panadiga_core::re::matshita_emu::{
    BootSequenceReport, FrmUnlockVerificationSummary, MatshitaEmulatorSnapshot,
    WriteInjectionRecord,
};
use panadiga_core::re::{
    FrmModuleExtractionReport, MainPassthroughReport, MainPassthroughRspPlan, Prog061BgoInspection,
    SdDiskInspectionReport, SdMediaVerificationReport, SdUnbrickCheck, SdUnbrickKitReport,
    UnbrickWorkspaceReport,
};
use serde::Serialize;
use std::path::Path;

pub fn emit_json_if_requested<T: Serialize>(val: &T, json_path: Option<&Path>) -> Result<()> {
    crate::cli::handlers::json_io::emit_json_output(val, json_path)
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
            "SD Unbrick Kit Ready ({}, profile={}) in {}",
            report.verification.overall_status, report.loader_profile_applied, report.output_dir
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
    if report.sata_bridge_reset_fix_applied {
        lines.push(
            "  SATA Bridge  : SataBridgeResetFix (1000 ms @ PROG061 0x5041D928 + MAIN3060NP 0x571122B2)"
                .to_string(),
        );
    }
    lines
}

fn push_sd_checks(lines: &mut Vec<String>, checks: &[SdUnbrickCheck]) {
    for chk in checks {
        lines.push(format!(
            "  [{}] {}: {}",
            chk.severity.to_uppercase(),
            chk.title,
            chk.detail
        ));
    }
}

pub fn format_sd_verification_lines(report: &SdMediaVerificationReport) -> Vec<String> {
    let mut lines = vec![format!(
        "SD Verification ({}): {}",
        report.sd_root_dir, report.overall_status
    )];
    push_sd_checks(&mut lines, &report.checks);
    lines
}

pub fn format_sd_disk_inspection_lines(report: &SdDiskInspectionReport) -> Vec<String> {
    let mut lines = vec![format!(
        "SD Raw Inspection ({}) [{}]: {}",
        report.target, report.partition_scheme, report.overall_status
    )];
    if !report.mbr_partitions.is_empty() {
        for part in &report.mbr_partitions {
            lines.push(format!(
                "  MBR[{}] type=0x{:02X} ({}) boot=0x{:02X} start_lba={} sectors={}",
                part.slot,
                part.partition_type,
                part.type_name,
                part.boot_flag,
                part.lba_start,
                part.sector_count
            ));
        }
    }
    if let Some(ref bpb) = report.bpb {
        lines.push(format!(
            "  BPB: kind={} oem='{}' label='{}' spc={} ({}B/clus) reserved={} fats={} root_entries={} sectors/fat={} hidden={}",
            bpb.computed_fat_kind,
            bpb.oem_name,
            bpb.volume_label,
            bpb.sectors_per_cluster,
            bpb.cluster_size_bytes,
            bpb.reserved_sectors,
            bpb.num_fats,
            bpb.root_entry_count,
            bpb.sectors_per_fat,
            bpb.hidden_sectors
        ));
    }
    push_sd_checks(&mut lines, &report.checks);
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
    use panadiga_core::re::{build_passthrough_rsp_plan, inspect_sd_disk_bytes};

    #[test]
    fn pure_cli_formatters_emit_expected_headers() {
        let plan = build_passthrough_rsp_plan(Some(0x5040_1F3C));
        let lines = format_rsp_plan_lines(&plan);
        assert!(lines.iter().any(|l| l.contains("0e0402")));
        assert!(lines.iter().any(|l| l.contains("RSP $M:")));
    }

    #[test]
    fn format_sd_disk_inspection_emits_scheme_and_status() {
        let gpt = {
            let mut buf = vec![0u8; 4096];
            buf[510] = 0x55;
            buf[511] = 0xAA;
            buf[0x1BE + 4] = 0xEE;
            buf[0x1BE + 8..0x1BE + 12].copy_from_slice(&1u32.to_le_bytes());
            buf[0x1BE + 12..0x1BE + 16].copy_from_slice(&1000u32.to_le_bytes());
            buf
        };
        let rep = inspect_sd_disk_bytes("mem:gpt.img", &gpt).unwrap();
        let lines = format_sd_disk_inspection_lines(&rep);
        assert!(lines[0].contains("[GPT]: FAIL"));
        assert!(lines.iter().any(|l| l.contains("MBR[0] type=0xEE")));
    }
}
