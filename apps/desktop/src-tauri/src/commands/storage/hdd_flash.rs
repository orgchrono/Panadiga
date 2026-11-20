#![deny(unsafe_code)]

//! Tauri IPC commands for safe EH55 HDD discovery & verified raw disk flashing (`hdd://flash-progress`).

use std::path::Path;

use panadiga_core::constants::DEFAULT_HDD_IMAGE_PATH;
use panadiga_core::hdd::capacity_patch::specs::blocks_2kb_for_gb_bounded;
use panadiga_core::hdd::writer::{
    BlockFlashDevice, Eh55FlashStage, Eh55HddFlashReport, Eh55HddFlashRequest, SafeTargetDiskInfo,
    WindowsPhysicalFlashDevice, execute_eh55_hdd_flash, inspect_safe_target_disks,
};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::commands::constants::EVENT_HDD_FLASH_PROGRESS;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::resolve_read_path;
use crate::commands::unbrick_dto::{opt_trimmed, resolve_dir_target};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashEh55HddSafeRequestDto {
    pub device_path: String,
    pub confirm_disk_model_token: String,
    pub image_path: Option<String>,
    pub mirror_p1_path: Option<String>,
    pub backup_dir: String,
    pub expected_min_gb: Option<u32>,
    pub expected_max_gb: Option<u32>,
    pub allow_extended_4tb: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HddFlashProgressEventDto {
    pub device_path: String,
    pub stage: Eh55FlashStage,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub detail: String,
}

pub fn normalize_disk_confirm_token(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        String::new()
    } else if trimmed.starts_with("CONFIRM-") {
        trimmed.to_string()
    } else {
        format!("CONFIRM-{trimmed}")
    }
}

pub fn build_eh55_flash_request_from_dto(
    dto: &FlashEh55HddSafeRequestDto,
) -> IpcResult<Eh55HddFlashRequest> {
    let dev = dto.device_path.trim();
    if dev.is_empty() {
        return Err(PanadigaIpcError::validation(
            "devicePath não pode ser vazio.",
        ));
    }
    if dev.to_ascii_lowercase().contains("physicaldrive0") {
        return Err(PanadigaIpcError::validation(
            "LOCKOUT FÍSICO FATAL: PhysicalDrive0 (disco do SO) é permanentemente bloqueado.",
        ));
    }
    let token = normalize_disk_confirm_token(&dto.confirm_disk_model_token);
    if token.is_empty() {
        return Err(PanadigaIpcError::validation(
            "confirmDiskModelToken obrigatório para autorizar escrita em disco.",
        ));
    }
    let allow_4tb = dto.allow_extended_4tb.unwrap_or(false);
    let expected_gb_range = match (dto.expected_min_gb, dto.expected_max_gb) {
        (None, None) => None,
        (min_opt, max_opt) => {
            let min_gb = min_opt.unwrap_or(5);
            let max_gb = max_opt.unwrap_or(260);
            blocks_2kb_for_gb_bounded(max_gb, allow_4tb).map_err(|e| {
                PanadigaIpcError::validation(format!("Limite de capacidade inválido: {e}"))
            })?;
            Some((min_gb, max_gb))
        }
    };

    let image_raw = opt_trimmed(&dto.image_path).unwrap_or(DEFAULT_HDD_IMAGE_PATH);
    let image_path = resolve_read_path(image_raw);
    let mirror_p1_path = opt_trimmed(&dto.mirror_p1_path).map(resolve_read_path);
    let backup_dir = resolve_dir_target(dto.backup_dir.trim())?;

    Ok(Eh55HddFlashRequest {
        device_path: dev.to_string(),
        confirmation_token: token,
        image_path,
        mirror_p1_path,
        extract_p1_mirror_from_image: opt_trimmed(&dto.mirror_p1_path).is_none(),
        backup_dir,
        expected_gb_range,
        detected_os_disks: None,
    })
}

pub fn run_flash_eh55_with_device<D, F>(
    device: &mut D,
    dto: &FlashEh55HddSafeRequestDto,
    mut on_progress: F,
) -> IpcResult<Eh55HddFlashReport>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(HddFlashProgressEventDto),
{
    let req = build_eh55_flash_request_from_dto(dto)?;
    let dev_label = req.device_path.clone();
    execute_eh55_hdd_flash(device, &req, |p| {
        on_progress(HddFlashProgressEventDto {
            device_path: dev_label.clone(),
            stage: p.stage,
            bytes_done: p.bytes_done,
            bytes_total: p.bytes_total,
            detail: p.detail,
        });
    })
    .map_err(|e| PanadigaIpcError::classify(format!("Falha no flash seguro EH55 HDD: {e}")))
}

/// Lists target block devices enriched with OS-disk / `BusTypeNvme` safety lockouts and `mirror_main_lba`.
#[tauri::command]
pub fn inspect_safe_target_disks_command() -> IpcResult<Vec<SafeTargetDiskInfo>> {
    Ok(inspect_safe_target_disks())
}

/// Safely flashes a verified EH55 disk image + tail `MAIN` mirror, emitting `"hdd://flash-progress"`.
#[tauri::command]
pub async fn flash_eh55_hdd_safe_command(
    app: tauri::AppHandle,
    request: FlashEh55HddSafeRequestDto,
) -> IpcResult<Eh55HddFlashReport> {
    let req = build_eh55_flash_request_from_dto(&request)?;
    if cfg!(target_os = "android") {
        return Err(PanadigaIpcError::validation(
            "Gravação direta de bloco físico ATA/SATA/USB não é suportada no Android APK; utilize o workbench Desktop.",
        ));
    }
    let dev_path = req.device_path.clone();
    let app_clone = app.clone();
    let req_clone = request.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut device = WindowsPhysicalFlashDevice::open(
            Path::new(&dev_path).to_str().unwrap_or(&dev_path),
            true,
        )
        .map_err(|e| {
            PanadigaIpcError::classify(format!("Falha abrindo disco '{dev_path}': {e}"))
        })?;
        run_flash_eh55_with_device(&mut device, &req_clone, |ev| {
            let _ = app_clone.emit(EVENT_HDD_FLASH_PROGRESS, &ev);
        })
    })
    .await
    .map_err(|e| {
        PanadigaIpcError::internal(format!("Falha de execução assíncrona HDD flash: {e}"))
    })?
}
