//! Pure DTO validation, request/parameter mapping, and JSON response formatting for Unbrick IPC.

use panadiga_core::constants::resolve_writable_data_root;
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::{
    MainPassthroughMode, Prog061BgoPatchMode, SdLoaderProfile, SdUnbrickKitRequest,
};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

use super::constants::{
    DEFAULT_BGO_PATCH_MODE, DEFAULT_MAIN_PASSTHROUGH_MODE, DEFAULT_NOR_DUMP_BLOCK_SIZE,
    DEFAULT_NOR_DUMP_LEN_HEX, DEFAULT_NOR_DUMP_START_HEX, NOR_STUB_HEX_PREVIEW_BYTES,
};
use super::error::{IpcResult, PanadigaIpcError};
use super::frm_pack::{resolve_safe_read_path, resolve_safe_write_dir};

#[derive(Debug, Clone, Deserialize)]
pub struct SdUnbrickKitRequestDto {
    pub output_dir: String,
    pub pana_dvd_path: Option<String>,
    pub prog061_path: Option<String>,
    pub main_path: Option<String>,
    pub drv_path: Option<String>,
    pub bgo_mode: Option<String>,
    pub allow_unsafe_auto_format: Option<bool>,
    pub main_passthrough_mode: Option<String>,
    pub loader_profile: Option<String>,
    pub include_pana_sd: Option<bool>,
    pub unlock_hdd_dynamic: Option<bool>,
    pub hdd_target_gb: Option<u32>,
    pub write_extracted_sources: Option<bool>,
    pub expected_main_build: Option<String>,
    pub allow_target_mismatch: Option<bool>,
    #[serde(default)]
    pub sata_bridge_reset_fix: Option<bool>,
    #[serde(default)]
    pub sata_hdd_autoboot: Option<bool>,
    #[serde(default)]
    pub main_bgo_retarget: Option<bool>,
    #[serde(default)]
    pub nor_autoboot_capsule: Option<bool>,
    #[serde(default)]
    pub m6930lb_localization: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonMediaKitDto {
    pub model: Option<String>,
    pub loader_profile: Option<String>,
    pub digadiag_path: Option<String>,
    pub pana_dvd_path: Option<String>,
    pub cleanroom_only: Option<bool>,
    pub session_id: Option<String>,
}

impl CommonMediaKitDto {
    pub(crate) fn parse_loader_profile(&self) -> IpcResult<SdLoaderProfile> {
        opt_trimmed(&self.loader_profile)
            .unwrap_or("ver061")
            .parse()
            .map_err(|e: anyhow::Error| PanadigaIpcError::validation(e.to_string()))
    }
}

pub(crate) fn opt_trimmed(raw: &Option<String>) -> Option<&str> {
    raw.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

pub(crate) fn opt_safe_read_path(raw: &Option<String>) -> IpcResult<Option<PathBuf>> {
    opt_trimmed(raw).map(resolve_safe_read_path).transpose()
}

pub(crate) fn resolve_dir_target(raw: &str) -> IpcResult<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PanadigaIpcError::validation(
            "Caminho do diretório SD é obrigatório.",
        ));
    }
    resolve_safe_write_dir(trimmed)
}

