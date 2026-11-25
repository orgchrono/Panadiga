//! Tauri 2 IPC Commands for Visual DigaOS SSOT Schema Customizer & Multi-SoC Studio.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use serde::Serialize;

use panadiga_core::bench::OpKind;
use panadiga_core::constants::resolve_workspace_root;
use panadiga_core::ssot::{
    DIGAOS_SSOT_CONTRACT_HASH, DigaOsConfigSpec, compute_digaos_contract_hash_for_raw,
    export_digaos_ssot_files, generate_ts_digaos_module_for, load_digaos_config_from_root,
    validate_digaos_config,
};

use crate::state::PanadigaBackendState;

use super::error::{IpcResult, PanadigaIpcError};
use super::hardware_diag::require_destructive_nonce;

const DIGAOS_CFG_REL_PATH: &str = "profiles/digaos/default_digaos_config.json";
const DIGAOS_TS_REL_PATH: &str = "apps/desktop/src/models/generated/digaos.ts";
const DIGAOS_C11_DIR_REL_PATH: &str = "firmware/digaos";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsotStudioBundleDto {
    pub contract_hash: u32,
    pub contract_hash_hex: String,
    pub config: DigaOsConfigSpec,
    pub raw_json: String,
    pub config_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsotStudioValidationDto {
    pub valid: bool,
    pub errors: Vec<String>,
    pub contract_hash: u32,
    pub contract_hash_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsotStudioSaveResultDto {
    pub saved: bool,
    pub contract_hash: u32,
    pub contract_hash_hex: String,
    pub updated_paths: Vec<String>,
    pub message: String,
}

pub(super) fn resolve_workspace_with_marker(marker: &str) -> PathBuf {
    let root = resolve_workspace_root();
    if root.join(marker).exists() {
        return root;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or(root)
}

fn studio_workspace_root() -> PathBuf {
    resolve_workspace_with_marker(DIGAOS_CFG_REL_PATH)
}

fn validate_raw_digaos_config_json(
    raw_json: &str,
) -> std::result::Result<(DigaOsConfigSpec, String, u32), Vec<String>> {
    let cfg: DigaOsConfigSpec = match serde_json::from_str(raw_json) {
        Ok(c) => c,
        Err(err) => return Err(vec![format!("Invalid DigaOS JSON schema syntax: {err}")]),
    };
    if let Err(err) = validate_digaos_config(&cfg) {
        return Err(vec![err.to_string()]);
    }
    let pretty = match serde_json::to_string_pretty(&cfg) {
        Ok(text) => format!("{text}\n"),
        Err(err) => return Err(vec![format!("Serialization failure: {err}")]),
    };
    let hash = compute_digaos_contract_hash_for_raw(&pretty);
    Ok((cfg, pretty, hash))
}

/// Loads the canonical declarative `DigaOS` SSOT configuration, hashes, menus, themes, power profiles, and multi-SoC matrix.
#[tauri::command]
pub fn ssot_studio_get_bundle() -> IpcResult<SsotStudioBundleDto> {
    let root = studio_workspace_root();
    let cfg_path = root.join(DIGAOS_CFG_REL_PATH);
    let raw_json = std::fs::read_to_string(&cfg_path)
        .unwrap_or_else(|_| panadiga_core::ssot::DEFAULT_DIGAOS_CONFIG_JSON.to_string());
    let config = load_digaos_config_from_root(&root)
        .map_err(|err| PanadigaIpcError::validation(err.to_string()))?;
    let hash = compute_digaos_contract_hash_for_raw(&raw_json);
    Ok(SsotStudioBundleDto {
        contract_hash: hash,
        contract_hash_hex: format!("0x{hash:08X}"),
        config,
        raw_json,
        config_path: cfg_path.to_string_lossy().into_owned(),
    })
}

/// Validates proposed custom `DigaOS` SSOT JSON fail-closed against hardware/silicon invariants without writing disk.
#[tauri::command]
pub fn ssot_studio_validate_config(config_json: String) -> IpcResult<SsotStudioValidationDto> {
    match validate_raw_digaos_config_json(&config_json) {
        Ok((_, _, hash)) => Ok(SsotStudioValidationDto {
            valid: true,
            errors: Vec::new(),
            contract_hash: hash,
            contract_hash_hex: format!("0x{hash:08X}"),
        }),
        Err(errors) => Ok(SsotStudioValidationDto {
            valid: false,
            errors,
            contract_hash: DIGAOS_SSOT_CONTRACT_HASH,
            contract_hash_hex: format!("0x{DIGAOS_SSOT_CONTRACT_HASH:08X}"),
        }),
    }
}

/// Nonce-gated atomic save of custom `DigaOS` SSOT JSON with immediate C11 + TypeScript codegen synchronization.
#[tauri::command]
pub fn ssot_studio_save_custom_config(
    state: tauri::State<'_, PanadigaBackendState>,
    config_json: String,
    nonce_id: Option<String>,
) -> IpcResult<SsotStudioSaveResultDto> {
    let (cfg, pretty, hash) = validate_raw_digaos_config_json(&config_json)
        .map_err(|errs| PanadigaIpcError::validation(errs.join("; ")))?;

    let hash_hex = format!("0x{hash:08X}");
    require_destructive_nonce(
        state.inner(),
        nonce_id.as_deref(),
        OpKind::RamProbe,
        "digaos_ssot",
        &hash_hex,
    )?;

    let root = studio_workspace_root();
    let cfg_path = root.join(DIGAOS_CFG_REL_PATH);
    std::fs::write(&cfg_path, pretty.as_bytes()).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Failed to persist DigaOS config '{}': {e}",
            cfg_path.display()
        ))
    })?;

    export_digaos_ssot_files(&root.join(DIGAOS_C11_DIR_REL_PATH), Some(&root))
        .map_err(|e| PanadigaIpcError::io(format!("Failed exporting DigaOS C11 SSOT: {e}")))?;

    let ts_path = root.join(DIGAOS_TS_REL_PATH);
    let ts_out = generate_ts_digaos_module_for(&cfg, hash);
    std::fs::write(&ts_path, ts_out.as_bytes()).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Failed exporting DigaOS TS SSOT '{}': {e}",
            ts_path.display()
        ))
    })?;

    let updated_paths = vec![
        DIGAOS_CFG_REL_PATH.to_string(),
        DIGAOS_TS_REL_PATH.to_string(),
        "firmware/digaos/generated/digaos_ssot.h".to_string(),
        "firmware/digaos/generated/digaos_ssot_tables.c".to_string(),
    ];

    Ok(SsotStudioSaveResultDto {
        saved: true,
        contract_hash: hash,
        contract_hash_hex: hash_hex.clone(),
        updated_paths,
        message: format!(
            "Saved DigaOS SSOT configuration cleanly (DIGAOS_SSOT_CONTRACT_HASH {hash_hex})."
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{ssot_studio_get_bundle, ssot_studio_validate_config};

    #[test]
    fn studio_bundle_loads_and_blocks_lethal_mmio_and_ipl_writes() {
        let bundle = ssot_studio_get_bundle().expect("bundle");
        assert_eq!(bundle.config.prompt, "DIGAOS>");
        assert!(
            bundle
                .config
                .soc_targets
                .iter()
                .any(|s| s.soc_core_id == "mn10300_am34")
        );

        let mut cfg = bundle.config.clone();
        cfg.power_profiles[0]
            .allowed_mmio_writes
            .push(panadiga_core::ssot::DigaOsMmioRangeSpec {
                start: 0xC000_100C,
                size: 4,
                note: "lethal probe".to_string(),
            });
        let bad_json = serde_json::to_string(&cfg).expect("serialize");
        let res = ssot_studio_validate_config(bad_json).expect("validate");
        assert!(!res.valid);
        assert!(
            res.errors.iter().any(|e| e.contains("0xC000100C")),
            "errors={:?}",
            res.errors
        );

        let mut ipl_cfg = bundle.config;
        ipl_cfg.power_profiles[0].allowed_mmio_writes.push(
            panadiga_core::ssot::DigaOsMmioRangeSpec {
                start: 0x8000_0000,
                size: 4,
                note: "ipl block".to_string(),
            },
        );
        let res_ipl = ssot_studio_validate_config(serde_json::to_string(&ipl_cfg).expect("json"))
            .expect("validate ipl");
        assert!(!res_ipl.valid);
    }
}
