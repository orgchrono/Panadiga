//! Panasonic DMR-EH55 DIGA Suite - GDB Remote Serial Protocol (RSP) Bridge Controller
//!
//! Imperative shell managing the background TCP-to-Serial proxy thread,
//! client connection lifecycle, and AM34 shadow register file state.
//! Pure RSP framing and validation reside in `rsp_protocol.rs`.

mod controller;
mod session;

pub use controller::GdbBridgeController;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use panadiga_core::constants::DEFAULT_GDB_LISTEN_SOCKET;

use crate::state::PanadigaBackendState;

use super::types::{
    Am34RegisterState, GdbBridgeStatus, GdbPacketTelemetryEvent, extract_port_name,
};
use session::{handle_local_stub_session, handle_serial_session};

/// Retrieves all recorded GDB packet telemetry events from managed state.
pub fn get_gdb_telemetry(backend: &PanadigaBackendState) -> Vec<GdbPacketTelemetryEvent> {
    backend.gdb_bridge.telemetry_snapshot()
}

fn open_serial_gdb_target(
    backend: &PanadigaBackendState,
    clean_port: &str,
) -> Result<(Option<Box<dyn serialport::SerialPort>>, String), String> {
    if clean_port.eq_ignore_ascii_case("LOCAL_STUB") || clean_port.eq_ignore_ascii_case("LOCAL") {
        return Ok((None, "LOCAL_STUB (AM34)".to_string()));
    }
    super::types::reject_mock_port(clean_port)?;

    let maybe_serial = backend.serial.with_lock(|| {
        match panadiga_core::pico::cdc::open_cdc(clean_port, 115_200, Duration::from_millis(120)) {
            Ok(mut p) => {
                let _ = p.write_all(b"\r\nGDB\r\n");
                let _ = p.flush();
                let mut drain_buf = [0u8; 512];
                let mut accumulated = String::new();
                for _ in 0..8 {
                    match p.read(&mut drain_buf) {
                        Ok(n) if n > 0 => {
                            accumulated.push_str(&String::from_utf8_lossy(&drain_buf[..n]));
                            if accumulated.contains("Auto-ready") {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
                let _ = p.set_timeout(Duration::from_millis(50));
                Some(p)
            }
            Err(e) => {
                panadiga_core::log_warn!(
                    "[GDB Bridge] Porta serial {} indisponível ({}); iniciando servidor em modo Local AM34 Stub.",
                    clean_port,
                    e
                );
                None
            }
        }
    });

    let label = if maybe_serial.is_some() {
        clean_port.to_string()
    } else {
        format!("LOCAL_STUB (fallback de {})", clean_port)
    };
    Ok((maybe_serial, label))
}

/// Starts the GDB TCP-to-Serial bridge server in a background thread.
pub fn start_gdb_bridge(
    backend: &PanadigaBackendState,
    port_name: String,
    listen_addr: String,
) -> Result<GdbBridgeStatus, String> {
    let ctrl = &backend.gdb_bridge;
    if ctrl.is_active() {
        return get_gdb_bridge_status(backend);
    }

    let mut clean_port = extract_port_name(&port_name);
    if clean_port.is_empty() || clean_port.eq_ignore_ascii_case("AUTO") {
        clean_port = super::serial::list_pico_serial_ports()
            .ok()
            .and_then(|ports| ports.into_iter().next())
            .map(|p| extract_port_name(&p))
            .unwrap_or_else(|| "LOCAL_STUB".to_string());
    }

    let addr = if listen_addr.trim().is_empty() {
        DEFAULT_GDB_LISTEN_SOCKET.to_string()
    } else {
        listen_addr.trim().to_string()
    };

    let listener = TcpListener::bind(&addr).map_err(|e| {
        format!("Falha ao abrir porta TCP GDB {}: {}. Verifique se outra instância do GDB/Ghidra não está utilizando esta porta.", addr, e)
    })?;
    let _ = listener.set_nonblocking(true);

    let (mut maybe_serial, active_mode_label) = open_serial_gdb_target(backend, &clean_port)?;

    ctrl.set_running(true);
    ctrl.reset_client_counters();
    ctrl.with_state(|st| {
        st.port_name = active_mode_label.clone();
        st.listen_addr = addr.clone();
        st.last_event = format!("Ouvindo conexões GDB em {} [{}]", addr, active_mode_label);
    });

    let worker_ctrl = ctrl.clone();
    let handle = thread::spawn(move || {
        let running_flag = worker_ctrl.running_flag();
        let connected_flag = worker_ctrl.client_connected_flag();
        let count_flag = worker_ctrl.client_count_counter();

        while running_flag.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((tcp_stream, _peer)) => {
                    connected_flag.store(true, Ordering::Relaxed);
                    count_flag.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref mut serial_port) = maybe_serial {
                        handle_serial_session(&worker_ctrl, tcp_stream, serial_port);
                    } else {
                        handle_local_stub_session(&worker_ctrl, tcp_stream);
                    }
                    connected_flag.store(false, Ordering::Relaxed);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => break,
            }
        }

        if let Some(ref mut serial_port) = maybe_serial {
            let _ = serial_port.write_all(b"+$D#44\r\n");
            let _ = serial_port.flush();
            let mut detach_drain = [0u8; 64];
            let _ = serial_port.read(&mut detach_drain);
        }
    });

    ctrl.store_thread_handle(handle);
    get_gdb_bridge_status(backend)
}

/// Stops the GDB TCP-to-Serial bridge server cleanly via wakeup TCP dial and thread join.
pub fn stop_gdb_bridge(backend: &PanadigaBackendState) -> Result<GdbBridgeStatus, String> {
    let ctrl = &backend.gdb_bridge;
    if !ctrl.is_active() {
        return get_gdb_bridge_status(backend);
    }

    ctrl.set_running(false);
    ctrl.set_client_connected(false);

    let addr = ctrl.with_state(|st| st.listen_addr.clone());
    if let Ok(sock_addr) = addr
        .parse::<SocketAddr>()
        .or_else(|_| DEFAULT_GDB_LISTEN_SOCKET.parse::<SocketAddr>())
    {
        let _ = TcpStream::connect_timeout(&sock_addr, Duration::from_millis(100));
    }

    if let Some(h) = ctrl.take_thread_handle() {
        let _ = h.join();
    }

    ctrl.with_state(|st| {
        st.last_event = "Servidor GDB finalizado".to_string();
    });
    get_gdb_bridge_status(backend)
}

/// Retrieves the current status and metrics of the GDB bridge.
pub fn get_gdb_bridge_status(backend: &PanadigaBackendState) -> Result<GdbBridgeStatus, String> {
    Ok(backend.gdb_bridge.status_snapshot())
}

/// Retrieves the current AM34 register state.
pub fn get_am34_registers(backend: &PanadigaBackendState) -> Result<Am34RegisterState, String> {
    Ok(backend.gdb_bridge.registers_snapshot())
}
