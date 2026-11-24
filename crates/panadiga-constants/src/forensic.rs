//! Shared pure forensic hashing, Shannon entropy, path resolution, and zero-copy mmap I/O helpers.
//!
//! Extracted to `panadiga-constants` so `panadiga-crypto`, `nvram`, `hdd`, and `re` share a
//! single implementation without circular module dependencies or code duplication.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use md5::{Digest as _, Md5};
use memmap2::Mmap;
use sha2::Sha256;

const PARALLEL_HASH_THRESHOLD_BYTES: usize = 64 * 1024;
const ENTROPY_PARALLEL_CHUNK_BYTES: usize = 256 * 1024;
const MAX_BLOCK_DEVICE_INSPECT_BYTES: usize = 128 * 1024 * 1024;
const BLOCK_READ_CHUNK_BYTES: usize = 64 * 1024;

/// Smart abstraction encapsulating a zero-copy `Mmap` image or in-memory/decompressed GZIP buffer.
pub enum ImageBuffer {
    Mmap((File, Mmap)),
    Ram(Vec<u8>),
}

impl Deref for ImageBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Mmap((_, mmap)) => mmap,
            Self::Ram(vec) => vec,
        }
    }
}

/// Ensures `path.parent()` exists before creating an output artifact.
pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Creating output directory {}", parent.display()))?;
    }
    Ok(())
}

/// Computes MD5 and SHA-256 hashes of a byte slice (in parallel above 64 KiB).
pub fn calculate_hashes(data: &[u8]) -> (String, String) {
    let compute_md5 = || {
        let mut md5_hasher = Md5::new();
        md5_hasher.update(data);
        hex::encode(md5_hasher.finalize())
    };
    let compute_sha256 = || {
        let mut sha_hasher = Sha256::new();
        sha_hasher.update(data);
        hex::encode(sha_hasher.finalize())
    };

    if data.len() >= PARALLEL_HASH_THRESHOLD_BYTES {
        rayon::join(compute_md5, compute_sha256)
    } else {
        (compute_md5(), compute_sha256())
    }
}

/// Shannon entropy calculator (0.0000 to 8.0000 bits/byte), sharded in parallel for large slices.
pub fn calculate_entropy(data: &[u8]) -> f64 {
    use rayon::prelude::*;

    if data.is_empty() {
        return 0.0;
    }
    let counts = if data.len() >= ENTROPY_PARALLEL_CHUNK_BYTES * 2 {
        data.par_chunks(ENTROPY_PARALLEL_CHUNK_BYTES)
            .fold(
                || [0usize; 256],
                |mut local, chunk| {
                    for &b in chunk {
                        local[b as usize] += 1;
                    }
                    local
                },
            )
            .reduce(
                || [0usize; 256],
                |mut acc, local| {
                    for (dst, src) in acc.iter_mut().zip(local) {
                        *dst += src;
                    }
                    acc
                },
            )
    } else {
        let mut local = [0usize; 256];
        for &b in data {
            local[b as usize] += 1;
        }
        local
    };
    let total = data.len() as f64;
    let mut entropy = 0.0;
    for c in counts {
        if c > 0 {
            let p = c as f64 / total;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Resolves an input file across working directory, `firmware_and_images`, and executable ancestors,
/// transparently supporting `.bin.gz` siblings and raw OS block device paths.
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

/// Returns `true` when `path` addresses a raw OS block device, disk handle, or logical volume.
pub fn is_raw_block_device_path(path: &Path) -> bool {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim();
    let prefix_match = trimmed.starts_with(r"\\.\") || trimmed.starts_with("/dev/");
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        prefix_match || fs::metadata(path).is_ok_and(|meta| meta.file_type().is_block_device())
    }
    #[cfg(not(unix))]
    {
        prefix_match
    }
}

fn read_block_device_prefix(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            anyhow::anyhow!(
                "Permission denied opening block device '{}' read-only ({}). Linux: add user to 'disk' group (`sudo usermod -aG disk $USER`) or run `pkexec panadiga ...`. Windows: run as Administrator.",
                path.display(),
                e
            )
        } else {
            anyhow::Error::new(e).context(format!("Failed opening block device: {}", path.display()))
        }
    })?;

    let mut buffer = Vec::with_capacity(BLOCK_READ_CHUNK_BYTES);
    let mut chunk = vec![0u8; BLOCK_READ_CHUNK_BYTES];
    while buffer.len() < MAX_BLOCK_DEVICE_INSPECT_BYTES {
        let remaining = MAX_BLOCK_DEVICE_INSPECT_BYTES - buffer.len();
        let to_read = remaining.min(chunk.len());
        match file.read(&mut chunk[..to_read]) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) if !buffer.is_empty() => break,
            Err(e) => {
                return Err(e).with_context(|| {
                    format!(
                        "Failed reading sectors from block device: {}",
                        path.display()
                    )
                });
            }
        }
    }
    if buffer.is_empty() {
        anyhow::bail!("Block device returned 0 readable bytes: {}", path.display());
    }
    Ok(buffer)
}

/// Opens a resolved file or raw OS block device as zero-copy `Mmap` or GZIP-decompressed buffer.
pub fn open_image_buffer_resolved(resolved: &Path) -> Result<ImageBuffer> {
    if is_raw_block_device_path(resolved) {
        return read_block_device_prefix(resolved).map(ImageBuffer::Ram);
    }

    let mut file = File::open(resolved)
        .with_context(|| format!("Failed opening image file: {}", resolved.display()))?;
    let file_len = file.metadata()?.len();
    if file_len == 0 {
        anyhow::bail!("Cannot load empty file (0 bytes): {}", resolved.display());
    }

    let mut magic = [0u8; 2];
    if file.read_exact(&mut magic).is_ok() && magic == [0x1f, 0x8b] {
        file.seek(SeekFrom::Start(0))?;
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut buffer = Vec::new();
        decoder
            .read_to_end(&mut buffer)
            .with_context(|| format!("Failed decompressing GZIP image: {}", resolved.display()))?;
        return Ok(ImageBuffer::Ram(buffer));
    }

    file.seek(SeekFrom::Start(0))?;
    // SAFETY: Read-only forensic image mapping; callers preserve immutable dump files.
    #[allow(unsafe_code)]
    let mmap = unsafe { Mmap::map(&file) }
        .with_context(|| format!("Failed creating mmap on: {}", resolved.display()))?;
    Ok(ImageBuffer::Mmap((file, mmap)))
}

/// Resolves and opens an image file or raw block device into an [`ImageBuffer`].
pub fn open_image_buffer(path: &Path) -> Result<ImageBuffer> {
    let resolved = resolve_input_path(path)?;
    open_image_buffer_resolved(&resolved)
}

/// Opens a disk image with zero-copy memory mapping.
pub fn open_mmap(path: &Path) -> Result<(File, Mmap)> {
    let resolved = resolve_input_path(path)?;
    let file = File::open(&resolved)
        .with_context(|| format!("Failed opening HDD image: {}", resolved.display()))?;
    let file_len = file.metadata()?.len();
    if file_len == 0 {
        anyhow::bail!(
            "Cannot memory map empty file (0 bytes): {}",
            resolved.display()
        );
    }
    // SAFETY: Read-only forensic image mapping; callers preserve immutable dump files.
    #[allow(unsafe_code)]
    let mmap = unsafe { Mmap::map(&file) }
        .with_context(|| format!("Failed creating mmap on: {}", resolved.display()))?;
    Ok((file, mmap))
}
