#![deny(unsafe_code)]

//! Tauri IPC commands for Region & Analog TV Standard calibration (`DEV.BIN`/`ifodev2`, `MAIN`, `.FRM`).

use std::fs;
use std::path::Path;

use panadiga_core::re::region_compat::{
    LbsRegionPresetId, TvCalibrationBundleReport, TvStandardKind, inspect_tv_calibration_bundle,
};
use panadiga_core::re::{
    DevBinReport, FrmRetargetReport, MainRegionCompatReport, RegionCompatMode, patch_dev_bin_bytes,
    patch_main_region_compat_in_place, retarget_frm_toc_bytes,
};
use serde::{Deserialize, Serialize};

use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::resolve_read_path;
use crate::commands::unbrick_dto::{opt_read_path, opt_trimmed, resolve_dir_target};

pub(crate) const REGION_CALIBRATION_CONFIRM_TOKEN: &str = "CALIBRATE_REGION_COW";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectRegionCalibrationRequestDto {
    pub preset: Option<String>,
    pub tv_standard: Option<String>,
    pub dev_bin_path: Option<String>,
    pub main_path: Option<String>,
    pub frm_path: Option<String>,
    pub nvram_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRegionCalibrationRequestDto {
    pub preset: Option<String>,
    pub tv_standard: Option<String>,
    pub output_dir: String,
    pub dev_bin_path: Option<String>,
    pub model: Option<String>,
    pub oem: Option<String>,
    pub main_path: Option<String>,
    pub main_compat_mode: Option<String>,
    pub frm_path: Option<String>,
    pub hw_family: Option<String>,
    pub model_group: Option<String>,
    pub target_flag: Option<u8>,
    pub confirm_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRegionCalibrationResponseDto {
    pub output_dir: String,
    pub metadata_and_dev_oem_only: bool,
    pub dev_bin_output_path: Option<String>,
    pub dev_bin_report: Option<DevBinReport>,
    pub main_output_path: Option<String>,
    pub main_report: Option<MainRegionCompatReport>,
    pub frm_output_path: Option<String>,
    pub frm_report: Option<FrmRetargetReport>,
    pub bundle_audit: TvCalibrationBundleReport,
}

pub(crate) fn parse_preset_and_standard(
    preset_opt: Option<&str>,
    std_opt: Option<&str>,
) -> IpcResult<(LbsRegionPresetId, Option<TvStandardKind>)> {
    let raw_preset = preset_opt.unwrap_or("br-pal-m").trim().to_ascii_lowercase();
    let preset = match raw_preset.as_str() {
        "br-pal-m" => LbsRegionPresetId::BrPalMR4,
        "us-ntsc" => LbsRegionPresetId::UsNtscR1,
        "eu-pal" => LbsRegionPresetId::EuPalR2,
        other => other
            .parse()
            .map_err(|e| PanadigaIpcError::validation(format!("Preset LBS inválido: {e}")))?,
    };
    let tv_std = std_opt
        .map(str::parse::<TvStandardKind>)
        .transpose()
        .map_err(|e| PanadigaIpcError::validation(format!("Padrão de TV inválido: {e}")))?;
    Ok((preset, tv_std))
}

fn read_opt_path_bytes(path: Option<&Path>, label: &str) -> IpcResult<Option<Vec<u8>>> {
    path.map(|p| {
        fs::read(p).map_err(|e| {
            PanadigaIpcError::io(format!("Falha lendo {label} '{}': {e}", p.display()))
        })
    })
    .transpose()
}

fn ensure_cow_distinct(src: &Path, dst: &Path) -> IpcResult<()> {
    if src == dst
        || (src.exists()
            && dst.exists()
            && fs::canonicalize(src).ok() == fs::canonicalize(dst).ok())
    {
        return Err(PanadigaIpcError::validation(format!(
            "Violação CoW: destino '{}' não pode sobrescrever '{}'",
            dst.display(),
            src.display()
        )));
    }
    Ok(())
}

/// Inspects `DEV.BIN`/`ifodev2`, `MAIN`, `.FRM`, and NVRAM `SA134..SA141` against an LBS/TV profile.
#[tauri::command]
pub fn inspect_region_calibration_command(
    request: InspectRegionCalibrationRequestDto,
) -> IpcResult<TvCalibrationBundleReport> {
    let (preset, tv_std) = parse_preset_and_standard(
        opt_trimmed(&request.preset),
        opt_trimmed(&request.tv_standard),
    )?;
    let dev_b = read_opt_path_bytes(opt_read_path(&request.dev_bin_path).as_deref(), "DEV.BIN")?;
    let main_b = read_opt_path_bytes(opt_read_path(&request.main_path).as_deref(), "MAIN")?;
    let frm_b = read_opt_path_bytes(opt_read_path(&request.frm_path).as_deref(), ".FRM")?;
    let nv_b = read_opt_path_bytes(opt_read_path(&request.nvram_path).as_deref(), "NVRAM/NOR")?;

    inspect_tv_calibration_bundle(
        preset,
        tv_std,
        dev_b.as_deref(),
        main_b.as_deref(),
        frm_b.as_deref(),
        nv_b.as_deref(),
    )
    .map_err(|e| PanadigaIpcError::classify(format!("Falha na auditoria TV/LBS: {e}")))
}

/// Applies Copy-on-Write region/model calibration while keeping `ifodev2` `0x400..0x7FF` read-only.
#[tauri::command]
pub fn apply_region_calibration_command(
    request: ApplyRegionCalibrationRequestDto,
) -> IpcResult<ApplyRegionCalibrationResponseDto> {
    if request.confirm_token.trim() != REGION_CALIBRATION_CONFIRM_TOKEN {
        return Err(PanadigaIpcError::validation(format!(
            "Token de confirmação inválido (esperado '{}').",
            REGION_CALIBRATION_CONFIRM_TOKEN
        )));
    }
    let (preset, tv_std) = parse_preset_and_standard(
        opt_trimmed(&request.preset),
        opt_trimmed(&request.tv_standard),
    )?;
    let out_dir = resolve_dir_target(request.output_dir.trim())?;
    fs::create_dir_all(&out_dir)
        .map_err(|e| PanadigaIpcError::io(format!("Falha criando '{}': {e}", out_dir.display())))?;

    let mut out_dev_bytes = None;
    let mut dev_out_path = None;
    let mut dev_report = None;
    if let Some(src_raw) = opt_trimmed(&request.dev_bin_path) {
        let src = resolve_read_path(src_raw);
        let dst = out_dir.join(
            src.file_name()
                .unwrap_or_else(|| "DEV_calibrated.BIN".as_ref()),
        );
        ensure_cow_distinct(&src, &dst)?;
        let raw = fs::read(&src).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
        let target_model = opt_trimmed(&request.model).unwrap_or("EH55LB");
        let oem_char = match opt_trimmed(&request.oem) {
            Some(s) if s.chars().count() == 1 => s.chars().next(),
            Some(bad) => {
                return Err(PanadigaIpcError::validation(format!(
                    "OEM deve ter exatamente 1 caractere ASCII, recebido '{bad}'"
                )));
            }
            None => preset.expected_oem_char(),
        };
        let (patched, rep) = patch_dev_bin_bytes(&raw, target_model, oem_char)
            .map_err(|e| PanadigaIpcError::validation(format!("Falha patching DEV.BIN: {e}")))?;
        fs::write(&dst, &patched).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
        out_dev_bytes = Some(patched);
        dev_out_path = Some(dst.to_string_lossy().into_owned());
        dev_report = Some(rep);
    }

    let mut out_main_bytes = None;
    let mut main_out_path = None;
    let mut main_report = None;
    if let Some(src_raw) = opt_trimmed(&request.main_path) {
        let src = resolve_read_path(src_raw);
        let dst = out_dir.join("MAIN_calibrated.bin");
        ensure_cow_distinct(&src, &dst)?;
        let mut raw = fs::read(&src).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
        let mode: RegionCompatMode = opt_trimmed(&request.main_compat_mode)
            .unwrap_or("bypass")
            .parse()
            .map_err(|e| PanadigaIpcError::validation(format!("{e}")))?;
        let rep = patch_main_region_compat_in_place(&mut raw, mode)
            .map_err(|e| PanadigaIpcError::classify(format!("Falha patching MAIN: {e}")))?;
        fs::write(&dst, &raw).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
        out_main_bytes = Some(raw);
        main_out_path = Some(dst.to_string_lossy().into_owned());
        main_report = Some(rep);
    }

    let mut out_frm_bytes = None;
    let mut frm_out_path = None;
    let mut frm_report = None;
    if let Some(src_raw) = opt_trimmed(&request.frm_path) {
        let src = resolve_read_path(src_raw);
        let dst = out_dir.join(
            src.file_name()
                .unwrap_or_else(|| "PANA__SD_calibrated.FRM".as_ref()),
        );
        ensure_cow_distinct(&src, &dst)?;
        let raw = fs::read(&src).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
        let oem_c = opt_trimmed(&request.oem).and_then(|s| s.chars().next());
        let (patched, rep) = retarget_frm_toc_bytes(
            &raw,
            opt_trimmed(&request.hw_family).or(Some(preset.expected_chassis_family())),
            opt_trimmed(&request.model_group),
            oem_c,
            request.target_flag,
        )
        .map_err(|e| PanadigaIpcError::validation(format!("Falha retarget .FRM: {e}")))?;
        fs::write(&dst, &patched).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
        out_frm_bytes = Some(patched);
        frm_out_path = Some(dst.to_string_lossy().into_owned());
        frm_report = Some(rep);
    }

    let bundle_audit = inspect_tv_calibration_bundle(
        preset,
        tv_std,
        out_dev_bytes.as_deref(),
        out_main_bytes.as_deref(),
        out_frm_bytes.as_deref(),
        None,
    )
    .map_err(|e| PanadigaIpcError::classify(format!("Auditoria pós-calibragem falhou: {e}")))?;

    Ok(ApplyRegionCalibrationResponseDto {
        output_dir: out_dir.to_string_lossy().into_owned(),
        metadata_and_dev_oem_only: true,
        dev_bin_output_path: dev_out_path,
        dev_bin_report: dev_report,
        main_output_path: main_out_path,
        main_report,
        frm_output_path: frm_out_path,
        frm_report,
        bundle_audit,
    })
}
