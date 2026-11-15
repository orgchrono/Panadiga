//! Thin Tauri IPC shell for the Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Center.

use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::{
    FrmModuleExtractionReport, MainPassthroughMode, Prog061BgoPatchMode, SdMediaVerificationReport,
    SdUnbrickKitReport, UnbrickWorkspaceReport, build_nor_dumper_artifact,
    build_passthrough_rsp_plan, extract_modules_from_frm, inspect_main_passthrough_sites,
    inspect_prog061_container_bytes, inspect_unbrick_workspace, patch_main_passthrough_file,
    patch_prog061_or_build_digadiag_with_policy, prepare_sd_unbrick_kit, verify_sd_unbrick_media,
};
use serde_json::Value;
use std::path::PathBuf;

use super::frm_pack::{resolve_read_path, resolve_write_path};
use super::unbrick_dto::{
    SdUnbrickKitRequestDto, format_nor_dumper_response, opt_trimmed, parse_nor_dumper_params,
    resolve_dir_target, to_sd_unbrick_kit_request,
};

fn serialize_value<T: serde::Serialize>(val: T) -> Result<Value, String> {
    serde_json::to_value(val).map_err(|e| e.to_string())
}

/// Inspects workspace binaries, PROG061 BGO status, MAIN TBC passthrough, RSP plan, & hardware facts.
#[tauri::command]
pub fn inspect_unbrick_workbench(
    workspace_path: Option<String>,
) -> Result<UnbrickWorkspaceReport, String> {
    let override_path = opt_trimmed(&workspace_path).map(PathBuf::from);
    Ok(inspect_unbrick_workspace(override_path.as_deref()))
}

/// Extracts and Feistel-decrypts modules (`DRV S211`, `PROG061`, `MAIN`) from `.FRM`.
#[tauri::command]
pub fn extract_frm_modules_command(
    frm_path: String,
    out_dir: String,
    unpack_payloads: Option<bool>,
) -> Result<FrmModuleExtractionReport, String> {
    let in_file = resolve_read_path(frm_path.trim());
    let dest = resolve_dir_target(&out_dir)?;
    extract_modules_from_frm(&in_file, &dest, unpack_payloads.unwrap_or(true))
        .map_err(|e| format!("Falha ao extrair módulos do .FRM: {e}"))
}

