#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use panadiga_bench::stream::{IncompleteOpNotice, JournalAlertKind};
use serde_json::value::RawValue;

use crate::lock::{VaultLock, VaultLockMode, sync_parent_dir};

use super::replay::{quarantine_truncated_tail, scan_journal_lines};
use super::types::{
    JournalActor, JournalBody, JournalEnvelope, JournalError, JournalEvent,
    JournalVerificationReport, OpBeginParams, OpOutcome,
};
use super::{
    GENESIS_PREV_HASH, JOURNAL_FILENAME, JOURNAL_SCHEMA_VERSION, JOURNAL_SUBDIR,
    compute_journal_entry_hash,
};

/// Exclusive write handle to `<vault_dir>/journal/ops.jsonl` guarded by `.vault.lock`.
#[derive(Debug)]
pub struct Journal {
    file: File,
    actor: JournalActor,
    seq: u64,
    head_hash: String,
    open_ops: BTreeMap<String, IncompleteOpNotice>,
    alerts: Vec<JournalAlertKind>,
    _lock: VaultLock,
}

fn journal_file_path(vault_dir: &Path) -> PathBuf {
    vault_dir.join(JOURNAL_SUBDIR).join(JOURNAL_FILENAME)
}

impl Journal {
    /// Opens `<vault_dir>/journal/ops.jsonl` exclusively, verifies the chain,
    /// and forensically quarantines a crash-truncated EOF line if present.
    pub fn open(vault_dir: &Path, actor: JournalActor, now_ms: u64) -> Result<Self, JournalError> {
        let actor_str = actor.to_string();
        let lock = VaultLock::try_acquire(vault_dir, VaultLockMode::Exclusive, &actor_str)?;
        let journal_dir = vault_dir.join(JOURNAL_SUBDIR);
        let ops_path = journal_file_path(vault_dir);
        let existed = ops_path.is_file();
        std::fs::create_dir_all(&journal_dir)?;

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&ops_path)?;
        if !existed {
            file.sync_all()?;
            sync_parent_dir(&ops_path)?;
        }

        let file_len = file.metadata()?.len();
        let state = scan_journal_lines(&file, file_len)?;
        let mut alerts = Vec::new();

        if let Some((truncate_at, corrupt_bytes)) = state.truncated_tail {
            alerts.push(quarantine_truncated_tail(
                &mut file,
                &journal_dir,
                truncate_at,
                &corrupt_bytes,
                now_ms,
            )?);
        }

        file.seek(SeekFrom::Start(state.valid_offset))?;
        for notice in state.open_ops.values() {
            alerts.push(JournalAlertKind::IncompleteOpDetected(notice.clone()));
        }

