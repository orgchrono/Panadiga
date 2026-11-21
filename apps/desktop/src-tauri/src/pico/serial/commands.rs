//! Foreground serial command dispatchers (raw CDC, Break-In, and GDB bridge mode).

use std::io::Write;
use std::time::Duration;

use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::state::PanadigaBackendState;

use super::super::types::extract_port_name;
use super::transport::{open_validated_cdc, read_serial_response_complete};

/// Sends an arbitrary interactive command to the USB CDC serial port and returns the response.
pub fn pico_send_raw_serial_command(
    backend: &PanadigaBackendState,
    port_name: String,
    command: String,
) -> IpcResult<String> {
    let clean = extract_port_name(&port_name);
    let trimmed = command.trim().to_uppercase();

    backend.with_exclusive_serial(&clean, || {
        if trimmed == "ISOLATE" || trimmed == "HIZ" {
            backend.telemetry.set_ram_loader_injected(false);
        }

        let mut port =
            panadiga_core::pico::cdc::open_cdc(&clean, 115_200, Duration::from_millis(2000))
                .map_err(|e| {
                    PanadigaIpcError::serial(format!(
                        "Falha ao abrir porta serial {}: {}",
                        clean, e
                    ))
                })?;

        let payload = format!("{}\r\n", trimmed);
        port.write_all(payload.as_bytes()).map_err(|e| {
            PanadigaIpcError::serial(format!("Falha ao transmitir comando CDC: {}", e))
        })?;
        let _ = port.flush();

        let response = read_serial_response_complete(&mut *port, Duration::from_millis(8000))?;

        if trimmed.starts_with("IDENTIFY")
            && let Some(ident) = panadiga_core::pico::parse_ident_line(&response)
        {
            backend
                .telemetry
                .record_scan_telemetry(Some(4), Some(u32::from(ident.nsw5v)));
        }

        Ok(response)
    })
}

/// Executes Cold Break-In sequence on the DMR-EH55 via RP2040 PIO state machine.
pub fn pico_execute_cold_break_in(
    backend: &PanadigaBackendState,
    port_name: String,
    mode: u8,
) -> IpcResult<String> {
    backend.serial.with_lock(|| {
        let (_, mut port) = open_validated_cdc(backend, &port_name, 115_200, 3000)?;

        let cmd = format!("BREAK_IN {}\r\n", mode);
        port.write_all(cmd.as_bytes()).map_err(|e| {
            PanadigaIpcError::serial(format!("Falha ao enviar comando BREAK_IN: {}", e))
        })?;
        let _ = port.flush();

        read_serial_response_complete(&mut *port, Duration::from_millis(3000))
    })
}

/// Switches the Pico firmware into GDB Remote Serial Protocol (RSP) bridge mode.
pub fn pico_switch_to_gdb_mode(
    backend: &PanadigaBackendState,
    port_name: String,
) -> IpcResult<String> {
    let clean = extract_port_name(&port_name);

    backend.with_exclusive_serial(&clean, || {
        let mut port =
            panadiga_core::pico::cdc::open_cdc(&clean, 115_200, Duration::from_millis(2000))
                .map_err(|e| {
                    PanadigaIpcError::serial(format!(
                        "Falha ao conectar na porta serial {}: {}",
                        clean, e
                    ))
                })?;

        port.write_all(b"GDB\r\n")
            .map_err(|e| PanadigaIpcError::serial(format!("Falha ao enviar comando GDB: {}", e)))?;
        let _ = port.flush();

        read_serial_response_complete(&mut *port, Duration::from_millis(2000))
    })
}
