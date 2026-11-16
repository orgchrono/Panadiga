//! Scoped synchronization primitives and Tauri-managed runtime state container.
//!
//! Replaces module-level `static Mutex` / `OnceLock` globals with instance-scoped
//! state managed by Tauri (`.manage(PanadigaBackendState::new())`) and enforces
//! scoped closure access (`with_lock` / `try_with_lock`) so no raw `MutexGuard`
//! escapes across function boundaries.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::pico::bridge_session::Sc0BridgeController;
use crate::pico::gdb_bridge::GdbBridgeController;

/// Thread-safe lock wrapper exposing strictly scoped closure access (zero leaked `MutexGuard`s).
#[derive(Debug)]
pub struct ScopedMutex<T>(Arc<Mutex<T>>);

impl<T> Clone for ScopedMutex<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Default> Default for ScopedMutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> ScopedMutex<T> {
    /// Creates a new scoped mutex wrapping `value`.
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    /// Executes `f` while holding the mutex with automatic poison recovery.
    pub fn with_lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut guard)
    }

    /// Attempts non-blocking execution of `f`; returns `None` when the mutex is contended.
    pub fn try_with_lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let mut guard = self.0.try_lock().ok()?;
        Some(f(&mut guard))
    }
}

/// Exclusive foreground serial I/O coordinator using scoped closures.
#[derive(Debug, Clone, Default)]
pub struct SerialIoGate(ScopedMutex<()>);

impl SerialIoGate {
    /// Runs `f` with exclusive access to USB CDC serial I/O.
    pub fn with_lock<R>(&self, f: impl FnOnce() -> R) -> R {
        self.0.with_lock(|()| f())
    }

    /// Attempts to run `f` without blocking; returns `None` when serial I/O is busy.
    pub fn try_with_lock<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
        self.0.try_with_lock(|()| f())
    }
}

/// Mutable runtime counters and active port selection for live Pico telemetry streaming.
#[derive(Debug, Clone)]
pub struct TelemetryRuntimeState {
    active: Arc<AtomicBool>,
    worker_started: Arc<AtomicBool>,
    port_observer_started: Arc<AtomicBool>,
    ram_loader_injected: Arc<AtomicBool>,
    scan_total_probes: Arc<AtomicU32>,
    scan_hits_detected: Arc<AtomicU32>,
    port_name: ScopedMutex<String>,
}

impl Default for TelemetryRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryRuntimeState {
    /// Constructs a fresh telemetry runtime state instance.
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            worker_started: Arc::new(AtomicBool::new(false)),
            port_observer_started: Arc::new(AtomicBool::new(false)),
            ram_loader_injected: Arc::new(AtomicBool::new(false)),
            scan_total_probes: Arc::new(AtomicU32::new(0)),
            scan_hits_detected: Arc::new(AtomicU32::new(0)),
            port_name: ScopedMutex::new(String::new()),
        }
    }

    /// Reads the configured telemetry serial port name via a scoped mutex closure.
    pub fn port_name(&self) -> String {
        self.port_name.with_lock(|p| p.clone())
    }

    /// Updates the telemetry serial port name via a scoped mutex closure.
    pub fn set_port_name(&self, port: String) {
        self.port_name.with_lock(|slot| {
            *slot = port;
        });
    }

    /// Returns whether telemetry streaming is currently enabled.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Sets telemetry stream activation state.
    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Relaxed);
    }

    /// Claims one-time startup of the background telemetry polling worker.
    pub fn claim_worker_start(&self) -> bool {
        self.worker_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Claims one-time startup of the passive USB port mutation observer.
    pub fn claim_port_observer_start(&self) -> bool {
        self.port_observer_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Updates whether the SRAM loader has been injected during this session.
    pub fn set_ram_loader_injected(&self, injected: bool) {
        self.ram_loader_injected.store(injected, Ordering::SeqCst);
    }

    /// Checks whether the in-circuit SRAM loader has been injected.
    pub fn is_ram_loader_injected(&self) -> bool {
        self.ram_loader_injected.load(Ordering::SeqCst)
    }

    /// Updates scan probe and hit telemetry counters.
    pub fn record_scan_telemetry(&self, probes: Option<u32>, hits: Option<u32>) {
        if let Some(h) = hits {
            self.scan_hits_detected.store(h, Ordering::SeqCst);
        }
        if let Some(p) = probes {
            self.scan_total_probes.store(p, Ordering::SeqCst);
        }
    }

    /// Returns `(total_probes, hits_detected)` snapshot.
    pub fn scan_counts(&self) -> (u32, u32) {
        (
            self.scan_total_probes.load(Ordering::SeqCst),
            self.scan_hits_detected.load(Ordering::SeqCst),
        )
    }
}

/// Unified Tauri-managed hardware runtime state injected via `tauri::State<'_, PanadigaBackendState>`.
#[derive(Clone, Default)]
pub struct PanadigaBackendState {
    pub serial: SerialIoGate,
    pub telemetry: TelemetryRuntimeState,
    pub sc0_bridge: Sc0BridgeController,
    pub gdb_bridge: GdbBridgeController,
}

impl PanadigaBackendState {
    /// Creates an isolated backend runtime container for Tauri or unit tests.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if either the GDB RSP bridge or SC0 UART bridge owns the serial port.
    pub fn is_any_bridge_active(&self) -> bool {
        self.gdb_bridge.is_active() || self.sc0_bridge.is_active()
    }

    /// Verifies that `clean_port` is non-mock and not owned by an active bridge session.
    pub fn ensure_serial_port_unlocked(&self, clean_port: &str) -> IpcResult<()> {
        crate::pico::types::reject_mock_port(clean_port)?;
        if self.is_any_bridge_active() {
            return Err(PanadigaIpcError::serial(format!(
                "Porta serial {} ocupada pela Ponte GDB/SC0 ativa. Encerre a sessão antes de enviar comandos CDC.",
                clean_port
            )));
        }
        Ok(())
    }

    /// Executes `f` under the exclusive serial I/O lock after validating bridge ownership.
    pub fn with_exclusive_serial<R>(
        &self,
        clean_port: &str,
        f: impl FnOnce() -> IpcResult<R>,
    ) -> IpcResult<R> {
        crate::pico::types::reject_mock_port(clean_port)?;
        self.serial.with_lock(|| {
            self.ensure_serial_port_unlocked(clean_port)?;
            f()
        })
    }
}

/// Formats current UTC wall-clock time as `HH:MM:SS` for live event streams.
pub(crate) fn format_utc_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let hh = (secs / 3600) % 24;
    let mm = (secs / 60) % 60;
    let ss = secs % 60;
    format!("{hh:02}:{mm:02}:{ss:02}")
}
