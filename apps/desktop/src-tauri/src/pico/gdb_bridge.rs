//! Panasonic DMR-EH55 DIGA Suite - GDB Remote Serial Protocol (RSP) Bridge Controller
//!
//! Imperative shell managing the background TCP-to-Serial proxy thread,
//! client connection lifecycle, and AM34 shadow register file state.
//! Pure RSP framing and validation reside in `rsp_protocol.rs`.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::Duration;

use panadiga_core::constants::DEFAULT_GDB_LISTEN_SOCKET;

use super::rsp_protocol::{
    compute_mock_rsp_response_pure, decode_am34_registers_hex, verify_and_classify_rsp_packet,
};
use super::types::{
    Am34RegisterState, GdbBridgeStatus, GdbPacketTelemetryEvent, extract_port_name,
};

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

struct GdbBridgeMutableState {
    port_name: String,
    listen_addr: String,
    last_event: String,
    registers: Am34RegisterState,
    telemetry_ring: VecDeque<GdbPacketTelemetryEvent>,
}

struct GdbBridgeController {
    running: Arc<AtomicBool>,
    client_connected: Arc<AtomicBool>,
    client_count: Arc<AtomicU32>,
    state: Mutex<GdbBridgeMutableState>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

static GDB_CONTROLLER: OnceLock<GdbBridgeController> = OnceLock::new();

fn get_controller() -> &'static GdbBridgeController {
    GDB_CONTROLLER.get_or_init(|| GdbBridgeController {
        running: Arc::new(AtomicBool::new(false)),
        client_connected: Arc::new(AtomicBool::new(false)),
        client_count: Arc::new(AtomicU32::new(0)),
        state: Mutex::new(GdbBridgeMutableState {
            port_name: String::new(),
            listen_addr: DEFAULT_GDB_LISTEN_SOCKET.to_string(),
            last_event: "Servidor GDB offline".to_string(),
            registers: Am34RegisterState {
                d: [0x00000000, 0x00000000, 0x00000000, 0x00000000],
                a: [0x00000000, 0x00000000, 0x00000000, 0x0003FFF0],
                mdr: 0x00000000,
                psw: 0x00000100,
                pc: 0x80000000,
                sp: 0x0003FFF0,
                is_halted: true,
            },
            telemetry_ring: VecDeque::with_capacity(128),
        }),
        thread_handle: Mutex::new(None),
    })
}

/// Checks if the GDB TCP-to-Serial bridge currently owns the serial port.
pub fn is_gdb_bridge_active() -> bool {
    get_controller().running.load(Ordering::Relaxed)
}

/// Synchronizes the controller's AM34 shadow register file from a 96-char hex string.
fn sync_controller_registers(hex_str: &str) {
    let ctrl = get_controller();
    let mut guard = lock_unpoisoned(&ctrl.state);
    if let Some(updated) = decode_am34_registers_hex(hex_str, guard.registers.is_halted) {
        guard.registers = updated;
    }
}

/// Records a packet event into the circular telemetry ring buffer.
pub fn record_telemetry(direction: &str, raw_packet: &str) {
    let ctrl = get_controller();
    let (command_type, checksum_valid) = verify_and_classify_rsp_packet(raw_packet);
    let event = GdbPacketTelemetryEvent {
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        direction: direction.to_string(),
        command_type,
        raw_packet: raw_packet.to_string(),
        checksum_valid,
    };

    let mut guard = lock_unpoisoned(&ctrl.state);
    if guard.telemetry_ring.len() >= 128 {
        guard.telemetry_ring.pop_front();
    }
    guard.telemetry_ring.push_back(event);
}

/// Retrieves all recorded GDB packet telemetry events.
pub fn get_gdb_telemetry() -> Vec<GdbPacketTelemetryEvent> {
    let ctrl = get_controller();
    lock_unpoisoned(&ctrl.state)
        .telemetry_ring
        .iter()
        .cloned()
        .collect()
}

