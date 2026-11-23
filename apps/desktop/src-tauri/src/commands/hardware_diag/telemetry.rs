//! Background USB port mutation observer and managed Pico telemetry stream worker.

use std::thread;
use std::time::Duration;
use tauri::Emitter;

use crate::commands::constants::{
    EVENT_PICO_PORTS_CHANGED, EVENT_PICO_TELEMETRY, PORT_OBSERVER_POLL_MS,
    TELEMETRY_WORKER_POLL_MS, TRANSPORT_PREFIX_ANDROID_OTG, TRANSPORT_PREFIX_BLE,
};
use crate::commands::error::IpcResult;
use crate::state::PanadigaBackendState;

/// Starts a passive USB port-mutation observer that emits `EVENT_PICO_PORTS_CHANGED`
/// only when the physical serial port list changes.
pub fn start_port_mutation_observer(app: tauri::AppHandle, backend: &PanadigaBackendState) {
    if !backend.telemetry.claim_port_observer_start() {
        return;
    }
    thread::spawn(move || {
        let mut last_ports = crate::pico::list_pico_serial_ports().unwrap_or_default();
        loop {
            thread::sleep(Duration::from_millis(PORT_OBSERVER_POLL_MS));
            if let Ok(current_ports) = crate::pico::list_pico_serial_ports()
                && current_ports != last_ports
            {
                last_ports = current_ports.clone();
                let _ = app.emit(EVENT_PICO_PORTS_CHANGED, &current_ports);
            }
        }
    });
}

/// Enables or pauses the single background Pico telemetry push stream (`pico://telemetry`).
#[tauri::command]
pub fn set_pico_telemetry_stream(
    app: tauri::AppHandle,
    state: tauri::State<'_, PanadigaBackendState>,
    active: bool,
    port_name: Option<String>,
) -> IpcResult<bool> {
    let backend = state.inner().clone();
    start_port_mutation_observer(app.clone(), &backend);
    if let Some(port) = port_name.filter(|p| !p.trim().is_empty()) {
        backend.telemetry.set_port_name(port);
    }
    backend.telemetry.set_active(active);

    if active && backend.telemetry.claim_worker_start() {
        let app_clone = app.clone();
        let worker_state = backend.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(TELEMETRY_WORKER_POLL_MS));
                if !worker_state.telemetry.is_active() {
                    continue;
                }
                let port = worker_state.telemetry.port_name();
                if port.trim().is_empty() {
                    continue;
                }
                if port.starts_with(TRANSPORT_PREFIX_BLE)
                    || port.starts_with(TRANSPORT_PREFIX_ANDROID_OTG)
                {
                    continue;
                }
                if let Some(Ok(status)) =
                    crate::pico::try_pico_get_device_status_dto(&worker_state, port)
                {
                    let _ = app_clone.emit(EVENT_PICO_TELEMETRY, &status);
                    worker_state.bench_stream.with_publisher(|pub_ref, now_ms| {
                        let current_state = pub_ref.snapshot().state;
                        if status.connected
                            && matches!(
                                current_state,
                                panadiga_core::bench::BenchState::Disconnected
                            )
                        {
                            pub_ref.set_state(
                                panadiga_core::bench::BenchState::Idle {
                                    port: status.port.clone(),
                                    fw_contract_hash: status.firmware_version.clone(),
                                },
                                now_ms,
                            );
                        }
                        pub_ref.push_telemetry(
                            panadiga_core::bench::TelemetrySample {
                                ts_ms: now_ms,
                                vdd_mv: (status.vdd_volts.clamp(0.0, 65.535) * 1000.0).round()
                                    as u16,
                                nsw5v_high: status.target_powered,
                                jig_asserted: status.breakin_mode > 0,
                                uart_rx_bytes: u64::from(status.total_probes),
                            },
                            now_ms,
                        );
                    });
                }
            }
        });
    }

    Ok(active)
}
