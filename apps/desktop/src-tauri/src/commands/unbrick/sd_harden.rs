#![deny(unsafe_code)]

//! Tauri IPC command for building & verifying hardened `PANAMON1` `Loader ver0.61` SD recovery kits.

use std::fs;
use std::path::Path;

use panadiga_core::re::unbrick_sd_kit::{
    DualFamilyParityReport, SdKitAuditProfile, SdLoaderProfile, SdMediaVerificationReport,
    SdUnbrickKitReport, SdUnbrickKitRequest, prepare_sd_unbrick_kit,
    verify_sd_buffers_with_profile,
};
use panadiga_core::re::{MainPassthroughMode, Prog061BgoPatchMode};
use serde::{Deserialize, Serialize};

use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::unbrick_dto::{opt_safe_read_path, opt_trimmed, resolve_dir_target};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardenSdUnbrickKitRequestDto {
    pub output_dir: String,
    pub pana_dvd_path: Option<String>,
    pub prog061_path: Option<String>,
    pub main_path: Option<String>,
    pub drv_path: Option<String>,
    pub main_passthrough_mode: Option<String>,
    pub unlock_hdd_dynamic: Option<bool>,
    pub hdd_target_gb: Option<u32>,
    pub expected_main_build: Option<String>,
    pub allow_target_mismatch: Option<bool>,
    pub allow_factory_prog061: Option<bool>,
    pub write_extracted_sources: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardenSdUnbrickKitResponseDto {
    pub kit_report: SdUnbrickKitReport,
    pub hardened_verification: SdMediaVerificationReport,
    pub dual_family_parity: Option<DualFamilyParityReport>,
    pub factory_prog061_blocked: bool,
}

/// Builds a hardened `PANAMON1` `ver0.61` (`262,144 B` raw + dual `M6930LB`/`M6930` `sd48`) SD kit
/// and verifies it under `SdKitAuditProfile::HardenedRecovery`.
#[tauri::command]
pub fn harden_sd_unbrick_kit_command(
    request: HardenSdUnbrickKitRequestDto,
) -> IpcResult<HardenSdUnbrickKitResponseDto> {
    let out_dir = resolve_dir_target(request.output_dir.trim())?;
    let mut req = SdUnbrickKitRequest::hardened_panamon1(out_dir);
    req.pana_dvd_path = opt_safe_read_path(&request.pana_dvd_path)?;
    req.prog061_path = opt_safe_read_path(&request.prog061_path)?;
    req.main_path = opt_safe_read_path(&request.main_path)?;
    req.drv_path = opt_safe_read_path(&request.drv_path)?;
    req.loader_profile = SdLoaderProfile::Ver061RawSd48;
    req.bgo_mode = if request.allow_factory_prog061.unwrap_or(false) {
        Prog061BgoPatchMode::FactoryOriginal
    } else {
        Prog061BgoPatchMode::UartMonitor
    };
    req.main_passthrough_mode = opt_trimmed(&request.main_passthrough_mode)
        .unwrap_or("av_passthrough")
        .parse::<MainPassthroughMode>()
        .map_err(|e| PanadigaIpcError::validation(format!("Modo passthrough inválido: {e}")))?;
    req.hdd_target_gb = request.hdd_target_gb;
    req.unlock_hdd_dynamic =
        request.unlock_hdd_dynamic.unwrap_or(true) && request.hdd_target_gb.is_none();
    if let Some(exp) = opt_trimmed(&request.expected_main_build) {
        req.expected_main_build = Some(exp.to_string());
    }
    req.allow_target_mismatch = request.allow_target_mismatch.unwrap_or(false);
    req.write_extracted_sources = request.write_extracted_sources.unwrap_or(false);

    let kit_report = prepare_sd_unbrick_kit(&req)
        .map_err(|e| PanadigaIpcError::classify(format!("Falha gerando kit SD hardened: {e}")))?;

    let diag_bytes = fs::read(Path::new(&kit_report.digadiag_path))
        .map_err(|e| PanadigaIpcError::io(format!("Falha lendo DIGADIAG.FRM gerado: {e}")))?;
    let pana_bytes = kit_report
        .pana_sd_path
        .as_deref()
        .map(fs::read)
        .transpose()
        .map_err(|e| PanadigaIpcError::io(format!("Falha lendo PANA__SD.FRM gerado: {e}")))?;

    let hardened_verification = verify_sd_buffers_with_profile(
        &kit_report.output_dir,
        Some(Ok(&diag_bytes)),
        pana_bytes.as_deref(),
        SdKitAuditProfile::HardenedRecovery,
    );

    let allow_factory = request.allow_factory_prog061.unwrap_or(false);
    if hardened_verification.factory_unpatched_prog061_detected && !allow_factory {
        return Err(PanadigaIpcError::validation(
            "Kit rejeitado: PROG061 de fábrica [05/11/01] não patched detectado sem override explícito.",
        ));
    }
    if hardened_verification.overall_status == "INCOMPLETE_OR_INVALID" {
        return Err(PanadigaIpcError::classify(
            "Kit hardened falhou na validação estrita dual-family ver0.61 + PANAMON1.",
        ));
    }

    let dual_family_parity = hardened_verification.dual_family_parity.clone();
    Ok(HardenSdUnbrickKitResponseDto {
        kit_report,
        hardened_verification,
        dual_family_parity,
        factory_prog061_blocked: !allow_factory,
    })
}
