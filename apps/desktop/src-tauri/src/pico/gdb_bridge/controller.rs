//! Instance-scoped GDB RSP bridge controller using scoped mutex closures.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread;

use panadiga_core::constants::{AM34_FLASH_BASE, DEFAULT_GDB_LISTEN_SOCKET};

use crate::state::ScopedMutex;

use super::super::rsp_protocol::{decode_am34_registers_hex, verify_and_classify_rsp_packet};
use super::super::types::{Am34RegisterState, GdbBridgeStatus, GdbPacketTelemetryEvent};

pub(super) struct GdbBridgeMutableState {
    pub port_name: String,
    pub listen_addr: String,
    pub last_event: String,
    pub registers: Am34RegisterState,
    pub telemetry_ring: VecDeque<GdbPacketTelemetryEvent>,
}

/// Thread-safe, Tauri-managed GDB RSP bridge controller with zero global locks.
#[derive(Clone)]
pub struct GdbBridgeController {
    running: Arc<AtomicBool>,
    client_connected: Arc<AtomicBool>,
    client_count: Arc<AtomicU32>,
    state: ScopedMutex<GdbBridgeMutableState>,
    thread_handle: ScopedMutex<Option<thread::JoinHandle<()>>>,
}

impl Default for GdbBridgeController {
    fn default() -> Self {
        Self::new()
    }
}

impl GdbBridgeController {
    /// Creates an isolated GDB bridge controller initialized with default AM34 reset registers.
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            client_connected: Arc::new(AtomicBool::new(false)),
            client_count: Arc::new(AtomicU32::new(0)),
            state: ScopedMutex::new(GdbBridgeMutableState {
                port_name: String::new(),
                listen_addr: DEFAULT_GDB_LISTEN_SOCKET.to_string(),
                last_event: "Servidor GDB offline".to_string(),
                registers: Am34RegisterState {
                    d: [0x00000000; 4],
                    a: [0x00000000, 0x00000000, 0x00000000, 0x0003FFF0],
                    mdr: 0x00000000,
                    psw: 0x00000100,
                    pc: AM34_FLASH_BASE,
                    sp: 0x0003FFF0,
                    is_halted: true,
                },
                telemetry_ring: VecDeque::with_capacity(128),
            }),
            thread_handle: ScopedMutex::new(None),
        }
    }

    /// Returns true if the GDB TCP-to-Serial bridge currently owns the serial port.
    pub fn is_active(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub(super) fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }

    pub(super) fn client_connected_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.client_connected)
    }

    pub(super) fn client_count_counter(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.client_count)
    }

    pub(super) fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::Relaxed);
    }

    pub(super) fn set_client_connected(&self, connected: bool) {
        self.client_connected.store(connected, Ordering::Relaxed);
    }

    pub(super) fn reset_client_counters(&self) {
        self.client_connected.store(false, Ordering::Relaxed);
        self.client_count.store(0, Ordering::Relaxed);
    }

    pub(super) fn with_state<R>(&self, f: impl FnOnce(&mut GdbBridgeMutableState) -> R) -> R {
        self.state.with_lock(f)
    }

    /// Synchronizes AM34 shadow registers from a 96-char hex RSP reply or `G` packet.
    pub fn sync_registers(&self, hex_str: &str) {
        self.state.with_lock(|st| {
            if let Some(updated) = decode_am34_registers_hex(hex_str, st.registers.is_halted) {
                st.registers = updated;
            }
        });
    }

    /// Records a packet event into the bounded 128-entry telemetry ring buffer.
    pub fn record_telemetry(&self, direction: &str, raw_packet: &str) {
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

        self.state.with_lock(|st| {
            if st.telemetry_ring.len() >= 128 {
                st.telemetry_ring.pop_front();
            }
            st.telemetry_ring.push_back(event);
        });
    }

    /// Returns a cloned snapshot of the GDB telemetry ring buffer.
    pub fn telemetry_snapshot(&self) -> Vec<GdbPacketTelemetryEvent> {
        self.state
            .with_lock(|st| st.telemetry_ring.iter().cloned().collect())
    }

    /// Returns the current GDB bridge server status DTO.
    pub fn status_snapshot(&self) -> GdbBridgeStatus {
        self.state.with_lock(|st| GdbBridgeStatus {
            running: self.is_active(),
            port_name: st.port_name.clone(),
            listen_addr: st.listen_addr.clone(),
            connected_clients: self.client_count.load(Ordering::Relaxed),
            last_event: st.last_event.clone(),
        })
    }

    /// Returns a cloned snapshot of the AM34 shadow register file.
    pub fn registers_snapshot(&self) -> Am34RegisterState {
        self.state.with_lock(|st| st.registers.clone())
    }

    pub(super) fn store_thread_handle(&self, handle: thread::JoinHandle<()>) {
        self.thread_handle.with_lock(|slot| {
            *slot = Some(handle);
        });
    }

    pub(super) fn take_thread_handle(&self) -> Option<thread::JoinHandle<()>> {
        self.thread_handle.with_lock(|slot| slot.take())
    }
}

#[cfg(test)]
mod tests {
    use super::GdbBridgeController;

    #[test]
    fn test_telemetry_ring_capacity() {
        let ctrl = GdbBridgeController::new();
        for i in 0..150 {
            ctrl.record_telemetry("INBOUND", &format!("$m{:x},4#00", i));
        }
        assert_eq!(ctrl.telemetry_snapshot().len(), 128);
    }
}
