//! Panasonic DMR-EH55 DIGA Suite - SSOT, Granular Diagnostics & Reactive Streaming Commands
//!
//! Single Responsibility: Exposes the Rust hardware/firmware SSOT DTO (`get_hardware_ssot_spec`),
//! async non-blocking Pico diagnostic execution (`pico_execute_diagnostic`) with real-time
//! Tauri event emission (`pico://serial-log`, `pico://diagnostic-result`, `pico://telemetry`),
//! and a single managed telemetry stream toggle (`set_pico_telemetry_stream`).

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;

use super::constants::{
    AM34_NVRAM_BASE, BYTES_PER_U32_WORD, DEFAULT_DUMP_DIAGNOSTIC_CMD, DEFAULT_NVRAM_CAPTURE_WORDS,
    EVENT_PICO_DIAG_RESULT, EVENT_PICO_NVRAM_CAPTURE, EVENT_PICO_PORTS_CHANGED,
    EVENT_PICO_SERIAL_LOG, EVENT_PICO_TELEMETRY, MAX_NVRAM_CAPTURE_WORDS, MIN_NVRAM_CAPTURE_WORDS,
    NVRAM_DUMP_CHUNK_WORDS, PORT_OBSERVER_POLL_MS, SOURCE_PICO_DIAG_DUMP, SOURCE_PICO_LIVE_NVRAM,
    STATUS_REFRESH_COMMANDS, TELEMETRY_WORKER_POLL_MS, TRANSPORT_PREFIX_ANDROID_OTG,
    TRANSPORT_PREFIX_BLE, VDD_TARGET_FLOATING_CEIL_VOLTS, VDD_TARGET_MIN_VOLTS,
    VDD_ZERO_EPSILON_VOLTS,
};
use super::dto::{HardwareSsotSpecDto, LiveNvramCaptureEventDto, build_hardware_ssot_spec};
use super::hardware::pico_get_status;
use crate::pico::diagnostics::{PicoDiagnosticResultDto, execute_diagnostic_on_port};

/// Real-time serial log entry pushed over Tauri event `"pico://serial-log"`.
#[derive(Debug, Clone, Serialize)]
pub struct ReactiveSerialEventDto {
    pub id: String,
    pub timestamp: String,
    pub source: String,
    pub level: String,
    pub message: String,
}

struct TelemetryStreamState {
    active: Arc<AtomicBool>,
    worker_started: Arc<AtomicBool>,
    port_observer_started: Arc<AtomicBool>,
    port_name: Mutex<String>,
}

static STREAM_STATE: OnceLock<TelemetryStreamState> = OnceLock::new();

fn stream_state() -> &'static TelemetryStreamState {
    STREAM_STATE.get_or_init(|| TelemetryStreamState {
        active: Arc::new(AtomicBool::new(false)),
        worker_started: Arc::new(AtomicBool::new(false)),
        port_observer_started: Arc::new(AtomicBool::new(false)),
        port_name: Mutex::new(String::new()),
    })
}

/// Starts a passive USB port-mutation observer that emits `EVENT_PICO_PORTS_CHANGED`
/// only when the physical serial port list changes.
pub fn start_port_mutation_observer(app: tauri::AppHandle) {
    let state = stream_state();
    if state
        .port_observer_started
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
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

fn current_time_hhmmss() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hh = (secs / 3600) % 24;
    let mm = (secs / 60) % 60;
    let ss = secs % 60;
    format!("{:02}:{:02}:{:02}", hh, mm, ss)
}

fn build_and_emit_live_nvram_capture(
    app: &tauri::AppHandle,
    source: &str,
    port_name: &str,
    base_bus_address: u32,
    bytes: &[u8],
) -> Result<LiveNvramCaptureEventDto, String> {
    let label = format!("{} ({})", source, port_name.trim());
    let report =
        panadiga_core::nvram::inspect_partial_nvram_capture(bytes, base_bus_address, &label)
            .map_err(|e| e.to_string())?;
    let event = LiveNvramCaptureEventDto {
        source: source.to_string(),
        port: port_name.trim().to_string(),
        base_bus_address: format!("0x{:08X}", base_bus_address),
        captured_bytes: bytes.len(),
        report,
    };
    let _ = app.emit(EVENT_PICO_NVRAM_CAPTURE, &event);
    Ok(event)
}

