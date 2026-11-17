//! Input path resolution, read-only flash inspection, and shared CoW filesystem primitives.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::super::pure::{NvramCleanReport, clean_flash_nvram_slice};
use crate::constants::SPANSION_FLASH_SIZE_BYTES;

/// Intelligently resolves an input file path across working directory and known folders,
/// transparently handling both uncompressed (.bin) and compressed (.bin.gz) files.
pub fn resolve_input_path(input: &Path) -> Result<PathBuf> {
    let raw = input.to_string_lossy();
    let trimmed = raw.trim();
    if trimmed.starts_with(r"\\.\") || trimmed.starts_with("/dev/") {
        return Ok(PathBuf::from(trimmed));
    }
    if input.exists() {
        return Ok(input.to_path_buf());
    }

    let gz_input = PathBuf::from(format!("{}.gz", input.display()));
    if gz_input.exists() {
        return Ok(gz_input);
    }

    let filename = input.file_name().unwrap_or(input.as_os_str());

    let mut candidate_dirs = vec![
        PathBuf::from("firmware_and_images"),
        PathBuf::from("../firmware_and_images"),
        PathBuf::from("../../firmware_and_images"),
        PathBuf::from(".."),
        PathBuf::from("../.."),
        PathBuf::from("."),
    ];

    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        candidate_dirs.push(exe_dir.join("firmware_and_images"));
        candidate_dirs.push(exe_dir.to_path_buf());
        if let Some(parent) = exe_dir.parent() {
            candidate_dirs.push(parent.join("firmware_and_images"));
            candidate_dirs.push(parent.to_path_buf());
        }
    }

    for dir in candidate_dirs {
        let direct = dir.join(input);
        if direct.exists() {
            return Ok(direct);
        }
        let direct_gz = dir.join(&gz_input);
        if direct_gz.exists() {
            return Ok(direct_gz);
        }
        let by_name = dir.join(filename);
        if by_name.exists() {
            return Ok(by_name);
        }
        let by_name_gz = dir.join(format!("{}.gz", filename.to_string_lossy()));
        if by_name_gz.exists() {
            return Ok(by_name_gz);
        }
    }

    bail!("Flash binary input file not found: {}", input.display())
}

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

    let dummy_output = resolved_input.with_file_name("flash_clean_preview.bin");
    let dummy_backup = resolved_input.with_file_name("nvram_corrupt_backup.bin");

    let pure_output = clean_flash_nvram_slice(
        &flash_bytes,
        &resolved_input.to_string_lossy(),
        &dummy_output.to_string_lossy(),
        &dummy_backup.to_string_lossy(),
    )?;

    Ok(pure_output.report)
}
