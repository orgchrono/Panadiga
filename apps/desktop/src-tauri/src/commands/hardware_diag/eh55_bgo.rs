#![deny(unsafe_code)]

//! Tauri IPC commands for live Spansion NOR capture and EH55 BGO FTL audit (`DUMP_FILE` / `LIVE_PICO`).

use std::fs;
use std::time::Instant;

use panadiga_core::constants::{
    AM34_EH55_FLASH_BGO_BASE, AM34_EH55_FLASH_BGO_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE,
};
use panadiga_core::pico::panamon1::{Panamon1Client, Panamon1Transport, SerialPanamon1Transport};
use panadiga_core::pico::{StreamProgressTelemetry, compute_rate_and_eta, flash_va_to_sector_sa};
use panadiga_core::re::eh55_bgo::{Eh55BgoAuditReport, audit_eh55_bgo_nor_8mb};
use panadiga_core::re::nor_dumper::{NorCaptureOrchestrator, NorSlicePreset};
use tauri::Emitter;

use super::eh55_bgo_dto::{
    NorDumpProgressEventDto, PicoEh55BgoAuditRequestDto, PicoEh55NorDumpRequestDto,
    PicoEh55NorDumpResponseDto, parse_nor_slice_preset, resolve_auto_backup_file,
    run_exclusive_panamon1,
};
use crate::commands::constants::EVENT_NOR_DUMP_PROGRESS;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::resolve_read_path;
use crate::commands::unbrick_dto::opt_trimmed;
use crate::state::PanadigaBackendState;

pub fn run_eh55_nor_dump_with_transport<T: Panamon1Transport, F>(
    transport: T,
    req: &PicoEh55NorDumpRequestDto,
    mut on_progress: F,
) -> IpcResult<PicoEh55NorDumpResponseDto>
where
    F: FnMut(NorDumpProgressEventDto),
{
    let preset = parse_nor_slice_preset(&req.slice, &req.custom_start_va_hex, &req.custom_len_hex)?;
    let out_path = resolve_auto_backup_file(opt_trimmed(&req.output_path), "eh55_nor_dump", "bin")?;
    let orch = NorCaptureOrchestrator::try_default().map_err(|e| {
        PanadigaIpcError::internal(format!("Falha init NorCaptureOrchestrator: {e}"))
    })?;
    let mut client = Panamon1Client::new(transport);
    let port_label = req.port_name.trim().to_string();
    let (start_va, total_bytes_u32) = preset
        .bounds()
        .map_err(|e| PanadigaIpcError::validation(format!("Intervalo NOR inválido: {e}")))?;
    let total_bytes = total_bytes_u32 as usize;
    let started = Instant::now();
    let cap = orch
        .capture_preset(&mut client, preset, &mut |stage, cur, tot| {
            let bytes_done = if tot == 0 {
                0
            } else {
                total_bytes
                    .saturating_mul(cur)
                    .checked_div(tot)
                    .unwrap_or(0)
                    .min(total_bytes)
            };
            let active_va =
                start_va.saturating_add(bytes_done.min(total_bytes.saturating_sub(1)) as u32);
            let (bps, eta) =
                compute_rate_and_eta(started.elapsed().as_millis(), bytes_done, total_bytes);
            on_progress(NorDumpProgressEventDto {
                port: port_label.clone(),
                stage: stage.to_string(),
                current: cur,
                total: tot,
                telemetry: StreamProgressTelemetry {
                    active_sa: flash_va_to_sector_sa(active_va),
                    bytes_done: Some(bytes_done),
                    bytes_total: Some(total_bytes),
                    bytes_per_sec: bps,
                    eta_seconds: eta,
                },
            });
        })
        .map_err(|e| PanadigaIpcError::serial(format!("Falha capturando NOR via PANAMON1: {e}")))?;
    fs::write(&out_path, &cap.data).map_err(|e| {
        PanadigaIpcError::io(format!("Falha gravando '{}': {e}", out_path.display()))
    })?;
    Ok(PicoEh55NorDumpResponseDto {
        output_path: out_path.to_string_lossy().into_owned(),
        start_va_hex: format!("0x{:08X}", cap.start_va),
        total_len: cap.total_len,
        sha256: cap.sha256,
        slices: cap.slices,
    })
}

