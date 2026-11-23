#![deny(unsafe_code)]

//! Append-only chained SHA-256 Operation Journal (`vault_dir/journal/ops.jsonl`) with
//! write-ahead `fsync`, advisory locking, EOF truncated-tail forensic quarantine, and crash notice recovery.

pub mod replay;
pub mod types;
pub mod writer;

#[cfg(test)]
mod tests;

use sha2::{Digest, Sha256};

pub use types::{
    JournalActor, JournalBody, JournalEnvelope, JournalError, JournalEvent,
    JournalVerificationReport, OpBeginParams, OpOutcome,
};
pub use writer::Journal;

pub const JOURNAL_SUBDIR: &str = "journal";
pub const JOURNAL_FILENAME: &str = "ops.jsonl";
pub const JOURNAL_SCHEMA_VERSION: u8 = 1;
pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Computes `SHA256("{v}|{seq}|{prev_hash}|" || exact_utf8_bytes_of_body_json)` in lowercase hex.
#[must_use]
pub fn compute_journal_entry_hash(v: u8, seq: u64, prev_hash: &str, body_bytes: &[u8]) -> String {
    let prefix = format!("{v}|{seq}|{prev_hash}|");
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(body_bytes);
    hex::encode(hasher.finalize())
}
