//! Tauri IPC Commands for Canonical Rust Runtime Settings & Unified Frontend Telemetry.

use panadiga_core::logger::emit_frontend_telemetry;
use panadiga_core::settings::{
    RuntimeSettings, load_runtime_settings, reset_runtime_settings as core_reset_settings,
    save_runtime_settings as core_save_settings,
};

use super::error::{IpcResult, PanadigaIpcError};

/// Returns persisted runtime operational settings (or hardware-safe factory defaults).
#[tauri::command]
pub fn get_runtime_settings() -> IpcResult<RuntimeSettings> {
    Ok(load_runtime_settings())
}

/// Validates and atomically persists updated runtime operational settings in Rust config dir.
#[tauri::command]
pub fn save_runtime_settings(settings: RuntimeSettings) -> IpcResult<RuntimeSettings> {
    core_save_settings(&settings).map_err(|err| PanadigaIpcError::validation(err.to_string()))
}

/// Restores runtime operational settings to factory defaults in Rust config dir.
#[tauri::command]
pub fn reset_runtime_settings() -> IpcResult<RuntimeSettings> {
    core_reset_settings().map_err(|err| PanadigaIpcError::io(err.to_string()))
}

/// Routes structured frontend Pino/OpenTelemetry events into the unified Rust `tracing` + OTel pipeline.
#[tauri::command]
pub fn log_frontend_event(
    severity: String,
    subsystem: String,
    message: String,
    details: Option<String>,
) -> IpcResult<()> {
    emit_frontend_telemetry(&severity, &subsystem, &message, details.as_deref());
    Ok(())
}
