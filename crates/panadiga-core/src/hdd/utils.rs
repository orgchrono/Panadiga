use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use memmap2::Mmap;

pub use panadiga_constants::forensic::{
    ImageBuffer, calculate_entropy, is_raw_block_device_path, open_image_buffer_resolved,
    open_mmap, resolve_input_path,
};

/// Opens a disk image, Unix mount point, gzip stream, or live block device as [`ImageBuffer`].
pub fn open_image_buffer(path: &Path) -> Result<ImageBuffer> {
    let mut resolved = resolve_input_path(path)?;
    if resolved.is_dir()
        && let Some(blk) =
            super::writer::resolve_mount_point_to_block_device(&resolved.to_string_lossy())
    {
        resolved = PathBuf::from(blk);
    }
    open_image_buffer_resolved(&resolved)
}

/// Helper function to serialize any report struct to formatted JSON and write to disk.
pub fn export_report_to_json<T: serde::Serialize>(
    report: &T,
    json_path: &Path,
    report_desc: &str,
) -> Result<()> {
    let json_data = serde_json::to_string_pretty(report)
        .with_context(|| format!("Failed serializing {report_desc} to JSON"))?;
    fs::write(json_path, json_data)
        .with_context(|| format!("Failed writing JSON report to {}", json_path.display()))?;
    log_println!(
        "\n[+] Structured JSON report exported: {}",
        json_path.display()
    );
    Ok(())
}

#[allow(dead_code)]
fn _keep_file_mmap_types(_: &(File, Mmap)) {}
