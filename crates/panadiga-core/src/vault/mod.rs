#![deny(unsafe_code)]

//! Strict Forensic Session Vault segregating immutable `raw/` dumps, `.FRM` containers (`frm/`),
//! and `Mn103Vm`-verified recompilations (`recompiled/`).

pub use panadiga_vault::{journal, lock};

pub mod layout;
pub mod manifest;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::re::prewrite_gate::PreWriteGateReport;
use crate::re::unbrick_sd_kit::SdLoaderProfile;

pub use journal::{
    GENESIS_PREV_HASH, JOURNAL_FILENAME, JOURNAL_SCHEMA_VERSION, JOURNAL_SUBDIR, Journal,
    JournalActor, JournalBody, JournalEnvelope, JournalError, JournalEvent,
    JournalVerificationReport, OpBeginParams, OpOutcome, compute_journal_entry_hash,
};
pub use layout::{
    auto_classify_and_ingest_file, extract_nor_drv_into_vault, ingest_vault_artifact,
    session_bucket_dir, validate_safe_segment,
};
pub use lock::{VAULT_LOCK_FILENAME, VaultLock, VaultLockMode};
pub use manifest::{
    init_vault_session, list_vault_sessions, load_vault_session, mark_workflow_step_complete,
    record_session_emu_gate, save_vault_session,
};

/// Isolated storage bucket inside a forensic session directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultBucket {
    Raw,
    Frm,
    Recompiled,
}

impl VaultBucket {
    /// Directory name for this bucket (`raw`, `frm`, `recompiled`).
    #[must_use]
    pub const fn dir_name(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Frm => "frm",
            Self::Recompiled => "recompiled",
        }
    }

    /// Per-bucket SHA-256 manifest filename.
    #[must_use]
    pub const fn sums_filename(self) -> &'static str {
        match self {
            Self::Raw => "SHA256SUMS.raw.txt",
            Self::Frm => "SHA256SUMS.frm.txt",
            Self::Recompiled => "SHA256SUMS.recompiled.txt",
        }
    }
}

impl std::str::FromStr for VaultBucket {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> anyhow::Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "raw" => Ok(Self::Raw),
            "frm" => Ok(Self::Frm),
            "recompiled" => Ok(Self::Recompiled),
            other => {
                anyhow::bail!("Invalid vault bucket '{other}': expected raw, frm, or recompiled")
            }
        }
    }
}

/// Deterministic strict 6-step engineering workflow order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrictWorkflowStep {
    SelectModel,
    BuildCleanroomSd,
    SetupPicoProbe,
    CaptureAndBackup,
    OrganizeAndRecompile,
    EmulateAndFlash,
}

impl StrictWorkflowStep {
    /// Canonical ordered slice of all 6 workflow stages.
    pub const ALL: [Self; 6] = [
        Self::SelectModel,
        Self::BuildCleanroomSd,
        Self::SetupPicoProbe,
        Self::CaptureAndBackup,
        Self::OrganizeAndRecompile,
        Self::EmulateAndFlash,
    ];

    /// Computes next incomplete step after `self`.
    #[must_use]
    pub const fn next_step(self) -> Self {
        match self {
            Self::SelectModel => Self::BuildCleanroomSd,
            Self::BuildCleanroomSd => Self::SetupPicoProbe,
            Self::SetupPicoProbe => Self::CaptureAndBackup,
            Self::CaptureAndBackup => Self::OrganizeAndRecompile,
            Self::OrganizeAndRecompile | Self::EmulateAndFlash => Self::EmulateAndFlash,
        }
    }
}

/// Verified artifact entry tracked inside a [`VaultSessionManifest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultArtifactRecord {
    pub id: String,
    pub bucket: VaultBucket,
    pub filename: String,
    pub relative_path: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub origin: String,
    pub created_at_epoch_s: u64,
    pub emu_verified: bool,
}

/// Persistent manifest coordinating segregated backups and `Mn103Vm` pre-write gate approvals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultSessionManifest {
    pub session_id: String,
    pub model_id: String,
    pub loader_profile: SdLoaderProfile,
    pub created_at_epoch_s: u64,
    pub session_dir: String,
    pub completed_steps: Vec<StrictWorkflowStep>,
    pub current_step: StrictWorkflowStep,
    pub raw_artifacts: Vec<VaultArtifactRecord>,
    pub frm_artifacts: Vec<VaultArtifactRecord>,
    pub recompiled_artifacts: Vec<VaultArtifactRecord>,
    pub last_emu_gate: Option<PreWriteGateReport>,
}
