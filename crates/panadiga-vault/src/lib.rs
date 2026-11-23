#![deny(unsafe_code)]

//! Panadiga L1 Forensic Operation Journal (`ops.jsonl`) & Multi-Process Advisory Vault Locking.

pub mod journal;
pub mod lock;

pub use journal::{
    GENESIS_PREV_HASH, JOURNAL_FILENAME, JOURNAL_SCHEMA_VERSION, JOURNAL_SUBDIR, Journal,
    JournalActor, JournalBody, JournalEnvelope, JournalError, JournalEvent,
    JournalVerificationReport, OpBeginParams, OpOutcome, compute_journal_entry_hash,
};
pub use lock::{VAULT_LOCK_FILENAME, VaultLock, VaultLockMode};
