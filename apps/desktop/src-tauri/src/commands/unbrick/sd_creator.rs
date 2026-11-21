#![deny(unsafe_code)]

//! Tauri IPC commands for safe SD card target discovery (`StorageMediaRole::SdCard`)
//! and deterministic FAT16B (`0x06`, `32 MiB`) recovery card / `.img` creation (`sd://flash-progress`).

use std::path::PathBuf;

use panadiga_core::constants::resolve_writable_data_root;
use panadiga_core::re::unbrick_sd_kit::{
    FAT16_DEFAULT_VOLUME_MB, SdCardCreatorReport, SdCardCreatorRequest, SdCreatorProgress,
    SdCreatorStage, SdTargetDriveInfo, build_cleanroom_digadiag, create_sd_recovery_card_on_device,
    create_sd_recovery_image, inspect_sd_card_targets,
};
use panadiga_core::vault::{StrictWorkflowStep, VaultBucket};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::vault_session::record_vault_outcome_in;

use crate::commands::constants::EVENT_SD_FLASH_PROGRESS;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::hardware_diag::resolve_auto_backup_file;
use crate::commands::unbrick_dto::{CommonMediaKitDto, opt_read_path, opt_trimmed};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashSdRecoveryCardRequestDto {
    pub device_path: Option<String>,
    pub confirm_token: Option<String>,
    pub output_image_path: Option<String>,
    pub volume_mb: Option<u32>,
    pub pana_sd_path: Option<String>,
    pub include_pana_sd: Option<bool>,
    #[serde(flatten)]
    pub media: CommonMediaKitDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdFlashProgressEventDto {
    pub target: String,
    pub stage: SdCreatorStage,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub detail: String,
}

fn build_sd_creator_request(
    dto: &FlashSdRecoveryCardRequestDto,
) -> IpcResult<(SdCardCreatorRequest, Option<PathBuf>)> {
    let loader_profile = dto.media.parse_loader_profile()?;
    let device_path = opt_trimmed(&dto.device_path).map(str::to_string);
    let confirm_token = opt_trimmed(&dto.confirm_token).map(str::to_string);

    let out_img_path = if device_path.is_none() {
        Some(resolve_auto_backup_file(
            opt_trimmed(&dto.output_image_path),
            "eh55_sd_recovery_32mb",
            "img",
        )?)
    } else {
        if cfg!(target_os = "android") {
            return Err(PanadigaIpcError::validation(
                "Gravação direta de bloco físico SD não é suportada no Android APK; gere o arquivo .img ou use o desktop.",
            ));
        }
        if confirm_token.is_none() {
            return Err(PanadigaIpcError::validation(
                "confirmToken obrigatório (CONFIRM-<dispositivo>) para gravar cartão SD físico.",
            ));
        }
        None
    };

    let model = opt_trimmed(&dto.media.model)
        .unwrap_or("DMR-EH55")
        .to_string();
    let volume_mb = dto.volume_mb.unwrap_or(FAT16_DEFAULT_VOLUME_MB);

    let digadiag_path = opt_read_path(&dto.media.digadiag_path);
    let pana_sd_path = opt_read_path(&dto.pana_sd_path);
    let pana_dvd_path = opt_read_path(&dto.media.pana_dvd_path);
    let include_pana_sd = dto.include_pana_sd.unwrap_or(false);
    let has_proprietary = digadiag_path.is_some()
        || pana_sd_path.is_some()
        || pana_dvd_path.is_some()
        || include_pana_sd;
    let cleanroom_only = dto.media.cleanroom_only.unwrap_or(!has_proprietary);
    let req = SdCardCreatorRequest {
        device_path,
        confirm_token,
        output_image_path: out_img_path.clone(),
        model,
        volume_mb,
        loader_profile,
        digadiag_path,
        pana_sd_path,
        pana_dvd_path,
        include_pana_sd,
        cleanroom_only,
    };
    Ok((req, out_img_path))
}

fn record_sd_in_vault_session(
    session_id_opt: Option<&str>,
    req: &SdCardCreatorRequest,
    report: &SdCardCreatorReport,
) -> IpcResult<()> {
    let art_bytes = if report.cleanroom_only {
        build_cleanroom_digadiag(req.loader_profile)
            .ok()
            .map(|a| a.bytes)
    } else {
        None
    };
    let artifact = art_bytes.as_deref().map(|b| {
        (
            b,
            "DIGADIAG.FRM",
            VaultBucket::Recompiled,
            "cleanroom_sd_creator",
        )
    });
    record_vault_outcome_in(
        &resolve_writable_data_root(),
        session_id_opt,
        &report.emu_gate,
        artifact,
        &[StrictWorkflowStep::BuildCleanroomSd],
    )
}

pub fn run_sd_creator_with_progress<F>(
    dto: &FlashSdRecoveryCardRequestDto,
    mut on_progress: F,
) -> IpcResult<SdCardCreatorReport>
where
    F: FnMut(SdFlashProgressEventDto),
{
    let (req, out_img_opt) = build_sd_creator_request(dto)?;
    let report = if let Some(dev_path) = req.device_path.clone() {
        create_sd_recovery_card_on_device(&req, |prog: SdCreatorProgress| {
            on_progress(SdFlashProgressEventDto {
                target: dev_path.clone(),
                stage: prog.stage,
                bytes_done: prog.bytes_done,
                bytes_total: prog.bytes_total,
                detail: prog.detail,
            });
        })
        .map_err(|e| PanadigaIpcError::classify(format!("Falha gravando cartão SD FAT16B: {e}")))?
    } else {
        let out_path = out_img_opt.ok_or_else(|| {
            PanadigaIpcError::internal("Caminho .img ausente no modo offline SD Creator.")
        })?;
        let target_label = out_path.display().to_string();
        on_progress(SdFlashProgressEventDto {
            target: target_label.clone(),
            stage: SdCreatorStage::Synthesize,
            bytes_done: 0,
            bytes_total: 1,
            detail: format!("Sintetizando imagem FAT16B (0x06) {} MiB...", req.volume_mb),
        });
        let rep = create_sd_recovery_image(&req, &out_path).map_err(|e| {
            PanadigaIpcError::classify(format!("Falha gerando imagem SD FAT16B: {e}"))
        })?;
        on_progress(SdFlashProgressEventDto {
            target: target_label,
            stage: SdCreatorStage::InspectFat16,
            bytes_done: rep.bytes_written as u64,
            bytes_total: rep.bytes_written as u64,
            detail: format!(
                "Imagem SD FAT16B verificada ({} B, status {})",
                rep.bytes_written, rep.inspection.overall_status
            ),
        });
        rep
    };
    record_sd_in_vault_session(dto.media.session_id.as_deref(), &req, &report)?;
    Ok(report)
}

/// Enumerates physical drives tagged with `StorageMediaRole::SdCard` eligibility and SDXC `>= 64 GiB` advisories.
#[tauri::command]
pub fn inspect_sd_card_targets_command() -> IpcResult<Vec<SdTargetDriveInfo>> {
    Ok(inspect_sd_card_targets())
}

/// Creates a deterministic FAT16B (`0x06`, `32 MiB`) SD recovery `.img` OR flashes a physical SD card
/// under strict `StorageMediaRole::SdCard` + `CONFIRM-<device>` lockout, emitting `"sd://flash-progress"`.
#[tauri::command]
pub async fn flash_sd_recovery_card_command(
    app: tauri::AppHandle,
    request: FlashSdRecoveryCardRequestDto,
) -> IpcResult<SdCardCreatorReport> {
    let _ = build_sd_creator_request(&request)?;
    let app_clone = app.clone();
    let req_clone = request.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_sd_creator_with_progress(&req_clone, |ev| {
            let _ = app_clone.emit(EVENT_SD_FLASH_PROGRESS, &ev);
        })
    })
    .await
    .map_err(|e| PanadigaIpcError::internal(format!("Falha assíncrona SD Creator: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn offline_sd_creator_emits_progress_and_verifies_fat16b_0x06() {
        let dir = std::env::temp_dir().join("panadiga_sd_creator_cmd_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let diag_path = dir.join("DIGADIAG.FRM");
        let mut diag = vec![0xFFu8; 262_144];
        diag[0..4].copy_from_slice(&[0xDC, 0x14, 0x00, 0x00]);
        fs::write(&diag_path, &diag).unwrap();
        let out_img = dir.join("sd_recovery.img");

        let dto = FlashSdRecoveryCardRequestDto {
            device_path: None,
            confirm_token: None,
            output_image_path: Some(out_img.to_string_lossy().into_owned()),
            volume_mb: Some(32),
            pana_sd_path: None,
            include_pana_sd: Some(false),
            media: CommonMediaKitDto {
                model: Some("DMR-EH55".to_string()),
                loader_profile: Some("ver061".to_string()),
                digadiag_path: Some(diag_path.to_string_lossy().into_owned()),
                pana_dvd_path: None,
                cleanroom_only: Some(false),
                session_id: None,
            },
        };

        let mut events = Vec::new();
        let rep = run_sd_creator_with_progress(&dto, |ev| events.push(ev)).unwrap();
        assert_eq!(rep.volume_mb, 32);
        assert!(!rep.cleanroom_only);
        assert!(rep.emu_gate.passed);
        assert_eq!(rep.target_kind, "image_file");
        assert!(rep.readback_verified);
        assert_eq!(rep.inspection.overall_status, "PASS");
        assert_eq!(rep.inspection.mbr_partitions[0].partition_type, 0x06);
        assert_eq!(events.len(), 2);

        let cr_img = dir.join("sd_cleanroom.img");
        let cr_dto = FlashSdRecoveryCardRequestDto {
            output_image_path: Some(cr_img.to_string_lossy().into_owned()),
            media: CommonMediaKitDto {
                digadiag_path: None,
                cleanroom_only: Some(true),
                ..dto.media.clone()
            },
            ..dto.clone()
        };
        let cr_rep = run_sd_creator_with_progress(&cr_dto, |_| {}).unwrap();
        assert!(cr_rep.cleanroom_only);
        assert!(cr_rep.emu_gate.passed);
        assert_eq!(cr_rep.inspection.overall_status, "PASS");

        let missing_confirm = FlashSdRecoveryCardRequestDto {
            device_path: Some("\\\\.\\PhysicalDrive9".to_string()),
            confirm_token: None,
            ..dto
        };
        assert!(build_sd_creator_request(&missing_confirm).is_err());

        let _ = fs::remove_dir_all(&dir);
    }
}
