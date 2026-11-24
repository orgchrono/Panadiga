#![deny(unsafe_code)]

//! Tauri IPC command for streaming MEIHFS HDD composition flashing without any 100 MiB `.img` file,
//! enforced by the mandatory `Mn103Vm` pre-write gate (`flash_composed_hdd_safe_command`).

use panadiga_core::constants::resolve_writable_data_root;
use panadiga_core::hdd::writer::{
    BlockFlashDevice, ComposedHddFlashReport, ComposedHddFlashRequest,
    execute_composed_meihfs_hdd_flash,
};
use panadiga_core::vault::StrictWorkflowStep;
use serde::Deserialize;

use super::hdd_flash::{
    HddFlashProgressEventDto, run_physical_hdd_flash_task, validate_hdd_target_and_range,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::resolve_safe_read_path;
use crate::commands::hardware_diag::resolve_auto_backup_dir;
use crate::commands::unbrick::record_vault_outcome_in;
use crate::commands::unbrick_dto::opt_trimmed;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashComposedHddSafeRequestDto {
    pub device_path: String,
    pub confirm_disk_model_token: String,
    pub backup_dir: Option<String>,
    pub main_source_path: Option<String>,
    pub drv_source_path: Option<String>,
    pub apply_dynamic_capacity_unlock: Option<bool>,
    pub target_capacity_gb: Option<u32>,
    pub apply_av_passthrough: Option<bool>,
    pub expected_min_gb: Option<u32>,
    pub expected_max_gb: Option<u32>,
    pub allow_extended_4tb: Option<bool>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub nonce_id: Option<String>,
}

pub fn build_composed_hdd_request_from_dto(
    dto: &FlashComposedHddSafeRequestDto,
) -> IpcResult<ComposedHddFlashRequest> {
    let target = validate_hdd_target_and_range(
        &dto.device_path,
        &dto.confirm_disk_model_token,
        dto.expected_min_gb,
        dto.expected_max_gb,
        dto.allow_extended_4tb,
    )?;
    let backup_dir = resolve_auto_backup_dir(opt_trimmed(&dto.backup_dir), "hdd_composed_backup")?;
    Ok(ComposedHddFlashRequest {
        device_path: target.device_path,
        confirmation_token: target.confirm_token,
        backup_dir,
        expected_gb_range: target.expected_gb_range,
        detected_os_disks: None,
        main_source_path: opt_trimmed(&dto.main_source_path)
            .map(resolve_safe_read_path)
            .transpose()?,
        drv_source_path: opt_trimmed(&dto.drv_source_path)
            .map(resolve_safe_read_path)
            .transpose()?,
        apply_dynamic_capacity_unlock: dto.apply_dynamic_capacity_unlock.unwrap_or(false),
        target_capacity_gb: dto.target_capacity_gb,
        apply_av_passthrough: dto.apply_av_passthrough.unwrap_or(false),
        custom_guid: None,
    })
}

pub fn run_flash_composed_with_device<D, F>(
    device: &mut D,
    dto: &FlashComposedHddSafeRequestDto,
    detected_os_override: Option<Vec<String>>,
    mut on_progress: F,
) -> IpcResult<ComposedHddFlashReport>
where
    D: BlockFlashDevice + ?Sized,
    F: FnMut(HddFlashProgressEventDto),
{
    let mut req = build_composed_hdd_request_from_dto(dto)?;
    req.detected_os_disks = detected_os_override;
    let dev_label = req.device_path.clone();
    let rep = execute_composed_meihfs_hdd_flash(device, &req, |p| {
        on_progress(HddFlashProgressEventDto::from_core(&dev_label, p));
    })
    .map_err(|e| PanadigaIpcError::classify(format!("Falha no streaming MEIHFS composto: {e}")))?;

    record_vault_outcome_in(
        &resolve_writable_data_root(),
        dto.session_id.as_deref(),
        &rep.emu_gate,
        None,
        &[StrictWorkflowStep::EmulateAndFlash],
    )?;
    Ok(rep)
}

#[tauri::command]
pub async fn flash_composed_hdd_safe_command(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::PanadigaBackendState>,
    request: FlashComposedHddSafeRequestDto,
) -> IpcResult<ComposedHddFlashReport> {
    let req = build_composed_hdd_request_from_dto(&request)?;
    let input_ref = opt_trimmed(&request.main_source_path).unwrap_or("composed_hdd");
    crate::commands::hardware_diag::require_destructive_nonce(
        state.inner(),
        request.nonce_id.as_deref(),
        panadiga_core::bench::OpKind::NorWrite,
        req.device_path.trim(),
        input_ref,
    )?;
    let req_clone = request.clone();
    run_physical_hdd_flash_task(
        app,
        req.device_path,
        "HDD streaming flash",
        move |device, emit| run_flash_composed_with_device(device, &req_clone, None, emit),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use panadiga_core::hdd::writer::FileBackedMockFlashDevice;
    use std::fs;

    #[test]
    fn composed_hdd_ipc_streams_meihfs_with_mn103vm_gate_and_blocks_physicaldrive0() {
        let dir =
            std::env::temp_dir().join(format!("panadiga_composed_ipc_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let bdir = dir.join("backups");
        fs::create_dir_all(&bdir).unwrap();
        let disk_img = dir.join("mock_disk.img");
        let mut dev =
            FileBackedMockFlashDevice::create(&disk_img, 5 * 1024 * 1024 * 1024u64, 7).unwrap();

        let blocked_dto = FlashComposedHddSafeRequestDto {
            device_path: r"\\.\PhysicalDrive0".into(),
            confirm_disk_model_token: "CONFIRM-PhysicalDrive0".into(),
            backup_dir: Some(bdir.to_string_lossy().into_owned()),
            main_source_path: None,
            drv_source_path: None,
            apply_dynamic_capacity_unlock: None,
            target_capacity_gb: None,
            apply_av_passthrough: None,
            expected_min_gb: Some(5),
            expected_max_gb: Some(260),
            allow_extended_4tb: None,
            session_id: None,
            nonce_id: None,
        };
        assert!(build_composed_hdd_request_from_dto(&blocked_dto).is_err());

        let ok_dto = FlashComposedHddSafeRequestDto {
            device_path: r"\\.\PhysicalDrive4".into(),
            confirm_disk_model_token: "PhysicalDrive4".into(),
            ..blocked_dto
        };
        let mut events = Vec::new();
        let rep = run_flash_composed_with_device(
            &mut dev,
            &ok_dto,
            Some(vec![r"\\.\PhysicalDrive0".into()]),
            |ev| events.push(ev),
        )
        .unwrap();
        assert!(rep.emu_gate.passed);
        assert!(rep.readback_verified);
        assert!(!events.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
