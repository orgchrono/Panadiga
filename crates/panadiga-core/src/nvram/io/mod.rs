//! File I/O workflows for NVRAM sanitization, inspection, and Copy-on-Write Upper NOR patching.

mod capture;
mod clean_patch;
mod upper_nor;

pub use capture::{inspect_flash_nvram, resolve_input_path};
pub use clean_patch::{
    clean_flash_nvram, clean_flash_nvram_with_policy, patch_flash_nvram,
    patch_flash_nvram_with_policy,
};
pub use upper_nor::{patch_flash_upper_nor_file, patch_flash_upper_nor_file_with_policy};
