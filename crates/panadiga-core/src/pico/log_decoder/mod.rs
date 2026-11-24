//! Offline AM34 UART0 boot log, SCSI/ATAPI Sense packet, and Pico telemetry decoder.

mod diagnosis;
mod parsers;
mod stream;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use super::cdc::{parse_pico_dump_lines, parse_vdd_reply};
use super::protocol::{
    PicoBootClassFrame, PicoTbusAuditFrame, parse_boot_class_line, parse_tbus_audit_line,
};
use diagnosis::{
    BootDiagnosisContext, build_boot_stages, build_diagnostic_notes, classify_boot_diagnosis,
};
use parsers::ParsedLogLines;
use stream::{extract_ctrl_edge_events, reconstruct_diagwatch_target_stream};

pub use diagnosis::describe_boot_fat_error_code;
pub use stream::decode_captured_log_bytes;

/// Decoded SCSI/ATAPI `SENSE[...]` packet from AM34 `[RTSD]` drive logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensePacketReport {
    pub raw_hex: String,
    pub sense_key: u8,
    pub asc: u8,
    pub ascq: u8,
    pub mecha_error_code: Option<String>,
    pub interpretation: String,
}

/// Structured report extracted from an offline COM10 / AM34 / Pico capture log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Am34BootLogReport {
    pub loader_version: Option<String>,
    pub boot_mode: Option<String>,
    pub bgo_failed_code: Option<i32>,
    pub startup_reason: Option<String>,
    pub sd_capacity_lba: Option<u32>,
    pub sd_capacity_bytes: Option<u64>,
    pub sd_bus_width: Option<u8>,
    pub sd_transfer_kbps: Option<u32>,
    pub sd_hw_init_ok: bool,
    pub sd_diag_load_failed: Option<i32>,
    pub sd_uncompress_failed: bool,
    pub diag_started: bool,
    pub fat_mount_failed: Option<i32>,
    pub fat_open_failed: Option<(String, i32)>,
    pub odd_tray_failed: bool,
    pub odd_tray_open_count: usize,
    pub odd_tray_close_count: usize,
    pub odd_spinup_count: usize,
    pub reset_cycle_count: usize,
    pub ctrl_edge_events: Vec<String>,
    pub boot_stages: Vec<String>,
    pub diagnosis: String,
    pub sdcd_errors: Vec<i32>,
    pub drive_error_codes: Vec<String>,
    pub sense_packets: Vec<SensePacketReport>,
    pub vdd_volts: Option<f32>,
    pub dump_line_count: usize,
    pub panarsp_summaries: Vec<String>,
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_class: Option<PicoBootClassFrame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tbus_audit: Option<PicoTbusAuditFrame>,
}

/// Parses an offline AM34 / Pico capture text buffer into structured boot & diagnostic telemetry.
pub fn parse_bench_log_text(text: &str) -> Am34BootLogReport {
    let ctrl_edge_events = extract_ctrl_edge_events(text);
    let reconstructed = reconstruct_diagwatch_target_stream(text);
    let target_text = reconstructed.as_str();
    let ddr_ok = target_text.contains("DDR CHECK: OK");
    let parsed = ParsedLogLines::parse_all_lines(target_text);

    let has_83a7 = parsed
        .sense_packets
        .iter()
        .any(|s| s.mecha_error_code.as_deref() == Some("0x83A7"))
        || target_text.contains("0x83A7")
        || target_text.contains("83 A7");

    let ctx = BootDiagnosisContext {
        panamon_active: parsed.panamon_active,
        diag_started: parsed.diag_started,
        sd_uncompress_failed: parsed.sd_uncompress_failed,
        sd_diag_load_failed: parsed.sd_diag_load_failed,
        sd_hw_init_ok: parsed.sd_hw_init_ok,
        fat_mount_failed: parsed.fat_mount_failed,
        fat_open_failed: parsed.fat_open_failed.as_ref(),
        bgo_failed_code: parsed.bgo_failed_code,
        sdcd_errors: &parsed.sdcd_errors,
        odd_tray_failed: parsed.odd_tray_failed,
        odd_tray_open_count: parsed.odd_tray_open_count,
        odd_tray_close_count: parsed.odd_tray_close_count,
        odd_spinup_count: parsed.odd_spinup_count,
        reset_cycle_count: parsed.reset_cycle_count,
        has_83a7,
        boot_mode: parsed.boot_mode.as_deref(),
        ddr_ok,
    };

    let boot_stages = build_boot_stages(&ctx, parsed.saw_serial_boot, parsed.saw_normal_mode);
    let diagnosis = classify_boot_diagnosis(&ctx);
    let vdd_volts = parse_vdd_reply(text);
    let dump_line_count = parse_pico_dump_lines(text).len();
    let notes = build_diagnostic_notes(&ctx, vdd_volts);
    let boot_class = parse_boot_class_line(text);
    let tbus_audit = parse_tbus_audit_line(text);

    Am34BootLogReport {
        loader_version: parsed.loader_version,
        boot_mode: parsed.boot_mode,
        bgo_failed_code: parsed.bgo_failed_code,
        startup_reason: parsed.startup_reason,
        sd_capacity_lba: parsed.sd_capacity_lba,
        sd_capacity_bytes: parsed.sd_capacity_bytes,
        sd_bus_width: parsed.sd_bus_width,
        sd_transfer_kbps: parsed.sd_transfer_kbps,
        sd_hw_init_ok: parsed.sd_hw_init_ok,
        sd_diag_load_failed: parsed.sd_diag_load_failed,
        sd_uncompress_failed: parsed.sd_uncompress_failed,
        diag_started: parsed.diag_started,
        fat_mount_failed: parsed.fat_mount_failed,
        fat_open_failed: parsed.fat_open_failed,
        odd_tray_failed: parsed.odd_tray_failed,
        odd_tray_open_count: parsed.odd_tray_open_count,
        odd_tray_close_count: parsed.odd_tray_close_count,
        odd_spinup_count: parsed.odd_spinup_count,
        reset_cycle_count: parsed.reset_cycle_count,
        ctrl_edge_events,
        boot_stages,
        diagnosis,
        sdcd_errors: parsed.sdcd_errors,
        drive_error_codes: parsed.drive_error_codes,
        sense_packets: parsed.sense_packets,
        vdd_volts,
        dump_line_count,
        panarsp_summaries: parsed.panarsp_summaries,
        notes,
        boot_class,
        tbus_audit,
    }
}
