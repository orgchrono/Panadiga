#![deny(unsafe_code)]

//! Tauri IPC command for live U99 surgical NVRAM reset (`SA134..SA141`, `0x847F0000..+64 KiB`)
//! over Pico `P7501` `BRIDGE 115200 PASSIVE` (`PANAMON1>`), emitting `"nvram://clean-progress"`.

use panadiga_core::constants::{
    AM34_FLASH_BASE, AM34_FLASH_SIZE, AM34_NVRAM_BASE, AM34_NVRAM_SIZE,
    SPANSION_MICROSECTOR_BASE_INDEX, SPANSION_MICROSECTOR_SIZE, resolve_writable_data_root,
};
use panadiga_core::nvram::calculate_hashes;
use panadiga_core::pico::panamon1::{Panamon1Client, Panamon1Transport};
use panadiga_core::re::prewrite_gate::{PreWriteTarget, require_prewrite_gate};
use panadiga_core::vault::{StrictWorkflowStep, VaultBucket};
use tauri::Emitter;

use super::eh55_bgo_dto::{resolve_auto_backup_dir, run_exclusive_panamon1};
pub use super::nvram_clean_steps::*;
use crate::commands::constants::EVENT_NVRAM_CLEAN_PROGRESS;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::unbrick::record_vault_outcome_in;
use crate::commands::unbrick_dto::opt_trimmed;
use crate::state::PanadigaBackendState;

pub fn run_panamon1_clean_nvram_with_transport<T: Panamon1Transport, F>(
    transport: T,
    req: &PicoPanamon1CleanNvramRequestDto,
    mut on_progress: F,
) -> IpcResult<PicoPanamon1CleanNvramResponseDto>
where
    F: FnMut(NvramCleanProgressEventDto),
{
    let port_label = req.port_name.trim().to_string();
    if port_label.is_empty() {
        return Err(PanadigaIpcError::validation(
            "portName obrigatório para limpeza ao vivo da NVRAM via Pico.",
        ));
    }
    let sector_vas = validate_nvram_sector_addresses()?;
    let total_sectors = sector_vas.len();
    let backup_dir = resolve_auto_backup_dir(opt_trimmed(&req.backup_dir), "nvram_pre_clean")?;

    let emit_stage = |cb: &mut F, stage: &str, cur: usize, tot: usize, detail: String| {
        cb(NvramCleanProgressEventDto {
            port: port_label.clone(),
            stage: stage.to_string(),
            current: cur,
            total: tot,
            detail,
        });
    };

    emit_stage(
        &mut on_progress,
        "connect_bridge",
        0,
        total_sectors,
        "Sincronizando ponte P7501 115200 8N1 (PANAMON1>)...".to_string(),
    );
    let mut client = Panamon1Client::new(transport);
    let _ = client
        .ensure_panamon1_bridge(Some("JIG:2500"), 6_000)
        .map_err(|e| PanadigaIpcError::serial(format!("Falha entrando no PANAMON1>: {e}")))?;

    let mut before_bytes = Vec::with_capacity(AM34_NVRAM_SIZE as usize);
    for (idx, &va) in sector_vas.iter().enumerate() {
        let sa = SPANSION_MICROSECTOR_BASE_INDEX + idx;
        emit_stage(
            &mut on_progress,
            "backup_read",
            idx + 1,
            total_sectors,
            format!("Lendo backup SA{sa} @ 0x{va:08X} (8 KiB)..."),
        );
        let chunk = client
            .mem_read_bytes(va, SPANSION_MICROSECTOR_SIZE)
            .map_err(|e| {
                PanadigaIpcError::serial(format!("Falha lendo SA{sa} @ 0x{va:08X}: {e}"))
            })?;
        before_bytes.extend_from_slice(&chunk);
    }
    if before_bytes.len() != AM34_NVRAM_SIZE as usize {
        return Err(PanadigaIpcError::serial(format!(
            "Backup pré-clean NVRAM incompleto ({} bytes, esperados {} bytes). Erase abortado.",
            before_bytes.len(),
            AM34_NVRAM_SIZE
        )));
    }
    if before_bytes.iter().all(|&b| b == 0x00) || before_bytes.iter().all(|&b| b == 0xFF) {
        return Err(PanadigaIpcError::serial(
            "Backup pré-clean NVRAM uniforme (0x00 ou 0xFF — barramento morto/flutuante ou NVRAM já limpa). Erase abortado por segurança.",
        ));
    }
    for (idx, &sector_va) in sector_vas.iter().enumerate() {
        let sa = 134 + idx;
        let off = idx * SPANSION_MICROSECTOR_SIZE;
        let verify_head = client
            .mem_read_bytes(sector_va, POLL_CHECK_BYTES)
            .map_err(|e| PanadigaIpcError::serial(format!("Falha double-read SA{sa}: {e}")))?;
        if verify_head != before_bytes[off..off + POLL_CHECK_BYTES] {
            return Err(PanadigaIpcError::serial(format!(
                "Double-read do cabeçalho SA{sa} (0x{sector_va:08X}) divergiu do backup lido (instabilidade na UART P7501). Erase abortado."
            )));
        }
    }
    let (_, sha256_before) = calculate_hashes(&before_bytes);
    let backup_path = write_verified_nvram_backup(&backup_dir, &before_bytes, &sha256_before)?;

    let mut orig_sim = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    let nvram_off = (AM34_NVRAM_BASE - AM34_FLASH_BASE) as usize;
    let nvram_copy_len = before_bytes.len().min(AM34_NVRAM_SIZE as usize);
    orig_sim[nvram_off..nvram_off + nvram_copy_len]
        .copy_from_slice(&before_bytes[..nvram_copy_len]);
    let mut clean_sim = orig_sim.clone();
    clean_sim[nvram_off..nvram_off + nvram_copy_len].fill(0xFF);
    let emu_gate = require_prewrite_gate(PreWriteTarget::NorCowRepair {
        original_nor: &orig_sim,
        repaired_nor: &clean_sim,
        allow_upper_nor: false,
    })
    .map_err(|e| PanadigaIpcError::classify(format!("Mn103Vm Gate bloqueou erase NVRAM: {e}")))?;

    emit_stage(
        &mut on_progress,
        "backup_verify",
        total_sectors,
        total_sectors,
        format!("Backup 64 KiB + Gate Mn103Vm OK (SHA-256: {sha256_before})"),
    );

    let mut erased_sectors = 0usize;
    for (idx, &va) in sector_vas.iter().enumerate() {
        let sa = SPANSION_MICROSECTOR_BASE_INDEX + idx;
        emit_stage(
            &mut on_progress,
            "erase_sector",
            idx + 1,
            total_sectors,
            format!("Limpando SA{sa} @ 0x{va:08X} (IPL SA0..SA10 protegido)..."),
        );
        erase_nvram_microsector(&mut client, va)?;
        erased_sectors += 1;
    }

    let mut after_bytes = Vec::with_capacity(AM34_NVRAM_SIZE as usize);
    for (idx, &va) in sector_vas.iter().enumerate() {
        let sa = SPANSION_MICROSECTOR_BASE_INDEX + idx;
        emit_stage(
            &mut on_progress,
            "readback_verify",
            idx + 1,
            total_sectors,
            format!("Auditando readback 0xFF em SA{sa} @ 0x{va:08X}..."),
        );
        let chunk = client
            .mem_read_bytes(va, SPANSION_MICROSECTOR_SIZE)
            .map_err(|e| {
                PanadigaIpcError::serial(format!("Falha readback SA{sa} @ 0x{va:08X}: {e}"))
            })?;
        after_bytes.extend_from_slice(&chunk);
    }

    let exit_summary = client
        .exit_bridge_checked()
        .map_err(|e| PanadigaIpcError::serial(format!("Falha fechando BRIDGE P7501: {e}")))?;
    exit_summary
        .validate_healthy()
        .map_err(|e| PanadigaIpcError::serial(format!("Telemetria BRIDGE anômala: {e}")))?;

    let verified_all_ff =
        after_bytes.len() == AM34_NVRAM_SIZE as usize && after_bytes.iter().all(|&b| b == 0xFF);
    let (_, sha256_after) = calculate_hashes(&after_bytes);
    if !verified_all_ff {
        return Err(PanadigaIpcError::classify(
            "Verificação final 0xFF da NVRAM (0x847F0000..+64 KiB) falhou: bytes não zerados detectados.",
        ));
    }

    let prefix = &sha256_before[..sha256_before.len().min(12)];
    let raw_name = format!("nvram_pre_clean_{prefix}.bin");
    record_vault_outcome_in(
        &resolve_writable_data_root(),
        req.session_id.as_deref(),
        &emu_gate,
        Some((
            &before_bytes,
            &raw_name,
            VaultBucket::Raw,
            "pico_panamon1_pre_clean",
        )),
        &[
            StrictWorkflowStep::CaptureAndBackup,
            StrictWorkflowStep::EmulateAndFlash,
        ],
    )?;

    emit_stage(
        &mut on_progress,
        "complete",
        total_sectors,
        total_sectors,
        "NVRAM SA134..SA141 100% 0xFF verificada.".to_string(),
    );

    Ok(PicoPanamon1CleanNvramResponseDto {
        backup_path,
        sha256_before,
        sha256_after,
        erased_sectors,
        verified_all_ff,
        bytes_erased: after_bytes.len(),
        emu_gate,
        next_step: "Desconecte o cabo P7501, aguarde 10 segundos sem cabo AC na tomada e ligue o Panasonic DMR-EH55 para confirmar boot limpo sem erro U99.".to_string(),
    })
}

