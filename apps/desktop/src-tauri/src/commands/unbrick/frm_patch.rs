//! Tauri IPC commands for `.FRM` module extraction, BGO patching, TBC passthrough, and unlock verification.

use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::matshita_emu::{
    FrmUnlockVerificationSummary, build_sample_unlock_frm, parse_hex_payload,
    parse_unlock_container_mode, verify_unlock_frm_container,
};
use panadiga_core::re::{
    FrmModuleExtractionReport, MainPassthroughMode, Prog061BgoPatchMode, UnbrickWorkspaceReport,
    build_passthrough_rsp_plan, extract_modules_from_frm, inspect_main_passthrough_sites,
    inspect_prog061_container_bytes, inspect_unbrick_workspace, patch_main_passthrough_file,
    patch_prog061_or_build_digadiag_with_policy,
};
use serde_json::Value;
use std::path::PathBuf;

use crate::commands::constants::{
    DEFAULT_MAIN_PASSTHROUGH_MODE, SAMPLE_UNLOCK_PAYLOAD_BYTE, SAMPLE_UNLOCK_PAYLOAD_LEN,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::{resolve_safe_read_path, resolve_write_path};
use crate::commands::unbrick_dto::{opt_trimmed, resolve_dir_target};

fn serialize_value<T: serde::Serialize>(val: T) -> IpcResult<Value> {
    serde_json::to_value(val).map_err(|e| PanadigaIpcError::internal(e.to_string()))
}

/// Inspects workspace binaries, PROG061 BGO status, MAIN TBC passthrough, RSP plan, & hardware facts.
#[tauri::command]
pub fn inspect_unbrick_workbench(
    workspace_path: Option<String>,
) -> IpcResult<UnbrickWorkspaceReport> {
    let override_path = opt_trimmed(&workspace_path).map(PathBuf::from);
    Ok(inspect_unbrick_workspace(override_path.as_deref()))
}

/// Extracts and Feistel-decrypts modules (`DRV S211`, `PROG061`, `MAIN`) from `.FRM`.
#[tauri::command]
pub fn extract_frm_modules_command(
    frm_path: String,
    out_dir: String,
    unpack_payloads: Option<bool>,
) -> IpcResult<FrmModuleExtractionReport> {
    let in_file = resolve_safe_read_path(frm_path.trim())?;
    let dest = resolve_dir_target(&out_dir)?;
    extract_modules_from_frm(&in_file, &dest, unpack_payloads.unwrap_or(true))
        .map_err(|e| PanadigaIpcError::classify(format!("Falha ao extrair módulos do .FRM: {e}")))
}

/// Inspects or patches `PROG061` BGO (`U99`) sites and optionally wraps as 128 KiB `DIGADIAG.FRM`.
#[tauri::command]
pub fn patch_bgo_unbrick_command(
    input_path: String,
    out_path: Option<String>,
    mode: String,
    wrap_as_digadiag: Option<bool>,
    allow_unsafe_bgo_format: Option<bool>,
) -> IpcResult<Value> {
    let in_file = resolve_safe_read_path(input_path.trim())?;
    let parsed_mode: Prog061BgoPatchMode = mode.parse().map_err(|e: anyhow::Error| {
        PanadigaIpcError::validation(format!("Modo BGO inválido: {e}"))
    })?;

    if let Some(out_raw) = opt_trimmed(&out_path) {
        let out_file = resolve_write_path(out_raw)?;
        let report = patch_prog061_or_build_digadiag_with_policy(
            &in_file,
            &out_file,
            parsed_mode,
            wrap_as_digadiag.unwrap_or(true),
            allow_unsafe_bgo_format.unwrap_or(false),
        )
        .map_err(|e| {
            PanadigaIpcError::classify(format!("Falha ao aplicar patch BGO PROG061: {e}"))
        })?;
        serialize_value(report)
    } else {
        let raw = std::fs::read(&in_file).map_err(|e| {
            PanadigaIpcError::io(format!("Falha lendo '{}': {e}", in_file.display()))
        })?;
        let report = inspect_prog061_container_bytes(&raw)?;
        serialize_value(report)
    }
}

/// Inspects/patches `MAIN` (`3410PQ`) AV TBC (`E-E`) passthrough sites or emits `_dev_SERIAL` RSP plan.
#[tauri::command]
pub fn patch_main_passthrough_command(
    action: String,
    input_path: Option<String>,
    out_path: Option<String>,
    mode: Option<String>,
    callback_target_va: Option<String>,
) -> IpcResult<Value> {
    if action.trim().eq_ignore_ascii_case("rsp_plan") {
        let cb = opt_trimmed(&callback_target_va)
            .map(parse_u32_radix)
            .transpose()?;
        return serialize_value(build_passthrough_rsp_plan(cb));
    }

    let in_raw = opt_trimmed(&input_path).ok_or_else(|| {
        PanadigaIpcError::validation("Caminho do binário MAIN (3410PQ) é obrigatório.")
    })?;
    let in_file = resolve_safe_read_path(in_raw)?;

    if action.trim().eq_ignore_ascii_case("patch") {
        let out_raw = opt_trimmed(&out_path).ok_or_else(|| {
            PanadigaIpcError::validation("Caminho de saída do MAIN patched é obrigatório.")
        })?;
        let parsed_mode: MainPassthroughMode = mode
            .as_deref()
            .unwrap_or(DEFAULT_MAIN_PASSTHROUGH_MODE)
            .parse()
            .map_err(|e: anyhow::Error| PanadigaIpcError::validation(e.to_string()))?;
        let report =
            patch_main_passthrough_file(&in_file, &resolve_write_path(out_raw)?, parsed_mode)
                .map_err(|e| {
                    PanadigaIpcError::classify(format!("Falha ao patchear MAIN passthrough: {e}"))
                })?;
        return serialize_value(report);
    }

    let buf = std::fs::read(&in_file)
        .map_err(|e| PanadigaIpcError::io(format!("Falha lendo MAIN: {e}")))?;
    let report = inspect_main_passthrough_sites(&buf)?;
    serialize_value(report)
}

/// Verifies an SD (`EXTRHEAD` / `PANA__SD`) or CD (`PANA_DVD.FRM` / `cd`) unlock container against EH55 bootloader rules.
#[tauri::command]
pub fn verify_frm_unlock_command(
    container_kind: String,
    frm_path: String,
) -> IpcResult<FrmUnlockVerificationSummary> {
    let path = resolve_safe_read_path(frm_path.trim())?;
    let bytes = std::fs::read(&path).map_err(|e| {
        PanadigaIpcError::io(format!("Falha lendo container '{}': {e}", path.display()))
    })?;
    verify_unlock_frm_container(container_kind.trim(), &bytes)
        .map_err(|e| PanadigaIpcError::classify(format!("Falha ao verificar container .FRM: {e}")))
}

/// Synthesizes and verifies a sample SD (`sd`/`sd48`) or CD (`cd`/`dvd`) unlock `.FRM` container (optionally saved to disk).
#[tauri::command]
pub fn pack_unlock_frm_command(
    container_kind: String,
    out_path: Option<String>,
    payload_hex: Option<String>,
) -> IpcResult<Value> {
    let mode = parse_unlock_container_mode(container_kind.trim()).map_err(|e| {
        PanadigaIpcError::validation(format!("Modo de container unlock inválido: {e}"))
    })?;
    let payload = match opt_trimmed(&payload_hex) {
        Some(h) => parse_hex_payload(h)
            .map_err(|e| PanadigaIpcError::validation(format!("Payload hex inválido: {e}")))?,
        None => vec![SAMPLE_UNLOCK_PAYLOAD_BYTE; SAMPLE_UNLOCK_PAYLOAD_LEN],
    };
    let (bytes, verification) = build_sample_unlock_frm(mode, &payload).map_err(|e| {
        PanadigaIpcError::classify(format!("Falha ao empacotar container unlock .FRM: {e}"))
    })?;
    let written_path = opt_trimmed(&out_path)
        .map(|raw| {
            let dest = resolve_write_path(raw)?;
            std::fs::write(&dest, &bytes).map_err(|e| {
                PanadigaIpcError::io(format!("Falha gravando '{}': {e}", dest.display()))
            })?;
            Ok::<_, PanadigaIpcError>(dest.to_string_lossy().into_owned())
        })
        .transpose()?;
    Ok(serde_json::json!({
        "verification": verification,
        "output_path": written_path,
        "container_bytes": bytes.len(),
    }))
}
