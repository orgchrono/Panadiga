//! Filesystem I/O shell for Panasonic Flash NVRAM inspection and surgical cleaning.

mod capture;
mod clean_patch;
mod upper_nor;

pub use capture::{inspect_flash_nvram, resolve_input_path};
pub use clean_patch::{clean_flash_nvram, patch_flash_nvram};
pub use upper_nor::patch_flash_upper_nor_file;
