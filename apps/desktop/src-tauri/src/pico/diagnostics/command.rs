//! Diagnostic result DTOs and SSOT command composition/validation.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::commands::dto::build_hardware_ssot_spec;

/// Single pin row parsed from `PINCHECK`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PincheckRowDto {
    pub pin_label: String,
    pub hiz: String,
    pub pull_down: String,
    pub pull_up: String,
    pub state: String,
}

/// Signal transition / level row parsed from `SNIFF` or `BUSDIAG`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusTransitionRowDto {
    pub signal: String,
    pub gpio: String,
    pub final_level: Option<u8>,
    pub transitions: u32,
}

/// Decoded 32-bit word row parsed from `DUMP`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpWordRowDto {
    pub address: String,
    pub words: Vec<String>,
    pub ascii: String,
}

/// Structured result of running any granular Pico firmware diagnostic command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicoDiagnosticResultDto {
    pub command_id: String,
    pub raw_command: String,
    pub success: bool,
    pub elapsed_ms: u64,
    pub summary: String,
    pub raw_output: String,
    pub vdd_volts: Option<f32>,
    pub detected_flash_mb: Option<u32>,
    pub xreset_rise_us: Option<String>,
    pub pincheck_rows: Vec<PincheckRowDto>,
    pub transition_rows: Vec<BusTransitionRowDto>,
    pub dump_rows: Vec<DumpWordRowDto>,
    pub captured_bytes: Option<u32>,
}

/// Validates command ID & optional argument against SSOT catalog and prevents CDC injection.
pub fn compose_diagnostic_command(
    command_id: &str,
    arg: Option<&str>,
) -> Result<(String, Duration), String> {
    let normalized = command_id.trim().to_ascii_uppercase();
    let spec = build_hardware_ssot_spec();
    let entry = spec
        .pico_commands
        .iter()
        .find(|c| c.id == normalized)
        .ok_or_else(|| format!("Comando diagnóstico desconhecido: '{}'", command_id))?;

    let clean_arg = arg
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.chars().any(|c| c == '\r' || c == '\n' || c.is_control()) {
                return Err("Argumento contém caracteres de controle inválidos.".to_string());
            }
            Ok(s)
        })
        .transpose()?;

    // Enforce bounded parameter formats for critical commands
    if normalized == "BREAK_IN" {
        if let Some(m) = clean_arg
            && !matches!(m, "0" | "1" | "2" | "3")
        {
            return Err(format!("Modo Break-In inválido '{}': use 0, 1, 2 ou 3.", m));
        }
    } else if normalized == "FLASH_SIZE" {
        if let Some(sz) = clean_arg
            && !matches!(sz, "2" | "4" | "8" | "16")
        {
            return Err(format!(
                "Tamanho Flash inválido '{}': use 2, 4, 8 ou 16.",
                sz
            ));
        }
    } else if normalized == "LED"
        && let Some(mode) = clean_arg
    {
        let upper = mode.to_ascii_uppercase();
        if !matches!(
            upper.as_str(),
            "IDLE" | "ACTIVE" | "GDB" | "FAULT" | "OFF" | "AUTO" | "STATUS"
        ) {
            return Err(
                "Modo LED inválido: use IDLE, ACTIVE, GDB, FAULT, OFF, AUTO ou STATUS.".to_string(),
            );
        }
    }

    let raw_cmd = match clean_arg {
        Some(a) => format!("{} {}", normalized, a),
        None => normalized.clone(),
    };

    // Scale wait window dynamically for variable-duration captures
    let mut timeout_ms = entry.timeout_ms;
    if matches!(
        normalized.as_str(),
        "SNIFF"
            | "BOOTCAP"
            | "UARTMON"
            | "JIGBOOT"
            | "UARTSTREAM"
            | "HEXCAP"
            | "BAUD"
            | "BAUDSCAN"
            | "PWRTRACE"
            | "BOOTTRACE"
            | "PIOLOGIC"
            | "PANARSP"
            | "WAITTX"
    ) && let Some(first_tok) = clean_arg.and_then(|s| s.split_whitespace().next())
        && let Ok(ms) = first_tok.parse::<u64>()
    {
        timeout_ms = ms.saturating_add(2500).clamp(2000, 125_000);
    }

    Ok((raw_cmd, Duration::from_millis(timeout_ms)))
}
