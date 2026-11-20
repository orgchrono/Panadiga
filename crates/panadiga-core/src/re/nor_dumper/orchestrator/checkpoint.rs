#![deny(unsafe_code)]

//! On-disk SHA-256 checkpoint persistence for sliced live NOR captures.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::nvram::calculate_hashes;

fn checkpoint_paths(dir: &Path, slice_va: u32, slice_len: u32) -> (PathBuf, PathBuf) {
    let stem = format!("slice_{slice_va:08x}_{slice_len:08x}");
    (
        dir.join(format!("{stem}.bin")),
        dir.join(format!("{stem}.sha256")),
    )
}

pub(super) fn try_load_checkpoint(
    dir: &Path,
    slice_va: u32,
    slice_len: u32,
) -> Result<Option<(Vec<u8>, String, PathBuf)>> {
    let (bin_path, sha_path) = checkpoint_paths(dir, slice_va, slice_len);
    if !bin_path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&bin_path)
        .with_context(|| format!("Reading slice checkpoint {}", bin_path.display()))?;
    if bytes.len() != slice_len as usize {
        return Ok(None);
    }
    let (_, actual_sha) = calculate_hashes(&bytes);
    if sha_path.is_file() {
        let expected = fs::read_to_string(&sha_path)
            .with_context(|| format!("Reading hash {}", sha_path.display()))?;
        if expected.trim() != actual_sha {
            return Ok(None);
        }
    }
    Ok(Some((bytes, actual_sha, bin_path)))
}

pub(super) fn save_checkpoint(
    dir: &Path,
    slice_va: u32,
    bytes: &[u8],
    sha256: &str,
) -> Result<PathBuf> {
    fs::create_dir_all(dir)
        .with_context(|| format!("Creating NOR checkpoint dir {}", dir.display()))?;
    let (bin_path, sha_path) = checkpoint_paths(dir, slice_va, bytes.len() as u32);
    fs::write(&bin_path, bytes)
        .with_context(|| format!("Writing NOR slice {}", bin_path.display()))?;
    fs::write(&sha_path, format!("{sha256}\n"))
        .with_context(|| format!("Writing hash {}", sha_path.display()))?;
    Ok(bin_path)
}
