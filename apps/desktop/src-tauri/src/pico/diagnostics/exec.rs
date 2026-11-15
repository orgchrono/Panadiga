//! USB CDC execution runner for validated Pico P7501 diagnostic commands.

use std::io::Write;
use std::time::{Duration, Instant};

use crate::pico::serial::{ensure_serial_port_unlocked, read_serial_response_complete};
use crate::pico::types::extract_port_name;

use super::command::{PicoDiagnosticResultDto, compose_diagnostic_command};
use super::parser::parse_diagnostic_output;

/// Executes a granular diagnostic command against the connected USB CDC Pico.
pub fn execute_diagnostic_on_port(
    port_name: &str,
    command_id: &str,
    arg: Option<&str>,
) -> Result<PicoDiagnosticResultDto, String> {
    let (raw_cmd, timeout) = compose_diagnostic_command(command_id, arg)?;
    let clean = extract_port_name(port_name);
    let started = Instant::now();

    crate::pico::types::reject_mock_port(&clean)?;

    if clean.starts_with("BLE") || clean.starts_with("ANDROID-USB-OTG") {
        return Err(
            "Comandos em modo BLE NUS / USB-OTG Android devem ser despachados diretamente pela camada móvel."
                .to_string(),
        );
    }

    let _io_guard = crate::pico::serial::lock_serial_io();
    ensure_serial_port_unlocked(&clean)?;
    let mut port = panadiga_core::pico::cdc::open_cdc(&clean, 115200, Duration::from_millis(4000))
        .map_err(|e| format!("Falha ao abrir porta '{}': {}", clean, e))?;

    let wire = format!("{}\r\n", raw_cmd);
    port.write_all(wire.as_bytes())
        .map_err(|e| format!("Falha ao enviar '{}' para Pico: {}", raw_cmd, e))?;
    let _ = port.flush();

    let raw_out = read_serial_response_complete(&mut *port, timeout)?;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(parse_diagnostic_output(
        command_id, &raw_cmd, &raw_out, elapsed_ms,
    ))
}
