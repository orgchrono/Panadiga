//! Panasonic DMR-EH55 DIGA Suite - Pico Hardware & GDB Bridge Commands
//!
//! Single Responsibility: Tauri IPC command dispatchers for RP2040/RP2350 USB CDC
//! communication, Cold Break-In sequencing, In-Circuit NVRAM erasure, and GDB RSP bridge lifecycle.

use crate::state::PanadigaBackendState;

use super::dto::{
    Am34RegisterState, DigaModelProfileDto, FirstContactBackupResult, GdbBridgeStatus,
    GdbPacketTelemetryEvent, InCircuitNvramEraseResult, PicoDeviceStatusDto, PicoStatusResponse,
    get_diga_model_catalog_dto,
};
use super::error::{IpcResult, PanadigaIpcError};

pub(super) async fn run_blocking<T, F>(work: F) -> IpcResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> IpcResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| {
            PanadigaIpcError::internal(format!("Falha interna de execução assíncrona: {}", e))
        })?
}

/// Returns the internal Panasonic DIGA Model Database (2MB/4MB/8MB/16MB NOR Flash profiles).
#[tauri::command]
pub fn get_diga_model_catalog() -> IpcResult<Vec<DigaModelProfileDto>> {
    Ok(get_diga_model_catalog_dto())
}

/// Enumerates connected USB CDC serial ports for Raspberry Pi Pico hardware.
#[tauri::command]
pub async fn list_pico_serial_ports() -> IpcResult<Vec<String>> {
    run_blocking(super::super::pico::list_pico_serial_ports).await
}

/// Connects to a Pico serial COM port and reads device telemetry and VDD voltage.
#[tauri::command]
pub async fn pico_connect_and_test(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    baud_rate: u32,
) -> IpcResult<PicoStatusResponse> {
    let backend = state.inner().clone();
    run_blocking(move || super::super::pico::pico_connect_and_test(&backend, port_name, baud_rate))
        .await
}

/// Triggers P7501 discrete boot/reset sequence on the RP2040/RP2350 hardware probe.
#[tauri::command]
pub async fn pico_execute_cold_break_in(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    mode: u8,
) -> IpcResult<String> {
    let backend = state.inner().clone();
    run_blocking(move || super::super::pico::pico_execute_cold_break_in(&backend, port_name, mode))
        .await
}

/// Executes in-circuit surgical sector erase of NVRAM micro-sectors on Spansion S29JL064H.
#[tauri::command]
pub async fn pico_execute_in_circuit_erase(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    nonce_id: Option<String>,
) -> IpcResult<String> {
    let target = if port_name.trim().is_empty() {
        "auto"
    } else {
        port_name.trim()
    };
    crate::commands::hardware_diag::require_destructive_nonce(
        state.inner(),
        nonce_id.as_deref(),
        panadiga_core::bench::OpKind::NorWrite,
        target,
        "in_circuit_erase",
    )?;
    let backend = state.inner().clone();
    run_blocking(move || super::super::pico::pico_execute_in_circuit_erase(&backend, port_name))
        .await
}

/// Switches the Pico firmware into GDB Remote Serial Protocol bridge mode.
#[tauri::command]
pub async fn pico_switch_to_gdb_mode(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
) -> IpcResult<String> {
    let backend = state.inner().clone();
    run_blocking(move || super::super::pico::pico_switch_to_gdb_mode(&backend, port_name)).await
}

/// Frontend `pico_get_status` command.
#[tauri::command]
pub async fn pico_get_status(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: Option<String>,
) -> IpcResult<PicoDeviceStatusDto> {
    let backend = state.inner().clone();
    let port = port_name.unwrap_or_default();
    run_blocking(move || super::super::pico::pico_get_device_status_dto(&backend, port, None)).await
}

/// Frontend `pico_set_breakin_mode` command.
#[tauri::command]
pub async fn pico_set_breakin_mode(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: Option<String>,
    mode: u8,
) -> IpcResult<PicoDeviceStatusDto> {
    let backend = state.inner().clone();
    let port = port_name.unwrap_or_default();
    run_blocking(move || {
        super::super::pico::pico_execute_cold_break_in(&backend, port.clone(), mode)?;
        super::super::pico::pico_get_device_status_dto(&backend, port, Some(mode))
    })
    .await
}

