//! Token, line, and SENSE packet parsers for reconstructed AM34 / Pico capture logs.

use super::SensePacketReport;
use super::stream::{
    extract_bracket_value_after, parse_hex_prefixed_u32, parse_sense_line, push_unique_str,
};

#[derive(Default)]
pub(super) struct ParsedLogLines {
    pub(super) loader_version: Option<String>,
    pub(super) boot_mode: Option<String>,
    pub(super) saw_serial_boot: bool,
    pub(super) saw_normal_mode: bool,
    pub(super) bgo_failed_code: Option<i32>,
    pub(super) startup_reason: Option<String>,
    pub(super) sd_capacity_lba: Option<u32>,
    pub(super) sd_capacity_bytes: Option<u64>,
    pub(super) sd_bus_width: Option<u8>,
    pub(super) sd_transfer_kbps: Option<u32>,
    pub(super) sd_hw_init_ok: bool,
    pub(super) sd_diag_load_failed: Option<i32>,
    pub(super) sd_uncompress_failed: bool,
    pub(super) diag_started: bool,
    pub(super) fat_mount_failed: Option<i32>,
    pub(super) fat_open_failed: Option<(String, i32)>,
    pub(super) odd_tray_failed: bool,
    pub(super) odd_tray_open_count: usize,
    pub(super) odd_tray_close_count: usize,
    pub(super) odd_spinup_count: usize,
    pub(super) reset_cycle_count: usize,
    pub(super) panamon_active: bool,
    pub(super) sdcd_errors: Vec<i32>,
    pub(super) drive_error_codes: Vec<String>,
    pub(super) sense_packets: Vec<SensePacketReport>,
    pub(super) panarsp_summaries: Vec<String>,
}

impl ParsedLogLines {
    pub(super) fn parse_all_lines(target_text: &str) -> Self {
        let mut parsed = Self::default();
        for line in target_text.lines() {
            let trimmed = line.trim();
            parsed.parse_boot_and_loader(trimmed);
            parsed.parse_sd_and_fat(trimmed);
            parsed.parse_odd_and_errors(trimmed);
        }
        if !parsed.sd_hw_init_ok
            && parsed.sd_capacity_lba.is_some()
            && parsed.sd_bus_width.is_some()
        {
            parsed.sd_hw_init_ok = true;
        }
        parsed
    }

    fn parse_boot_and_loader(&mut self, trimmed: &str) {
        if trimmed.contains("DDR CHECK: OK") {
            self.reset_cycle_count += 1;
        }
        if let Some(idx) = trimmed.find("Loader ver") {
            self.loader_version = Some(trimmed[idx..].trim_end_matches('*').trim().to_string());
        }
        if trimmed.contains("normal mode") {
            self.saw_normal_mode = true;
            self.boot_mode = Some("normal".to_string());
        } else if trimmed.contains("serial boot mode") {
            self.saw_serial_boot = true;
            self.boot_mode = Some("serial_boot".to_string());
        } else if trimmed.contains("CD diag boot mode") {
            self.boot_mode = Some("cd_diag".to_string());
        } else if trimmed.contains("Emergency") {
            self.boot_mode = Some("emergency".to_string());
        }

        if let Some(val) = extract_bracket_value_after(trimmed, "BGO driver init failed")
            && let Ok(code) = val.parse::<i32>()
        {
            self.bgo_failed_code = Some(code);
        }
        if let Some(val) = extract_bracket_value_after(trimmed, "startup reason =") {
            self.startup_reason = Some(val.to_string());
        }
        if trimmed.contains("*** start diag ***") || trimmed.contains("diag loader is preserved") {
            self.diag_started = true;
        }
        if trimmed.contains("PANAMON1>") {
            self.panamon_active = true;
        }
        if trimmed.starts_with("PANARSP:") || trimmed.starts_with("[PANARSP]") {
            self.panarsp_summaries.push(trimmed.to_string());
        }
    }