        Ok(Self {
            file,
            actor,
            seq: state.seq,
            head_hash: state.head_hash,
            open_ops: state.open_ops,
            alerts,
            _lock: lock,
        })
    }

    /// Reads and verifies `ops.jsonl` under a shared advisory lock without modifying the file.
    pub fn verify_readonly(vault_dir: &Path) -> Result<JournalVerificationReport, JournalError> {
        let _lock = VaultLock::try_acquire(vault_dir, VaultLockMode::Shared, "verifier")?;
        let ops_path = journal_file_path(vault_dir);
        if !ops_path.is_file() {
            return Ok(JournalVerificationReport {
                valid: true,
                seq: 0,
                head_hash: GENESIS_PREV_HASH.to_string(),
                incomplete_ops: Vec::new(),
                truncated_tail_bytes: None,
            });
        }
        let file = OpenOptions::new().read(true).open(&ops_path)?;
        let file_len = file.metadata()?.len();
        let state = scan_journal_lines(&file, file_len)?;
        let truncated_tail_bytes = state.truncated_tail.as_ref().map(|(_, b)| b.len());
        Ok(JournalVerificationReport {
            valid: truncated_tail_bytes.is_none(),
            seq: state.seq,
            head_hash: state.head_hash,
            incomplete_ops: state.open_ops.into_values().collect(),
            truncated_tail_bytes,
        })
    }

    /// Returns forensic alerts discovered while opening this journal.
    #[must_use]
    pub fn alerts(&self) -> &[JournalAlertKind] {
        &self.alerts
    }

    /// Returns any unclosed operations (`OpBegin` without `OpEnd`) currently blocking new writes.
    #[must_use]
    pub fn incomplete_ops(&self) -> Vec<IncompleteOpNotice> {
        self.open_ops.values().cloned().collect()
    }

    /// Current highest verified sequence number (`0` when empty).
    #[must_use]
    pub const fn seq(&self) -> u64 {
        self.seq
    }

    /// Current chain head SHA-256 hex digest (`GENESIS_PREV_HASH` when empty).
    #[must_use]
    pub fn head_hash(&self) -> &str {
        &self.head_hash
    }

    fn write_entry(
        &mut self,
        ts_ms: u64,
        op_id: String,
        contract_hash: String,
        event: JournalEvent,
        sync_durable: bool,
    ) -> Result<u64, JournalError> {
        let next_seq = self.seq.saturating_add(1);
        let body = JournalBody {
            ts_ms,
            op_id,
            contract_hash,
            actor: self.actor,
            event,
        };
        let body_json = serde_json::to_string(&body)?;
        let entry_hash = compute_journal_entry_hash(
            JOURNAL_SCHEMA_VERSION,
            next_seq,
            &self.head_hash,
            body_json.as_bytes(),
        );
        let raw_body = RawValue::from_string(body_json)?;
        let env = JournalEnvelope {
            v: JOURNAL_SCHEMA_VERSION,
            seq: next_seq,
            prev_hash: self.head_hash.clone(),
            entry_hash: entry_hash.clone(),
            body: raw_body.as_ref(),
        };
        let mut line = serde_json::to_string(&env)?;
        line.push('\n');
        self.file.write_all(line.as_bytes())?;
        self.file.flush()?;
        if sync_durable {
            self.file.sync_data()?;
        }
        self.seq = next_seq;
        self.head_hash = entry_hash;
        Ok(next_seq)
    }

    /// Records `OpBegin` intent durably (`sync_data()`) before physical I/O begins.
    /// Refuses while an unclosed operation remains open (`IncompleteOpBlocking`).
    pub fn append_op_begin(&mut self, params: OpBeginParams) -> Result<u64, JournalError> {
        if let Some(existing) = self.open_ops.values().next() {
            return Err(JournalError::IncompleteOpBlocking {
                op_id: existing.op_id.clone(),
                op_kind: existing.op_kind,
                target: existing.target.clone(),
            });
        }
        let op_id = params.op_id.clone();
        let notice = IncompleteOpNotice {
            op_id: op_id.clone(),
            op_kind: params.op_kind,
            target: params.target.clone(),
            started_ts_ms: params.ts_ms,
        };
        let seq = self.write_entry(
            params.ts_ms,
            op_id.clone(),
            params.contract_hash,
            JournalEvent::OpBegin {
                op_kind: params.op_kind,
                target: params.target,
                input_sha256: params.input_sha256,
                nonce_id: params.nonce_id,
                vdd_mv: params.vdd_mv,
                nsw5v_high: params.nsw5v_high,
                meta: params.meta,
            },
            true,
        )?;
        self.open_ops.insert(op_id, notice);
        Ok(seq)
    }

    /// Records an intermediate checkpoint (durable flush when `sync_durable` is set).
    #[allow(clippy::too_many_arguments)]
    pub fn append_checkpoint(
        &mut self,
        op_id: &str,
        contract_hash: &str,
        phase: &str,
        done_units: u64,
        total_units: u64,
        sync_durable: bool,
        ts_ms: u64,
    ) -> Result<u64, JournalError> {
        if !self.open_ops.contains_key(op_id) {
            return Err(JournalError::UnknownOpId(op_id.to_string()));
        }
        self.write_entry(
            ts_ms,
            op_id.to_string(),
            contract_hash.to_string(),
            JournalEvent::OpCheckpoint {
                phase: phase.to_string(),
                done_units,
                total_units,
            },
            sync_durable,
        )
    }

    /// Records `OpEnd` durably (`sync_data()`) and closes the active `op_id`.
    #[allow(clippy::too_many_arguments)]
    pub fn append_op_end(
        &mut self,
        op_id: &str,
        contract_hash: &str,
        outcome: OpOutcome,
        duration_ms: u64,
        artifact_sha256: Option<String>,
        error_code: Option<String>,
        detail: Option<String>,
        ts_ms: u64,
    ) -> Result<u64, JournalError> {
        if !self.open_ops.contains_key(op_id) {
            return Err(JournalError::UnknownOpId(op_id.to_string()));
        }
        let seq = self.write_entry(
            ts_ms,
            op_id.to_string(),
            contract_hash.to_string(),
            JournalEvent::OpEnd {
                outcome,
                duration_ms,
                artifact_sha256,
                error_code,
                detail,
            },
            true,
        )?;
        self.open_ops.remove(op_id);
        Ok(seq)
    }

    /// Explicitly closes a crash-interrupted operation after operator audit (`RecoveredAfterCrash`).
    pub fn acknowledge_incomplete_op(
        &mut self,
        op_id: &str,
        contract_hash: &str,
        detail: &str,
        ts_ms: u64,
    ) -> Result<u64, JournalError> {
        let notice = self
            .open_ops
            .get(op_id)
            .cloned()
            .ok_or_else(|| JournalError::UnknownOpId(op_id.to_string()))?;
        let duration_ms = ts_ms.saturating_sub(notice.started_ts_ms);
        self.append_op_end(
            op_id,
            contract_hash,
            OpOutcome::RecoveredAfterCrash,
            duration_ms,
            None,
            Some("RECOVERED_AFTER_CRASH".to_string()),
            Some(detail.to_string()),
            ts_ms,
        )
    }
}
