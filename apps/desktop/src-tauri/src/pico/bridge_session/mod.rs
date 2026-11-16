//! Bidirectional Live USB-CDC <-> P7501 SC0 Bridge Session Manager.
//!
//! Spawns a dedicated serial reader/writer worker while the Pico executes
//! `BRIDGE [baud] [NORMAL|JIG[:ms]|NOPULL]`, streaming raw bytes/ASCII/hex live to
//! the desktop UI (`pico://bridge-data`) and cleanly terminating via `'~~~'`.

pub mod codec;
mod controller;
mod worker;

pub use codec::encode_bridge_input;
pub use controller::{Sc0BridgeController, Sc0BridgeDataEvent, Sc0BridgeStatusDto};

use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

use crate::state::PanadigaBackendState;

use super::types::{extract_port_name, reject_mock_port};
use controller::{ActiveSc0Bridge, BridgeWorkerMsg};
use worker::{emit_bridge_event, run_bridge_worker};

/// Returns the current SC0 bridge status snapshot from Tauri-managed runtime state.
pub fn get_sc0_bridge_status_dto(backend: &PanadigaBackendState) -> Sc0BridgeStatusDto {
    backend.sc0_bridge.status_snapshot()
}

/// Opens USB CDC and spawns the dedicated bidirectional SC0 UART bridge worker thread.
pub fn start_sc0_bridge_session(
    backend: &PanadigaBackendState,
    app: AppHandle,
    port_name: String,
    baud_rate: Option<u32>,
    boot_mode: Option<String>,
) -> Result<Sc0BridgeStatusDto, String> {
    let mut clean_port = extract_port_name(&port_name);
    if clean_port.is_empty() || clean_port.eq_ignore_ascii_case("AUTO") {
        let detected = super::serial::list_pico_serial_ports()?;
        let first = detected.first().ok_or_else(|| {
            "Nenhuma porta serial Pico detectada. Conecte o Raspberry Pi Pico via USB e clique em Atualizar Portas."
                .to_string()
        })?;
        clean_port = extract_port_name(first);
    }
    reject_mock_port(&clean_port)?;
    if backend.gdb_bridge.is_active() {
        return Err("Porta ocupada pela Ponte GDB RSP ativa.".to_string());
    }
    if backend.sc0_bridge.is_active() {
        let _ = stop_sc0_bridge_session(backend);
    }

    let baud = baud_rate.unwrap_or(115_200).clamp(1200, 460_800);
    let clean_boot = boot_mode.unwrap_or_default().trim().to_string();
    let cmd = if clean_boot.is_empty() || clean_boot.eq_ignore_ascii_case("PASSIVE") {
        format!("BRIDGE {}\r\n", baud)
    } else {
        format!("BRIDGE {} {}\r\n", baud, clean_boot)
    };

    let port = backend.serial.with_lock(|| {
        let mut p =
            panadiga_core::pico::cdc::open_cdc(&clean_port, 115_200, Duration::from_millis(25))
                .map_err(|e| {
                    format!("Falha ao abrir porta {} para SC0 Bridge: {}", clean_port, e)
                })?;
        p.write_all(cmd.as_bytes())
            .map_err(|e| format!("Falha ao iniciar BRIDGE na porta {}: {}", clean_port, e))?;
        let _ = p.flush();
        Ok::<_, String>(p)
    })?;

    let (tx, rx) = mpsc::channel();
    backend.sc0_bridge.set_active(true);

    let initial_status = Sc0BridgeStatusDto {
        running: true,
        port_name: clean_port.clone(),
        baud_rate: baud,
        boot_mode: if clean_boot.is_empty() {
            "PASSIVE".to_string()
        } else {
            clean_boot
        },
        bytes_rx: 0,
        bytes_tx: 0,
        last_event: format!(
            "BRIDGE {} 8N1 ativa em {} (~~~ para sair)",
            baud, clean_port
        ),
    };
    backend.sc0_bridge.replace_status(initial_status.clone());
    emit_bridge_event(&app, "sys", b"", Some(&initial_status.last_event));

    let worker_ctrl = backend.sc0_bridge.clone();
    let handle = thread::spawn(move || run_bridge_worker(worker_ctrl, app, port, rx));
    backend.sc0_bridge.set_session(ActiveSc0Bridge {
        tx,
        join: Some(handle),
    });

    Ok(initial_status)
}

/// Encodes and enqueues payload bytes to the active SC0 bridge worker channel.
pub fn send_sc0_bridge_input(
    backend: &PanadigaBackendState,
    input_mode: String,
    payload: String,
) -> Result<usize, String> {
    if !backend.sc0_bridge.is_active() {
        return Err(
            "Nenhuma sessão SC0 Bridge ativa. Inicie a ponte antes de transmitir.".to_string(),
        );
    }
    let bytes = encode_bridge_input(&input_mode, &payload)?;
    let byte_len = bytes.len();
    let tx = backend
        .sc0_bridge
        .sender()
        .ok_or_else(|| "Canal da sessão SC0 Bridge indisponível.".to_string())?;
    tx.send(BridgeWorkerMsg::Data(bytes))
        .map_err(|_| "Worker SC0 Bridge já encerrado.".to_string())?;
    Ok(byte_len)
}

/// Sends `~~~` exit sequence, joins the SC0 worker thread, and releases serial ownership.
pub fn stop_sc0_bridge_session(
    backend: &PanadigaBackendState,
) -> Result<Sc0BridgeStatusDto, String> {
    if let Some(mut session) = backend.sc0_bridge.take_session() {
        let _ = session.tx.send(BridgeWorkerMsg::StopAndExit);
        if let Some(handle) = session.join.take() {
            let _ = handle.join();
        }
    }
    backend.sc0_bridge.set_active(false);
    Ok(get_sc0_bridge_status_dto(backend))
}