    fn parse_sd_and_fat(&mut self, trimmed: &str) {
        if let Some((_, cap_rhs)) = trimmed.split_once("capacity:")
            && let Some((hex_tok, byte_rest)) = cap_rhs.split_once("[LBA](")
            && let Some(byte_tok) = byte_rest.split("Byte").next()
        {
            if let Some(lba) = parse_hex_prefixed_u32(hex_tok) {
                self.sd_capacity_lba = Some(lba);
            }
            if let Ok(b) = byte_tok.trim().parse::<u64>() {
                self.sd_capacity_bytes = Some(b);
            }
        }
        if let Some((_, spd_rhs)) = trimmed.split_once("transfer speed:")
            && let Some((kbps_tok, _)) = spd_rhs.split_once("[Kbps]")
            && let Ok(kbps) = kbps_tok.trim().parse::<u32>()
        {
            self.sd_transfer_kbps = Some(kbps);
        }
        if let Some((_, bw_rhs)) = trimmed.split_once("Bus Width:")
            && let Some((bw_tok, _)) = bw_rhs.split_once("(current)")
            && let Ok(bw) = bw_tok.trim().parse::<u8>()
        {
            self.sd_bus_width = Some(bw);
        }
        if let Some((_, rhs)) = trimmed.split_once("SD = ")
            && let Some((hex_tok, _)) = rhs.split_once('[')
            && let Some(lba) = parse_hex_prefixed_u32(hex_tok)
        {
            self.sd_capacity_lba = Some(lba);
            if self.sd_capacity_bytes.is_none() {
                self.sd_capacity_bytes = Some(u64::from(lba) * 512);
            }
        }
        if trimmed.contains("... SD init end") {
            self.sd_hw_init_ok = true;
        }
        if let Some(val) = extract_bracket_value_after(trimmed, "SD diag load failed")
            && let Ok(code) = val.parse::<i32>()
        {
            self.sd_diag_load_failed = Some(code);
        }
        if let Some(val) = extract_bracket_value_after(trimmed, "FAT Mount failed")
            && let Ok(code) = val.parse::<i32>()
        {
            self.fat_mount_failed = Some(code);
        }
        if let Some((_, open_rhs)) = trimmed.split_once("FAT Open(")
            && let Some((fname, rest)) = open_rhs.split_once(") failed[")
            && let Some((code_tok, _)) = rest.split_once(']')
            && let Ok(code) = code_tok.trim().parse::<i32>()
        {
            self.fat_open_failed = Some((fname.trim().to_string(), code));
        }
        if trimmed.contains("diag loader uncompress failed")
            || trimmed.contains("fvu uncompress failed")
            || trimmed.contains("no header!")
            || trimmed.contains("no footer!")
            || trimmed.contains("check sum mismatch")
        {
            self.sd_uncompress_failed = true;
        }
    }

    fn parse_odd_and_errors(&mut self, trimmed: &str) {
        if trimmed.contains("** can not close tray") || trimmed.contains("80045300") {
            self.odd_tray_failed = true;
        }
        if trimmed.contains("** open tray **") {
            self.odd_tray_open_count += 1;
        }
        if trimmed.contains("** close tray **") {
            self.odd_tray_close_count += 1;
        }
        self.odd_spinup_count += trimmed.matches("X(80020401)").count();

        for (marker, end_delim) in [
            ("ERROR[", Some(']')),
            ("SD init failed[", Some(']')),
            ("No Card[", Some(']')),
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
                    && !self.sdcd_errors.contains(&code)
                {
                    self.sdcd_errors.push(code);
                }
            }
        }

        for marker in ["DRIVE ACCESS ERROR", "drive start error("] {
            if trimmed.contains(marker)
                && let Some((_, tail)) = trimmed.split_once('(')
                && let Some((hex_code, _)) = tail.split_once(')')
            {
                push_unique_str(
                    &mut self.drive_error_codes,
                    hex_code.trim().to_ascii_uppercase(),
                );
            }
        }

        if trimmed.contains("SENSE[")
            && let Some(pkt) = parse_sense_line(trimmed)
        {
            self.sense_packets.push(pkt);
        }
    }
}
