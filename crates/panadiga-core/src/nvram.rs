//! Panasonic DMR-EH55 NVRAM / Flash Management Subsystem
//!
//! Handles surgical cleaning, auditing, and verification of Spansion S29JL064H NOR Flash dumps.

mod io;
mod pure;

#[cfg(test)]
mod tests;

pub use io::{
    clean_flash_nvram, inspect_flash_nvram, patch_flash_nvram, patch_flash_upper_nor_file,
    resolve_input_path,
};
pub use pure::{
    MicroSectorAudit, NvramCleanReport, PureNvramCleanOutput, PureUpperNorPatchOutput,
    UpperNorPatchReport, audit_nvram_slice, calculate_hashes, clean_flash_nvram_slice,
    inspect_partial_nvram_capture, patch_flash_nvram_slice, patch_flash_upper_nor_slice,
};

#[cfg(test)]
use crate::constants::*;
