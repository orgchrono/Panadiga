//! Diagnostic result DTOs and SSOT command composition/validation.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::commands::constants::{
    DIAG_DURATION_SLACK_MS, DIAG_MAX_TIMEOUT_MS, DIAG_MIN_TIMEOUT_MS, VALID_BREAK_IN_MODES,
    VALID_FLASH_SIZE_ARGS, VALID_LED_MODE_ARGS, VARIABLE_DURATION_COMMANDS,
};
use crate::commands::dto::build_hardware_ssot_spec;

pub use panadiga_core::pico::diag_rows::{
    BusTransitionRowSpec as BusTransitionRowDto, DumpWordRowSpec as DumpWordRowDto,
    PincheckRowSpec as PincheckRowDto,
};

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
            && !VALID_BREAK_IN_MODES.contains(&m)
        {
            return Err(format!("Modo Break-In inválido '{}': use 0, 1, 2 ou 3.", m));
        }
    } else if normalized == "FLASH_SIZE" {
        if let Some(sz) = clean_arg
            && !VALID_FLASH_SIZE_ARGS.contains(&sz)
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
        if !VALID_LED_MODE_ARGS.contains(&upper.as_str()) {
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
    if VARIABLE_DURATION_COMMANDS.contains(&normalized.as_str())
        && let Some(first_tok) = clean_arg.and_then(|s| s.split_whitespace().next())
        && let Ok(ms) = first_tok.parse::<u64>()
    {
        timeout_ms = ms
            .saturating_add(DIAG_DURATION_SLACK_MS)
            .clamp(DIAG_MIN_TIMEOUT_MS, DIAG_MAX_TIMEOUT_MS);
    }

    Ok((raw_cmd, Duration::from_millis(timeout_ms)))
}
