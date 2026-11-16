//! Tauri IPC commands for SD Recovery Kit creation, NOR stub assembly, and Matshita ODD emulation.

use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::matshita_emu::{
    BootSequenceReport, MatshitaBootScenario, MatshitaOddEmulator, SerialCliResponse,
    parse_hex_payload,
};
use panadiga_core::re::{
    SdMediaVerificationReport, SdUnbrickKitReport, build_nor_dumper_artifact,
    prepare_sd_unbrick_kit, verify_sd_unbrick_media,
};
use serde_json::Value;

use crate::commands::constants::{DEFAULT_MATSHITA_INJECT_MODE, MATSHITA_DRV_PASS_UNLOCK_CMD};
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::{resolve_read_path, resolve_write_path};
use crate::commands::unbrick_dto::{
    SdUnbrickKitRequestDto, format_nor_dumper_response, opt_trimmed, parse_nor_dumper_params,
    resolve_dir_target, to_sd_unbrick_kit_request,
};

/// Builds and post-verifies a full SD Recovery Kit (`DIGADIAG.FRM`, `PANA__SD.FRM`, `SHA256SUMS.txt`).
#[tauri::command]
pub fn prepare_sd_unbrick_kit_command(
    request: SdUnbrickKitRequestDto,
) -> IpcResult<SdUnbrickKitReport> {
    let core_req = to_sd_unbrick_kit_request(&request)?;
    prepare_sd_unbrick_kit(&core_req)
        .map_err(|e| PanadigaIpcError::classify(format!("Falha ao gerar Kit Unbrick SD: {e}")))
}

/// Validates an SD card root directory (`DIGADIAG.FRM`, `PANA__SD.FRM`, & -91006 hardware warnings).
#[tauri::command]
pub fn verify_sd_unbrick_media_command(
    sd_root_dir: String,
) -> IpcResult<SdMediaVerificationReport> {
    Ok(verify_sd_unbrick_media(&resolve_dir_target(&sd_root_dir)?))
}

/// Audits raw SD card MBR/GPT/Superfloppy, FAT16 BPB, 8.3 root entries, and cluster chain (`inspect-sd`).
#[tauri::command]
pub fn inspect_sd_disk_command(
    target: String,
) -> IpcResult<panadiga_core::re::SdDiskInspectionReport> {
    let clean = target.trim();
    if clean.is_empty() {
        return Err(PanadigaIpcError::classify(
            "Alvo de disco/volume SD é obrigatório (ex.: E: ou sd.img).",
        ));
    }
    let resolved_target = if clean.len() == 2 && clean.ends_with(':')
        || clean.starts_with(r"\\.\")
        || clean.starts_with('/')
    {
        clean.to_string()
    } else {
        resolve_read_path(clean).to_string_lossy().into_owned()
    };
    panadiga_core::re::inspect_sd_target(&resolved_target)
        .map_err(|e| PanadigaIpcError::classify(format!("Falha ao auditar SD '{clean}': {e}")))
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
) -> IpcResult<Value> {
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
pub fn run_matshita_boot_emu(scenario: String) -> IpcResult<BootSequenceReport> {
    let parsed: MatshitaBootScenario = scenario
        .trim()
        .parse()
        .map_err(|e: anyhow::Error| format!("Cenário de boot Matshita inválido: {e}"))?;
    let mut emu = MatshitaOddEmulator::new(parsed);
    emu.run_boot_sequence()
        .map_err(|e| PanadigaIpcError::classify(format!("Falha ao emular sequência de boot: {e}")))
}

/// Executes an emulated Matshita ATAPI write-injection (`0x3B WRITE BUFFER` or vendor `0xED`) with IPL `E05` guard.
#[tauri::command]
pub fn inject_matshita_atapi_write(
    target_va: Option<String>,
    offset: Option<usize>,
    mode: Option<u8>,
    use_vendor_ed: Option<bool>,
    payload_hex: String,
) -> IpcResult<Value> {
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
) -> IpcResult<SerialCliResponse> {
    let cmd = command.trim();
    if cmd.is_empty() {
        return Err(PanadigaIpcError::classify(
            "Comando serial DRV não pode ser vazio.",
        ));
    }
    let mut emu = MatshitaOddEmulator::default();
    if unlock_pass.unwrap_or(true) {
        let _ = emu.execute_drv_serial_command(MATSHITA_DRV_PASS_UNLOCK_CMD)?;
    }
    emu.execute_drv_serial_command(cmd)
        .map_err(|e| PanadigaIpcError::classify(format!("Falha no CLI serial DRV: {e}")))
}
