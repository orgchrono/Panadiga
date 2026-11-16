//! Offline AM34 UART0 boot log, SCSI/ATAPI Sense packet, and Pico telemetry decoder.

mod diagnosis;
mod stream;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use super::cdc::{parse_pico_dump_lines, parse_vdd_reply};
use diagnosis::{
    BootDiagnosisContext, build_boot_stages, build_diagnostic_notes, classify_boot_diagnosis,
};
use stream::{
    extract_bracket_value, parse_hex_prefixed_u32, parse_sense_line, push_unique_str,
    reconstruct_diagwatch_target_stream,
};

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
    pub boot_stages: Vec<String>,
    pub diagnosis: String,
    pub sdcd_errors: Vec<i32>,
    pub drive_error_codes: Vec<String>,
    pub sense_packets: Vec<SensePacketReport>,
    pub vdd_volts: Option<f32>,
    pub dump_line_count: usize,
    pub panarsp_summaries: Vec<String>,
    pub notes: Vec<String>,
}

/// Parses an offline AM34 / Pico capture text buffer into structured boot & diagnostic telemetry.
pub fn parse_bench_log_text(text: &str) -> Am34BootLogReport {
    let reconstructed = reconstruct_diagwatch_target_stream(text);
    let target_text = reconstructed.as_str();
    let mut loader_version = None;
    let mut boot_mode = None;
    let mut saw_serial_boot = false;
    let mut saw_normal_mode = false;
    let mut bgo_failed_code = None;
    let mut startup_reason = None;
    let mut sd_capacity_lba = None;
    let mut sd_capacity_bytes = None;
    let mut sd_bus_width = None;
    let mut sd_transfer_kbps = None;
    let mut sd_hw_init_ok = false;
    let mut sd_diag_load_failed = None;
    let mut sd_uncompress_failed = false;
    let mut diag_started = false;
    let mut fat_mount_failed = None;
    let mut fat_open_failed = None;
    let mut odd_tray_failed = false;
    let mut panamon_active = false;
    let mut sdcd_errors = Vec::new();
    let mut drive_error_codes = Vec::new();
    let mut sense_packets = Vec::new();
    let mut panarsp_summaries = Vec::new();
    let ddr_ok = target_text.contains("DDR CHECK: OK");

    for line in target_text.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find("Loader ver") {
            loader_version = Some(trimmed[idx..].trim_end_matches('*').trim().to_string());
        }
        if trimmed.contains("normal mode") {
            saw_normal_mode = true;
            boot_mode = Some("normal".to_string());
        } else if trimmed.contains("serial boot mode") {
            saw_serial_boot = true;
            boot_mode = Some("serial_boot".to_string());
        } else if trimmed.contains("CD diag boot mode") {
            boot_mode = Some("cd_diag".to_string());
        } else if trimmed.contains("Emergency") {
            boot_mode = Some("emergency".to_string());
        }

        if trimmed.contains("BGO driver init failed")
            && let Some(val) = extract_bracket_value(trimmed)
            && let Ok(code) = val.parse::<i32>()
        {
            bgo_failed_code = Some(code);
        }

        if trimmed.contains("startup reason =")
            && let Some(val) = extract_bracket_value(trimmed)
        {
            startup_reason = Some(val.to_string());
        }

        if let Some((_, cap_rhs)) = trimmed.split_once("capacity:")
            && let Some((hex_tok, byte_rest)) = cap_rhs.split_once("[LBA](")
            && let Some(byte_tok) = byte_rest.split("Byte").next()
        {
            if let Some(lba) = parse_hex_prefixed_u32(hex_tok) {
                sd_capacity_lba = Some(lba);
            }
            if let Ok(b) = byte_tok.trim().parse::<u64>() {
                sd_capacity_bytes = Some(b);
            }
        }
        if let Some((_, spd_rhs)) = trimmed.split_once("transfer speed:")
            && let Some((kbps_tok, _)) = spd_rhs.split_once("[Kbps]")
            && let Ok(kbps) = kbps_tok.trim().parse::<u32>()
        {
            sd_transfer_kbps = Some(kbps);
        }
        if let Some((_, bw_rhs)) = trimmed.split_once("Bus Width:")
            && let Some((bw_tok, _)) = bw_rhs.split_once("(current)")
            && let Ok(bw) = bw_tok.trim().parse::<u8>()
        {
            sd_bus_width = Some(bw);
        }

        if let Some((_, rhs)) = trimmed.split_once("SD = ")
            && let Some((hex_tok, _)) = rhs.split_once('[')
            && let Some(lba) = parse_hex_prefixed_u32(hex_tok)
        {
            sd_capacity_lba = Some(lba);
            if sd_capacity_bytes.is_none() {
                sd_capacity_bytes = Some(u64::from(lba) * 512);
            }
        }

        if trimmed.contains("... SD init end") {
            sd_hw_init_ok = true;
        }
        if trimmed.contains("SD diag load failed[")
            && let Some(val) = extract_bracket_value(trimmed)
            && let Ok(code) = val.parse::<i32>()
        {
            sd_diag_load_failed = Some(code);
        }
        if trimmed.contains("FAT Mount failed[")
            && let Some(val) = extract_bracket_value(trimmed)
            && let Ok(code) = val.parse::<i32>()
        {
            fat_mount_failed = Some(code);
        }
        if let Some((_, open_rhs)) = trimmed.split_once("FAT Open(")
            && let Some((fname, rest)) = open_rhs.split_once(") failed[")
            && let Some((code_tok, _)) = rest.split_once(']')
            && let Ok(code) = code_tok.trim().parse::<i32>()
        {
            fat_open_failed = Some((fname.trim().to_string(), code));
        }

        if trimmed.contains("diag loader uncompress failed")
            || trimmed.contains("fvu uncompress failed")
            || trimmed.contains("no header!")
            || trimmed.contains("no footer!")
            || trimmed.contains("check sum mismatch")
        {
            sd_uncompress_failed = true;
        }
        if trimmed.contains("*** start diag ***") || trimmed.contains("diag loader is preserved") {
            diag_started = true;
        }
        if trimmed.contains("** can not close tray") || trimmed.contains("80045300") {
            odd_tray_failed = true;
        }
        if trimmed.contains("PANAMON1>") {
            panamon_active = true;
        }

        for (marker, end_delim) in [
            ("ERROR[", Some(']')),
            ("SD init failed[", Some(']')),
            ("ERROR:", None),
            ("ERROR ", None),
        ] {
            if let Some((_, tail)) = trimmed.rsplit_once(marker) {
                let raw_num = match end_delim {
                    Some(ch) => tail.split_once(ch).map(|(s, _)| s),
                    None => tail.split_whitespace().next(),
                };
                if let Some(code_str) = raw_num
                    && let Ok(code) = code_str.trim().trim_end_matches('.').parse::<i32>()
                    && !sdcd_errors.contains(&code)
                {
                    sdcd_errors.push(code);
                }
            }
        }

        for marker in ["DRIVE ACCESS ERROR", "drive start error("] {
            if trimmed.contains(marker)
                && let Some((_, tail)) = trimmed.split_once('(')
                && let Some((hex_code, _)) = tail.split_once(')')
            {
                push_unique_str(&mut drive_error_codes, hex_code.trim().to_ascii_uppercase());
            }
        }

        if trimmed.contains("SENSE[")
            && let Some(pkt) = parse_sense_line(trimmed)
        {
            sense_packets.push(pkt);
        }

        if trimmed.starts_with("PANARSP:") || trimmed.starts_with("[PANARSP]") {
            panarsp_summaries.push(trimmed.to_string());
        }
    }

    if !sd_hw_init_ok && sd_capacity_lba.is_some() && sd_bus_width.is_some() {
        sd_hw_init_ok = true;
    }

    let has_83a7 = sense_packets
        .iter()
        .any(|s| s.mecha_error_code.as_deref() == Some("0x83A7"))
        || target_text.contains("0x83A7")
        || target_text.contains("83 A7");

    let ctx = BootDiagnosisContext {
        panamon_active,
        diag_started,
        sd_uncompress_failed,
        sd_diag_load_failed,
        sd_hw_init_ok,
        fat_mount_failed,
        fat_open_failed: fat_open_failed.as_ref(),
        bgo_failed_code,
        sdcd_errors: &sdcd_errors,
        odd_tray_failed,
        has_83a7,
        boot_mode: boot_mode.as_deref(),
        ddr_ok,
    };

    let boot_stages = build_boot_stages(&ctx, saw_serial_boot, saw_normal_mode);
    let diagnosis = classify_boot_diagnosis(&ctx);
    let vdd_volts = parse_vdd_reply(text);
    let dump_line_count = parse_pico_dump_lines(text).len();
    let notes = build_diagnostic_notes(&ctx, vdd_volts);

    Am34BootLogReport {
        loader_version,
        boot_mode,
        bgo_failed_code,
        startup_reason,
        sd_capacity_lba,
        sd_capacity_bytes,
        sd_bus_width,
        sd_transfer_kbps,
        sd_hw_init_ok,
        sd_diag_load_failed,
        sd_uncompress_failed,
        diag_started,
        fat_mount_failed,
        fat_open_failed,
        odd_tray_failed,
        boot_stages,
        diagnosis,
        sdcd_errors,
        drive_error_codes,
        sense_packets,
        vdd_volts,
        dump_line_count,
        panarsp_summaries,
        notes,
    }
}
