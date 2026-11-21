#![deny(unsafe_code)]

//! Tauri IPC commands for Strict Segregated Forensic Vault Sessions (`raw/`, `frm/`, `recompiled/`)
//! and mandatory `Mn103Vm` pre-write gate verification.

use std::fs;
use std::path::{Path, PathBuf};

use panadiga_core::constants::{
    AM34_FLASH_BASE, AM34_FLASH_SIZE, AM34_NVRAM_BASE, AM34_NVRAM_SIZE, resolve_writable_data_root,
};
use panadiga_core::re::Prog061BgoPatchMode;
use panadiga_core::re::prewrite_gate::{PreWriteGateReport, PreWriteTarget, verify_prewrite_gate};
use panadiga_core::re::unbrick_sd_kit::{SdLoaderProfile, build_cleanroom_digadiag};
use panadiga_core::vault::{
    StrictWorkflowStep, VaultArtifactRecord, VaultBucket, VaultSessionManifest,
    auto_classify_and_ingest_file, ingest_vault_artifact, init_vault_session, list_vault_sessions,
    load_vault_session, mark_workflow_step_complete, record_session_emu_gate,
    validate_safe_segment,
};
use serde::Deserialize;

use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::resolve_read_path;
use crate::commands::unbrick_dto::opt_trimmed;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultInitSessionRequestDto {
    pub model_id: String,
    pub loader_profile: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultIngestFileRequestDto {
    pub session_id: String,
    pub source_path: String,
    pub bucket_override: Option<String>,
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPreWriteEmuGateRequestDto {
    pub session_id: Option<String>,
    pub target_kind: String,
    pub loader_profile: Option<String>,
    pub artifact_path: Option<String>,
    pub total_sectors: Option<u64>,
    pub expect_passthrough: Option<bool>,
    pub expect_capacity_unlock: Option<bool>,
    pub allow_upper_nor: Option<bool>,
}

/// Validates `session_id` and returns `<base_root>/sessions/<clean_id>`.
pub(crate) fn resolve_vault_session_dir_in(
    base_root: &Path,
    session_id: &str,
) -> IpcResult<PathBuf> {
    let clean = validate_safe_segment(session_id, "session_id")
        .map_err(|e| PanadigaIpcError::validation(e.to_string()))?;
    Ok(base_root.join("sessions").join(clean))
}

/// Shared helper persisting `emu_gate`, optional artifact, and workflow step into an active vault session.
pub(crate) fn record_vault_outcome_in(
    base_root: &Path,
    session_id_opt: Option<&str>,
    emu_gate: &PreWriteGateReport,
    artifact: Option<(&[u8], &str, VaultBucket, &str)>,
    completed_steps: &[StrictWorkflowStep],
) -> IpcResult<()> {
    let Some(raw_sid) = session_id_opt.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let sdir = resolve_vault_session_dir_in(base_root, raw_sid)?;
    if !sdir.join("session_manifest.json").is_file() {
        return Ok(());
    }
    record_session_emu_gate(&sdir, emu_gate).map_err(|e| PanadigaIpcError::io(e.to_string()))?;
    if let Some((bytes, filename, bucket, origin)) = artifact {
        let gate_opt = emu_gate.passed.then_some(emu_gate);
        ingest_vault_artifact(&sdir, bytes, filename, bucket, origin, gate_opt)
            .map_err(|e| PanadigaIpcError::io(e.to_string()))?;
    }
    for &step in completed_steps {
        mark_workflow_step_complete(&sdir, step)
            .map_err(|e| PanadigaIpcError::io(e.to_string()))?;
    }
    Ok(())
}

fn parse_bucket_opt(raw: Option<&str>) -> IpcResult<Option<VaultBucket>> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<VaultBucket>()
                .map_err(|e| PanadigaIpcError::validation(e.to_string()))
        })
        .transpose()
}

pub(crate) fn init_session_with_root(
    base_root: &Path,
    request: &VaultInitSessionRequestDto,
) -> IpcResult<VaultSessionManifest> {
    let profile: SdLoaderProfile = opt_trimmed(&request.loader_profile)
        .unwrap_or("ver061")
        .parse()
        .map_err(|e: anyhow::Error| PanadigaIpcError::validation(e.to_string()))?;
    init_vault_session(
        base_root,
        &request.model_id,
        profile,
        opt_trimmed(&request.session_id),
    )
    .map_err(|e| PanadigaIpcError::validation(e.to_string()))
}

