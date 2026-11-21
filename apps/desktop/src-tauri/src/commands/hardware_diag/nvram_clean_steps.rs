#![deny(unsafe_code)]

//! DTOs and validated hardware sector steps for `nvram_clean`.

use std::fs;
use std::path::Path;
use std::time::Duration;

use panadiga_core::constants::{
    AM34_NVRAM_SIZE, AM34_NVRAM_UNCACHED_BASE, SPANSION_MICROSECTOR_COUNT,
    SPANSION_MICROSECTOR_SIZE,
};
use panadiga_core::nvram::calculate_hashes;
use panadiga_core::pico::panamon1::{
    PANAMON1_FULL_PROMPT, PANAMON1_RETRY_PACING_US, Panamon1Client, Panamon1Transport,
};
use panadiga_core::pico::rsp_codec::policy::{NorWritePolicy, is_nor_write_blocked_with_policy};
use panadiga_core::re::prewrite_gate::PreWriteGateReport;
use serde::{Deserialize, Serialize};

use crate::commands::error::{IpcResult, PanadigaIpcError};

pub(super) const POLL_CHECK_BYTES: usize = 32;
pub(super) const POLL_MAX_ATTEMPTS: usize = 32;
pub(super) const POLL_INTERVAL_MS: u64 = 250;
pub(super) const ERASE_TIMEOUT_MS: u64 = 10_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicoPanamon1CleanNvramRequestDto {
    pub port_name: String,
    pub backup_dir: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NvramCleanProgressEventDto {
    pub port: String,
    pub stage: String,
    pub current: usize,
    pub total: usize,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PicoPanamon1CleanNvramResponseDto {
    pub backup_path: String,
    pub sha256_before: String,
    pub sha256_after: String,
    pub erased_sectors: usize,
    pub verified_all_ff: bool,
    pub bytes_erased: usize,
    pub emu_gate: PreWriteGateReport,
    pub next_step: String,
}

pub(super) fn validate_nvram_sector_addresses() -> IpcResult<Vec<u32>> {
    let mut sector_vas = Vec::with_capacity(SPANSION_MICROSECTOR_COUNT);
    for idx in 0..SPANSION_MICROSECTOR_COUNT {
        let sector_va =
            AM34_NVRAM_UNCACHED_BASE.wrapping_add((idx * SPANSION_MICROSECTOR_SIZE) as u32);
        if is_nor_write_blocked_with_policy(
            sector_va,
            SPANSION_MICROSECTOR_SIZE,
            NorWritePolicy::NvramOnly,
        ) {
            return Err(PanadigaIpcError::validation(format!(
                "Hardware Safety Lockout (E05): setor 0x{sector_va:08X} fora da janela NVRAM SA134..SA141."
            )));
        }
        sector_vas.push(sector_va);
    }
    Ok(sector_vas)
}

pub(super) fn write_verified_nvram_backup(
    backup_dir: &Path,
    before_bytes: &[u8],
    sha256_before: &str,
) -> IpcResult<String> {
    fs::create_dir_all(backup_dir).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha criando diretório de backup NVRAM '{}': {e}",
            backup_dir.display()
        ))
    })?;
    let prefix = &sha256_before[..sha256_before.len().min(12)];
    let bin_path = backup_dir.join(format!("nvram_pre_clean_{prefix}.bin"));
    let sha_path = backup_dir.join(format!("nvram_pre_clean_{prefix}.sha256"));
    fs::write(&bin_path, before_bytes).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha gravando backup '{}': {e}",
            bin_path.display()
        ))
    })?;
    fs::write(&sha_path, format!("{sha256_before}\n")).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha gravando SHA-256 '{}': {e}",
            sha_path.display()
        ))
    })?;
    let read_back = fs::read(&bin_path).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha verificando leitura do backup '{}': {e}",
            bin_path.display()
        ))
    })?;
    let (_, actual_sha) = calculate_hashes(&read_back);
    if actual_sha != sha256_before || read_back.len() != AM34_NVRAM_SIZE as usize {
        return Err(PanadigaIpcError::classify(
            "Abortado antes de apagar: verificação SHA-256/tamanho do backup NVRAM 64 KiB falhou.",
        ));
    }
    Ok(bin_path.to_string_lossy().into_owned())
}

pub(super) fn erase_nvram_microsector<T: Panamon1Transport>(
    client: &mut Panamon1Client<T>,
    sector_va: u32,
) -> IpcResult<()> {
    let cmd = format!("E {sector_va:08X}");
    let retry_pace = Duration::from_micros(PANAMON1_RETRY_PACING_US);
    let mut erased_reply_ok = false;

    for attempt in 0..2 {
        let pace = if attempt == 0 {
            Duration::ZERO
        } else {
            client.sleep(retry_pace);
            retry_pace
        };
        let raw = client
            .transport_mut()
            .target_exchange_bytes(
                &cmd,
                pace,
                &[PANAMON1_FULL_PROMPT.as_bytes()],
                Duration::from_millis(ERASE_TIMEOUT_MS),
            )
            .map_err(|e| PanadigaIpcError::serial(format!("Erro UART em '{cmd}': {e}")))?;
        let text = String::from_utf8_lossy(&raw);
        if text.contains("ERR:E05") {
            return Err(PanadigaIpcError::validation(format!(
                "Bloqueio físico E05 reportado pelo target em '{cmd}'."
            )));
        }
        if text.contains("ERR:ERASE") {
            return Err(PanadigaIpcError::serial(format!(
                "Target reportou ERR:ERASE ao apagar 0x{sector_va:08X}: {text:?}"
            )));
        }
        let has_syntax_err =
            text.contains("\r\n?\r\n") || text.lines().any(|line| line.trim() == "?");
        if !has_syntax_err && text.contains(PANAMON1_FULL_PROMPT) && text.contains("OK:ERASE") {
            erased_reply_ok = true;
            break;
        }
    }
    if !erased_reply_ok {
        return Err(PanadigaIpcError::serial(format!(
            "Falha confirmando OK:ERASE para setor NVRAM 0x{sector_va:08X}."
        )));
    }

    for _ in 0..POLL_MAX_ATTEMPTS {
        let head = client
            .mem_read_bytes(sector_va, POLL_CHECK_BYTES)
            .map_err(|e| {
                PanadigaIpcError::serial(format!("Falha poll NVRAM 0x{sector_va:08X}: {e}"))
            })?;
        if head.iter().all(|&b| b == 0xFF) {
            return Ok(());
        }
        client.sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }

    Err(PanadigaIpcError::serial(format!(
        "Timeout aguardando setor NVRAM 0x{sector_va:08X} retornar 0xFF após erase."
    )))
}
