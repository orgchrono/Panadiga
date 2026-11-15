//! Offline AM34 UART0 boot log, SCSI/ATAPI Sense packet, and Pico telemetry decoder.

use serde::{Deserialize, Serialize};

use super::cdc::{parse_pico_dump_lines, parse_vdd_reply};

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
    pub sdcd_errors: Vec<i32>,
    pub drive_error_codes: Vec<String>,
    pub sense_packets: Vec<SensePacketReport>,
    pub vdd_volts: Option<f32>,
    pub dump_line_count: usize,
    pub panarsp_summaries: Vec<String>,
    pub notes: Vec<String>,
}

/// Decodes raw captured log bytes supporting UTF-16LE/BE BOM, BOM-less UTF-16LE, and UTF-8.
pub fn decode_captured_log_bytes(raw: &[u8]) -> String {
    if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
        return decode_utf16_slice(&raw[2..], false);
    }
    if raw.len() >= 2 && raw[0] == 0xFE && raw[1] == 0xFF {
        return decode_utf16_slice(&raw[2..], true);
    }
    if raw.len() >= 4 && raw[1] == 0 && raw[3] == 0 && raw.iter().take(64).any(|&b| b == 0) {
        return decode_utf16_slice(raw, false);
    }
    String::from_utf8_lossy(raw).into_owned()
}