pub(crate) fn ingest_file_with_root(
    base_root: &Path,
    request: &VaultIngestFileRequestDto,
) -> IpcResult<VaultArtifactRecord> {
    let sdir = resolve_vault_session_dir_in(base_root, &request.session_id)?;
    let src = resolve_read_path(request.source_path.trim());
    let bucket = parse_bucket_opt(opt_trimmed(&request.bucket_override))?;
    let origin = opt_trimmed(&request.origin).unwrap_or("desktop_manual_ingest");
    auto_classify_and_ingest_file(&sdir, &src, bucket, origin, None)
        .map_err(|e| PanadigaIpcError::validation(e.to_string()))
}

pub(crate) fn run_prewrite_emu_gate_with_root(
    base_root: &Path,
    request: &RunPreWriteEmuGateRequestDto,
) -> IpcResult<PreWriteGateReport> {
    let profile: SdLoaderProfile = opt_trimmed(&request.loader_profile)
        .unwrap_or("ver061")
        .parse()
        .map_err(|e: anyhow::Error| PanadigaIpcError::validation(e.to_string()))?;
    let kind = request.target_kind.trim().to_ascii_lowercase();
    let artifact_bytes = opt_trimmed(&request.artifact_path)
        .map(|p| {
            let path = resolve_read_path(p);
            fs::read(&path).map_err(|e| {
                PanadigaIpcError::io(format!("Falha lendo artefato '{}': {e}", path.display()))
            })
        })
        .transpose()?;

    let report = match kind.as_str() {
        "cleanroom_digadiag" => {
            let bytes = match artifact_bytes {
                Some(b) => b,
                None => {
                    build_cleanroom_digadiag(profile)
                        .map_err(|e| PanadigaIpcError::internal(e.to_string()))?
                        .bytes
                }
            };
            verify_prewrite_gate(PreWriteTarget::CleanroomDigadiag {
                bytes: &bytes,
                loader_profile: profile,
            })
        }
        "composed_hdd_stream" => verify_prewrite_gate(PreWriteTarget::ComposedHddStream {
            total_sectors: request.total_sectors.unwrap_or(488_397_168),
            main_extrhead: artifact_bytes.as_deref(),
            drv_extrhead: None,
        }),
        "patched_prog061" => {
            let bytes = artifact_bytes.as_deref().ok_or_else(|| {
                PanadigaIpcError::validation("artifactPath obrigatório para patched_prog061.")
            })?;
            verify_prewrite_gate(PreWriteTarget::PatchedProg061 {
                bytes,
                mode: Prog061BgoPatchMode::UartMonitor,
            })
        }
        "patched_main" => {
            let bytes = artifact_bytes.as_deref().ok_or_else(|| {
                PanadigaIpcError::validation("artifactPath obrigatório para patched_main.")
            })?;
            verify_prewrite_gate(PreWriteTarget::PatchedMain {
                raw_main: bytes,
                expect_passthrough: request.expect_passthrough.unwrap_or(false),
                expect_capacity_unlock: request.expect_capacity_unlock.unwrap_or(false),
            })
        }
        "nor_cow_repair" => {
            let orig = artifact_bytes.unwrap_or_else(|| vec![0xFFu8; AM34_FLASH_SIZE as usize]);
            let mut repaired = orig.clone();
            let nvram_off = (AM34_NVRAM_BASE - AM34_FLASH_BASE) as usize;
            let nvram_end = nvram_off.saturating_add(AM34_NVRAM_SIZE as usize);
            if nvram_end <= repaired.len() {
                repaired[nvram_off..nvram_end].fill(0xFF);
            }
            verify_prewrite_gate(PreWriteTarget::NorCowRepair {
                original_nor: &orig,
                repaired_nor: &repaired,
                allow_upper_nor: request.allow_upper_nor.unwrap_or(false),
            })
        }
        other => {
            return Err(PanadigaIpcError::validation(format!(
                "targetKind desconhecido '{other}'."
            )));
        }
    }
    .map_err(|e| PanadigaIpcError::classify(format!("Falha Mn103Vm gate: {e}")))?;

    record_vault_outcome_in(base_root, request.session_id.as_deref(), &report, None, &[])?;
    Ok(report)
}

