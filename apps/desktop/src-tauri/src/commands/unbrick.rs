//! Thin Tauri IPC shell for the Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Center.

use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::matshita_emu::{
    BootSequenceReport, FrmUnlockVerificationSummary, MatshitaBootScenario, MatshitaOddEmulator,
    SerialCliResponse, build_sample_unlock_frm, parse_hex_payload, parse_unlock_container_mode,
    verify_unlock_frm_container,
};
use panadiga_core::re::{
    FrmModuleExtractionReport, MainPassthroughMode, Prog061BgoPatchMode, SdMediaVerificationReport,
    SdUnbrickKitReport, UnbrickWorkspaceReport, build_nor_dumper_artifact,
    build_passthrough_rsp_plan, extract_modules_from_frm, inspect_main_passthrough_sites,
    inspect_prog061_container_bytes, inspect_unbrick_workspace, patch_main_passthrough_file,
    patch_prog061_or_build_digadiag_with_policy, prepare_sd_unbrick_kit, verify_sd_unbrick_media,
};
use serde_json::Value;
use std::path::PathBuf;

use super::constants::{
    DEFAULT_MAIN_PASSTHROUGH_MODE, DEFAULT_MATSHITA_INJECT_MODE, MATSHITA_DRV_PASS_UNLOCK_CMD,
    SAMPLE_UNLOCK_PAYLOAD_BYTE, SAMPLE_UNLOCK_PAYLOAD_LEN,
};
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
            .unwrap_or(DEFAULT_MAIN_PASSTHROUGH_MODE)
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

/// Runs a synthetic Matshita R6-MV4 optical drive boot sequence scenario (`healthy_with_disc`, `unlock_disc_frm`, etc.).
#[tauri::command]
pub fn run_matshita_boot_emu(scenario: String) -> Result<BootSequenceReport, String> {
    let parsed: MatshitaBootScenario = scenario
        .trim()
        .parse()
        .map_err(|e: anyhow::Error| format!("Cenário de boot Matshita inválido: {e}"))?;
    let mut emu = MatshitaOddEmulator::new(parsed);
    emu.run_boot_sequence()
        .map_err(|e| format!("Falha ao emular sequência de boot: {e}"))
}

/// Executes an emulated Matshita ATAPI write-injection (`0x3B WRITE BUFFER` or vendor `0xED`) with IPL `E05` guard.
#[tauri::command]
pub fn inject_matshita_atapi_write(
    target_va: Option<String>,
    offset: Option<usize>,
    mode: Option<u8>,
    use_vendor_ed: Option<bool>,
    payload_hex: String,
) -> Result<Value, String> {
    let payload = parse_hex_payload(payload_hex.trim())
        .map_err(|e| format!("Payload hexadecimal inválido: {e}"))?;
    let va_opt = opt_trimmed(&target_va)
        .map(parse_u32_radix)
        .transpose()
        .map_err(|e| format!("Endereço VA inválido: {e}"))?;
    let mut emu = MatshitaOddEmulator::default();
    let (record, snapshot) = emu
        .run_inject_write(
            va_opt,
            offset.unwrap_or(0),
            mode.unwrap_or(DEFAULT_MATSHITA_INJECT_MODE),
            use_vendor_ed.unwrap_or(false),
            &payload,
        )
        .map_err(|e| format!("Falha na injeção ATAPI Matshita: {e}"))?;
    Ok(serde_json::json!({
        "record": record,
        "snapshot": snapshot,
    }))
}

/// Sends a command to the emulated Matshita `DRV` serial CLI (optionally unlocking via `PASS` first).
#[tauri::command]
pub fn run_matshita_serial_cmd(
    command: String,
    unlock_pass: Option<bool>,
) -> Result<SerialCliResponse, String> {
    let cmd = command.trim();
    if cmd.is_empty() {
        return Err("Comando serial DRV não pode ser vazio.".to_string());
    }
    let mut emu = MatshitaOddEmulator::default();
    if unlock_pass.unwrap_or(true) {
        let _ = emu
            .execute_drv_serial_command(MATSHITA_DRV_PASS_UNLOCK_CMD)
            .map_err(|e| e.to_string())?;
    }
    emu.execute_drv_serial_command(cmd)
        .map_err(|e| format!("Falha no CLI serial DRV: {e}"))
}

/// Verifies an SD (`EXTRHEAD` / `PANA__SD`) or CD (`PANA_DVD.FRM` / `cd`) unlock container against EH55 bootloader rules.
#[tauri::command]
pub fn verify_frm_unlock_command(
    container_kind: String,
    frm_path: String,
) -> Result<FrmUnlockVerificationSummary, String> {
    let path = resolve_read_path(frm_path.trim());
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("Falha lendo container '{}': {e}", path.display()))?;
    verify_unlock_frm_container(container_kind.trim(), &bytes)
        .map_err(|e| format!("Falha ao verificar container .FRM: {e}"))
}

/// Synthesizes and verifies a sample SD (`sd`/`sd48`) or CD (`cd`/`dvd`) unlock `.FRM` container (optionally saved to disk).
#[tauri::command]
pub fn pack_unlock_frm_command(
    container_kind: String,
    out_path: Option<String>,
    payload_hex: Option<String>,
) -> Result<Value, String> {
    let mode = parse_unlock_container_mode(container_kind.trim())
        .map_err(|e| format!("Modo de container unlock inválido: {e}"))?;
    let payload = match opt_trimmed(&payload_hex) {
        Some(h) => parse_hex_payload(h).map_err(|e| format!("Payload hex inválido: {e}"))?,
        None => vec![SAMPLE_UNLOCK_PAYLOAD_BYTE; SAMPLE_UNLOCK_PAYLOAD_LEN],
    };
    let (bytes, verification) = build_sample_unlock_frm(mode, &payload)
        .map_err(|e| format!("Falha ao empacotar container unlock .FRM: {e}"))?;
    let written_path = opt_trimmed(&out_path)
        .map(|raw| {
            let dest = resolve_write_path(raw)?;
            std::fs::write(&dest, &bytes)
                .map_err(|e| format!("Falha gravando '{}': {e}", dest.display()))?;
            Ok::<_, String>(dest.to_string_lossy().into_owned())
        })
        .transpose()?;
    Ok(serde_json::json!({
        "verification": verification,
        "output_path": written_path,
        "container_bytes": bytes.len(),
    }))
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
            None,
            None,
            Some(true),
        )
        .unwrap();
        assert_eq!(stub["output_bytes"], PANA_DIGADIAG_FRM_SIZE);
        assert_eq!(stub["wrapped_digadiag"], true);
        assert_eq!(stub["load_addr"], "0x50400000");

        let boot = run_matshita_boot_emu("unlock_disc_frm".to_string()).unwrap();
        assert!(boot.startup_succeeded);
        assert!(boot.unlock_media_verified);
        assert_eq!(boot.jump_target_va_hex.as_deref(), Some("0x50400000"));

        let inj = inject_matshita_atapi_write(
            Some("0x50400000".to_string()),
            Some(0),
            Some(2),
            Some(true),
            "deadbeef".to_string(),
        )
        .unwrap();
        assert_eq!(inj["record"]["len_bytes"], 4);

        let ipl_err = run_matshita_serial_cmd("WW 0x84000000 0x11223344".to_string(), Some(true));
        assert!(ipl_err.unwrap_err().contains("E05"));
    }
}
