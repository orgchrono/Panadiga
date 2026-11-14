//! Tauri IPC Commands for the Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Center.

use panadiga_core::constants::{PANA_DIGADIAG_FRM_SIZE, resolve_workspace_root};
use panadiga_core::crypto::{
    ExtrheadCryptMode, build_digadiag_frm, parse_u32_radix, unpack_extrhead,
};
use panadiga_core::re::{
    FrmModuleExtractionReport, MainPassthroughMode, Prog061BgoPatchMode, SdMediaVerificationReport,
    SdUnbrickKitReport, SdUnbrickKitRequest, UnbrickWorkspaceReport, build_passthrough_rsp_plan,
    build_readonly_nor_dumper_stub, extract_modules_from_frm, inspect_main_passthrough_sites,
    inspect_prog061_bgo_sites, inspect_unbrick_workspace, patch_main_passthrough_file,
    patch_prog061_or_build_digadiag, prepare_sd_unbrick_kit, verify_sd_unbrick_media,
};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

use super::frm_pack::{resolve_read_path, resolve_write_path};

#[derive(Debug, Clone, Deserialize)]
pub struct SdUnbrickKitRequestDto {
    pub output_dir: String,
    pub pana_dvd_path: Option<String>,
    pub prog061_path: Option<String>,
    pub main_path: Option<String>,
    pub drv_path: Option<String>,
    pub bgo_mode: Option<String>,
    pub main_passthrough_mode: Option<String>,
    pub include_pana_sd: Option<bool>,
    pub unlock_hdd_dynamic: Option<bool>,
    pub hdd_target_gb: Option<u32>,
    pub write_extracted_sources: Option<bool>,
}

fn resolve_dir_target(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Caminho do diretório SD é obrigatório.".to_string());
    }
    let p = PathBuf::from(trimmed);
    Ok(if p.is_absolute() {
        p
    } else {
        resolve_workspace_root().join(p)
    })
}

/// Inspects workspace binaries, PROG061 BGO status, MAIN TBC passthrough, RSP plan, & hardware facts.
#[tauri::command]
pub fn inspect_unbrick_workbench(
    workspace_path: Option<String>,
) -> Result<UnbrickWorkspaceReport, String> {
    let override_path = workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
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
) -> Result<Value, String> {
    let in_file = resolve_read_path(input_path.trim());
    let parsed_mode: Prog061BgoPatchMode = mode
        .parse()
        .map_err(|e: anyhow::Error| format!("Modo BGO inválido: {e}"))?;

    if let Some(out_raw) = out_path.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let out_file = resolve_write_path(out_raw)?;
        let report = patch_prog061_or_build_digadiag(
            &in_file,
            &out_file,
            parsed_mode,
            wrap_as_digadiag.unwrap_or(true),
        )
        .map_err(|e| format!("Falha ao aplicar patch BGO PROG061: {e}"))?;
        serde_json::to_value(report).map_err(|e| e.to_string())
    } else {
        let raw = std::fs::read(&in_file)
            .map_err(|e| format!("Falha lendo '{}': {e}", in_file.display()))?;
        let payload = if raw.starts_with(b"EXTRHEAD") {
            unpack_extrhead(&raw, ExtrheadCryptMode::Plain)
                .map_err(|e| e.to_string())?
                .decompressed
        } else {
            raw
        };
        let report = inspect_prog061_bgo_sites(&payload).map_err(|e| e.to_string())?;
        serde_json::to_value(report).map_err(|e| e.to_string())
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
    match action.trim().to_ascii_lowercase().as_str() {
        "rsp_plan" => {
            let cb = match callback_target_va
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(raw) => Some(parse_u32_radix(raw).map_err(|e| e.to_string())?),
                None => None,
            };
            serde_json::to_value(build_passthrough_rsp_plan(cb)).map_err(|e| e.to_string())
        }
        "patch" => {
            let in_raw = input_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Caminho do binário MAIN (3410PQ) é obrigatório.".to_string())?;
            let out_raw = out_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Caminho de saída do MAIN patched é obrigatório.".to_string())?;
            let parsed_mode: MainPassthroughMode = mode
                .as_deref()
                .unwrap_or("av_passthrough")
                .parse()
                .map_err(|e: anyhow::Error| e.to_string())?;
            let report = patch_main_passthrough_file(
                &resolve_read_path(in_raw),
                &resolve_write_path(out_raw)?,
                parsed_mode,
            )
            .map_err(|e| format!("Falha ao patchear MAIN passthrough: {e}"))?;
            serde_json::to_value(report).map_err(|e| e.to_string())
        }
        _ => {
            let in_raw = input_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Caminho do binário MAIN (3410PQ) é obrigatório.".to_string())?;
            let buf = std::fs::read(resolve_read_path(in_raw))
                .map_err(|e| format!("Falha lendo MAIN: {e}"))?;
            let report = inspect_main_passthrough_sites(&buf).map_err(|e| e.to_string())?;
            serde_json::to_value(report).map_err(|e| e.to_string())
        }
    }
}