pub(crate) fn to_sd_unbrick_kit_request(
    dto: &SdUnbrickKitRequestDto,
) -> IpcResult<SdUnbrickKitRequest> {
    let bgo_mode: Prog061BgoPatchMode = dto
        .bgo_mode
        .as_deref()
        .unwrap_or(DEFAULT_BGO_PATCH_MODE)
        .parse()
        .map_err(|e: anyhow::Error| PanadigaIpcError::validation(e.to_string()))?;
    let main_passthrough_mode: MainPassthroughMode = dto
        .main_passthrough_mode
        .as_deref()
        .unwrap_or(DEFAULT_MAIN_PASSTHROUGH_MODE)
        .parse()
        .map_err(|e: anyhow::Error| PanadigaIpcError::validation(e.to_string()))?;
    let loader_profile: SdLoaderProfile = dto
        .loader_profile
        .as_deref()
        .unwrap_or("ver061")
        .parse()
        .map_err(|e: anyhow::Error| PanadigaIpcError::validation(e.to_string()))?;

    let out_trimmed = dto.output_dir.trim();
    if out_trimmed.is_empty() {
        return Err(PanadigaIpcError::validation(
            "Diretório de saída do Kit SD é obrigatório.",
        ));
    }
    let out_p = PathBuf::from(out_trimmed);
    let resolved_out = if out_p.is_absolute() {
        resolve_safe_write_dir(out_trimmed)?
    } else {
        let candidate = resolve_writable_data_root().join(out_p);
        resolve_safe_write_dir(&candidate.to_string_lossy())?
    };

    Ok(SdUnbrickKitRequest {
        output_dir: resolved_out,
        pana_dvd_path: opt_safe_read_path(&dto.pana_dvd_path)?,
        prog061_path: opt_safe_read_path(&dto.prog061_path)?,
        main_path: opt_safe_read_path(&dto.main_path)?,
        drv_path: opt_safe_read_path(&dto.drv_path)?,
        bgo_mode,
        allow_unsafe_auto_format: dto.allow_unsafe_auto_format.unwrap_or(false),
        main_passthrough_mode,
        loader_profile,
        include_pana_sd: dto.include_pana_sd.unwrap_or(true),
        unlock_hdd_dynamic: dto.unlock_hdd_dynamic.unwrap_or(true),
        hdd_target_gb: dto.hdd_target_gb,
        write_extracted_sources: dto.write_extracted_sources.unwrap_or(false),
        expected_main_build: opt_trimmed(&dto.expected_main_build).map(str::to_string),
        allow_target_mismatch: dto.allow_target_mismatch.unwrap_or(false),
        sata_bridge_reset_fix: dto.sata_bridge_reset_fix.unwrap_or(false),
        sata_hdd_autoboot: dto.sata_hdd_autoboot.unwrap_or(false),
        main_bgo_retarget: dto.main_bgo_retarget.unwrap_or(false),
        nor_autoboot_capsule: dto.nor_autoboot_capsule.unwrap_or(false),
        m6930lb_localization: dto.m6930lb_localization.unwrap_or(false),
    })
}

pub(crate) struct NorDumperParams {
    pub start_u32: u32,
    pub len_u32: u32,
    pub block_size: u32,
    pub load_u32: u32,
    pub wrapped: bool,
}

pub(crate) fn parse_nor_dumper_params(
    start_addr: Option<&str>,
    total_len: Option<&str>,
    block_size: Option<u32>,
    load_addr: Option<&str>,
    wrap_as_digadiag: Option<bool>,
) -> IpcResult<NorDumperParams> {
    let wrapped = wrap_as_digadiag.unwrap_or(false);
    let explicit_load = load_addr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_u32_radix)
        .transpose()
        .map_err(|e| PanadigaIpcError::validation(format!("Load address inválido: {e}")))?;

    Ok(NorDumperParams {
        start_u32: parse_u32_radix(start_addr.unwrap_or(DEFAULT_NOR_DUMP_START_HEX)).map_err(
            |e| PanadigaIpcError::validation(format!("Endereço inicial NOR inválido: {e}")),
        )?,
        len_u32: parse_u32_radix(total_len.unwrap_or(DEFAULT_NOR_DUMP_LEN_HEX))
            .map_err(|e| PanadigaIpcError::validation(format!("Tamanho total inválido: {e}")))?,
        block_size: block_size.unwrap_or(DEFAULT_NOR_DUMP_BLOCK_SIZE),
        load_u32: panadiga_core::re::resolve_nor_stub_load_addr(explicit_load, wrapped),
        wrapped,
    })
}

pub(crate) fn format_nor_dumper_response(
    params: &NorDumperParams,
    stub_len: usize,
    payload: &[u8],
    written_path: Option<String>,
) -> Value {
    serde_json::json!({
        "start_addr": format!("0x{:08X}", params.start_u32),
        "total_len_bytes": params.len_u32,
        "block_size": params.block_size,
        "load_addr": format!("0x{:08X}", params.load_u32),
        "raw_stub_bytes": stub_len,
        "wrapped_digadiag": params.wrapped,
        "output_bytes": payload.len(),
        "written_path": written_path,
        "stub_hex": hex::encode(&payload[..payload.len().min(NOR_STUB_HEX_PREVIEW_BYTES)]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nor_dumper_param_defaults_and_dir_validation() {
        let wrapped = parse_nor_dumper_params(None, None, None, None, Some(true)).unwrap();
        assert_eq!(wrapped.start_u32, 0x8400_0000);
        assert_eq!(wrapped.len_u32, 0x80_0000);
        assert_eq!(wrapped.block_size, 4096);
        assert_eq!(wrapped.load_u32, 0x5040_0000);
        assert!(wrapped.wrapped);

        let raw = parse_nor_dumper_params(None, None, None, None, Some(false)).unwrap();
        assert_eq!(raw.load_u32, 0x5180_0000);
        assert!(!raw.wrapped);

        assert!(resolve_dir_target("   ").is_err());
    }
}