/// Captures Spansion `S29JL064H` NOR slices over Pico `PANAMON1>`, emitting `"nor://dump-progress"`.
#[tauri::command]
pub async fn pico_eh55_nor_dump_command(
    app: tauri::AppHandle,
    state: tauri::State<'_, PanadigaBackendState>,
    request: PicoEh55NorDumpRequestDto,
) -> IpcResult<PicoEh55NorDumpResponseDto> {
    let _ = parse_nor_slice_preset(
        &request.slice,
        &request.custom_start_va_hex,
        &request.custom_len_hex,
    )?;
    let resolved_out =
        resolve_auto_backup_file(opt_trimmed(&request.output_path), "eh55_nor_dump", "bin")?;
    let clean_port = request.port_name.trim().to_string();
    let backend = state.inner().clone();
    let app_clone = app.clone();
    let mut req_clone = request.clone();
    req_clone.output_path = Some(resolved_out.to_string_lossy().into_owned());

    run_exclusive_panamon1(backend, clean_port, "NOR dump", move |transport| {
        run_eh55_nor_dump_with_transport(transport, &req_clone, |ev| {
            let _ = app_clone.emit(EVENT_NOR_DUMP_PROGRESS, &ev);
        })
    })
    .await
}

pub fn run_eh55_bgo_audit_with_transport<T: Panamon1Transport>(
    req: &PicoEh55BgoAuditRequestDto,
    transport: Option<T>,
) -> IpcResult<Eh55BgoAuditReport> {
    match req.source_mode.trim().to_ascii_uppercase().as_str() {
        "DUMP_FILE" => {
            let nor_raw = opt_trimmed(&req.nor_path).ok_or_else(|| {
                PanadigaIpcError::validation("norPath obrigatório quando sourceMode='DUMP_FILE'.")
            })?;
            let path = resolve_read_path(nor_raw);
            let nor = fs::read(&path).map_err(|e| {
                PanadigaIpcError::io(format!("Falha lendo '{}': {e}", path.display()))
            })?;
            audit_eh55_bgo_nor_8mb(&nor)
                .map_err(|e| PanadigaIpcError::classify(format!("Falha auditoria BGO NOR: {e}")))
        }
        "LIVE_PICO" => {
            let t = transport.ok_or_else(|| {
                PanadigaIpcError::validation("Transporte serial Pico obrigatório em LIVE_PICO.")
            })?;
            let orch = NorCaptureOrchestrator::try_default()
                .map_err(|e| PanadigaIpcError::internal(format!("{e}")))?;
            let mut client = Panamon1Client::new(t);
            let cap = orch
                .capture_preset(
                    &mut client,
                    NorSlicePreset::Custom {
                        start_va: AM34_EH55_FLASH_BGO_BASE,
                        len: AM34_EH55_FLASH_BGO_SIZE,
                    },
                    &mut |_, _, _| {},
                )
                .map_err(|e| {
                    PanadigaIpcError::serial(format!("Falha lendo SA26..SA70 ao vivo: {e}"))
                })?;
            let mut nor_8mb = vec![0xFFu8; AM34_FLASH_SIZE as usize];
            let bgo_off = (AM34_EH55_FLASH_BGO_BASE - AM34_FLASH_UNCACHED_BASE) as usize;
            nor_8mb[bgo_off..bgo_off + cap.data.len()].copy_from_slice(&cap.data);
            audit_eh55_bgo_nor_8mb(&nor_8mb)
                .map_err(|e| PanadigaIpcError::classify(format!("Falha auditoria BGO live: {e}")))
        }
        other => Err(PanadigaIpcError::validation(format!(
            "sourceMode inválido '{other}': use 'DUMP_FILE' ou 'LIVE_PICO'."
        ))),
    }
}

/// Audits EH55 45-sector BGO FTL (`SA26..=SA70`) from an offline `'DUMP_FILE'` or `'LIVE_PICO'`.
#[tauri::command]
pub async fn pico_eh55_bgo_audit_command(
    state: tauri::State<'_, PanadigaBackendState>,
    request: PicoEh55BgoAuditRequestDto,
) -> IpcResult<Eh55BgoAuditReport> {
    if !request.source_mode.trim().eq_ignore_ascii_case("LIVE_PICO") {
        return run_eh55_bgo_audit_with_transport::<SerialPanamon1Transport>(&request, None);
    }
    let clean_port = opt_trimmed(&request.port_name)
        .ok_or_else(|| {
            PanadigaIpcError::validation("portName obrigatório quando sourceMode='LIVE_PICO'.")
        })?
        .to_string();
    let backend = state.inner().clone();
    let req_clone = request.clone();
    run_exclusive_panamon1(backend, clean_port, "BGO audit", move |t| {
        run_eh55_bgo_audit_with_transport(&req_clone, Some(t))
    })
    .await
}