/// Backs up `0x847F0000..+64 KiB` (SHA-256 verified), erases `SA134..SA141` over `PANAMON1>`,
/// and verifies 100% `0xFF` readback while keeping IPL `SA0..SA10` (`E05`) untouched,
/// emitting `"nvram://clean-progress"`.
#[tauri::command]
pub async fn pico_panamon1_clean_nvram_command(
    app: tauri::AppHandle,
    state: tauri::State<'_, PanadigaBackendState>,
    request: PicoPanamon1CleanNvramRequestDto,
) -> IpcResult<PicoPanamon1CleanNvramResponseDto> {
    let clean_port = request.port_name.trim().to_string();
    if clean_port.is_empty() {
        return Err(PanadigaIpcError::validation(
            "portName obrigatório para limpeza ao vivo da NVRAM via Pico.",
        ));
    }
    super::bench_control::require_destructive_nonce(
        state.inner(),
        request.nonce_id.as_deref(),
        panadiga_core::bench::OpKind::NorWrite,
        "SA134..SA141",
        &clean_port,
    )?;
    let backend = state.inner().clone();
    let app_clone = app.clone();
    let req_clone = request.clone();
    run_exclusive_panamon1(backend, clean_port, "PANAMON1 NVRAM clean", move |t| {
        run_panamon1_clean_nvram_with_transport(t, &req_clone, |ev| {
            let _ = app_clone.emit(EVENT_NVRAM_CLEAN_PROGRESS, &ev);
        })
    })
    .await
}

#[cfg(test)]
#[path = "nvram_clean_tests.rs"]
mod tests;
