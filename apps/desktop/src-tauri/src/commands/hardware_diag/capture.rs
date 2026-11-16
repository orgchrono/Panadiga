//! Live in-circuit NVRAM capture command (`pico_capture_nvram_live`) and event emission.

use tauri::Emitter;

use crate::commands::constants::{
    AM34_NVRAM_BASE, BYTES_PER_U32_WORD, DEFAULT_DUMP_DIAGNOSTIC_CMD, DEFAULT_NVRAM_CAPTURE_WORDS,
    EVENT_PICO_NVRAM_CAPTURE, MAX_NVRAM_CAPTURE_WORDS, MIN_NVRAM_CAPTURE_WORDS,
    NVRAM_DUMP_CHUNK_WORDS, SOURCE_PICO_LIVE_NVRAM,
};
use crate::commands::dto::LiveNvramCaptureEventDto;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::pico::diagnostics::execute_diagnostic_on_port;
use crate::state::PanadigaBackendState;

pub(super) fn build_and_emit_live_nvram_capture(
    app: &tauri::AppHandle,
    source: &str,
    port_name: &str,
    base_bus_address: u32,
    bytes: &[u8],
) -> IpcResult<LiveNvramCaptureEventDto> {
    let label = format!("{} ({})", source, port_name.trim());
    let report =
        panadiga_core::nvram::inspect_partial_nvram_capture(bytes, base_bus_address, &label)
            .map_err(|e| PanadigaIpcError::validation(e.to_string()))?;
    let event = LiveNvramCaptureEventDto {
        source: source.to_string(),
        port: port_name.trim().to_string(),
        base_bus_address: format!("0x{:08X}", base_bus_address),
        captured_bytes: bytes.len(),
        report,
    };
    let _ = app.emit(EVENT_PICO_NVRAM_CAPTURE, &event);
    Ok(event)
}

/// Captures real live NVRAM bytes in-circuit over Pico P7501 `DUMP` / GDB RSP in 8 KB microsector chunks,
/// audits sector activity (`SA134..SA141`) via `panadiga-core`, and emits `"pico://nvram-live-capture"`.
#[tauri::command]
pub async fn pico_capture_nvram_live(
    app: tauri::AppHandle,
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    base_addr: Option<u32>,
    word_count: Option<u32>,
) -> IpcResult<LiveNvramCaptureEventDto> {
    let backend = state.inner().clone();
    let start_addr = base_addr.unwrap_or(AM34_NVRAM_BASE);
    let total_words = word_count
        .unwrap_or(DEFAULT_NVRAM_CAPTURE_WORDS)
        .clamp(MIN_NVRAM_CAPTURE_WORDS, MAX_NVRAM_CAPTURE_WORDS);
    let chunk_words_limit: u32 = NVRAM_DUMP_CHUNK_WORDS;
    let mut accumulated: Vec<u8> = Vec::with_capacity((total_words * BYTES_PER_U32_WORD) as usize);
    let mut words_read: u32 = 0;

    while words_read < total_words {
        let cur_addr = start_addr.wrapping_add(words_read * BYTES_PER_U32_WORD);
        let chunk_words = (total_words - words_read).min(chunk_words_limit);
        let arg = format!("0x{:08X} {}", cur_addr, chunk_words);
        let port_clone = port_name.clone();
        let backend_clone = backend.clone();
        let res = tauri::async_runtime::spawn_blocking(move || {
            execute_diagnostic_on_port(
                &backend_clone,
                &port_clone,
                DEFAULT_DUMP_DIAGNOSTIC_CMD,
                Some(&arg),
            )
        })
        .await
        .map_err(|e| {
            PanadigaIpcError::internal(format!("Falha interna de execução assíncrona: {}", e))
        })?
        .map_err(PanadigaIpcError::serial)?;

        if !res.success {
            return Err(PanadigaIpcError::serial(format!(
                "Falha ao capturar bloco NVRAM em 0x{:08X}: {}",
                cur_addr, res.summary
            )));
        }
        let mut chunk_bytes =
            panadiga_core::pico::cdc::parse_pico_dump_hex_to_bytes(&res.raw_output);
        if chunk_bytes.is_empty() {
            return Err(PanadigaIpcError::serial(format!(
                "DUMP retornou 0 bytes legíveis em 0x{:08X} ({}).",
                cur_addr, res.summary
            )));
        }
        accumulated.append(&mut chunk_bytes);
        words_read += chunk_words;
    }

    build_and_emit_live_nvram_capture(
        &app,
        SOURCE_PICO_LIVE_NVRAM,
        &port_name,
        start_addr,
        &accumulated,
    )
}