/// Inspects or patches `PROG061` BGO (`U99`) sites and optionally wraps as 128 KiB `DIGADIAG.FRM`.
#[tauri::command]
pub fn patch_bgo_unbrick_command(
    input_path: String,
    out_path: Option<String>,
    mode: String,
    wrap_as_digadiag: Option<bool>,
    allow_unsafe_bgo_format: Option<bool>,
) -> Result<Value, String> {
    let in_file = resolve_read_path(input_path.trim());
    let parsed_mode: Prog061BgoPatchMode = mode
        .parse()
        .map_err(|e: anyhow::Error| format!("Modo BGO inválido: {e}"))?;

    if let Some(out_raw) = opt_trimmed(&out_path) {
        let out_file = resolve_write_path(out_raw)?;
        let report = patch_prog061_or_build_digadiag_with_policy(
            &in_file,
            &out_file,
            parsed_mode,
            wrap_as_digadiag.unwrap_or(true),
            allow_unsafe_bgo_format.unwrap_or(false),
        )
        .map_err(|e| format!("Falha ao aplicar patch BGO PROG061: {e}"))?;
        serialize_value(report)
    } else {
        let raw = std::fs::read(&in_file)
            .map_err(|e| format!("Falha lendo '{}': {e}", in_file.display()))?;
        let report = inspect_prog061_container_bytes(&raw).map_err(|e| e.to_string())?;
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
) -> Result<Value, String> {
    if action.trim().eq_ignore_ascii_case("rsp_plan") {
        let cb = opt_trimmed(&callback_target_va)
            .map(parse_u32_radix)
            .transpose()
            .map_err(|e| e.to_string())?;
        return serialize_value(build_passthrough_rsp_plan(cb));
    }

    let in_raw = opt_trimmed(&input_path)
        .ok_or_else(|| "Caminho do binário MAIN (3410PQ) é obrigatório.".to_string())?;
    let in_file = resolve_read_path(in_raw);

    if action.trim().eq_ignore_ascii_case("patch") {
        let out_raw = opt_trimmed(&out_path)
            .ok_or_else(|| "Caminho de saída do MAIN patched é obrigatório.".to_string())?;
        let parsed_mode: MainPassthroughMode = mode
            .as_deref()
            .unwrap_or("av_passthrough")
            .parse()
            .map_err(|e: anyhow::Error| e.to_string())?;
        let report =
            patch_main_passthrough_file(&in_file, &resolve_write_path(out_raw)?, parsed_mode)
                .map_err(|e| format!("Falha ao patchear MAIN passthrough: {e}"))?;
        return serialize_value(report);
    }

    let buf = std::fs::read(&in_file).map_err(|e| format!("Falha lendo MAIN: {e}"))?;
    let report = inspect_main_passthrough_sites(&buf).map_err(|e| e.to_string())?;
    serialize_value(report)
}

/// Builds and post-verifies a full SD Recovery Kit (`DIGADIAG.FRM`, `PANA__SD.FRM`, `SHA256SUMS.txt`).
#[tauri::command]
pub fn prepare_sd_unbrick_kit_command(
    request: SdUnbrickKitRequestDto,
) -> Result<SdUnbrickKitReport, String> {
    let core_req = to_sd_unbrick_kit_request(&request)?;
    prepare_sd_unbrick_kit(&core_req).map_err(|e| format!("Falha ao gerar Kit Unbrick SD: {e}"))
}

/// Validates an SD card root directory (`DIGADIAG.FRM`, `PANA__SD.FRM`, & -91006 hardware warnings).
#[tauri::command]
pub fn verify_sd_unbrick_media_command(
    sd_root_dir: String,
) -> Result<SdMediaVerificationReport, String> {
    Ok(verify_sd_unbrick_media(&resolve_dir_target(&sd_root_dir)?))
}

/// Assembles a read-only MN103S UART NOR flash dumper stub (optionally wrapped in 128 KiB `DIGADIAG.FRM`).
#[tauri::command]
pub fn build_nor_dumper_stub_command(
    start_addr: Option<String>,
    total_len: Option<String>,
    block_size: Option<u32>,
    load_addr: Option<String>,
    out_path: Option<String>,
    wrap_as_digadiag: Option<bool>,
) -> Result<Value, String> {
    let params = parse_nor_dumper_params(
        start_addr.as_deref(),
        total_len.as_deref(),
        block_size,
        load_addr.as_deref(),
        wrap_as_digadiag,
    )?;

    let (stub_len, payload) = build_nor_dumper_artifact(
        params.start_u32,
        params.len_u32,
        params.block_size,
        params.load_u32,
        params.wrapped,
    )
    .map_err(|e| format!("Parâmetros/empacotamento NOR inválidos: {e}"))?;

    let written_path = opt_trimmed(&out_path)
        .map(|out_raw| {
            let dest = resolve_write_path(out_raw)?;
            std::fs::write(&dest, &payload)
                .map_err(|e| format!("Falha gravando stub '{}': {e}", dest.display()))?;
            Ok::<_, String>(dest.to_string_lossy().to_string())
        })
        .transpose()?;

    Ok(format_nor_dumper_response(
        &params,
        stub_len,
        &payload,
        written_path,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use panadiga_core::constants::PANA_DIGADIAG_FRM_SIZE;

    #[test]
    fn rsp_plan_and_nor_stub_ipc_commands_work() {
        let plan = patch_main_passthrough_command(
            "rsp_plan".to_string(),
            None,
            None,
            None,
            Some("0x50401F3C".to_string()),
        )
        .unwrap();
        assert_eq!(plan["handshake_hex"], "0e0402");
        assert_eq!(plan["callback_write_frame"], "$M515B2E40,4:3c1f4050#95");

        let stub = build_nor_dumper_stub_command(
            Some("0x84000000".to_string()),
            Some("0x10000".to_string()),
            Some(4096),
            Some("0x51800000".to_string()),
            None,
            Some(true),
        )
        .unwrap();
        assert_eq!(stub["output_bytes"], PANA_DIGADIAG_FRM_SIZE);
        assert_eq!(stub["wrapped_digadiag"], true);
    }
}
