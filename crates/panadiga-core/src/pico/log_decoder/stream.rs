//! Stream normalization, UTF-16 decoding, and SCSI/ATAPI SENSE parsing helpers.

use super::SensePacketReport;

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
    let units = bytes.as_chunks::<2>().0.iter().map(|pair| {
        if big_endian {
            u16::from_be_bytes(*pair)
        } else {
            u16::from_le_bytes(*pair)
        }
    });
    char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

pub(super) fn extract_bracket_value_after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let (_, rest) = line.split_once(marker)?;
    let (_, after) = rest.split_once('[')?;
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

pub(super) fn parse_sense_line(line: &str) -> Option<SensePacketReport> {
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

pub(super) fn parse_hex_prefixed_u32(token: &str) -> Option<u32> {
    let clean = token
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    u32::from_str_radix(clean, 16).ok()
}

pub(super) fn push_unique_str(vec: &mut Vec<String>, val: String) {
    if !val.is_empty() && !vec.contains(&val) {
        vec.push(val);
    }
}

fn strip_boottrace_prefix(line: &str) -> Option<&str> {
    let ts = line.trim_start();
    if let Some(after) = ts.strip_prefix("[BOOTTRACE +") {
        let (_, rest) = after.split_once(']')?;
        return Some(rest.strip_prefix(' ').unwrap_or(rest));
    }
    if ts.starts_with("[BOOTTRACE]")
        || ts.starts_with("=== BOOTTRACE:")
        || ts.starts_with("BOOTTRACE:")
        || ts.starts_with("[CTRL +")
    {
        return None;
    }
    Some(line)
}

/// Strips interleaved `[DIAGWATCH...]` / `[BOOTTRACE]` wrappers and rejoins target fragments
/// split mid-line during real-time UART streaming (e.g. `BGO driver init failed` + `[1].`).
pub(super) fn reconstruct_diagwatch_target_stream(text: &str) -> String {
    let lines: Vec<&str> = text.lines().filter_map(strip_boottrace_prefix).collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if line.trim_start().starts_with("[DIAGWATCH") {
            while i + 1 < lines.len() && lines[i + 1].trim_start().starts_with("[DIAGWATCH") {
                i += 1;
            }
            if let (Some(prev), Some(&nxt)) = (out.last_mut(), lines.get(i + 1))
                && nxt
                    .chars()
                    .next()
                    .is_some_and(|c| matches!(c, '[' | ' ' | '.' | ',' | '*' | ')' | ']'))
            {
                prev.push_str(nxt);
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        out.push(line.to_string());
        i += 1;
    }
    out.join("\n")
}
