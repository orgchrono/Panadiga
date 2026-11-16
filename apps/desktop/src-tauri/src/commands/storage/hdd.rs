//! Tauri IPC commands for HDD MEIHDFS inspection, allocation decoding, and partition extraction.

use std::path::Path;

use crate::commands::constants::{
    CANDIDATE_HDD_IMAGE_PATHS, DEFAULT_HDD_EXTRACT_DIR, DEFAULT_HDD_IMAGE_PATH,
};
use crate::commands::dto::{
    ExtractedPartitionItemDto, ExtractedPartitionReport, HddAnalysisReport,
    MeihdfsAllocationReport, PartitionExtractionManifestDto, resolve_candidate,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};

/// Inspects HDD image layout, MBR tables, partition metadata, and allocation density.
#[tauri::command]
pub fn inspect_hdd_command(image_path: String, show_density: bool) -> IpcResult<HddAnalysisReport> {
    panadiga_core::hdd::inspect_hdd_image(Path::new(&image_path), show_density, None)
        .map_err(PanadigaIpcError::from)
}

/// Decodes bit-inverted MEIHDFS filesystem allocation descriptors and journal blocks.
#[tauri::command]
pub fn decode_meihdfs_command(
    image_path: String,
    include_journal: bool,
) -> IpcResult<MeihdfsAllocationReport> {
    panadiga_core::hdd::decode_meihdfs_descriptors(Path::new(&image_path), include_journal, None)
        .map_err(PanadigaIpcError::from)
}

/// Surgically extracts firmware partitions, MBR structures, and cryptographic manifest.
#[tauri::command]
pub fn extract_hdd_partitions_command(
    image_path: String,
    output_dir: Option<String>,
) -> IpcResult<Vec<ExtractedPartitionReport>> {
    let out_dir = output_dir.unwrap_or_else(|| DEFAULT_HDD_EXTRACT_DIR.to_string());
    panadiga_core::hdd::extract_hdd_partitions(Path::new(&image_path), Path::new(&out_dir), None)
        .map_err(PanadigaIpcError::from)
}

/// Frontend `inspect_hdd` command.
#[tauri::command]
pub fn inspect_hdd(
    image_path: Option<String>,
    show_density: Option<bool>,
) -> IpcResult<HddAnalysisReport> {
    let path = image_path
        .unwrap_or_else(|| resolve_candidate(CANDIDATE_HDD_IMAGE_PATHS, DEFAULT_HDD_IMAGE_PATH));
    inspect_hdd_command(path, show_density.unwrap_or(true))
}

/// Frontend `decode_meihdfs_allocations` command.
#[tauri::command]
pub fn decode_meihdfs_allocations(
    image_path: Option<String>,
    include_journal: Option<bool>,
) -> IpcResult<MeihdfsAllocationReport> {
    let path = image_path
        .unwrap_or_else(|| resolve_candidate(CANDIDATE_HDD_IMAGE_PATHS, DEFAULT_HDD_IMAGE_PATH));
    decode_meihdfs_command(path, include_journal.unwrap_or(true))
}

/// Frontend `extract_partitions` command returning `PartitionExtractionManifestDto`.
#[tauri::command]
pub fn extract_partitions(
    image_path: Option<String>,
    output_dir: Option<String>,
) -> IpcResult<PartitionExtractionManifestDto> {
    let img = image_path
        .unwrap_or_else(|| resolve_candidate(CANDIDATE_HDD_IMAGE_PATHS, DEFAULT_HDD_IMAGE_PATH));
    let out_dir = output_dir.unwrap_or_else(|| DEFAULT_HDD_EXTRACT_DIR.to_string());
    let reports = extract_hdd_partitions_command(img.clone(), Some(out_dir.clone()))?;

    let mut sums = String::new();
    let mut items = Vec::with_capacity(reports.len());
    for (idx, r) in reports.iter().enumerate() {
        sums.push_str(&format!("{} *{}\n", r.sha256, r.filename));
        let offset = panadiga_core::hdd::PARTITION_LAYOUT_SPECS
            .get(idx)
            .map(|s| s.offset)
            .unwrap_or(0);
        let bytes_read = std::fs::read(&r.path).unwrap_or_default();
        let (md5_hex, _) = panadiga_core::nvram::calculate_hashes(&bytes_read);
        let entropy = panadiga_core::hdd::utils::calculate_entropy(&bytes_read);

        items.push(ExtractedPartitionItemDto {
            id: idx + 1,
            name: r.name.clone(),
            filename: r.filename.clone(),
            offset_hex: format!("0x{:08X}", offset),
            size_bytes: r.size_bytes,
            sha256: r.sha256.clone(),
            md5: md5_hex,
            entropy: (entropy * 100.0).round() / 100.0,
        });
    }

    Ok(PartitionExtractionManifestDto {
        timestamp: format_utc_iso8601_now(),
        source_image: img,
        output_directory: out_dir,
        manifest_sha256_sums: sums.trim_end().to_string(),
        extracted_partitions: items,
    })
}

fn format_utc_iso8601_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hour = rem / 3_600;
    let minute = (rem % 3_600) / 60;
    let second = rem % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d, hour, minute, second
    )
}