/// Starts the GDB TCP-to-Serial bridge server in a background thread.
///
/// Binds `listen_addr` (default `127.0.0.1:3333`) ONCE before spawning the accept loop to
/// eliminate TCP `TIME_WAIT` race conditions, and falls back to an offline Local AM34 RSP
/// Stub when no physical Pico serial port is attached so `gdb-multiarch` / Ghidra can always connect.
pub fn start_gdb_bridge(port_name: String, listen_addr: String) -> Result<GdbBridgeStatus, String> {
    let ctrl = get_controller();
    if ctrl.running.load(Ordering::Relaxed) {
        return get_gdb_bridge_status();
    }

    let mut clean_port = extract_port_name(&port_name);
    if clean_port.is_empty() || clean_port.eq_ignore_ascii_case("AUTO") {
        clean_port = super::serial::list_pico_serial_ports()
            .ok()
            .and_then(|ports| ports.into_iter().next())
            .map(|p| extract_port_name(&p))
            .unwrap_or_else(|| "LOCAL_STUB".to_string());
    }
    if !clean_port.eq_ignore_ascii_case("LOCAL_STUB") {
        super::types::reject_mock_port(&clean_port)?;
    }

    let addr = if listen_addr.trim().is_empty() {
        DEFAULT_GDB_LISTEN_SOCKET.to_string()
    } else {
        listen_addr.trim().to_string()
    };

    // Bind TCP socket once directly and hand the live TcpListener to the worker thread.
    let listener = TcpListener::bind(&addr).map_err(|e| {
        format!("Falha ao abrir porta TCP GDB {}: {}. Verifique se outra instância do GDB/Ghidra não está utilizando esta porta.", addr, e)
    })?;
    let _ = listener.set_nonblocking(true);

    // Try opening physical Pico CDC (holding serial mutex); fall back cleanly to Local AM34 RSP Stub if absent/busy.
    let mut maybe_serial: Option<Box<dyn serialport::SerialPort>> = None;
    let active_mode_label = if !clean_port.eq_ignore_ascii_case("LOCAL_STUB")
        && !clean_port.eq_ignore_ascii_case("LOCAL")
    {
        let _io_guard = super::serial::lock_serial_io();
        match panadiga_core::pico::cdc::open_cdc(&clean_port, 115200, Duration::from_millis(120)) {
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
                maybe_serial = Some(p);
                clean_port.clone()
            }
            Err(e) => {
                panadiga_core::log_warn!(
                    "[GDB Bridge] Porta serial {} indisponível ({}); iniciando servidor em modo Local AM34 Stub.",
                    clean_port,
                    e
                );
                format!("LOCAL_STUB (fallback de {})", clean_port)
            }
        }
    } else {
        "LOCAL_STUB (AM34)".to_string()
    };

    ctrl.running.store(true, Ordering::Relaxed);
    ctrl.client_connected.store(false, Ordering::Relaxed);
    ctrl.client_count.store(0, Ordering::Relaxed);

    {
        let mut guard = lock_unpoisoned(&ctrl.state);
        guard.port_name = active_mode_label.clone();
        guard.listen_addr = addr.clone();
        guard.last_event = format!("Ouvindo conexões GDB em {} [{}]", addr, active_mode_label);
    }

    let running_flag = ctrl.running.clone();
    let connected_flag = ctrl.client_connected.clone();
    let count_flag = ctrl.client_count.clone();

    let handle = thread::spawn(move || {
        while running_flag.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((tcp_stream, _peer)) => {
                    connected_flag.store(true, Ordering::Relaxed);
                    count_flag.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref mut serial_port) = maybe_serial {
                        handle_serial_session(
                            tcp_stream,
                            serial_port,
                            &running_flag,
                            &connected_flag,
                        );
                    } else {
                        handle_local_stub_session(tcp_stream, &running_flag, &connected_flag);
                    }
                    connected_flag.store(false, Ordering::Relaxed);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => break,
            }
        }

        // Send GDB Detach packet (+$D#44) before closing serial port so Pico exits binary RSP mode
        if let Some(ref mut serial_port) = maybe_serial {
            let _ = serial_port.write_all(b"+$D#44\r\n");
            let _ = serial_port.flush();
            let mut detach_drain = [0u8; 64];
            let _ = serial_port.read(&mut detach_drain);
        }
    });

    *lock_unpoisoned(&ctrl.thread_handle) = Some(handle);
    get_gdb_bridge_status()
}

fn prepare_gdb_tcp_stream(tcp_stream: &mut TcpStream) {
    let _ = tcp_stream.set_nonblocking(false);
    let _ = tcp_stream.set_read_timeout(Some(Duration::from_millis(50)));
}

fn handle_local_stub_session(
    mut tcp_stream: TcpStream,
    running_flag: &Arc<AtomicBool>,
    connected_flag: &Arc<AtomicBool>,
) {
    prepare_gdb_tcp_stream(&mut tcp_stream);
    let mut t_buf = [0u8; 1024];

    while running_flag.load(Ordering::Relaxed) && connected_flag.load(Ordering::Relaxed) {
        match tcp_stream.read(&mut t_buf) {
            Ok(0) => break,
            Ok(n) => {
                let raw = String::from_utf8_lossy(&t_buf[..n]);
                record_telemetry("INBOUND", &raw);
                for segment in raw.split('$').skip(1) {
                    let cmd = segment.split('#').next().unwrap_or("");
                    if cmd.is_empty() {
                        continue;
                    }
                    if cmd.starts_with('G') {
                        sync_controller_registers(cmd);
                    }
                    let regs_snapshot = {
                        let ctrl = get_controller();
                        lock_unpoisoned(&ctrl.state).registers.clone()
                    };
                    let reply_bytes = compute_mock_rsp_response_pure(cmd, &regs_snapshot);
                    let reply_str = String::from_utf8_lossy(&reply_bytes).into_owned();
                    record_telemetry("OUTBOUND", &reply_str);
                    if tcp_stream.write_all(&reply_bytes).is_err() {
                        return;
                    }
                }
                let _ = tcp_stream.flush();
            }
            Err(ref e) if crate::pico::is_transient_io_error(e) => {}
            Err(_) => break,
        }
    }
}

