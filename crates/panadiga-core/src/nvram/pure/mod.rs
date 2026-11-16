//! Pure functional NVRAM inspection and surgical erasure helpers.

mod audit;
mod clean_patch;
mod types;
mod upper_nor;

pub use audit::{audit_nvram_slice, calculate_hashes, inspect_partial_nvram_capture};
pub use clean_patch::{clean_flash_nvram_slice, patch_flash_nvram_slice};
pub use types::{
    MicroSectorAudit, NvramCleanReport, PureNvramCleanOutput, PureUpperNorPatchOutput,
    UpperNorPatchReport,
};
pub use upper_nor::patch_flash_upper_nor_slice;