/// Builds and post-verifies a full SD Recovery Kit (`DIGADIAG.FRM`, `PANA__SD.FRM`, `SHA256SUMS.txt`).
#[tauri::command]
pub fn prepare_sd_unbrick_kit_command(
    request: SdUnbrickKitRequestDto,
) -> Result<SdUnbrickKitReport, String> {
    let bgo_mode: Prog061BgoPatchMode = request
        .bgo_mode
        .as_deref()
        .unwrap_or("auto_format")
        .parse()
        .map_err(|e: anyhow::Error| e.to_string())?;
    let main_passthrough_mode: MainPassthroughMode = request
        .main_passthrough_mode
        .as_deref()
        .unwrap_or("av_passthrough")
        .parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let opt_read = |v: &Option<String>| -> Option<PathBuf> {
        v.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(resolve_read_path)
    };

    prepare_sd_unbrick_kit(&SdUnbrickKitRequest {
        output_dir: resolve_dir_target(&request.output_dir)?,
        pana_dvd_path: opt_read(&request.pana_dvd_path),
        prog061_path: opt_read(&request.prog061_path),
        main_path: opt_read(&request.main_path),
        drv_path: opt_read(&request.drv_path),
        bgo_mode,
        main_passthrough_mode,
        include_pana_sd: request.include_pana_sd.unwrap_or(true),
        unlock_hdd_dynamic: request.unlock_hdd_dynamic.unwrap_or(true),
        hdd_target_gb: request.hdd_target_gb,
        write_extracted_sources: request.write_extracted_sources.unwrap_or(false),
    })
    .map_err(|e| format!("Falha ao gerar Kit Unbrick SD: {e}"))
}

/// Validates an SD card root directory (`DIGADIAG.FRM`, `PANA__SD.FRM`, & -91006 hardware warnings).
#[tauri::command]
pub fn verify_sd_unbrick_media_command(
    sd_root_dir: String,
) -> Result<SdMediaVerificationReport, String> {
    let dir = resolve_dir_target(&sd_root_dir)?;
    Ok(verify_sd_unbrick_media(&dir))
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
    let start_u32 = parse_u32_radix(start_addr.as_deref().unwrap_or("0x84000000"))
        .map_err(|e| format!("Endereço inicial NOR inválido: {e}"))?;
    let len_u32 = parse_u32_radix(total_len.as_deref().unwrap_or("0x800000"))
        .map_err(|e| format!("Tamanho total inválido: {e}"))?;
    let load_u32 = parse_u32_radix(load_addr.as_deref().unwrap_or("0x51800000"))
        .map_err(|e| format!("Load address inválido: {e}"))?;
    let blk = block_size.unwrap_or(4096);

    let stub = build_readonly_nor_dumper_stub(start_u32, len_u32, blk, load_u32)
        .map_err(|e| format!("Parâmetros NOR inválidos: {e}"))?;
    let stub_len = stub.len();
    let wrapped = wrap_as_digadiag.unwrap_or(false);
    let payload = if wrapped {
        let (frm, _) = build_digadiag_frm(
            &stub,
            load_u32,
            "FVU ",
            "NOR_DMP",
            4,
            PANA_DIGADIAG_FRM_SIZE,
        )
        .map_err(|e| format!("Falha ao empacotar DIGADIAG NOR dumper: {e}"))?;
        frm
    } else {
        stub
    };

    let written_path =
        if let Some(out_raw) = out_path.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let dest = resolve_write_path(out_raw)?;
            std::fs::write(&dest, &payload)
                .map_err(|e| format!("Falha gravando stub '{}': {e}", dest.display()))?;
            Some(dest.to_string_lossy().to_string())
        } else {
            None
        };

    Ok(serde_json::json!({
        "start_addr": format!("0x{start_u32:08X}"),
        "total_len_bytes": len_u32,
        "block_size": blk,
        "load_addr": format!("0x{load_u32:08X}"),
        "raw_stub_bytes": stub_len,
        "wrapped_digadiag": wrapped,
        "output_bytes": payload.len(),
        "written_path": written_path,
        "stub_hex": hex::encode(&payload[..payload.len().min(128)]),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(plan["handshake_hex"], "0e020204");
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