fn handle_serial_session(
    mut tcp_stream: TcpStream,
    serial_port: &mut Box<dyn serialport::SerialPort>,
    running_flag: &Arc<AtomicBool>,
    connected_flag: &Arc<AtomicBool>,
) {
    prepare_gdb_tcp_stream(&mut tcp_stream);

    let mut s_buf = [0u8; 1024];
    let mut t_buf = [0u8; 1024];
    let mut last_inbound_was_g = false;
    let mut g_accum_buf = String::with_capacity(128);

    while running_flag.load(Ordering::Relaxed) && connected_flag.load(Ordering::Relaxed) {
        match tcp_stream.read(&mut t_buf) {
            Ok(0) => break,
            Ok(n) => {
                let packet = String::from_utf8_lossy(&t_buf[..n]);
                record_telemetry("INBOUND", &packet);
                let cmd = packet
                    .split('$')
                    .nth(1)
                    .and_then(|s| s.split('#').next())
                    .unwrap_or("");
                if cmd == "g" {
                    last_inbound_was_g = true;
                    g_accum_buf.clear();
                }
                if cmd.starts_with('G') {
                    sync_controller_registers(cmd);
                }
                let _ = serial_port.write_all(&t_buf[..n]);
            }
            Err(ref e) if crate::pico::is_transient_io_error(e) => {}
            Err(_) => break,
        }

        match serial_port.read(&mut s_buf) {
            Ok(0) => {}
            Ok(n) => {
                let packet = String::from_utf8_lossy(&s_buf[..n]);
                record_telemetry("OUTBOUND", &packet);
                if last_inbound_was_g {
                    g_accum_buf.push_str(&packet);
                    if g_accum_buf.contains('#') {
                        sync_controller_registers(&g_accum_buf);
                        last_inbound_was_g = false;
                        g_accum_buf.clear();
                    }
                }
                if tcp_stream.write_all(&s_buf[..n]).is_err() {
                    break;
                }
            }
            Err(ref e) if crate::pico::is_transient_io_error(e) => {}
            Err(_) => break,
        }
    }
}

/// Stops the GDB TCP-to-Serial bridge server.
pub fn stop_gdb_bridge() -> Result<GdbBridgeStatus, String> {
    let ctrl = get_controller();
    if !ctrl.running.load(Ordering::Relaxed) {
        return get_gdb_bridge_status();
    }

    ctrl.running.store(false, Ordering::Relaxed);
    ctrl.client_connected.store(false, Ordering::Relaxed);

    let addr = lock_unpoisoned(&ctrl.state).listen_addr.clone();
    if let Ok(sock_addr) = addr
        .parse::<SocketAddr>()
        .or_else(|_| DEFAULT_GDB_LISTEN_SOCKET.parse::<SocketAddr>())
    {
        let _ = TcpStream::connect_timeout(&sock_addr, Duration::from_millis(100));
    }

    if let Some(h) = lock_unpoisoned(&ctrl.thread_handle).take() {
        let _ = h.join();
    }

    lock_unpoisoned(&ctrl.state).last_event = "Servidor GDB finalizado".to_string();
    get_gdb_bridge_status()
}

/// Retrieves the current status and metrics of the GDB bridge.
pub fn get_gdb_bridge_status() -> Result<GdbBridgeStatus, String> {
    let ctrl = get_controller();
    let guard = lock_unpoisoned(&ctrl.state);
    Ok(GdbBridgeStatus {
        running: ctrl.running.load(Ordering::Relaxed),
        port_name: guard.port_name.clone(),
        listen_addr: guard.listen_addr.clone(),
        connected_clients: ctrl.client_count.load(Ordering::Relaxed),
        last_event: guard.last_event.clone(),
    })
}

/// Retrieves the current AM34 register state.
pub fn get_am34_registers() -> Result<Am34RegisterState, String> {
    let ctrl = get_controller();
    Ok(lock_unpoisoned(&ctrl.state).registers.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_ring_capacity() {
        for i in 0..150 {
            record_telemetry("INBOUND", &format!("$m{:x},4#00", i));
        }
        let telemetry = get_gdb_telemetry();
        assert_eq!(telemetry.len(), 128);
    }
}