#[tauri::command]
pub fn vault_init_session_command(
    request: VaultInitSessionRequestDto,
) -> IpcResult<VaultSessionManifest> {
    init_session_with_root(&resolve_writable_data_root(), &request)
}

#[tauri::command]
pub fn vault_list_sessions_command() -> IpcResult<Vec<VaultSessionManifest>> {
    list_vault_sessions(&resolve_writable_data_root())
        .map_err(|e| PanadigaIpcError::io(e.to_string()))
}

#[tauri::command]
pub fn vault_get_session_command(session_id: String) -> IpcResult<VaultSessionManifest> {
    let dir = resolve_vault_session_dir_in(&resolve_writable_data_root(), &session_id)?;
    load_vault_session(&dir).map_err(|e| PanadigaIpcError::io(e.to_string()))
}

#[tauri::command]
pub fn vault_ingest_file_command(
    request: VaultIngestFileRequestDto,
) -> IpcResult<VaultArtifactRecord> {
    ingest_file_with_root(&resolve_writable_data_root(), &request)
}

#[tauri::command]
pub fn vault_mark_step_command(
    session_id: String,
    step: StrictWorkflowStep,
) -> IpcResult<VaultSessionManifest> {
    let dir = resolve_vault_session_dir_in(&resolve_writable_data_root(), &session_id)?;
    mark_workflow_step_complete(&dir, step).map_err(|e| PanadigaIpcError::io(e.to_string()))
}

#[tauri::command]
pub fn vault_run_prewrite_emu_gate_command(
    request: RunPreWriteEmuGateRequestDto,
) -> IpcResult<PreWriteGateReport> {
    run_prewrite_emu_gate_with_root(&resolve_writable_data_root(), &request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_ipc_init_segregates_raw_frm_and_persists_mn103vm_gate() {
        let root =
            std::env::temp_dir().join(format!("panadiga_tauri_vault_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let sess = init_session_with_root(
            &root,
            &VaultInitSessionRequestDto {
                model_id: "DMR-EH55".into(),
                loader_profile: Some("ver061".into()),
                session_id: Some("ipc_test_sess".into()),
            },
        )
        .unwrap();
        assert_eq!(sess.model_id, "DMR-EH55");

        let bin_p = root.join("nor_dump.bin");
        let frm_p = root.join("PANA_DVD.FRM");
        fs::write(&bin_p, vec![0xAAu8; 64]).unwrap();
        fs::write(&frm_p, b"PANA_DVD_SAMPLE").unwrap();

        let rec_raw = ingest_file_with_root(
            &root,
            &VaultIngestFileRequestDto {
                session_id: "ipc_test_sess".into(),
                source_path: bin_p.to_string_lossy().into_owned(),
                bucket_override: None,
                origin: None,
            },
        )
        .unwrap();
        assert_eq!(rec_raw.bucket, VaultBucket::Raw);

        let rec_frm = ingest_file_with_root(
            &root,
            &VaultIngestFileRequestDto {
                session_id: "ipc_test_sess".into(),
                source_path: frm_p.to_string_lossy().into_owned(),
                bucket_override: None,
                origin: None,
            },
        )
        .unwrap();
        assert_eq!(rec_frm.bucket, VaultBucket::Frm);

        // Cross-bucket .FRM -> raw rejection
        assert!(
            ingest_file_with_root(
                &root,
                &VaultIngestFileRequestDto {
                    session_id: "ipc_test_sess".into(),
                    source_path: frm_p.to_string_lossy().into_owned(),
                    bucket_override: Some("raw".into()),
                    origin: None,
                },
            )
            .is_err()
        );

        let gate = run_prewrite_emu_gate_with_root(
            &root,
            &RunPreWriteEmuGateRequestDto {
                session_id: Some("ipc_test_sess".into()),
                target_kind: "cleanroom_digadiag".into(),
                loader_profile: Some("ver061".into()),
                artifact_path: None,
                total_sectors: None,
                expect_passthrough: None,
                expect_capacity_unlock: None,
                allow_upper_nor: None,
            },
        )
        .unwrap();
        assert!(gate.passed);
        assert!(gate.ipl_untouched);
        assert!(gate.lethal_mmio_blocked);

        let loaded = load_vault_session(Path::new(&sess.session_dir)).unwrap();
        assert!(loaded.last_emu_gate.is_some());
        let _ = fs::remove_dir_all(&root);
    }
}
