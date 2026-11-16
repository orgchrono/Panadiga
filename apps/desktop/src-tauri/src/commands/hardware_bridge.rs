//! Tauri IPC Commands for Interactive Bidirectional SC0 Bridge (`BRIDGE`).

use crate::pico::{
    Sc0BridgeStatusDto, get_sc0_bridge_status_dto, send_sc0_bridge_input, start_sc0_bridge_session,
    stop_sc0_bridge_session,
};

use super::error::IpcResult;
use super::hardware::run_blocking;

#[tauri::command]
pub async fn pico_start_sc0_bridge(
    app: tauri::AppHandle,
    port_name: String,
    baud_rate: Option<u32>,
    boot_mode: Option<String>,
) -> IpcResult<Sc0BridgeStatusDto> {
    run_blocking(move || start_sc0_bridge_session(app, port_name, baud_rate, boot_mode)).await
}

#[tauri::command]
pub async fn pico_send_bridge_input(input_mode: String, payload: String) -> IpcResult<usize> {
    run_blocking(move || send_sc0_bridge_input(input_mode, payload)).await
}

#[tauri::command]
pub async fn pico_stop_sc0_bridge() -> IpcResult<Sc0BridgeStatusDto> {
    run_blocking(stop_sc0_bridge_session).await
}

#[tauri::command]
pub async fn pico_get_bridge_status() -> IpcResult<Sc0BridgeStatusDto> {
    Ok(get_sc0_bridge_status_dto())
}
