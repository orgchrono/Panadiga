#![deny(unsafe_code)]

//! Read-only, Windows-safe `RomFS` and `CramFS` filesystem extractor for Panasonic `MAIN` `fmaX` partitions.
//!
//! Avoids OS symlink / device-node creation (recording them safely in the structured report)
//! and sanitizes relative paths against `..` traversal and reserved NTFS names/characters.

use std::path::Path;

use anyhow::{Result, bail};

use super::super::codec::read_u32_le;
use crate::constants::{PANA_CRAMFS_MAGIC, PANA_CRAMFS_SIGNATURE, PANA_ROMFS_MAGIC};

mod cramfs;
mod romfs;
mod types;

#[cfg(test)]
mod tests;

pub use cramfs::extract_cramfs_to_dir;
pub use romfs::extract_romfs_to_dir;
pub use types::{
    ElfMetadata, ExtractedFsEntry, ExtractedFsEntryKind, FsExtractReport, parse_elf_metadata,
    sanitize_fs_rel_path,
};

/// Auto-detects `RomFS` or `CramFS` in `data` and runs read-only Windows-safe extraction.
pub fn extract_partition_fs(
    data: &[u8],
    out_dir: Option<&Path>,
    elf_only: bool,
) -> Result<FsExtractReport> {
    if data.len() >= 32 && &data[0..8] == PANA_ROMFS_MAGIC {
        extract_romfs_to_dir(data, out_dir, elf_only)
    } else if data.len() >= 76
        && read_u32_le(data, 0) == PANA_CRAMFS_MAGIC
        && &data[16..32] == PANA_CRAMFS_SIGNATURE
    {
        extract_cramfs_to_dir(data, out_dir, elf_only)
    } else {
        bail!("Unsupported or non-filesystem partition slice (expected RomFS or CramFS)")
    }
}
