//! Tauri IPC Commands for Interactive Bidirectional SC0 Bridge (`BRIDGE`).

use crate::pico::{
    Sc0BridgeStatusDto, get_sc0_bridge_status_dto, send_sc0_bridge_input, start_sc0_bridge_session,
    stop_sc0_bridge_session,
};
use crate::state::PanadigaBackendState;

use super::error::IpcResult;
use super::hardware::run_blocking;

#[tauri::command]
pub async fn pico_start_sc0_bridge(
    app: tauri::AppHandle,
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    baud_rate: Option<u32>,
    boot_mode: Option<String>,
) -> IpcResult<Sc0BridgeStatusDto> {
    let backend = state.inner().clone();
    run_blocking(move || start_sc0_bridge_session(&backend, app, port_name, baud_rate, boot_mode))
        .await
}

#[tauri::command]
pub async fn pico_send_bridge_input(
    state: tauri::State<'_, PanadigaBackendState>,
    input_mode: String,
    payload: String,
) -> IpcResult<usize> {
    let backend = state.inner().clone();
    run_blocking(move || send_sc0_bridge_input(&backend, input_mode, payload)).await
}

#[tauri::command]
pub async fn pico_stop_sc0_bridge(
    state: tauri::State<'_, PanadigaBackendState>,
) -> IpcResult<Sc0BridgeStatusDto> {
    let backend = state.inner().clone();
    run_blocking(move || stop_sc0_bridge_session(&backend)).await
}

#[tauri::command]
pub async fn pico_get_bridge_status(
    state: tauri::State<'_, PanadigaBackendState>,
) -> IpcResult<Sc0BridgeStatusDto> {
    Ok(get_sc0_bridge_status_dto(state.inner()))
}
