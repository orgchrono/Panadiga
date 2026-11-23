#![deny(unsafe_code)]

//! Single-use, short-lived confirmation nonces binding WebView intent to exact operation parameters.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::stream::OpKind;

/// Default TTL (`120 s`) for ephemeral WebView operation confirmation nonces.
pub const DEFAULT_NONCE_TTL_MS: u64 = 120_000;

static NONCE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Issued single-use operation challenge bound to `(op_kind, target, input_sha256)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OperationNonceChallenge {
    pub nonce_id: String,
    pub op_kind: OpKind,
    pub target: String,
    pub input_sha256: String,
    pub ssot_contract_hash: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

/// Thread-safe inner store tracking active non-consumed operation confirmation nonces.
#[derive(Debug, Default)]
pub struct OperationNonceStore {
    ssot_contract_hash: String,
    challenges: BTreeMap<String, OperationNonceChallenge>,
}

impl OperationNonceStore {
    /// Creates an empty nonce store (uses an empty hash unless configured via [`Self::with_contract_hash`]).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty nonce store pre-bound to an explicit `SSOT_CONTRACT_HASH`.
    #[must_use]
    pub fn with_contract_hash(ssot_contract_hash: impl Into<String>) -> Self {
        Self {
            ssot_contract_hash: ssot_contract_hash.into(),
            challenges: BTreeMap::new(),
        }
    }

    fn prune_expired(&mut self, now_ms: u64) {
        self.challenges.retain(|_, ch| ch.expires_at_ms >= now_ms);
    }

    /// Issues a new single-use challenge bound strictly to `(op_kind, target, input_sha256)`.
    pub fn issue(
        &mut self,
        op_kind: OpKind,
        target: impl Into<String>,
        input_sha256: impl Into<String>,
        now_ms: u64,
    ) -> OperationNonceChallenge {
        self.prune_expired(now_ms);
        let clean_target = target.into().trim().to_string();
        let clean_hash = input_sha256.into().trim().to_ascii_lowercase();
        let seq = NONCE_SEQ.fetch_add(1, Ordering::Relaxed);
        let contract_hash = self.ssot_contract_hash.clone();

        let mut hasher = Sha256::new();
        hasher.update(std::process::id().to_le_bytes());
        hasher.update(seq.to_le_bytes());
        hasher.update(now_ms.to_le_bytes());
        hasher.update(op_kind.to_string().as_bytes());
        hasher.update(b"|");
        hasher.update(clean_target.as_bytes());
        hasher.update(b"|");
        hasher.update(clean_hash.as_bytes());
        let digest = hex::encode(hasher.finalize());
        let nonce_id = format!("n_{}", &digest[..16]);

        let challenge = OperationNonceChallenge {
            nonce_id: nonce_id.clone(),
            op_kind,
            target: clean_target,
            input_sha256: clean_hash,
            ssot_contract_hash: contract_hash,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(DEFAULT_NONCE_TTL_MS),
        };
        self.challenges.insert(nonce_id, challenge.clone());
        challenge
    }

    /// Consumes a valid un-expired challenge matching `(op_kind, target, input_sha256)`.
    pub fn consume(
        &mut self,
        nonce_id: &str,
        op_kind: OpKind,
        target: &str,
        input_sha256: &str,
        now_ms: u64,
    ) -> Result<OperationNonceChallenge> {
        self.prune_expired(now_ms);
        let clean_nonce = nonce_id.trim();
        let Some(challenge) = self.challenges.remove(clean_nonce) else {
            bail!("Operation nonce '{clean_nonce}' is missing, expired, or already consumed");
        };
        if now_ms > challenge.expires_at_ms {
            bail!("Operation nonce '{clean_nonce}' expired");
        }
        let clean_target = target.trim();
        let clean_hash = input_sha256.trim().to_ascii_lowercase();
        if challenge.op_kind != op_kind
            || challenge.target != clean_target
            || challenge.input_sha256 != clean_hash
        {
            bail!(
                "Operation nonce '{clean_nonce}' mismatch: expected ({}, '{}', '{}'), got ({}, '{}', '{}')",
                challenge.op_kind,
                challenge.target,
                challenge.input_sha256,
                op_kind,
                clean_target,
                clean_hash
            );
        }
        Ok(challenge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonces_are_single_use_ttl_checked_and_bound_to_parameters() {
        let mut store = OperationNonceStore::with_contract_hash("A6F75ECE");
        let ch = store.issue(OpKind::NorWrite, "SA26..SA70", "deadbeef", 1_000);
        assert!(ch.nonce_id.starts_with("n_"));
        assert_eq!(ch.ssot_contract_hash, "A6F75ECE");
        assert_eq!(ch.expires_at_ms, 1_000 + DEFAULT_NONCE_TTL_MS);

        assert!(
            store
                .consume(
                    &ch.nonce_id,
                    OpKind::NorWrite,
                    "SA134..SA141",
                    "deadbeef",
                    2_000
                )
                .is_err()
        );
        assert!(
            store
                .consume(
                    &ch.nonce_id,
                    OpKind::NorWrite,
                    "SA26..SA70",
                    "deadbeef",
                    2_000
                )
                .is_err()
        );

        let ch2 = store.issue(OpKind::NorWrite, "SA26..SA70", "cafebabe", 10_000);
        assert!(
            store
                .consume(
                    &ch2.nonce_id,
                    OpKind::NorWrite,
                    "SA26..SA70",
                    "CAFEBABE",
                    15_000
                )
                .is_ok()
        );
        assert!(
            store
                .consume(
                    &ch2.nonce_id,
                    OpKind::NorWrite,
                    "SA26..SA70",
                    "cafebabe",
                    15_001
                )
                .is_err()
        );

        let ch3 = store.issue(OpKind::NorWrite, "SA26..SA70", "abc", 100);
        assert!(
            store
                .consume(
                    &ch3.nonce_id,
                    OpKind::NorWrite,
                    "SA26..SA70",
                    "abc",
                    100 + DEFAULT_NONCE_TTL_MS + 1
                )
                .is_err()
        );
    }
}
