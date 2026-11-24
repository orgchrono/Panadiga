#![deny(unsafe_code)]

//! Tauri IPC commands for safe EH55 HDD discovery & verified raw disk flashing (`hdd://flash-progress`).

use std::path::Path;

use panadiga_core::constants::DEFAULT_HDD_IMAGE_PATH;
use panadiga_core::hdd::capacity_patch::specs::blocks_2kb_for_gb_bounded;
use panadiga_core::hdd::writer::{
    BlockFlashDevice, Eh55FlashProgress, Eh55FlashStage, Eh55HddFlashReport, Eh55HddFlashRequest,
    SafeTargetDiskInfo, WindowsPhysicalFlashDevice, execute_eh55_hdd_flash,
    inspect_safe_target_disks, is_unconditional_os_boot_disk,
};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::commands::constants::EVENT_HDD_FLASH_PROGRESS;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::resolve_safe_read_path;
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
    #[serde(default)]
    pub nonce_id: Option<String>,
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

impl HddFlashProgressEventDto {
    pub(super) fn from_core(device_path: &str, p: Eh55FlashProgress) -> Self {
        Self {
            device_path: device_path.to_string(),
            stage: p.stage,
            bytes_done: p.bytes_done,
            bytes_total: p.bytes_total,
            detail: p.detail,
        }
    }
}

pub(super) async fn run_physical_hdd_flash_task<T, F>(
    app: tauri::AppHandle,
    device_path: String,
    task_label: &'static str,
    operation: F,
) -> IpcResult<T>
where
    T: Send + 'static,
    F: FnOnce(&mut WindowsPhysicalFlashDevice, &dyn Fn(HddFlashProgressEventDto)) -> IpcResult<T>
        + Send
        + 'static,
{
    if cfg!(target_os = "android") {
        return Err(PanadigaIpcError::validation(
            "Gravação direta de bloco físico ATA/SATA/USB não é suportada no Android APK; utilize o workbench Desktop.",
        ));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut device = WindowsPhysicalFlashDevice::open(
            Path::new(&device_path).to_str().unwrap_or(&device_path),
            true,
        )
        .map_err(|e| {
            PanadigaIpcError::classify(format!("Falha abrindo disco '{device_path}': {e}"))
        })?;
        let emit = |ev: HddFlashProgressEventDto| {
            let _ = app.emit(EVENT_HDD_FLASH_PROGRESS, &ev);
        };
        operation(&mut device, &emit)
    })
    .await
    .map_err(|e| PanadigaIpcError::internal(format!("Falha assíncrona {task_label}: {e}")))?
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

pub(crate) struct ValidatedHddTarget {
    pub device_path: String,
    pub confirm_token: String,
    pub expected_gb_range: Option<(u32, u32)>,
}

pub(crate) fn validate_hdd_target_and_range(
    device_path: &str,
    confirm_token: &str,
    expected_min_gb: Option<u32>,
    expected_max_gb: Option<u32>,
    allow_extended_4tb: Option<bool>,
) -> IpcResult<ValidatedHddTarget> {
    let dev = device_path.trim();
    if dev.is_empty() {
        return Err(PanadigaIpcError::validation(
            "devicePath não pode ser vazio.",
        ));
    }
    if is_unconditional_os_boot_disk(dev) {
        return Err(PanadigaIpcError::validation(
            "LOCKOUT FÍSICO FATAL: Disco primário de boot do SO (PhysicalDrive0 / disk0) é permanentemente bloqueado.",
        ));
    }
    let token = normalize_disk_confirm_token(confirm_token);
    if token.is_empty() {
        return Err(PanadigaIpcError::validation(
            "confirmDiskModelToken obrigatório para autorizar escrita em disco.",
        ));
    }
    let allow_4tb = allow_extended_4tb.unwrap_or(false);
    let expected_gb_range = match (expected_min_gb, expected_max_gb) {
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
    Ok(ValidatedHddTarget {
        device_path: dev.to_string(),
        confirm_token: token,
        expected_gb_range,
    })
}

pub fn build_eh55_flash_request_from_dto(
    dto: &FlashEh55HddSafeRequestDto,
) -> IpcResult<Eh55HddFlashRequest> {
    let target = validate_hdd_target_and_range(
        &dto.device_path,
        &dto.confirm_disk_model_token,
        dto.expected_min_gb,
        dto.expected_max_gb,
        dto.allow_extended_4tb,
    )?;

    let image_raw = opt_trimmed(&dto.image_path).unwrap_or(DEFAULT_HDD_IMAGE_PATH);
    let image_path = resolve_safe_read_path(image_raw)?;
    let mirror_p1_path = opt_trimmed(&dto.mirror_p1_path)
        .map(resolve_safe_read_path)
        .transpose()?;
    let backup_dir = resolve_dir_target(dto.backup_dir.trim())?;

    Ok(Eh55HddFlashRequest {
        device_path: target.device_path,
        confirmation_token: target.confirm_token,
        image_path,
        mirror_p1_path,
        extract_p1_mirror_from_image: opt_trimmed(&dto.mirror_p1_path).is_none(),
        backup_dir,
        expected_gb_range: target.expected_gb_range,
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
        on_progress(HddFlashProgressEventDto::from_core(&dev_label, p));
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
    state: tauri::State<'_, crate::state::PanadigaBackendState>,
    request: FlashEh55HddSafeRequestDto,
) -> IpcResult<Eh55HddFlashReport> {
    let req = build_eh55_flash_request_from_dto(&request)?;
    let image_ref = opt_trimmed(&request.image_path).unwrap_or("default_hdd");
    crate::commands::hardware_diag::require_destructive_nonce(
        state.inner(),
        request.nonce_id.as_deref(),
        panadiga_core::bench::OpKind::NorWrite,
        req.device_path.trim(),
        image_ref,
    )?;
    let req_clone = request.clone();
    run_physical_hdd_flash_task(app, req.device_path, "HDD flash", move |device, emit| {
        run_flash_eh55_with_device(device, &req_clone, emit)
    })
    .await
}
