//! Low-level USB CDC transport, mutex synchronization, port discovery, and multi-chunk reader.

use serialport::SerialPortType;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use panadiga_core::pico::cdc::strip_leading_prompts;

use super::super::bridge_session::is_sc0_bridge_active;
use super::super::gdb_bridge::is_gdb_bridge_active;
use super::super::types::extract_port_name;

static SERIAL_IO_MUTEX: Mutex<()> = Mutex::new(());

/// Acquires the foreground serial I/O mutex cleanly (recovering from any poisoned lock).
pub fn lock_serial_io() -> MutexGuard<'static, ()> {
    SERIAL_IO_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn try_lock_serial_io() -> Option<MutexGuard<'static, ()>> {
    SERIAL_IO_MUTEX.try_lock().ok()
}

/// Verifies that the serial port is not currently owned by an active GDB RSP or SC0 bridge thread.
pub fn ensure_serial_port_unlocked(clean_port: &str) -> Result<(), String> {
    crate::pico::types::reject_mock_port(clean_port)?;
    if is_gdb_bridge_active() || is_sc0_bridge_active() {
        return Err(format!(
            "Porta serial {} ocupada pela Ponte GDB/SC0 ativa. Encerre a sessão antes de enviar comandos CDC.",
            clean_port
        ));
    }
    Ok(())
}

fn serial_response_has_completed(out: &str) -> bool {
    let body = strip_leading_prompts(out);
    if body.is_empty() {
        return false;
    }
    const COMPLETION_MARKERS: &[&str] = &[
        "PANADIGA>",
        "P7501>",
        "DUMP: OK",
        "PINCHECK: OK",
        "BUSDIAG: OK",
        "CFI: OK",
        "CFI: FAILED",
        "BOOTCAP: OK",
        "UARTMON: OK",
        "JIGBOOT: OK",
        "UARTSEND: OK",
        "UARTPROBE: OK",
        "BAUD: OK",
        "BAUD: ERROR",
        "FRAMING: OK",
        "FRAMING: ERROR",
        "BAUDSCAN: OK",
        "BAUDSCAN: ERROR",
        "UARTSTREAM: OK",
        "HEXCAP: OK",
        "UARTTXHEX: OK",
        "UARTTXHEX: ERROR",
        "WAITTX: OK",
        "WAITTX: ERROR",
        "PWRTRACE: OK",
        "BOOTTRACE: OK",
        "PIOLOGIC: OK",
        "PIOLOGIC: ERROR",
        "JIGSWEEP: OK",
        "JIGSWEEP: ERROR",
        "DIAGWATCH: OK",
        "PANARSP: OK",
        "PANARSP: ERROR",
        "BRIDGE: OK",
        "BRIDGE: ERROR",
        "LED: OK",
        "LED: ERROR",
        "FMUSCAN: OK",
        "FMUTEST: OK",
        "FMUBATCH: OK",
        "ERASE_NVRAM: OK",
        "ERASE_NVRAM: ERROR",
        "INJECT_LOADER: OK",
        "INJECT_LOADER: ERROR",
        "SAFETY INTERLOCK",
        "BREAK_IN: OK",
        "SCAN: COMPLETED",
        "SNIFF: OK",
        "ISOLATE: OK",
        "ENGAGE: OK",
        "AUTODETECT: OK",
        "FLASH_SIZE: OK",
        "FLASH_SIZE: ERROR",
        "[HANDSHAKE] SUCCESS",
        "[HANDSHAKE] FAILED",
        "[HANDSHAKE] ABORT",
        "[HANDSHAKE] ACTIVE",
        "Auto-ready",
        "PONG",
        "VDD:",
        "VDD=",
    ];
    COMPLETION_MARKERS
        .iter()
        .any(|marker| body.contains(marker))
}

/// Reads multi-chunk USB CDC serial output until a terminal prompt or completion marker.
pub fn read_serial_response_complete(
    port: &mut dyn serialport::SerialPort,
    max_wait: Duration,
) -> Result<String, String> {
    let _ = port.set_timeout(Duration::from_millis(180));
    let start = Instant::now();
    let mut out = String::new();
    let mut chunk = [0u8; 1024];

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
                    && start.elapsed() > Duration::from_millis(350)
                {
                    break;
                }
            }
            Err(e) => return Err(format!("Erro de leitura USB CDC: {}", e)),
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
pub fn list_pico_serial_ports() -> Result<Vec<String>, String> {
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
    port_name: &str,
    baud_rate: u32,
    timeout_ms: u64,
) -> Result<(String, Box<dyn serialport::SerialPort>), String> {
    let clean = extract_port_name(port_name);
    crate::pico::types::reject_mock_port(&clean)?;
    ensure_serial_port_unlocked(&clean)?;
    let port =
        panadiga_core::pico::cdc::open_cdc(&clean, baud_rate, Duration::from_millis(timeout_ms))
            .map_err(|e| format!("Falha ao abrir porta serial {}: {}", clean, e))?;
    Ok((clean, port))
}
