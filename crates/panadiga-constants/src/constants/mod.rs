//! Panasonic DMR-EH55 System Architecture Constants
//!
//! Strongly typed, exhaustive definition of all memory boundaries, flash microsectors,
//! disk LBA offsets, partition headers, and cryptographic ECB leak signatures.
//! ZERO magic numbers allowed anywhere in the codebase.

mod crypto_frm;
mod flash_hdd;
mod hardware_am34;
mod paths_runtime;
mod safety_invariants;
mod tbc_mmio;

pub use crypto_frm::*;
pub use flash_hdd::*;
pub use hardware_am34::*;
pub use paths_runtime::*;
pub use safety_invariants::*;
pub use tbc_mmio::*;
