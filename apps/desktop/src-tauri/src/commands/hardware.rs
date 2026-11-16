//! Panasonic DMR-EH55 DIGA Suite - Pico Hardware & GDB Bridge Commands
//!
//! Single Responsibility: Tauri IPC command dispatchers for RP2040/RP2350 USB CDC
//! communication, Cold Break-In sequencing, In-Circuit NVRAM erasure, and GDB RSP bridge lifecycle.

use super::dto::{
    Am34RegisterState, DigaModelProfileDto, FirstContactBackupResult, GdbBridgeStatus,
    GdbPacketTelemetryEvent, InCircuitNvramEraseResult, PicoDeviceStatusDto, PicoStatusResponse,
    get_diga_model_catalog_dto,
};

pub(super) async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| format!("Falha interna de execução assíncrona: {}", e))?
}

/// Returns the internal Panasonic DIGA Model Database (2MB/4MB/8MB/16MB NOR Flash profiles).
#[tauri::command]
pub fn get_diga_model_catalog() -> Result<Vec<DigaModelProfileDto>, String> {
    Ok(get_diga_model_catalog_dto())
}

/// Enumerates connected USB CDC serial ports for Raspberry Pi Pico hardware.
#[tauri::command]
pub async fn list_pico_serial_ports() -> Result<Vec<String>, String> {
    run_blocking(super::super::pico::list_pico_serial_ports).await
}

/// Connects to a Pico serial COM port and reads device telemetry and VDD voltage.
#[tauri::command]
pub async fn pico_connect_and_test(
    port_name: String,
    baud_rate: u32,
) -> Result<PicoStatusResponse, String> {
    run_blocking(move || super::super::pico::pico_connect_and_test(port_name, baud_rate)).await
}

/// Triggers P7501 discrete boot/reset sequence on the RP2040/RP2350 hardware probe.
#[tauri::command]
pub async fn pico_execute_cold_break_in(port_name: String, mode: u8) -> Result<String, String> {
    run_blocking(move || super::super::pico::pico_execute_cold_break_in(port_name, mode)).await
}

/// Executes in-circuit surgical sector erase of NVRAM micro-sectors on Spansion S29JL064H.
#[tauri::command]
pub async fn pico_execute_in_circuit_erase(port_name: String) -> Result<String, String> {
    run_blocking(move || super::super::pico::pico_execute_in_circuit_erase(port_name)).await
}

/// Switches the Pico firmware into GDB Remote Serial Protocol bridge mode.
#[tauri::command]
pub async fn pico_switch_to_gdb_mode(port_name: String) -> Result<String, String> {
    run_blocking(move || super::super::pico::pico_switch_to_gdb_mode(port_name)).await
}

/// Frontend `pico_get_status` command.
#[tauri::command]
pub async fn pico_get_status(port_name: Option<String>) -> Result<PicoDeviceStatusDto, String> {
    let port = port_name.unwrap_or_default();
    run_blocking(move || super::super::pico::pico_get_device_status_dto(port, None)).await
}

/// Frontend `pico_set_breakin_mode` command.
#[tauri::command]
pub async fn pico_set_breakin_mode(
    port_name: Option<String>,
    mode: u8,
) -> Result<PicoDeviceStatusDto, String> {
    let port = port_name.unwrap_or_default();
    run_blocking(move || {
        super::super::pico::pico_execute_cold_break_in(port.clone(), mode)?;
        super::super::pico::pico_get_device_status_dto(port, Some(mode))
    })
    .await
}

/// Frontend `pico_inject_ram_loader` command (Handshake + Backup + INJECT_LOADER CONFIRM + ERASE_NVRAM CONFIRM).
#[tauri::command]
pub async fn pico_inject_ram_loader(
    port_name: Option<String>,
) -> Result<InCircuitNvramEraseResult, String> {
    let port = port_name.unwrap_or_default();
    run_blocking(move || super::super::pico::pico_execute_in_circuit_erase_full(port)).await
}

/// Non-destructive 1st-Contact Read-Only Backup & Auto-Model Probe ("What's Next" protocol).
#[tauri::command]
pub async fn pico_first_contact_backup(
    port_name: Option<String>,
) -> Result<FirstContactBackupResult, String> {
    let port = port_name.unwrap_or_default();
    run_blocking(move || super::super::pico::pico_execute_first_contact_backup(port)).await
}

/// Frontend `pico_send_serial_command` command for interactive USB CDC terminal.
#[tauri::command]
pub async fn pico_send_serial_command(
    port_name: Option<String>,
    command: String,
) -> Result<String, String> {
    let port = port_name.unwrap_or_default();
    run_blocking(move || super::super::pico::pico_send_raw_serial_command(port, command)).await
}

/// Discovers any currently mounted Raspberry Pi BOOTSEL drives (RPI-RP2 / RP2350).
#[tauri::command]
pub async fn pico_detect_bootsel_drives() -> Result<Vec<String>, String> {
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
pub async fn pico_trigger_bootsel_reboot(port_name: String) -> Result<String, String> {
    run_blocking(move || {
        panadiga_core::pico::flasher::trigger_1200_baud_bootsel(&port_name)
            .map_err(|e| e.to_string())?;
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
    target_family: Option<String>,
) -> Result<panadiga_core::pico::flasher::PicoFlashReport, String> {
    run_blocking(move || {
        panadiga_core::pico::flasher::flash_embedded_firmware(target_family.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

/// Starts the GDB TCP-to-Serial bridge server.
#[tauri::command]
pub fn start_gdb_bridge_command(
    port_name: String,
    listen_addr: String,
) -> Result<GdbBridgeStatus, String> {
    super::super::pico::start_gdb_bridge(port_name, listen_addr)
}

/// Stops the GDB TCP-to-Serial bridge server.
#[tauri::command]
pub fn stop_gdb_bridge_command() -> Result<GdbBridgeStatus, String> {
    super::super::pico::stop_gdb_bridge()
}

/// Retrieves the current status of the GDB bridge server.
#[tauri::command]
pub fn get_gdb_bridge_status_command() -> Result<GdbBridgeStatus, String> {
    super::super::pico::get_gdb_bridge_status()
}

/// Retrieves the current AM34 CPU register file state.
#[tauri::command]
pub fn get_am34_registers_command() -> Result<Am34RegisterState, String> {
    super::super::pico::get_am34_registers()
}

/// Retrieves recorded GDB RSP packet telemetry events from the introspective ring buffer.
#[tauri::command]
pub fn get_gdb_telemetry_command() -> Result<Vec<GdbPacketTelemetryEvent>, String> {
    Ok(super::super::pico::get_gdb_telemetry())
}
