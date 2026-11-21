//! Pure functional NVRAM & Upper NOR flash transformations (zero file I/O side effects).

mod audit;
mod clean_patch;
mod types;
mod upper_nor;

pub use audit::{
    audit_nvram_sectors, audit_nvram_slice, calculate_hashes, inspect_flash_nvram_slice,
    inspect_partial_nvram_capture,
};
pub use clean_patch::{
    clean_flash_nvram_slice, clean_flash_nvram_slice_with_policy, patch_flash_nvram_slice,
    patch_flash_nvram_slice_with_policy,
};
pub use types::{
    MicroSectorAudit, NvramCleanReport, NvramPolicyOptions, PureNvramCleanOutput,
    PureUpperNorPatchOutput, SliceIoLabels, UpperNorPatchReport,
};
pub use upper_nor::{patch_flash_upper_nor_slice, patch_flash_upper_nor_slice_with_policy};
