#![deny(unsafe_code)]

//! Tauri IPC command for Diskless / TBC Appliance planning and Copy-on-Write `MAIN` AV bypass patching.

use panadiga_core::re::region_compat::TvStandardKind;
use panadiga_core::re::tbc_appliance::{
    TbcApplianceMode, TbcAppliancePlan, build_tbc_appliance_plan,
};
use panadiga_core::re::{MainPassthroughMode, MainPassthroughReport, patch_main_passthrough_file};
use serde::{Deserialize, Serialize};

use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::{resolve_read_path, resolve_write_path};
use crate::commands::unbrick_dto::opt_trimmed;

pub(crate) const TBC_BYPASS_CONFIRM_TOKEN: &str = "APPLY_MAIN_TBC_COW";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMainTbcBypassesRequestDto {
    pub input_main_path: Option<String>,
    pub output_main_path: Option<String>,
    pub passthrough_mode: Option<String>,
    pub appliance_mode: Option<String>,
    pub tv_standard: Option<String>,
    pub experimental_ram_chainload: Option<bool>,
    pub confirm_token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMainTbcBypassesResponseDto {
    pub appliance_plan: TbcAppliancePlan,
    pub main_patch_report: Option<MainPassthroughReport>,
    pub cow_output_path: Option<String>,
}

/// Plans a clean-room Diskless/TBC pipeline (`Stage15DigadiagTbc` or gated RAM chainload) and
/// optionally applies Copy-on-Write AV TBC bypass patches (`0x5011B33B`/`0x5011B40C`/`0x5011B522`) to `MAIN`.
#[tauri::command]
pub fn apply_main_tbc_bypasses_command(
    request: ApplyMainTbcBypassesRequestDto,
) -> IpcResult<ApplyMainTbcBypassesResponseDto> {
    if request.confirm_token.trim() != TBC_BYPASS_CONFIRM_TOKEN {
        return Err(PanadigaIpcError::validation(format!(
            "Token de confirmação inválido (esperado '{}').",
            TBC_BYPASS_CONFIRM_TOKEN
        )));
    }

    let app_mode: TbcApplianceMode = opt_trimmed(&request.appliance_mode)
        .unwrap_or("stage15_digadiag_tbc")
        .parse()
        .map_err(|e| PanadigaIpcError::validation(format!("Modo TBC inválido: {e}")))?;
    let tv_std: TvStandardKind = opt_trimmed(&request.tv_standard)
        .unwrap_or("ntsc")
        .parse()
        .map_err(|e| PanadigaIpcError::validation(format!("Padrão de TV inválido: {e}")))?;
    let exp_flag = request.experimental_ram_chainload.unwrap_or(false);

    let appliance_plan = build_tbc_appliance_plan(app_mode, tv_std, exp_flag)
        .map_err(|e| PanadigaIpcError::validation(format!("Falha gerando plano TBC: {e}")))?;

    let (main_patch_report, cow_output_path) = match opt_trimmed(&request.input_main_path) {
        Some(in_raw) => {
            let out_raw = opt_trimmed(&request.output_main_path).ok_or_else(|| {
                PanadigaIpcError::validation(
                    "outputMainPath obrigatório ao informar inputMainPath (CoW estrito).",
                )
            })?;
            let in_path = resolve_read_path(in_raw);
            let out_path = resolve_write_path(out_raw)?;
            let pt_mode: MainPassthroughMode = opt_trimmed(&request.passthrough_mode)
                .unwrap_or("av_passthrough")
                .parse()
                .map_err(|e| {
                    PanadigaIpcError::validation(format!("Modo passthrough inválido: {e}"))
                })?;
            let rep = patch_main_passthrough_file(&in_path, &out_path, pt_mode)
                .map_err(|e| PanadigaIpcError::classify(format!("Falha patching MAIN TBC: {e}")))?;
            let out_str = out_path.to_string_lossy().into_owned();
            (Some(rep), Some(out_str))
        }
        None => (None, None),
    };

    Ok(ApplyMainTbcBypassesResponseDto {
        appliance_plan,
        main_patch_report,
        cow_output_path,
    })
}
