//! Tauri IPC commands for Spansion S29JL064H NVRAM inspection, cleaning, and Copy-on-Write patching.

use std::path::Path;

use crate::commands::constants::{AM34_NVRAM_BASE, DEFAULT_LIVE_NVRAM_LABEL};
use crate::commands::dto::{NvramCleanReport, resolve_default_flash_path};
use crate::commands::error::{IpcResult, PanadigaIpcError};

/// Surgically cleans and audits the Spansion S29JL064H flash dump NVRAM emulation partitions.
#[tauri::command]
pub fn clean_nvram_command(
    input_path: String,
    output_path: Option<String>,
) -> IpcResult<NvramCleanReport> {
    let in_p = Path::new(&input_path);
    let out_p = output_path.as_deref().map(Path::new);
    panadiga_core::nvram::clean_flash_nvram(in_p, out_p).map_err(PanadigaIpcError::from)
}

/// Frontend `analyze_flash_dump` command (non-destructive inspection).
#[tauri::command]
pub fn analyze_flash_dump(input_path: Option<String>) -> IpcResult<NvramCleanReport> {
    let path = resolve_default_flash_path(input_path);
    panadiga_core::nvram::inspect_flash_nvram(Path::new(&path)).map_err(PanadigaIpcError::from)
}

/// Frontend `clean_nvram` command (surgical 64 KB NVRAM reset to 0xFF).
#[tauri::command]
pub fn clean_nvram(
    input_path: Option<String>,
    output_path: Option<String>,
) -> IpcResult<NvramCleanReport> {
    let path = resolve_default_flash_path(input_path);
    let in_p = Path::new(&path);
    let out = output_path.unwrap_or_else(|| {
        let parent = in_p.parent().unwrap_or_else(|| Path::new("."));
        let stem = in_p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("flash_dump");
        parent
            .join(format!("{stem}_CLEAN.bin"))
            .to_string_lossy()
            .to_string()
    });
    clean_nvram_command(path, Some(out))
}

/// Guarded copy-on-write patch within the 64 KB writable NVRAM sector (`SA134..SA141`); Spansion IPL stays read-only.
#[tauri::command]
pub fn patch_nvram_command(
    input_path: String,
    output_path: Option<String>,
    nvram_offset: u32,
    patch_hex: String,
) -> IpcResult<NvramCleanReport> {
    let patch_bytes = panadiga_core::re::matshita_emu::parse_hex_payload(patch_hex.trim())
        .map_err(|e| {
            PanadigaIpcError::validation(format!("Payload hex inválido para NVRAM: {e}"))
        })?;
    let in_p = Path::new(input_path.trim());
    let out_trimmed = output_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let out_p = out_trimmed.map(Path::new);
    panadiga_core::nvram::patch_flash_nvram(in_p, out_p, nvram_offset as usize, &patch_bytes)
        .map_err(PanadigaIpcError::from)
}

/// Audits a live NVRAM byte slice captured in-circuit via Pico (P7501 UART) / Android USB-OTG / BLE NUS.
#[tauri::command]
pub fn analyze_live_nvram_bytes(
    bytes: Vec<u8>,
    base_bus_address: Option<u32>,
    source_label: Option<String>,
) -> IpcResult<panadiga_core::nvram::NvramCleanReport> {
    let base_addr = base_bus_address.unwrap_or(AM34_NVRAM_BASE);
    let label = source_label.unwrap_or_else(|| DEFAULT_LIVE_NVRAM_LABEL.to_string());
    panadiga_core::nvram::inspect_partial_nvram_capture(&bytes, base_addr, &label)
        .map_err(PanadigaIpcError::from)
}