/// Captures real live NVRAM bytes in-circuit over Pico P7501 `DUMP` / GDB RSP in 8 KB microsector chunks,
/// audits sector activity (`SA134..SA141`) via `panadiga-core`, and emits `"pico://nvram-live-capture"`.
#[tauri::command]
pub async fn pico_capture_nvram_live(
    app: tauri::AppHandle,
    port_name: String,
    base_addr: Option<u32>,
    word_count: Option<u32>,
) -> Result<LiveNvramCaptureEventDto, String> {
    let start_addr = base_addr.unwrap_or(AM34_NVRAM_BASE);
    let total_words = word_count
        .unwrap_or(DEFAULT_NVRAM_CAPTURE_WORDS)
        .clamp(MIN_NVRAM_CAPTURE_WORDS, MAX_NVRAM_CAPTURE_WORDS);
    let chunk_words_limit: u32 = NVRAM_DUMP_CHUNK_WORDS;
    let mut accumulated: Vec<u8> = Vec::with_capacity((total_words * BYTES_PER_U32_WORD) as usize);
    let mut words_read: u32 = 0;

    while words_read < total_words {
        let cur_addr = start_addr.wrapping_add(words_read * BYTES_PER_U32_WORD);
        let chunk_words = (total_words - words_read).min(chunk_words_limit);
        let arg = format!("0x{:08X} {}", cur_addr, chunk_words);
        let port_clone = port_name.clone();
        let res = tauri::async_runtime::spawn_blocking(move || {
            execute_diagnostic_on_port(&port_clone, DEFAULT_DUMP_DIAGNOSTIC_CMD, Some(&arg))
        })
        .await
        .map_err(|e| format!("Falha interna de execução assíncrona: {}", e))??;

        if !res.success {
            return Err(format!(
                "Falha ao capturar bloco NVRAM em 0x{:08X}: {}",
                cur_addr, res.summary
            ));
        }
        let mut chunk_bytes =
            panadiga_core::pico::cdc::parse_pico_dump_hex_to_bytes(&res.raw_output);
        if chunk_bytes.is_empty() {
            return Err(format!(
                "DUMP retornou 0 bytes legíveis em 0x{:08X} ({}).",
                cur_addr, res.summary
            ));
        }
        accumulated.append(&mut chunk_bytes);
        words_read += chunk_words;
    }

    build_and_emit_live_nvram_capture(
        &app,
        SOURCE_PICO_LIVE_NVRAM,
        &port_name,
        start_addr,
        &accumulated,
    )
}

/// Returns the complete Hardware & Firmware Single Source of Truth (SSOT) specification.
#[tauri::command]
pub fn get_hardware_ssot_spec() -> Result<HardwareSsotSpecDto, String> {
    Ok(build_hardware_ssot_spec())
}

