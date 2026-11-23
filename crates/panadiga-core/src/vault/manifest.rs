#![deny(unsafe_code)]

//! Session manifest initialization, listing, persistence, and strict 6-step progression.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::re::prewrite_gate::PreWriteGateReport;
use crate::re::unbrick_sd_kit::SdLoaderProfile;

use super::layout::{session_bucket_dir, validate_safe_segment};
use super::{StrictWorkflowStep, VaultBucket, VaultSessionManifest};

const SESSION_MANIFEST_FILENAME: &str = "session_manifest.json";

pub(crate) fn epoch_secs_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn sessions_root(base_root: &Path) -> PathBuf {
    base_root.join("sessions")
}

/// Initializes a strict segregated vault session under `<base_root>/sessions/<session_id>/{raw,frm,recompiled}`.
pub fn init_vault_session(
    base_root: &Path,
    model_id: &str,
    loader_profile: SdLoaderProfile,
    session_id_opt: Option<&str>,
) -> Result<VaultSessionManifest> {
    let ts = epoch_secs_now();
    let clean_model = model_id.trim();
    if clean_model.is_empty() {
        bail!("model_id cannot be empty when initializing vault session");
    }
    let session_id = match session_id_opt {
        Some(raw) if !raw.trim().is_empty() => validate_safe_segment(raw, "session_id")?,
        _ => format!(
            "sess_{}_{}_{}",
            clean_model
                .to_ascii_lowercase()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect::<String>(),
            ts,
            std::process::id()
        ),
    };

    let session_dir = sessions_root(base_root).join(&session_id);
    for bucket in [VaultBucket::Raw, VaultBucket::Frm, VaultBucket::Recompiled] {
        let _ = session_bucket_dir(&session_dir, bucket)?;
    }

    let manifest_path = session_dir.join(SESSION_MANIFEST_FILENAME);
    if manifest_path.is_file() {
        return load_vault_session(&session_dir);
    }

    let manifest = VaultSessionManifest {
        session_id,
        model_id: clean_model.to_string(),
        loader_profile,
        created_at_epoch_s: ts,
        session_dir: session_dir.display().to_string(),
        completed_steps: vec![StrictWorkflowStep::SelectModel],
        current_step: StrictWorkflowStep::BuildCleanroomSd,
        raw_artifacts: Vec::new(),
        frm_artifacts: Vec::new(),
        recompiled_artifacts: Vec::new(),
        last_emu_gate: None,
    };
    save_vault_session(&session_dir, &manifest)?;
    Ok(manifest)
}

/// Loads `session_manifest.json` from `<session_dir>`.
pub fn load_vault_session(session_dir: &Path) -> Result<VaultSessionManifest> {
    let path = session_dir.join(SESSION_MANIFEST_FILENAME);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Reading vault session manifest {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("Parsing vault session manifest {}", path.display()))
}

/// Persists `manifest` atomically into `<session_dir>/session_manifest.json`.
pub fn save_vault_session(session_dir: &Path, manifest: &VaultSessionManifest) -> Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static MANIFEST_WRITE_SEQ: AtomicU64 = AtomicU64::new(0);

    fs::create_dir_all(session_dir)?;
    let path = session_dir.join(SESSION_MANIFEST_FILENAME);
    let seq = MANIFEST_WRITE_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp_path = session_dir.join(format!(
        ".session_manifest.{}.{}.tmp",
        std::process::id(),
        seq
    ));
    let json = serde_json::to_string_pretty(manifest)?;
    {
        use std::io::Write;
        let mut tmp_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .with_context(|| {
                format!("Opening temp vault session manifest {}", tmp_path.display())
            })?;
        tmp_file.write_all(json.as_bytes()).with_context(|| {
            format!("Writing temp vault session manifest {}", tmp_path.display())
        })?;
        tmp_file.sync_all().with_context(|| {
            format!("Syncing temp vault session manifest {}", tmp_path.display())
        })?;
    }
    if let Err(err) = fs::rename(&tmp_path, &path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err).with_context(|| {
            format!(
                "Atomically replacing vault session manifest {}",
                path.display()
            )
        });
    }
    let _ = super::lock::sync_parent_dir(&path);
    Ok(())
}

/// Marks a step complete in the strict 6-step workflow and advances `current_step`.
pub fn mark_workflow_step_complete(
    session_dir: &Path,
    step: StrictWorkflowStep,
) -> Result<VaultSessionManifest> {
    let mut manifest = load_vault_session(session_dir)?;
    if !manifest.completed_steps.contains(&step) {
        manifest.completed_steps.push(step);
    }
    let candidate = step.next_step();
    if manifest.current_step < candidate {
        manifest.current_step = candidate;
    }
    save_vault_session(session_dir, &manifest)?;
    Ok(manifest)
}

/// Records the latest `Mn103Vm` pre-write gate result inside `session_manifest.json` and `recompiled/`.
pub fn record_session_emu_gate(
    session_dir: &Path,
    gate: &PreWriteGateReport,
) -> Result<VaultSessionManifest> {
    let mut manifest = load_vault_session(session_dir)?;
    let recompiled_dir = session_bucket_dir(session_dir, VaultBucket::Recompiled)?;
    fs::write(
        recompiled_dir.join("emu_gate_report.json"),
        serde_json::to_string_pretty(gate)?,
    )?;
    manifest.last_emu_gate = Some(gate.clone());
    save_vault_session(session_dir, &manifest)?;
    Ok(manifest)
}

/// Lists all valid vault sessions beneath `<base_root>/sessions/`, sorted newest first.
pub fn list_vault_sessions(base_root: &Path) -> Result<Vec<VaultSessionManifest>> {
    let root = sessions_root(base_root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&root)? {
        let path = entry?.path();
        if path.is_dir()
            && path.join(SESSION_MANIFEST_FILENAME).is_file()
            && let Ok(m) = load_vault_session(&path)
        {
            out.push(m);
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.created_at_epoch_s));
    Ok(out)
}
