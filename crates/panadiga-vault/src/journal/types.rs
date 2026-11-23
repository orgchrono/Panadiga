#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use panadiga_bench::stream::{IncompleteOpNotice, OpKind};
use serde::{Deserialize, Serialize};

/// Process actor recording an operation into `ops.jsonl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalActor {
    Desktop,
    Cli,
    Mcp,
}

impl fmt::Display for JournalActor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Desktop => "desktop",
            Self::Cli => "cli",
            Self::Mcp => "mcp",
        };
        f.write_str(label)
    }
}

/// Chained SHA-256 JSONL envelope written to `ops.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JournalEnvelope<B = JournalBody> {
    pub v: u8,
    pub seq: u64,
    pub prev_hash: String,
    pub entry_hash: String,
    pub body: B,
}

/// Canonical operation record hashed verbatim before writing `JournalEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JournalBody {
    pub ts_ms: u64,
    pub op_id: String,
    pub contract_hash: String,
    pub actor: JournalActor,
    pub event: JournalEvent,
}

/// Intent/progress/completion event carried inside `JournalBody`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JournalEvent {
    OpBegin {
        op_kind: OpKind,
        target: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input_sha256: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        nonce_id: Option<String>,
        vdd_mv: Option<u16>,
        nsw5v_high: Option<bool>,
        /// Extra ordered metadata (`BTreeMap` preserves canonical key order).
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        meta: BTreeMap<String, String>,
    },
    OpCheckpoint {
        phase: String,
        done_units: u64,
        total_units: u64,
    },
    OpEnd {
        outcome: OpOutcome,
        duration_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        artifact_sha256: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

/// Final disposition recorded by `JournalEvent::OpEnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpOutcome {
    Ok,
    Err,
    Aborted,
    RecoveredAfterCrash,
}

/// Read-only verification summary over an existing `ops.jsonl` chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JournalVerificationReport {
    pub valid: bool,
    pub seq: u64,
    pub head_hash: String,
    pub incomplete_ops: Vec<IncompleteOpNotice>,
    pub truncated_tail_bytes: Option<usize>,
}

/// Optional fields attached when appending `JournalEvent::OpBegin`.
#[derive(Debug, Clone, Default)]
pub struct OpBeginParams {
    pub op_id: String,
    pub contract_hash: String,
    pub op_kind: OpKind,
    pub target: String,
    pub input_sha256: Option<String>,
    pub nonce_id: Option<String>,
    pub vdd_mv: Option<u16>,
    pub nsw5v_high: Option<bool>,
    pub meta: BTreeMap<String, String>,
    pub ts_ms: u64,
}

/// Typed errors returned by Vault locking and journal verification/appends.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("Vault locked by another active instance (pid={pid:?}, actor={actor:?})")]
    VaultLocked {
        pid: Option<u32>,
        actor: Option<String>,
    },
    #[error("Journal hash chain corrupted at line {line} (expected seq {expected_seq})")]
    ChainCorrupted { line: usize, expected_seq: u64 },
    #[error(
        "Unclosed operation '{op_id}' ({op_kind} on '{target}') blocks new destructive actions until acknowledged"
    )]
    IncompleteOpBlocking {
        op_id: String,
        op_kind: OpKind,
        target: String,
    },
    #[error("No open operation found with op_id '{0}'")]
    UnknownOpId(String),
    #[error("I/O error in vault journal: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error in vault journal: {0}")]
    Json(#[from] serde_json::Error),
}
