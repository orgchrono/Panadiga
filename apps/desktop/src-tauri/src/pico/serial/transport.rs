//! Low-level USB CDC transport, port discovery, and multi-chunk reader.

use serialport::SerialPortType;
use std::time::{Duration, Instant};

use panadiga_core::pico::cdc::strip_leading_prompts;

use crate::commands::constants::{
    COMPLETION_MARKERS, SERIAL_CHUNK_READ_TIMEOUT_MS, SERIAL_IDLE_COMPLETE_MS,
    SERIAL_READ_CHUNK_BYTES,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::state::PanadigaBackendState;

use super::super::types::extract_port_name;

/// Verifies that the serial port is not currently owned by an active GDB RSP or SC0 bridge thread.
pub fn ensure_serial_port_unlocked(
    backend: &PanadigaBackendState,
    clean_port: &str,
) -> IpcResult<()> {
    backend.ensure_serial_port_unlocked(clean_port)
}

fn serial_response_has_completed(out: &str) -> bool {
    let body = strip_leading_prompts(out);
    if body.is_empty() {
        return false;
    }
    COMPLETION_MARKERS
        .iter()
        .any(|marker| body.contains(marker))
}

/// Reads multi-chunk USB CDC serial output until a terminal prompt or completion marker.
pub fn read_serial_response_complete(
    port: &mut dyn serialport::SerialPort,
    max_wait: Duration,
) -> IpcResult<String> {
    let _ = port.set_timeout(Duration::from_millis(SERIAL_CHUNK_READ_TIMEOUT_MS));
    let start = Instant::now();
    let mut out = String::new();
    let mut chunk = [0u8; SERIAL_READ_CHUNK_BYTES];

    while start.elapsed() < max_wait {
        match port.read(&mut chunk) {
            Ok(n) if n > 0 => {
                out.push_str(&String::from_utf8_lossy(&chunk[..n]));
                if serial_response_has_completed(&out) {
                    break;
                }
            }
            Ok(_) => {}
            Err(ref e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                if !strip_leading_prompts(&out).is_empty()
                    && start.elapsed() > Duration::from_millis(SERIAL_IDLE_COMPLETE_MS)
                {
                    break;
                }
            }
            Err(e) => {
                return Err(PanadigaIpcError::serial(format!(
                    "Erro de leitura USB CDC: {}",
                    e
                )));
            }
        }
    }
    let clean = strip_leading_prompts(&out).trim();
    if clean.is_empty() {
        Ok(out.trim().to_string())
    } else {
        Ok(clean.to_string())
    }
}

/// Lists all available USB CDC serial COM ports, tagging recognized RP2040 and RP2350 hardware
/// while skipping BOOTSEL mass-storage entries.
pub fn list_pico_serial_ports() -> IpcResult<Vec<String>> {
    let mut ports = Vec::new();
    if let Ok(available) = serialport::available_ports() {
        for p in available {
            let mut label = p.port_name.clone();
            if let SerialPortType::UsbPort(info) = &p.port_type {
                if info.pid == panadiga_core::pico::platforms::PICO_BOOTSEL_PID {
                    continue;
                }
                let (_plat, desc) = panadiga_core::pico::classify_usb_device(
                    info.vid,
                    info.pid,
                    info.product.as_deref(),
                );
                label = format!(
                    "{} [{} - VID:{:04X} PID:{:04X}]",
                    p.port_name, desc, info.vid, info.pid
                );
            }
            ports.push(label);
        }
    }
    Ok(ports)
}

pub(super) fn open_validated_cdc(
    backend: &PanadigaBackendState,
    port_name: &str,
    baud_rate: u32,
    timeout_ms: u64,
) -> IpcResult<(String, Box<dyn serialport::SerialPort>)> {
    let clean = extract_port_name(port_name);
    backend.ensure_serial_port_unlocked(&clean)?;
    let port =
        panadiga_core::pico::cdc::open_cdc(&clean, baud_rate, Duration::from_millis(timeout_ms))
            .map_err(|e| {
                PanadigaIpcError::serial(format!("Falha ao abrir porta serial {}: {}", clean, e))
            })?;
    Ok((clean, port))
}
