//! Input path resolution, read-only flash inspection, and shared CoW filesystem primitives.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, bail};

pub use panadiga_constants::forensic::resolve_input_path;

use super::super::pure::{NvramCleanReport, inspect_flash_nvram_slice};
use crate::constants::SPANSION_FLASH_SIZE_BYTES;

/// Reads a resolved flash binary file into a byte vector.
/// Shared by both `inspect_flash_nvram` (read-only) and `clean_flash_nvram` (read-write).
pub(super) fn read_flash_file(resolved_input: &Path) -> Result<Vec<u8>> {
    let mut flash_bytes = Vec::with_capacity(SPANSION_FLASH_SIZE_BYTES);
    let mut file = File::open(resolved_input)
        .with_context(|| format!("Failed to open flash binary: {}", resolved_input.display()))?;
    file.read_to_end(&mut flash_bytes)
        .with_context(|| "Failed reading flash binary content")?;
    Ok(flash_bytes)
}

pub(super) fn ensure_distinct_cow_path(
    reference: &Path,
    candidate: &Path,
    role: &str,
    target_desc: &str,
) -> Result<()> {
    let same_path = reference == candidate
        || (reference.canonicalize().ok().is_some()
            && reference.canonicalize().ok() == candidate.canonicalize().ok());
    if same_path {
        bail!(
            "[FORENSIC INVARIANT VIOLATION] {role} path ({}) cannot overwrite {target_desc} ({}) in-place.",
            candidate.display(),
            reference.display()
        );
    }
    Ok(())
}

pub(super) fn write_cow_file(path: &Path, bytes: &[u8], desc: &str) -> Result<()> {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(fs::create_dir_all)
        .transpose()?;
    fs::write(path, bytes).with_context(|| format!("Failed writing {desc} to {}", path.display()))
}

/// Inspects a flash dump in a 100% read-only manner without any disk-write side effects.
pub fn inspect_flash_nvram(input_path: &Path) -> Result<NvramCleanReport> {
    let resolved_input = resolve_input_path(input_path)?;
    let flash_bytes = read_flash_file(&resolved_input)?;

    inspect_flash_nvram_slice(&flash_bytes, &resolved_input.to_string_lossy())
}
