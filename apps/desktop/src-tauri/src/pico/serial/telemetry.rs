//! Telemetry status querying, VDD/ADC validation, and session state flags.

use std::time::Duration;

use panadiga_core::pico::cdc::parse_vdd_reply;
use panadiga_core::pico::diag_rows::parse_wdt_status;

use crate::commands::constants::{
    CHIP_ID_RP2040, CHIP_ID_RP2350, DEFAULT_BAUD_RATE, DEFAULT_COLD_BREAKIN_MODE,
    DEFAULT_OCD_BUS_FREQ_HZ, DEFAULT_PICO_FW_FALLBACK, DEFAULT_PIO_UART_BAUD_115200_CMD,
    DEFAULT_PIO_UART_FRAMING_8N1_CMD, SERIAL_AUTODETECT_TIMEOUT_MS, SERIAL_OPEN_TIMEOUT_MS,
    SERIAL_PING_STATUS_TIMEOUT_MS, SERIAL_RETRY_BACKOFF_MS, SERIAL_UART_SYNC_TIMEOUT_MS,
    VDD_TARGET_FLOATING_CEIL_VOLTS, VDD_TARGET_MIN_VOLTS,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::state::PanadigaBackendState;

use super::super::types::{PicoDeviceStatusDto, PicoStatusResponse, PicoWdtFields};
use super::transport::open_validated_cdc;

fn ensure_default_uart_and_probe_target(
    port: &mut dyn serialport::SerialPort,
    target_powered: bool,
) -> Option<String> {
    let framing_out = panadiga_core::pico::cdc::send_and_read_until(
        port,
        DEFAULT_PIO_UART_FRAMING_8N1_CMD,
        &["FRAMING: OK", "FRAMING: ERROR", "PANADIGA>"],
        Duration::from_millis(SERIAL_UART_SYNC_TIMEOUT_MS),
    );
    let baud_out = panadiga_core::pico::cdc::send_and_read_until(
        port,
        DEFAULT_PIO_UART_BAUD_115200_CMD,
        &["BAUD: OK", "BAUD: ERROR", "PANADIGA>"],
        Duration::from_millis(SERIAL_UART_SYNC_TIMEOUT_MS),
    );
    let uart_synced = framing_out.contains("FRAMING: OK") && baud_out.contains("BAUD: OK");
    if !target_powered {
        return uart_synced.then(|| "PIO1 UART=115200 8N1 sincronizada".to_string());
    }
    let autodetect_out = panadiga_core::pico::cdc::send_and_read_until(
        port,
        "AUTODETECT",
        &["[AUTODETECT]", "AUTODETECT: OK", "PANADIGA>"],
        Duration::from_millis(SERIAL_AUTODETECT_TIMEOUT_MS),
    );
    let clean_probe = autodetect_out
        .lines()
        .map(str::trim)
        .find(|line| line.contains("NOR Flash detected") || line.contains("AUTODETECT: OK"));
    match (uart_synced, clean_probe) {
        (true, Some(probe)) => Some(format!("PIO1 UART=115200 8N1 | {probe}")),
        (true, None) => {
            Some("PIO1 UART=115200 8N1 pronta (P7501 Async UART RX/TX ativo)".to_string())
        }
        (false, Some(probe)) => Some(probe.to_string()),
        (false, None) => None,
    }
}

fn pico_connect_and_test_unlocked(
    backend: &PanadigaBackendState,
    port_name: &str,
    baud_rate: u32,
    probe_target_on_connect: bool,
) -> IpcResult<PicoStatusResponse> {
    let (clean, mut port) =
        open_validated_cdc(backend, port_name, baud_rate, SERIAL_OPEN_TIMEOUT_MS)?;

    let response_text = panadiga_core::pico::cdc::send_and_read_until(
        &mut *port,
        "PING",
        &["PONG"],
        Duration::from_millis(SERIAL_PING_STATUS_TIMEOUT_MS),
    );
    let mut vdd_text = panadiga_core::pico::cdc::send_and_read_until(
        &mut *port,
        "STATUS",
        &["PANADIGA>", "P7501>"],
        Duration::from_millis(SERIAL_PING_STATUS_TIMEOUT_MS),
    );
    let mut vdd_reading = parse_vdd_reply(&vdd_text);
    if vdd_reading.is_none() {
        std::thread::sleep(Duration::from_millis(SERIAL_RETRY_BACKOFF_MS));
        vdd_text = panadiga_core::pico::cdc::send_and_read_until(
            &mut *port,
            "STATUS",
            &["PANADIGA>", "P7501>"],
            Duration::from_millis(SERIAL_PING_STATUS_TIMEOUT_MS),
        );
        vdd_reading = parse_vdd_reply(&vdd_text);
    }
    let wdt = parse_wdt_status(&vdd_text);

    let has_pong = response_text.contains("PONG") || response_text.contains("PANADIGA");
    if (!has_pong && vdd_reading.is_none()) || (response_text.is_empty() && vdd_reading.is_none()) {
        return Err(PanadigaIpcError::serial(format!(
            "O Pico em {} abriu a porta mas não respondeu a PING/STATUS (firmware Panadiga ausente ou DTR não aceito). \
             Grave o firmware pelo botão \"Flash Pico (BOOTSEL)\" ou reconecte o cabo USB.",
            clean
        )));
    }

    let vdd_volts = vdd_reading.unwrap_or(0.0f32);
    let probe_floating = vdd_volts >= VDD_TARGET_FLOATING_CEIL_VOLTS;
    let target_powered = !probe_floating && vdd_volts >= VDD_TARGET_MIN_VOLTS;

    let is_pico2 = response_text.contains("RP2350") || response_text.contains("Pico 2");
    let dev_label = if is_pico2 {
        "Raspberry Pi Pico 2 (RP2350)"
    } else {
        "Raspberry Pi Pico (RP2040)"
    };

    let banner = if response_text.is_empty() {
        format!("{} Conectado em {}", dev_label, clean)
    } else {
        response_text.trim().to_string()
    };

    let target_note = if probe_target_on_connect {
        ensure_default_uart_and_probe_target(&mut *port, target_powered)
    } else {
        None
    };

    let base_msg = if probe_floating {
        format!(
            "ALERTA: VDD={:.2}V (ADC flutuante — cabo P7501 NÃO conectado ou pino SW_UART em circuito aberto em GP26).",
            vdd_volts
        )
    } else if !target_powered {
        format!(
            "ALERTA: VDD={:.2}V abaixo do limiar. Gravadora desligada ou cabo ruim.",
            vdd_volts
        )
    } else {
        format!(
            "{} operacional na porta. VDD={:.2}V OK.",
            dev_label, vdd_volts
        )
    };
    let vdd_msg = match target_note {
        Some(note) => format!("{base_msg} [{note}]"),
        None => base_msg,
    };

    Ok(PicoStatusResponse {
        connected: true,
        port_name: clean,
        baud_rate,
        vdd_voltage: vdd_volts,
        target_powered,
        firmware_banner: banner,
        message: vdd_msg,
        wdt: wdt
            .map(|w| PicoWdtFields {
                wdt_active: Some(w.active),
                wdt_timeout_ms: Some(w.timeout_ms),
                wdt_recoveries: Some(w.recoveries),
                wdt_reboot_src: Some(w.reboot_src),
                wdt_fault_stage: Some(w.fault_stage),
                wdt_last_cmd: Some(w.last_cmd),
            })
            .unwrap_or_default(),
    })
}

/// Connects to a hardware Pico CDC port and retrieves device telemetry.
pub fn pico_connect_and_test(
    backend: &PanadigaBackendState,
    port_name: String,
    baud_rate: u32,
) -> IpcResult<PicoStatusResponse> {
    backend
        .serial
        .with_lock(|| pico_connect_and_test_unlocked(backend, &port_name, baud_rate, true))
}

fn build_disconnected_status_dto() -> PicoDeviceStatusDto {
    PicoDeviceStatusDto {
        connected: false,
        port: "".to_string(),
        baud_rate: 0,
        vdd_volts: 0.0,
        target_powered: false,
        breakin_mode: DEFAULT_COLD_BREAKIN_MODE,
        core_state: "DISCONNECTED".to_string(),
        bus_freq_hz: 0,
        firmware_version: "".to_string(),
        chip_id: "".to_string(),
        total_probes: 0,
        hits_detected: 0,
        open_bus_high: 0,
        open_bus_low: 0,
        ram_loader_injected: false,
        last_keepalive: "".to_string(),
        wdt: PicoWdtFields::default(),
    }
}

fn pico_get_device_status_dto_unlocked(
    backend: &PanadigaBackendState,
    port_name: String,
    mode_override: Option<u8>,
) -> IpcResult<PicoDeviceStatusDto> {
    if port_name.trim().is_empty() {
        return Ok(build_disconnected_status_dto());
    }
    let status = pico_connect_and_test_unlocked(backend, &port_name, DEFAULT_BAUD_RATE, false)?;
    let is_rp2350 = status.port_name.contains("2350") || status.firmware_banner.contains("2350");

    let (probes, hits) = backend.telemetry.scan_counts();
    let ram_loaded = backend.telemetry.is_ram_loader_injected();
    let firmware_ver = if !status.firmware_banner.is_empty() {
        status.firmware_banner.clone()
    } else {
        DEFAULT_PICO_FW_FALLBACK.to_string()
    };

    Ok(PicoDeviceStatusDto {
        connected: status.connected,
        port: status.port_name,
        baud_rate: status.baud_rate,
        vdd_volts: status.vdd_voltage,
        target_powered: status.target_powered,
        breakin_mode: mode_override.unwrap_or(DEFAULT_COLD_BREAKIN_MODE),
        core_state: if !status.connected {
            "DISCONNECTED".to_string()
        } else if !status.target_powered {
            "RESET".to_string()
        } else if ram_loaded {
            "HALTED".to_string()
        } else {
            "RUNNING".to_string()
        },
        bus_freq_hz: DEFAULT_OCD_BUS_FREQ_HZ,
        firmware_version: firmware_ver,
        chip_id: if is_rp2350 {
            CHIP_ID_RP2350.to_string()
        } else {
            CHIP_ID_RP2040.to_string()
        },
        total_probes: probes,
        hits_detected: hits,
        open_bus_high: 0,
        open_bus_low: 0,
        ram_loader_injected: ram_loaded,
        last_keepalive: "OK".to_string(),
        wdt: status.wdt,
    })
}

/// Retrieves full `PicoDeviceStatusDto` matching the React Frontend `PicoDeviceStatus` interface.
pub fn pico_get_device_status_dto(
    backend: &PanadigaBackendState,
    port_name: String,
    mode_override: Option<u8>,
) -> IpcResult<PicoDeviceStatusDto> {
    if port_name.trim().is_empty() {
        return Ok(build_disconnected_status_dto());
    }
    backend
        .serial
        .with_lock(|| pico_get_device_status_dto_unlocked(backend, port_name, mode_override))
}

/// Non-blocking status poll for the background telemetry worker.
pub fn try_pico_get_device_status_dto(
    backend: &PanadigaBackendState,
    port_name: String,
) -> Option<IpcResult<PicoDeviceStatusDto>> {
    if port_name.trim().is_empty() || backend.is_any_bridge_active() {
        return None;
    }
    backend
        .serial
        .try_with_lock(|| pico_get_device_status_dto_unlocked(backend, port_name, None))
}
