#![deny(unsafe_code)]

//! Tauri IPC command for AM34 `DAVIS CH0/CH1` IDE & `0xEC IDENTIFY` probing (`davis://probe-result`).

use panadiga_core::constants::AM34_FORBIDDEN_LETHAL_REG;
use panadiga_core::pico::panamon1::{Panamon1Client, Panamon1Transport};
use panadiga_core::re::davis_probe::{
    DavisProbeConfig, DavisProbeEngine, DavisProbeReport, validate_safe_mmio_read_range,
};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::eh55_bgo_dto::run_exclusive_panamon1;

use crate::commands::constants::EVENT_DAVIS_PROBE_RESULT;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::unbrick_dto::opt_trimmed;
use crate::state::PanadigaBackendState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicoProbeDavisIdeRequestDto {
    pub port_name: String,
    pub drive: Option<String>,
    pub boot_mode: Option<String>,
    pub boot_timeout_ms: Option<u32>,
    #[serde(default)]
    pub nonce_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PicoProbeDavisIdeResponseDto {
    pub port: String,
    pub drive_sel_hex: String,
    pub lethal_0xc000100c_blocked: bool,
    pub report: DavisProbeReport,
}

pub fn parse_davis_drive_selector(drive: Option<&str>) -> IpcResult<u8> {
    match drive
        .unwrap_or("master")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "master" => Ok(0xA0),
        "slave" => Ok(0xB0),
        other => Err(PanadigaIpcError::validation(format!(
            "Seletor DAVIS inválido '{other}': use 'master' ou 'slave'."
        ))),
    }
}

pub fn run_davis_probe_with_transport<T: Panamon1Transport>(
    transport: T,
    req: &PicoProbeDavisIdeRequestDto,
) -> IpcResult<PicoProbeDavisIdeResponseDto> {
    let drive_sel = parse_davis_drive_selector(opt_trimmed(&req.drive))?;
    let lethal_blocked = validate_safe_mmio_read_range(AM34_FORBIDDEN_LETHAL_REG, 4).is_err();
    if !lethal_blocked {
        return Err(PanadigaIpcError::internal(
            "Invariante 0xC000100C não bloqueada pelo motor DAVIS.",
        ));
    }

    let cfg = DavisProbeConfig {
        drive_sel,
        boot_mode: Some(opt_trimmed(&req.boot_mode).unwrap_or("NORMAL").to_string()),
        boot_timeout_ms: req.boot_timeout_ms.unwrap_or(15_000),
        ..DavisProbeConfig::default()
    };
    let mut client = Panamon1Client::new(transport);
    let report = {
        let mut engine = DavisProbeEngine::new(&mut client, cfg);
        engine
            .run_probe()
            .map_err(|e| PanadigaIpcError::serial(format!("Falha no probe DAVIS IDE: {e}")))?
    };

    Ok(PicoProbeDavisIdeResponseDto {
        port: req.port_name.trim().to_string(),
        drive_sel_hex: format!("0x{drive_sel:02X}"),
        lethal_0xc000100c_blocked: lethal_blocked,
        report,
    })
}

/// Probes AM34 `DAVIS CH0/CH1` & `0xEC IDENTIFY` over Pico `PANAMON1>` (guarding `0xC000_100C`),
/// emitting `"davis://probe-result"`.
#[tauri::command]
pub async fn pico_probe_davis_ide_command(
    app: tauri::AppHandle,
    state: tauri::State<'_, PanadigaBackendState>,
    request: PicoProbeDavisIdeRequestDto,
) -> IpcResult<PicoProbeDavisIdeResponseDto> {
    let clean_port = request.port_name.trim().to_string();
    let _ = parse_davis_drive_selector(opt_trimmed(&request.drive))?;
    super::bench_control::require_destructive_nonce(
        state.inner(),
        request.nonce_id.as_deref(),
        panadiga_core::bench::OpKind::RamProbe,
        opt_trimmed(&request.drive).unwrap_or("master"),
        &clean_port,
    )?;
    let backend = state.inner().clone();
    let req_clone = request.clone();

    let res = run_exclusive_panamon1(backend, clean_port, "DAVIS probe", move |transport| {
        run_davis_probe_with_transport(transport, &req_clone)
    })
    .await?;

    let _ = app.emit(EVENT_DAVIS_PROBE_RESULT, &res);
    Ok(res)
}
