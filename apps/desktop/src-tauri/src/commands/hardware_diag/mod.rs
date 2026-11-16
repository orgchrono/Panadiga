//! Panasonic DMR-EH55 DIGA Suite - SSOT, Granular Diagnostics & Reactive Streaming Commands
//!
//! Modularized by SRP:
//! - `capture`: Live in-circuit NVRAM sector chunking & inspection
//! - `telemetry`: Managed background port observer and live telemetry streaming
//! - Root module: Hardware SSOT DTO, canonical reply parser, and `pico_execute_diagnostic`

mod capture;
mod telemetry;

pub use capture::pico_capture_nvram_live;
pub use telemetry::{set_pico_telemetry_stream, start_port_mutation_observer};

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;

use crate::commands::constants::{
    AM34_NVRAM_BASE, DEFAULT_DUMP_DIAGNOSTIC_CMD, EVENT_PICO_DIAG_RESULT, EVENT_PICO_SERIAL_LOG,
    EVENT_PICO_TELEMETRY, SOURCE_PICO_DIAG_DUMP, STATUS_REFRESH_COMMANDS,
    VDD_TARGET_FLOATING_CEIL_VOLTS, VDD_TARGET_MIN_VOLTS, VDD_ZERO_EPSILON_VOLTS,
};
use crate::commands::dto::{HardwareSsotSpecDto, build_hardware_ssot_spec};
use crate::commands::error::IpcResult;
use crate::pico::diagnostics::{
    PicoDiagnosticResultDto, execute_diagnostic_on_port, parse_diagnostic_output,
};
use crate::state::{PanadigaBackendState, format_utc_hms};

/// Real-time serial log entry pushed over Tauri event `"pico://serial-log"`.
#[derive(Debug, Clone, Serialize)]
pub struct ReactiveSerialEventDto {
    pub id: String,
    pub timestamp: String,
    pub source: String,
    pub level: String,
    pub message: String,
}

fn epoch_millis_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Returns the complete Hardware & Firmware Single Source of Truth (SSOT) specification.
#[tauri::command]
pub fn get_hardware_ssot_spec() -> IpcResult<HardwareSsotSpecDto> {
    Ok(build_hardware_ssot_spec())
}

/// Canonical diagnostic reply parser exposed over IPC so Android BLE / USB-OTG and Desktop CDC share `panadiga-core`.
#[tauri::command]
pub fn pico_parse_diagnostic_output(
    command_id: String,
    raw_command: String,
    raw_output: String,
    elapsed_ms: u64,
) -> IpcResult<PicoDiagnosticResultDto> {
    Ok(parse_diagnostic_output(
        &command_id,
        &raw_command,
        &raw_output,
        elapsed_ms,
    ))
}

/// Executes any granular Pico firmware command asynchronously without stalling the WebView UI,
/// pushing TX/RX logs, structured diagnostic output, and updated telemetry over Tauri events.
#[tauri::command]
pub async fn pico_execute_diagnostic(
    app: tauri::AppHandle,
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    command_id: String,
    arg: Option<String>,
) -> IpcResult<PicoDiagnosticResultDto> {
    let backend = state.inner().clone();
    let cmd_preview = match arg.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => format!("{} {}", command_id.trim().to_ascii_uppercase(), a),
        None => command_id.trim().to_ascii_uppercase(),
    };

    let _ = app.emit(
        EVENT_PICO_SERIAL_LOG,
        ReactiveSerialEventDto {
            id: format!("tx-{}", epoch_millis_now()),
            timestamp: format_utc_hms(),
            source: "HOST".to_string(),
            level: "tx".to_string(),
            message: format!("> {}", cmd_preview),
        },
    );

    let port_clone = port_name.clone();
    let cmd_clone = command_id.clone();
    let arg_clone = arg.clone();
    let exec_backend = backend.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        execute_diagnostic_on_port(&exec_backend, &port_clone, &cmd_clone, arg_clone.as_deref())
    })
    .await
    .map_err(|e| format!("Falha interna ao aguardar diagnóstico Pico: {}", e))??;

    let _ = app.emit(EVENT_PICO_DIAG_RESULT, &result);
    if result.command_id == DEFAULT_DUMP_DIAGNOSTIC_CMD && result.success {
        let bytes = panadiga_core::pico::cdc::parse_pico_dump_hex_to_bytes(&result.raw_output);
        if !bytes.is_empty() {
            let base_addr = result
                .dump_rows
                .first()
                .and_then(|row| {
                    u32::from_str_radix(
                        row.address
                            .trim_start_matches("0x")
                            .trim_start_matches("0X"),
                        16,
                    )
                    .ok()
                })
                .unwrap_or(AM34_NVRAM_BASE);
            let _ = capture::build_and_emit_live_nvram_capture(
                &app,
                SOURCE_PICO_DIAG_DUMP,
                &port_name,
                base_addr,
                &bytes,
            );
        }
    }
    let _ = app.emit(
        EVENT_PICO_SERIAL_LOG,
        ReactiveSerialEventDto {
            id: format!("rx-{}", epoch_millis_now()),
            timestamp: format_utc_hms(),
            source: "PICO".to_string(),
            level: if result.success { "rx" } else { "error" }.to_string(),
            message: result.summary.clone(),
        },
    );

    if STATUS_REFRESH_COMMANDS.contains(&result.command_id.as_str()) {
        let status_backend = backend.clone();
        let status_port = port_name.clone();
        if let Ok(mut status) = tauri::async_runtime::spawn_blocking(move || {
            crate::pico::pico_get_device_status_dto(&status_backend, status_port, None)
        })
        .await
        .unwrap_or_else(|_| Err(String::new()))
        {
            if let Some(diag_vdd) = result.vdd_volts
                && status.vdd_volts <= VDD_ZERO_EPSILON_VOLTS
                && diag_vdd > VDD_ZERO_EPSILON_VOLTS
            {
                status.vdd_volts = diag_vdd;
                status.target_powered =
                    (VDD_TARGET_MIN_VOLTS..VDD_TARGET_FLOATING_CEIL_VOLTS).contains(&diag_vdd);
                if status.target_powered && status.core_state == "RESET" {
                    status.core_state = "RUNNING".to_string();
                }
            }
            let _ = app.emit(EVENT_PICO_TELEMETRY, &status);
        }
    }

    Ok(result)
}
