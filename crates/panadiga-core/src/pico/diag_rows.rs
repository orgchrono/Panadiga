//! Canonical structural row parsers and validators for Pico P7501 CLI diagnostic replies.
//!
//! Shared as the Single Source of Truth (SSOT) across CLI, Tauri desktop/Android backend,
//! and host verification suites.

use serde::{Deserialize, Serialize};

use super::cdc::{parse_pico_dump_lines, parse_vdd_reply};

/// Single pin row parsed from `PINCHECK`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PincheckRowSpec {
    pub pin_label: String,
    pub hiz: String,
    pub pull_down: String,
    pub pull_up: String,
    pub state: String,
}

/// Signal transition / level row parsed from `SNIFF` or `BUSDIAG`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusTransitionRowSpec {
    pub signal: String,
    pub gpio: String,
    pub final_level: Option<u8>,
    pub transitions: u32,
}

/// Decoded 32-bit word row parsed from `DUMP`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DumpWordRowSpec {
    pub address: String,
    pub words: Vec<String>,
    pub ascii: String,
}

/// Parses structured pin electrical state rows and optional `XRESET` rise time from `PINCHECK`.
pub fn parse_pincheck_rows(raw_output: &str) -> (Vec<PincheckRowSpec>, Option<String>) {
    let mut rows = Vec::new();
    let mut rise_time = None;

    for line in raw_output.lines() {
        let trimmed = line.trim();
        if trimmed.contains('|')
            && (trimmed.starts_with("GP") || trimmed.contains("Pin"))
            && !trimmed.starts_with("Pin ")
            && !trimmed.contains("---+")
        {
            let parts: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            if parts.len() >= 5 {
                rows.push(PincheckRowSpec {
                    pin_label: parts[0].to_string(),
                    hiz: parts[1].to_string(),
                    pull_down: parts[2].to_string(),
                    pull_up: parts[3].to_string(),
                    state: parts[4].to_string(),
                });
            }
        } else if let Some((_, rhs)) = trimmed.split_once("XRESET Release Rise Time")
            && let Some((_, val)) = rhs.split_once(':')
        {
            rise_time = Some(val.trim().to_string());
        }
    }
    (rows, rise_time)
}

/// Parses GPIO transition counters and final logic levels from `SNIFF` or `BUSDIAG` output.
pub fn parse_transition_rows(raw_output: &str) -> Vec<BusTransitionRowSpec> {
    let mut rows = Vec::new();
    for line in raw_output.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Final=")
            && trimmed.contains("Transitions=")
            && let Some((left, right)) = trimmed.split_once(':')
        {
            let final_level = right
                .split("Final=")
                .nth(1)
                .and_then(|s| s.split([',', ' ']).next())
                .and_then(|n| n.parse::<u8>().ok());
            let transitions = right
                .split("Transitions=")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0);
            let parts: Vec<&str> = left.split_whitespace().collect();
            let signal = parts.first().copied().unwrap_or("SIG").to_string();
            let gpio = parts
                .get(1)
                .map(|g| g.trim_matches(['(', ')']).to_string())
                .unwrap_or_default();
            rows.push(BusTransitionRowSpec {
                signal,
                gpio,
                final_level,
                transitions,
            });
        }
    }
    rows
}

/// Converts normalized `DUMP` lines into uppercase word rows paired with printable ASCII preview.
pub fn parse_dump_rows(raw_output: &str) -> Vec<DumpWordRowSpec> {
    parse_pico_dump_lines(raw_output)
        .into_iter()
        .map(|row| {
            let ascii = row
                .raw_bytes
                .iter()
                .map(|&b| {
                    if (0x20..=0x7E).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            DumpWordRowSpec {
                address: row.address,
                words: row.words,
                ascii,
            }
        })
        .collect()
}

/// Parses byte capture count from `BOOTCAP` / `UARTSTREAM` headers (`N bytes`).
pub fn parse_captured_bytes(raw_output: &str) -> Option<u32> {
    for line in raw_output.lines() {
        if let Some(pos) = line.find(" bytes") {
            let prefix = &line[..pos];
            if let Some(token) = prefix.split(['(', ' ', ':']).next_back()
                && let Ok(val) = token.trim().parse::<u32>()
            {
                return Some(val);
            }
        }
    }
    None
}

/// Parses NOR Flash size in megabytes reported by `CFI` or `AUTODETECT`.
pub fn parse_flash_mb(raw_output: &str) -> Option<u32> {
    for mb in [16u32, 8, 4, 2] {
        let needle = format!("{} MB", mb);
        if raw_output.contains(&needle) {
            return Some(mb);
        }
    }
    None
}

fn parse_voltage_cell(cell: &str) -> Option<f32> {
    cell.trim()
        .trim_end_matches(['V', 'v'])
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
}

/// Extracts P7501 Pin 1 (`SW_UART` / `GP26`) target rail voltage from structured `PINCHECK` rows,
/// prioritizing the Pull-Down test (`< 1.50 V`) so floating/unplugged wires never report false HIGH.
pub fn extract_pincheck_vdd(rows: &[PincheckRowSpec]) -> Option<f32> {
    let vdd_row = rows.iter().find(|r| {
        r.pin_label.contains("SWUART")
            || r.pin_label.contains("GP26")
            || r.pin_label.contains("VDD")
    })?;
    let pd = parse_voltage_cell(&vdd_row.pull_down);
    let hiz = parse_voltage_cell(&vdd_row.hiz);
    if let Some(pd_val) = pd
        && pd_val < 1.50
    {
        return Some(pd_val);
    }
    hiz.or(pd)
}

/// Extracts `VDD:` scalar reply or falls back to `PINCHECK` SW_UART rail sense.
pub fn extract_diagnostic_vdd(raw_output: &str, pincheck_rows: &[PincheckRowSpec]) -> Option<f32> {
    parse_vdd_reply(raw_output).or_else(|| extract_pincheck_vdd(pincheck_rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pincheck_rows_and_floating_vdd_safely() {
        let sample = "\
GP2 (Pin6 RXD   ) |  1   |     0     |    1    | HI-Z
GP26(Pin1 SWUART) | 3.21V|   0.00V   |  3.28V  | P7501 SW_UART
XRESET Release Rise Time (Hi-Z)     : 42 us";
        let (rows, rise) = parse_pincheck_rows(sample);
        assert_eq!(rows.len(), 2);
        assert_eq!(rise.as_deref(), Some("42 us"));
        assert_eq!(extract_pincheck_vdd(&rows), Some(0.0));
    }

    #[test]
    fn parses_transitions_dump_bytes_and_flash_mb() {
        let sniff = "  TXD/DAT(GP4) : Final=1, Transitions=68";
        let t_rows = parse_transition_rows(sniff);
        assert_eq!(t_rows.len(), 1);
        assert_eq!(t_rows[0].transitions, 68);
        assert_eq!(t_rows[0].final_level, Some(1));

        let dump = "807F0010: 44494741 35354845";
        let d_rows = parse_dump_rows(dump);
        assert_eq!(d_rows.len(), 1);
        assert_eq!(d_rows[0].ascii, "AGIDEH55");

        assert_eq!(
            parse_captured_bytes("BOOTCAP (42 bytes, 7500 ms)"),
            Some(42)
        );
        assert_eq!(parse_flash_mb("Size: 8 MB / 64 Mbit"), Some(8));
    }
}
