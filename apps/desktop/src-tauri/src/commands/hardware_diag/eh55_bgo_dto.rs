#![deny(unsafe_code)]

//! DTOs and NOR slice parser for EH55 Spansion NOR dumping, BGO audit, and BGO repair commands.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use panadiga_core::constants::{
    AM34_EH55_FLASH_BGO_BASE, AM34_EH55_FLASH_BGO_SIZE, DEFAULT_BAUD_RATE,
    resolve_writable_data_root,
};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::pico::StreamProgressTelemetry;
use panadiga_core::pico::panamon1::SerialPanamon1Transport;
use panadiga_core::re::eh55_bgo::{BgoApplyStage, BgoLiveApplyReport, BgoRepairPlan};
use panadiga_core::re::nor_dumper::{NorSliceCaptureReport, NorSlicePreset};
use serde::{Deserialize, Serialize};

use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::resolve_write_path;
use crate::commands::unbrick_dto::{opt_trimmed, resolve_dir_target};
use crate::state::PanadigaBackendState;

pub(crate) const UPPER_NOR_CONFIRM_TOKEN: &str = "CONFIRM_UPPER_NOR";

fn epoch_secs_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Resolves an explicit output file path or auto-generates a timestamped file under `<writable_root>/backups/`.
pub(crate) fn resolve_auto_backup_file(
    explicit: Option<&str>,
    prefix: &str,
    ext: &str,
) -> IpcResult<PathBuf> {
    if let Some(raw) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return resolve_write_path(raw);
    }
    let dir = resolve_writable_data_root().join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha criando diretório de backup '{}': {e}",
            dir.display()
        ))
    })?;
    Ok(dir.join(format!("{prefix}_{}.{ext}", epoch_secs_now())))
}

/// Resolves an explicit backup directory or auto-generates a timestamped folder under `<writable_root>/backups/`.
pub(crate) fn resolve_auto_backup_dir(explicit: Option<&str>, prefix: &str) -> IpcResult<PathBuf> {
    if let Some(raw) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return resolve_dir_target(raw);
    }
    let dir = resolve_writable_data_root()
        .join("backups")
        .join(format!("{prefix}_{}", epoch_secs_now()));
    std::fs::create_dir_all(&dir).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha criando diretório de backup '{}': {e}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicoEh55NorDumpRequestDto {
    pub port_name: String,
    pub slice: Option<String>,
    pub custom_start_va_hex: Option<String>,
    pub custom_len_hex: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NorDumpProgressEventDto {
    pub port: String,
    pub stage: String,
    pub current: usize,
    pub total: usize,
    #[serde(flatten)]
    pub telemetry: StreamProgressTelemetry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicoEh55NorDumpResponseDto {
    pub output_path: String,
    pub start_va_hex: String,
    pub total_len: usize,
    pub sha256: String,
    pub slices: Vec<NorSliceCaptureReport>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicoEh55BgoAuditRequestDto {
    pub source_mode: String,
    pub nor_path: Option<String>,
    pub port_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicoEh55BgoRepairRequestDto {
    pub nor_path: Option<String>,
    pub plan_path: Option<String>,
    pub reference_nor_path: Option<String>,
    pub out_plan_path: Option<String>,
    pub cow_out_path: Option<String>,
    pub dry_run_only: bool,
    pub port_name: Option<String>,
    pub backup_dir: Option<String>,
    pub confirm_token: Option<String>,
    pub run_boot_trace_ms: Option<u32>,
    #[serde(default)]
    pub nonce_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BgoRepairProgressEventDto {
    pub stage: BgoApplyStage,
    pub current: usize,
    pub total: usize,
    pub detail: String,
    #[serde(flatten)]
    pub telemetry: StreamProgressTelemetry,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PicoEh55BgoRepairResponseDto {
    pub dry_run_only: bool,
    pub sa0_to_sa3_locked_e05: bool,
    pub plan: BgoRepairPlan,
    pub out_plan_path: Option<String>,
    pub cow_out_path: Option<String>,
    pub live_report: Option<BgoLiveApplyReport>,
}

pub(crate) async fn run_exclusive_panamon1<R, F>(
    backend: PanadigaBackendState,
    clean_port: String,
    op_label: &'static str,
    f: F,
) -> IpcResult<R>
where
    R: Send + 'static,
    F: FnOnce(SerialPanamon1Transport) -> IpcResult<R> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        backend.with_exclusive_serial(&clean_port, || {
            let transport = SerialPanamon1Transport::open(
                &clean_port,
                DEFAULT_BAUD_RATE,
                Duration::from_millis(1_500),
            )
            .map_err(|e| PanadigaIpcError::serial(format!("Falha abrindo '{clean_port}': {e}")))?;
            f(transport)
        })
    })
    .await
    .map_err(|e| PanadigaIpcError::internal(format!("Falha assíncrona {op_label}: {e}")))?
}

pub(crate) fn parse_nor_slice_preset(
    slice_opt: &Option<String>,
    start_opt: &Option<String>,
    len_opt: &Option<String>,
) -> IpcResult<NorSlicePreset> {
    let key = opt_trimmed(slice_opt)
        .unwrap_or("full")
        .to_ascii_lowercase();
    if key == "custom" {
        let va = parse_u32_radix(opt_trimmed(start_opt).ok_or_else(|| {
            PanadigaIpcError::validation("customStartVaHex obrigatório para slice 'custom'.")
        })?)
        .map_err(|e| PanadigaIpcError::validation(format!("Endereço hex inválido: {e}")))?;
        let bytes = parse_u32_radix(opt_trimmed(len_opt).ok_or_else(|| {
            PanadigaIpcError::validation("customLenHex obrigatório para slice 'custom'.")
        })?)
        .map_err(|e| PanadigaIpcError::validation(format!("Tamanho hex inválido: {e}")))?;
        return Ok(NorSlicePreset::Custom {
            start_va: va,
            len: bytes,
        });
    }
    match key.as_str() {
        "nvram" | "nvram64k" => Ok(NorSlicePreset::Nvram64k),
        "loader" | "loader256k" | "ipl" => Ok(NorSlicePreset::Loader256k),
        "bgo" => Ok(NorSlicePreset::Custom {
            start_va: AM34_EH55_FLASH_BGO_BASE,
            len: AM34_EH55_FLASH_BGO_SIZE,
        }),
        "full" | "full8mb" | "8mb" => Ok(NorSlicePreset::Full8Mb),
        other => Err(PanadigaIpcError::validation(format!(
            "Slice NOR inválido '{other}': use full, bgo, nvram, loader ou custom."
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_backup_path_and_dir_resolve_under_writable_root() {
        let f = resolve_auto_backup_file(None, "eh55_test_dump", "bin").unwrap();
        assert!(f.to_string_lossy().contains("backups"));
        assert!(f.to_string_lossy().ends_with(".bin"));

        let f_blank = resolve_auto_backup_file(Some("   "), "eh55_test_dump", "bin").unwrap();
        assert!(f_blank.to_string_lossy().contains("backups"));

        let d = resolve_auto_backup_dir(Some("   "), "eh55_test_dir").unwrap();
        assert!(d.is_dir());
        let _ = std::fs::remove_dir_all(&d);
    }
}