/// Frontend `pico_inject_ram_loader` command (Handshake + Backup + INJECT_LOADER CONFIRM + ERASE_NVRAM CONFIRM).
#[tauri::command]
pub async fn pico_inject_ram_loader(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: Option<String>,
    nonce_id: Option<String>,
) -> IpcResult<InCircuitNvramEraseResult> {
    let port = port_name.unwrap_or_default();
    let target = if port.trim().is_empty() {
        "auto"
    } else {
        port.trim()
    };
    crate::commands::hardware_diag::require_destructive_nonce(
        state.inner(),
        nonce_id.as_deref(),
        panadiga_core::bench::OpKind::RamProbe,
        target,
        "ram_loader",
    )?;
    let backend = state.inner().clone();
    run_blocking(move || super::super::pico::pico_execute_in_circuit_erase_full(&backend, port))
        .await
}

/// Non-destructive 1st-Contact Read-Only Backup & Auto-Model Probe ("What's Next" protocol).
#[tauri::command]
pub async fn pico_first_contact_backup(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: Option<String>,
) -> IpcResult<FirstContactBackupResult> {
    let backend = state.inner().clone();
    let port = port_name.unwrap_or_default();
    run_blocking(move || super::super::pico::pico_execute_first_contact_backup(&backend, port))
        .await
}

/// Frontend `pico_send_serial_command` command for interactive USB CDC terminal.
#[tauri::command]
pub async fn pico_send_serial_command(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: Option<String>,
    command: String,
) -> IpcResult<String> {
    let backend = state.inner().clone();
    let port = port_name.unwrap_or_default();
    run_blocking(move || super::super::pico::pico_send_raw_serial_command(&backend, port, command))
        .await
}

/// Discovers any currently mounted Raspberry Pi BOOTSEL drives (RPI-RP2 / RP2350).
#[tauri::command]
pub async fn pico_detect_bootsel_drives() -> IpcResult<Vec<String>> {
    run_blocking(|| {
        let drives = panadiga_core::pico::flasher::detect_bootsel_drives();
        Ok(drives
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect())
    })
    .await
}

/// Triggers 1200-baud touch reset on a running Pico to put it into BOOTSEL mode.
#[tauri::command]
pub async fn pico_trigger_bootsel_reboot(port_name: String) -> IpcResult<String> {
    run_blocking(move || {
        panadiga_core::pico::flasher::trigger_1200_baud_bootsel(&port_name)
            .map_err(|e| PanadigaIpcError::serial(e.to_string()))?;
        Ok(format!(
            "Port {} triggered 1200-baud reset into BOOTSEL mode",
            port_name
        ))
    })
    .await
}

/// Flashes the embedded P7501 Async UART firmware (RP2040 or RP2350) onto a detected BOOTSEL drive.
#[tauri::command]
pub async fn pico_flash_firmware(
    state: tauri::State<'_, PanadigaBackendState>,
    target_family: Option<String>,
    nonce_id: Option<String>,
) -> IpcResult<panadiga_core::pico::flasher::PicoFlashReport> {
    let target = target_family
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("auto");
    crate::commands::hardware_diag::require_destructive_nonce(
        state.inner(),
        nonce_id.as_deref(),
        panadiga_core::bench::OpKind::NorWrite,
        target,
        "pico_uf2",
    )?;
    run_blocking(move || {
        panadiga_core::pico::flasher::flash_embedded_firmware(target_family.as_deref())
            .map_err(PanadigaIpcError::from)
    })
    .await
}

/// Starts the GDB TCP-to-Serial bridge server.
#[tauri::command]
pub fn start_gdb_bridge_command(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    listen_addr: String,
) -> IpcResult<GdbBridgeStatus> {
    super::super::pico::start_gdb_bridge(state.inner(), port_name, listen_addr)
}

/// Stops the GDB TCP-to-Serial bridge server.
#[tauri::command]
pub fn stop_gdb_bridge_command(
    state: tauri::State<'_, PanadigaBackendState>,
) -> IpcResult<GdbBridgeStatus> {
    super::super::pico::stop_gdb_bridge(state.inner())
}

/// Retrieves the current status of the GDB bridge server.
#[tauri::command]
pub fn get_gdb_bridge_status_command(
    state: tauri::State<'_, PanadigaBackendState>,
) -> IpcResult<GdbBridgeStatus> {
    super::super::pico::get_gdb_bridge_status(state.inner())
}

/// Retrieves the current AM34 CPU register file state.
#[tauri::command]
pub fn get_am34_registers_command(
    state: tauri::State<'_, PanadigaBackendState>,
) -> IpcResult<Am34RegisterState> {
    super::super::pico::get_am34_registers(state.inner())
}

/// Retrieves recorded GDB RSP packet telemetry events from the introspective ring buffer.
#[tauri::command]
pub fn get_gdb_telemetry_command(
    state: tauri::State<'_, PanadigaBackendState>,
) -> IpcResult<Vec<GdbPacketTelemetryEvent>> {
    Ok(super::super::pico::get_gdb_telemetry(state.inner()))
}
