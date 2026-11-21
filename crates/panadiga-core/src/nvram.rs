//! Panasonic DMR-EH55 NVRAM / Flash Management Subsystem
//!
//! Handles surgical cleaning, auditing, and verification of Spansion S29JL064H NOR Flash dumps.

mod io;
mod pure;

#[cfg(test)]
mod tests;

pub use io::{
    clean_flash_nvram, clean_flash_nvram_with_policy, inspect_flash_nvram, patch_flash_nvram,
    patch_flash_nvram_with_policy, patch_flash_upper_nor_file,
    patch_flash_upper_nor_file_with_policy, resolve_input_path,
};
pub use pure::{
    MicroSectorAudit, NvramCleanReport, NvramPolicyOptions, PureNvramCleanOutput,
    PureUpperNorPatchOutput, SliceIoLabels, UpperNorPatchReport, audit_nvram_sectors,
    audit_nvram_slice, calculate_hashes, clean_flash_nvram_slice,
    clean_flash_nvram_slice_with_policy, inspect_flash_nvram_slice, inspect_partial_nvram_capture,
    patch_flash_nvram_slice, patch_flash_nvram_slice_with_policy, patch_flash_upper_nor_slice,
    patch_flash_upper_nor_slice_with_policy,
};

#[cfg(test)]
use crate::constants::*;
