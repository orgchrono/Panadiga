#![deny(unsafe_code)]

//! Tauri IPC commands for cross-platform `1x` / minimum-speed ISO 9660 Level 1 DVD recovery creation
//! (`dvd://burn-progress`).

use panadiga_core::constants::resolve_writable_data_root;
use panadiga_core::re::unbrick_sd_kit::{
    DvdCreatorProgress, DvdCreatorReport, DvdCreatorRequest, DvdCreatorStage,
    OpticalBurnerDriveInfo, create_dvd_recovery_disc, create_dvd_recovery_iso,
    inspect_optical_burners,
};
use panadiga_core::vault::StrictWorkflowStep;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::vault_session::record_vault_outcome_in;
use crate::commands::{
    constants::EVENT_DVD_BURN_PROGRESS,
    error::{IpcResult, PanadigaIpcError},
    hardware_diag::resolve_auto_backup_file,
    unbrick_dto::{CommonMediaKitDto, opt_safe_read_path, opt_trimmed},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DvdBurnProgressEventDto {
    pub target: String,
    pub stage: DvdCreatorStage,
    pub percent: u8,
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDvdRecoveryDiscRequestDto {
    pub burner_id: Option<String>,
    pub confirm_token: Option<String>,
    pub output_iso_path: Option<String>,
    pub volume_label: Option<String>,
    pub dry_run: Option<bool>,
    #[serde(default)]
    pub nonce_id: Option<String>,
    #[serde(flatten)]
    pub media: CommonMediaKitDto,
}

fn build_dvd_creator_request(
    dto: &CreateDvdRecoveryDiscRequestDto,
) -> IpcResult<DvdCreatorRequest> {
    let loader_profile = dto.media.parse_loader_profile()?;
    let burner_id = opt_trimmed(&dto.burner_id).map(str::to_string);
    let confirm_token = opt_trimmed(&dto.confirm_token).map(str::to_string);
    let output_iso_path = if burner_id.is_none() {
        Some(resolve_auto_backup_file(
            opt_trimmed(&dto.output_iso_path),
            "eh55_dvd_recovery_iso9660",
            "iso",
        )?)
    } else {
        if cfg!(target_os = "android") {
            return Err(PanadigaIpcError::validation(
                "Gravação óptica direta não é suportada no Android APK; gere o .iso offline.",
            ));
        }
        if confirm_token.is_none() {
            return Err(PanadigaIpcError::validation(
                "confirmToken obrigatório (CONFIRM-<burner>) para gravação de DVD.",
            ));
        }
        opt_trimmed(&dto.output_iso_path)
            .map(|p| resolve_auto_backup_file(Some(p), "eh55_dvd_recovery_iso9660", "iso"))
            .transpose()?
    };

    let digadiag_path = opt_safe_read_path(&dto.media.digadiag_path)?;
    let pana_dvd_path = opt_safe_read_path(&dto.media.pana_dvd_path)?;
    Ok(DvdCreatorRequest {
        burner_id,
        confirm_token,
        output_iso_path,
        model: opt_trimmed(&dto.media.model)
            .unwrap_or("DMR-EH55")
            .to_string(),
        volume_label: opt_trimmed(&dto.volume_label)
            .unwrap_or("PANADIGA_REC")
            .to_string(),
        loader_profile,
        cleanroom_only: dto.media.cleanroom_only.unwrap_or(digadiag_path.is_none()),
        digadiag_path,
        pana_dvd_path,
        dry_run: dto.dry_run.unwrap_or(false),
    })
}

pub fn run_dvd_creator_with_progress<F>(
    dto: &CreateDvdRecoveryDiscRequestDto,
    mut on_progress: F,
) -> IpcResult<DvdCreatorReport>
where
    F: FnMut(DvdBurnProgressEventDto),
{
    let req = build_dvd_creator_request(dto)?;
    let report = if req.burner_id.is_some() {
        let target_label = req.burner_id.clone().unwrap_or_default();
        create_dvd_recovery_disc(&req, |prog: DvdCreatorProgress| {
            on_progress(DvdBurnProgressEventDto {
                target: target_label.clone(),
                stage: prog.stage,
                percent: prog.percent,
                detail: prog.detail,
            });
        })
        .map_err(|e| PanadigaIpcError::classify(format!("Falha gravando DVD ISO 9660 1x: {e}")))?
    } else {
        let out_iso = req.output_iso_path.clone().ok_or_else(|| {
            PanadigaIpcError::internal("Caminho .iso ausente no modo ISO offline.")
        })?;
        let target_label = out_iso.display().to_string();
        on_progress(DvdBurnProgressEventDto {
            target: target_label.clone(),
            stage: DvdCreatorStage::SynthesizeIso,
            percent: 40,
            detail: "Sintetizando ISO 9660 Level 1 + validação Mn103Vm...".into(),
        });
        let rep = create_dvd_recovery_iso(&req, &out_iso).map_err(|e| {
            PanadigaIpcError::classify(format!("Falha gerando ISO 9660 Level 1: {e}"))
        })?;
        on_progress(DvdBurnProgressEventDto {
            target: target_label,
            stage: DvdCreatorStage::ReadbackVerify,
            percent: 100,
            detail: format!("ISO 9660 verificada ({} B)", rep.bytes_written),
        });
        rep
    };

    record_vault_outcome_in(
        &resolve_writable_data_root(),
        dto.media.session_id.as_deref(),
        &report.emu_gate,
        None,
        &[StrictWorkflowStep::BuildCleanroomSd],
    )?;
    Ok(report)
}

#[tauri::command]
pub fn inspect_optical_burners_command() -> IpcResult<Vec<OpticalBurnerDriveInfo>> {
    Ok(inspect_optical_burners())
}

#[tauri::command]
pub async fn create_dvd_recovery_disc_command(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::PanadigaBackendState>,
    request: CreateDvdRecoveryDiscRequestDto,
) -> IpcResult<DvdCreatorReport> {
    let req = build_dvd_creator_request(&request)?;
    if let Some(burner) = req.burner_id.as_deref()
        && !req.dry_run
    {
        crate::commands::hardware_diag::require_destructive_nonce(
            state.inner(),
            request.nonce_id.as_deref(),
            panadiga_core::bench::OpKind::SdKitBuild,
            burner,
            req.confirm_token.as_deref().unwrap_or("dvd_burn"),
        )?;
    }
    let app_clone = app.clone();
    let req_clone = request.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_dvd_creator_with_progress(&req_clone, |ev| {
            let _ = app_clone.emit(EVENT_DVD_BURN_PROGRESS, &ev);
        })
    })
    .await
    .map_err(|e| PanadigaIpcError::internal(format!("Falha assíncrona DVD Creator: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_dvd_creator_synthesizes_cleanroom_iso9660_and_enforces_emu_gate() {
        let dir = std::env::temp_dir().join(format!("panadiga_dvd_ipc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out_iso = dir.join("eh55_recovery.iso");

        let dto = CreateDvdRecoveryDiscRequestDto {
            burner_id: None,
            confirm_token: None,
            output_iso_path: Some(out_iso.to_string_lossy().into_owned()),
            volume_label: Some("PANADIGA_REC".into()),
            dry_run: Some(false),
            nonce_id: None,
            media: CommonMediaKitDto {
                model: Some("DMR-EH55".into()),
                loader_profile: Some("ver061".into()),
                digadiag_path: None,
                pana_dvd_path: None,
                cleanroom_only: Some(true),
                session_id: None,
            },
        };
        let mut events = Vec::new();
        let rep = run_dvd_creator_with_progress(&dto, |ev| events.push(ev)).unwrap();
        assert!(rep.emu_gate.passed);
        assert_eq!(rep.inspection.overall_status, "PASS");
        assert_eq!(rep.write_speed_policy, "1x_or_drive_minimum_dao");
        assert_eq!(events.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
