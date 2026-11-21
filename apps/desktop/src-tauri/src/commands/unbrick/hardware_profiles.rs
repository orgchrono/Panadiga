#![deny(unsafe_code)]

//! Tauri IPC commands exposing resolved v2 relational JSON hardware, NOR Flash, EEPROM & capability policies.

use std::fs;
use std::path::Path;

use panadiga_core::ssot::{
    Authorization, Capability, Denial, HardwareProfileRegistry, LabOverride, NorFingerprintReport,
    RegistryValidationReport, ResolvedModelProfile, RuntimeFingerprint,
    fingerprint_nor_or_firmware,
};

use crate::commands::error::{IpcResult, PanadigaIpcError};

/// Combined IPC decision payload returned by `authorize_profile_capability_command`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAuthorizationOutcome {
    pub allowed: bool,
    pub authorization: Option<Authorization>,
    pub denial: Option<Denial>,
}

#[tauri::command]
pub fn list_hardware_profiles_command() -> IpcResult<Vec<ResolvedModelProfile>> {
    Ok(HardwareProfileRegistry::builtin().list_models())
}

#[tauri::command]
pub fn get_resolved_model_profile_command(model_id: String) -> IpcResult<ResolvedModelProfile> {
    HardwareProfileRegistry::builtin()
        .resolve_model(&model_id)
        .map_err(|e| PanadigaIpcError::validation(e.to_string()))
}

#[tauri::command]
pub fn validate_hardware_profiles_command() -> IpcResult<RegistryValidationReport> {
    HardwareProfileRegistry::builtin()
        .validate()
        .map_err(|e| PanadigaIpcError::internal(e.to_string()))
}

#[tauri::command]
pub fn authorize_profile_capability_command(
    model_id: String,
    capability: Capability,
    fingerprint: Option<RuntimeFingerprint>,
    lab_override: Option<LabOverride>,
) -> IpcResult<ProfileAuthorizationOutcome> {
    let fp = fingerprint.unwrap_or_default();
    let outcome = HardwareProfileRegistry::builtin()
        .authorize(&model_id, capability, &fp, lab_override.as_ref())
        .map_err(|e| PanadigaIpcError::validation(e.to_string()))?;
    match outcome {
        Ok(auth) => Ok(ProfileAuthorizationOutcome {
            allowed: true,
            authorization: Some(auth),
            denial: None,
        }),
        Err(denial) => Ok(ProfileAuthorizationOutcome {
            allowed: false,
            authorization: None,
            denial: Some(denial),
        }),
    }
}

#[tauri::command]
pub fn fingerprint_nor_dump_command(file_path: String) -> IpcResult<NorFingerprintReport> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err(PanadigaIpcError::validation("file_path cannot be empty"));
    }
    let bytes = fs::read(Path::new(trimmed))
        .map_err(|e| PanadigaIpcError::io(format!("Failed reading '{trimmed}': {e}")))?;
    Ok(fingerprint_nor_or_firmware(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_profile_ipc_lists_validates_resolves_and_authorizes() {
        let rep = validate_hardware_profiles_command().unwrap();
        assert!(rep.valid);
        assert_eq!(rep.model_count, 89);

        let list = list_hardware_profiles_command().unwrap();
        assert_eq!(list.len(), 89);

        let eh55 = get_resolved_model_profile_command("DMR-EH55".into()).unwrap();
        assert_eq!(
            eh55.flash_chip.as_ref().unwrap().part_number,
            "S29JL064H70TFI00"
        );
        assert_eq!(eh55.nvram_uncached_va_hex.as_deref(), Some("0x847F0000"));

        let eh55_auth = authorize_profile_capability_command(
            "DMR-EH55".into(),
            Capability::NvramSanitizeOfflineCow,
            Some(RuntimeFingerprint {
                dump_size_bytes: Some(8_388_608),
                has_eemul_f1_marker: true,
                ..Default::default()
            }),
            None,
        )
        .unwrap();
        assert!(eh55_auth.allowed);

        let ez485v_auth = authorize_profile_capability_command(
            "DMR-EZ485V".into(),
            Capability::NvramSanitizeOfflineCow,
            None,
            None,
        )
        .unwrap();
        assert!(!ez485v_auth.allowed);

        assert!(get_resolved_model_profile_command("UNKNOWN_MODEL".into()).is_err());
    }
}
