#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::Path;

use panadiga_bench::stream::{IncompleteOpNotice, JournalAlertKind};
use serde_json::value::RawValue;

use crate::lock::sync_parent_dir;

use super::types::{JournalBody, JournalEnvelope, JournalError, JournalEvent};
use super::{
    GENESIS_PREV_HASH, JOURNAL_FILENAME, JOURNAL_SCHEMA_VERSION, compute_journal_entry_hash,
};

pub(crate) struct ScanState {
    pub(crate) seq: u64,
    pub(crate) head_hash: String,
    pub(crate) valid_offset: u64,
    pub(crate) open_ops: BTreeMap<String, IncompleteOpNotice>,
    pub(crate) truncated_tail: Option<(u64, Vec<u8>)>,
}

fn decode_replay_line(buf: &[u8]) -> Option<(JournalEnvelope<Box<RawValue>>, JournalBody)> {
    if !buf.ends_with(b"\n") {
        return None;
    }
    let trimmed = std::str::from_utf8(buf)
        .ok()?
        .trim_end_matches(['\n', '\r']);
    let env: JournalEnvelope<Box<RawValue>> = serde_json::from_str(trimmed).ok()?;
    let body: JournalBody = serde_json::from_str(env.body.get()).ok()?;
    Some((env, body))
}

pub(crate) fn scan_journal_lines(file: &File, file_len: u64) -> Result<ScanState, JournalError> {
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(0))?;

    let mut state = ScanState {
        seq: 0,
        head_hash: GENESIS_PREV_HASH.to_string(),
        valid_offset: 0,
        open_ops: BTreeMap::new(),
        truncated_tail: None,
    };

    let mut line_num: usize = 0;
    let mut line_start: u64 = 0;
    let mut buf = Vec::new();

    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        line_num += 1;
        let expected_seq = state.seq.saturating_add(1);
        let is_eof_line = line_start + (n as u64) >= file_len;
        let Some((env, body)) = decode_replay_line(&buf) else {
            if is_eof_line {
                state.truncated_tail = Some((line_start, buf.clone()));
                break;
            }
            return Err(JournalError::ChainCorrupted {
                line: line_num,
                expected_seq,
            });
        };
        let body_raw_bytes = env.body.get().as_bytes();

        let recomputed = compute_journal_entry_hash(env.v, env.seq, &env.prev_hash, body_raw_bytes);
        if env.v != JOURNAL_SCHEMA_VERSION
            || env.seq != expected_seq
            || env.prev_hash != state.head_hash
            || env.entry_hash != recomputed
        {
            return Err(JournalError::ChainCorrupted {
                line: line_num,
                expected_seq,
            });
        }

        match &body.event {
            JournalEvent::OpBegin {
                op_kind, target, ..
            } => {
                state.open_ops.insert(
                    body.op_id.clone(),
                    IncompleteOpNotice {
                        op_id: body.op_id.clone(),
                        op_kind: *op_kind,
                        target: target.clone(),
                        started_ts_ms: body.ts_ms,
                    },
                );
            }
            JournalEvent::OpCheckpoint { .. } => {}
            JournalEvent::OpEnd { .. } => {
                state.open_ops.remove(&body.op_id);
            }
        }

        state.seq = env.seq;
        state.head_hash = env.entry_hash;
        line_start += n as u64;
        state.valid_offset = line_start;
    }

    Ok(state)
}

pub(crate) fn quarantine_truncated_tail(
    file: &mut File,
    journal_dir: &Path,
    truncate_at: u64,
    corrupt_bytes: &[u8],
    now_ms: u64,
) -> Result<JournalAlertKind, JournalError> {
    let backup_name = format!("{JOURNAL_FILENAME}.corrupt.{now_ms}");
    let backup_path = journal_dir.join(&backup_name);
    let mut bkp = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&backup_path)?;
    bkp.write_all(corrupt_bytes)?;
    bkp.sync_all()?;
    sync_parent_dir(&backup_path)?;

    file.set_len(truncate_at)?;
    file.sync_all()?;
    Ok(JournalAlertKind::TruncatedTailQuarantined {
        backup_file: backup_name,
        bytes: corrupt_bytes.len(),
    })
}