fn decode_utf16_slice(bytes: &[u8], big_endian: bool) -> String {
    let units = bytes.chunks_exact(2).map(|pair| {
        if big_endian {
            u16::from_be_bytes([pair[0], pair[1]])
        } else {
            u16::from_le_bytes([pair[0], pair[1]])
        }
    });
    char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

fn extract_bracket_value(line: &str) -> Option<&str> {
    let (_, after) = line.split_once('[')?;
    let (inner, _) = after.split_once(']')?;
    Some(inner.trim())
}

fn interpret_sense_packet(sk: u8, asc: u8, ascq: u8, mecha: Option<&str>) -> String {
    match (sk, asc, ascq, mecha) {
        (_, _, _, Some("0x83A7")) => {
            "Optical ODD servo/laser hardware fault (0x83A7): tray close/spindle blocked before MAIN boot".to_string()
        }
        (0x04, 0x53, 0x00, _) => {
            "Hardware Error (SK=04h ASC=53h ASCQ=00h): Media load or eject failed".to_string()
        }
        (0x04, 0x09, 0x04, _) => {
            "Hardware Error (SK=04h ASC=09h ASCQ=04h): Head select / servo seek failure".to_string()
        }
        (0x06, 0x29, 0x00, _) => {
            "Unit Attention (SK=06h ASC=29h ASCQ=00h): Power-on or ATAPI device reset occurred".to_string()
        }
        (0x02, 0x3A, 0x00, _) => {
            "Not Ready (SK=02h ASC=3Ah ASCQ=00h): Medium not present".to_string()
        }
        _ => format!("SCSI SENSE SK=0x{sk:02X} ASC=0x{asc:02X} ASCQ=0x{ascq:02X}"),
    }
}

fn parse_sense_line(line: &str) -> Option<SensePacketReport> {
    let inner = line.split("SENSE[").nth(1)?.split(']').next()?;
    let bytes: Vec<u8> = inner
        .split_whitespace()
        .filter_map(|tok| u8::from_str_radix(tok, 16).ok())
        .collect();
    if bytes.len() < 14 {
        return None;
    }
    let sk = bytes[2] & 0x0F;
    let asc = bytes[12];
    let ascq = bytes[13];
    let mecha_error_code = if bytes.len() >= 12 && (bytes[10] != 0 || bytes[11] != 0) {
        let code = u16::from_be_bytes([bytes[10], bytes[11]]);
        if code >= 0x8000 {
            Some(format!("0x{code:04X}"))
        } else {
            None
        }
    } else {
        None
    };
    let interpretation = interpret_sense_packet(sk, asc, ascq, mecha_error_code.as_deref());
    Some(SensePacketReport {
        raw_hex: inner.trim().to_string(),
        sense_key: sk,
        asc,
        ascq,
        mecha_error_code,
        interpretation,
    })
}

fn parse_hex_prefixed_u32(token: &str) -> Option<u32> {
    let clean = token
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    u32::from_str_radix(clean, 16).ok()
}

fn push_unique_str(vec: &mut Vec<String>, val: String) {
    if !val.is_empty() && !vec.contains(&val) {
        vec.push(val);
    }
}

/// Parses an offline AM34 / Pico capture text buffer into structured boot & diagnostic telemetry.
pub fn parse_bench_log_text(text: &str) -> Am34BootLogReport {
    let mut loader_version = None;
    let mut boot_mode = None;
    let mut bgo_failed_code = None;
    let mut startup_reason = None;
    let mut sd_capacity_lba = None;
    let mut sdcd_errors = Vec::new();
    let mut drive_error_codes = Vec::new();
    let mut sense_packets = Vec::new();
    let mut panarsp_summaries = Vec::new();
    let mut notes = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find("Loader ver") {
            let ver = trimmed[idx..].trim_end_matches('*').trim().to_string();
            loader_version = Some(ver);
        }
        if trimmed.contains("normal mode") {
            boot_mode = Some("normal".to_string());
        } else if trimmed.contains("serial boot mode") {
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

        if let Some((_, rhs)) = trimmed.split_once("SD = ")
            && let Some((hex_tok, _)) = rhs.split_once('[')
            && let Some(lba) = parse_hex_prefixed_u32(hex_tok)
        {
            sd_capacity_lba = Some(lba);
        }

        for (marker, end_delim) in [
            ("ERROR[", Some(']')),
            ("SD diag load failed[", Some(']')),
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

    let vdd_volts = parse_vdd_reply(text);
    let dump_line_count = parse_pico_dump_lines(text).len();

    if let Some(code) = bgo_failed_code {
        notes.push(format!(
            "BGO driver init failed[{code}] detected (CS1 NOR BGO corruption -> U99 loop unless bypassed via JIGBOOT/DIGADIAG.FRM)."
        ));
    }
    if sdcd_errors.contains(&-91006) {
        notes.push(
            "SDCD -91006 (ACMD6/ACMD13 status failure @ 0x44027F79): replace SD card or use standard SLC <=2GB card."
                .to_string(),
        );
    }
    if sdcd_errors.contains(&0) && sd_capacity_lba.is_some() {
        notes.push(
            "SD bus initialized cleanly (4-bit mode) but stopped at SD diag load failed[0]: format FAT16 and place 131072-byte DIGADIAG.FRM in root."
                .to_string(),
        );
    }
    if let Some(v) = vdd_volts
        && v <= 0.10
    {
        notes.push(format!(
            "VDD={v:.2}V <= 0.10V (sticky-LOW / target rail off or unpowered P7501)."
        ));
    }

    Am34BootLogReport {
        loader_version,
        boot_mode,
        bgo_failed_code,
        startup_reason,
        sd_capacity_lba,
        sdcd_errors,
        drive_error_codes,
        sense_packets,
        vdd_volts,
        dump_line_count,
        panarsp_summaries,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16le_bom_and_bootcap_sense() {
        let sample = "***** Loader ver0.61 [06/03/06] *****\r\n\
                      ***** normal mode *****\r\n\
                      BGO driver init failed[1].\r\n\
                      [RTSD] === DRIVE ACCESS ERROR === ( 80045300 )\r\n\
                      *** SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00]\r\n\
                      VDD: 3.24V (OK)\r\n";
        let mut utf16_bytes = vec![0xFFu8, 0xFEu8];
        for unit in sample.encode_utf16() {
            utf16_bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let text = decode_captured_log_bytes(&utf16_bytes);
        let report = parse_bench_log_text(&text);

        assert_eq!(report.boot_mode.as_deref(), Some("normal"));
        assert_eq!(report.bgo_failed_code, Some(1));
        assert_eq!(report.drive_error_codes, vec!["80045300".to_string()]);
        assert_eq!(report.sense_packets.len(), 1);
        assert_eq!(
            report.sense_packets[0].mecha_error_code.as_deref(),
            Some("0x83A7")
        );
        assert_eq!(report.vdd_volts, Some(3.24));
    }

    #[test]
    fn detects_sd_bus_ok_with_missing_fat16_digadiag() {
        let sample = "***** Loader ver0.61 [06/03/06] *****\r\n\
                      ***** serial boot mode *****\r\n\
                      SD = 0x3C4000[LBA]\r\n\
                      ... SD init end\r\n\
                      SD diag load failed[0].\r\n";
        let report = parse_bench_log_text(sample);
        assert_eq!(report.boot_mode.as_deref(), Some("serial_boot"));
        assert_eq!(report.sd_capacity_lba, Some(0x003C_4000));
        assert_eq!(report.sdcd_errors, vec![0]);
        assert!(report.notes.iter().any(|n| n.contains("FAT16")));
    }

    #[test]
    fn decodes_sdcd_error_variants_without_brackets() {
        let sample = "[SDCD]SetBusWidth:SD Status ERROR -91006\r\n\
                      [SDCD] Set SD Bus Width fail ERROR:-91006\r\n\
                      SD init failed[-90002].\r\n";
        let report = parse_bench_log_text(sample);
        assert_eq!(report.sdcd_errors, vec![-91006, -90002]);
        assert!(report.notes.iter().any(|n| n.contains("-91006")));
    }
}