/// Executes any granular Pico firmware command asynchronously without stalling the WebView UI,
/// pushing TX/RX logs, structured diagnostic output, and updated telemetry over Tauri events.
#[tauri::command]
pub async fn pico_execute_diagnostic(
    app: tauri::AppHandle,
    port_name: String,
    command_id: String,
    arg: Option<String>,
) -> Result<PicoDiagnosticResultDto, String> {
    let cmd_preview = match arg.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => format!("{} {}", command_id.trim().to_ascii_uppercase(), a),
        None => command_id.trim().to_ascii_uppercase(),
    };

    let _ = app.emit(
        EVENT_PICO_SERIAL_LOG,
        ReactiveSerialEventDto {
            id: format!(
                "tx-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            timestamp: current_time_hhmmss(),
            source: "HOST".to_string(),
            level: "tx".to_string(),
            message: format!("> {}", cmd_preview),
        },
    );

    let port_clone = port_name.clone();
    let cmd_clone = command_id.clone();
    let arg_clone = arg.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        execute_diagnostic_on_port(&port_clone, &cmd_clone, arg_clone.as_deref())
    })
    .await
    .map_err(|e| format!("Falha interna ao aguardar diagnóstico Pico: {}", e))??;

    let _ = app.emit(EVENT_PICO_DIAG_RESULT, &result);
    if result.command_id == DEFAULT_DUMP_DIAGNOSTIC_CMD && result.success {
        let bytes = panadiga_core::pico::cdc::parse_pico_dump_hex_to_bytes(&result.raw_output);
        if !bytes.is_empty() {
            let base_addr = result
                .dump_rows
                .first()
                .and_then(|row| {
                    u32::from_str_radix(
                        row.address
                            .trim_start_matches("0x")
                            .trim_start_matches("0X"),
                        16,
                    )
                    .ok()
                })
                .unwrap_or(AM34_NVRAM_BASE);
            let _ = build_and_emit_live_nvram_capture(
                &app,
                SOURCE_PICO_DIAG_DUMP,
                &port_name,
                base_addr,
                &bytes,
            );
        }
    }
    let _ = app.emit(
        EVENT_PICO_SERIAL_LOG,
        ReactiveSerialEventDto {
            id: format!(
                "rx-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            timestamp: current_time_hhmmss(),
            source: "PICO".to_string(),
            level: if result.success { "rx" } else { "error" }.to_string(),
            message: result.summary.clone(),
        },
    );

    if STATUS_REFRESH_COMMANDS.contains(&result.command_id.as_str())
        && let Ok(mut status) = pico_get_status(Some(port_name)).await
    {
        if let Some(diag_vdd) = result.vdd_volts
            && status.vdd_volts <= VDD_ZERO_EPSILON_VOLTS
            && diag_vdd > VDD_ZERO_EPSILON_VOLTS
        {
            status.vdd_volts = diag_vdd;
            status.target_powered =
                (VDD_TARGET_MIN_VOLTS..VDD_TARGET_FLOATING_CEIL_VOLTS).contains(&diag_vdd);
            if status.target_powered && status.core_state == "RESET" {
                status.core_state = "RUNNING".to_string();
            }
        }
        let _ = app.emit(EVENT_PICO_TELEMETRY, &status);
    }

    Ok(result)
}

/// Enables or pauses the single background Pico telemetry push stream (`pico://telemetry`).
#[tauri::command]
pub fn set_pico_telemetry_stream(
    app: tauri::AppHandle,
    active: bool,
    port_name: Option<String>,
) -> Result<bool, String> {
    start_port_mutation_observer(app.clone());
    let state = stream_state();
    if let Some(port) = port_name.filter(|p| !p.trim().is_empty())
        && let Ok(mut guard) = state.port_name.lock()
    {
        *guard = port;
    }
    state.active.store(active, Ordering::Relaxed);

    if active
        && state
            .worker_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    {
        let app_clone = app.clone();
        thread::spawn(move || {
            let st = stream_state();
            loop {
                thread::sleep(Duration::from_millis(TELEMETRY_WORKER_POLL_MS));
                if !st.active.load(Ordering::Relaxed) {
                    continue;
                }
                let port = st.port_name.lock().map(|p| p.clone()).unwrap_or_default();
                if port.trim().is_empty() {
                    continue;
                }
                if port.starts_with(TRANSPORT_PREFIX_BLE)
                    || port.starts_with(TRANSPORT_PREFIX_ANDROID_OTG)
                {
                    continue;
                }
                if let Some(Ok(status)) = crate::pico::try_pico_get_device_status_dto(port) {
                    let _ = app_clone.emit(EVENT_PICO_TELEMETRY, &status);
                }
            }
        });
    }

    Ok(active)
}
