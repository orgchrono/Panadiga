//! Instance-scoped SC0 Bridge controller and DTO models using scoped mutex closures.

use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use crate::state::ScopedMutex;

/// Snapshot DTO describing the bidirectional SC0 UART bridge state.
#[derive(Debug, Clone, Serialize)]
pub struct Sc0BridgeStatusDto {
    pub running: bool,
    pub port_name: String,
    pub baud_rate: u32,
    pub boot_mode: String,
    pub bytes_rx: u64,
    pub bytes_tx: u64,
    pub last_event: String,
}

/// Real-time TX/RX event emitted on Tauri channel `"pico://bridge-data"`.
#[derive(Debug, Clone, Serialize)]
pub struct Sc0BridgeDataEvent {
    pub stream: String,
    pub text: String,
    pub hex: String,
    pub bytes: Vec<u8>,
    pub timestamp: String,
}

pub(super) enum BridgeWorkerMsg {
    Data(Vec<u8>),
    StopAndExit,
}

pub(super) struct ActiveSc0Bridge {
    pub tx: Sender<BridgeWorkerMsg>,
    pub join: Option<JoinHandle<()>>,
}

/// Thread-safe, Tauri-managed SC0 Bridge session controller with zero global statics.
#[derive(Clone)]
pub struct Sc0BridgeController {
    active: Arc<AtomicBool>,
    session: ScopedMutex<Option<ActiveSc0Bridge>>,
    status: ScopedMutex<Sc0BridgeStatusDto>,
}

impl Default for Sc0BridgeController {
    fn default() -> Self {
        Self::new()
    }
}

fn default_bridge_status() -> Sc0BridgeStatusDto {
    Sc0BridgeStatusDto {
        running: false,
        port_name: String::new(),
        baud_rate: 115_200,
        boot_mode: String::new(),
        bytes_rx: 0,
        bytes_tx: 0,
        last_event: "Ponte SC0 inativa".to_string(),
    }
}

impl Sc0BridgeController {
    /// Creates an inactive SC0 bridge controller instance.
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            session: ScopedMutex::new(None),
            status: ScopedMutex::new(default_bridge_status()),
        }
    }

    /// Returns true if an SC0 UART bridge session currently owns the serial port.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub(super) fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::SeqCst);
    }

    pub(super) fn replace_status(&self, status: Sc0BridgeStatusDto) {
        self.status.with_lock(|slot| {
            *slot = status;
        });
    }

    pub(super) fn update_status(
        &self,
        f: impl FnOnce(&mut Sc0BridgeStatusDto),
    ) -> Sc0BridgeStatusDto {
        self.status.with_lock(|slot| {
            f(slot);
            slot.clone()
        })
    }

    /// Returns an immutable snapshot of the current SC0 bridge status.
    pub fn status_snapshot(&self) -> Sc0BridgeStatusDto {
        let mut dto = self.status.with_lock(|slot| slot.clone());
        dto.running = self.is_active();
        dto
    }

    pub(super) fn set_session(&self, session: ActiveSc0Bridge) {
        self.session.with_lock(|slot| {
            *slot = Some(session);
        });
    }

    pub(super) fn sender(&self) -> Option<Sender<BridgeWorkerMsg>> {
        self.session
            .with_lock(|slot| slot.as_ref().map(|s| s.tx.clone()))
    }

    pub(super) fn take_session(&self) -> Option<ActiveSc0Bridge> {
        self.session.with_lock(|slot| slot.take())
    }
}
