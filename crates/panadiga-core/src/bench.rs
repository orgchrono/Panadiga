#![deny(unsafe_code)]

//! Unidirectional, coalesced bench event stream, snapshot state, and ephemeral operation nonces.

pub use panadiga_bench::*;

/// Instantiates a nonce store pre-bound to the current compiled `SSOT_CONTRACT_HASH`.
#[must_use]
pub fn new_ssot_nonce_store() -> OperationNonceStore {
    let hash = format!("{:08X}", crate::ssot::pico_header::compute_pico_ssot_hash());
    OperationNonceStore::with_contract_hash(hash)
}
