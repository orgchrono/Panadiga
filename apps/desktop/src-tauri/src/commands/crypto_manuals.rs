//! Panasonic DMR-EH55 DIGA Suite - Cryptanalysis & Manuals IPC Commands
//!
//! Single Responsibility: Tauri IPC command dispatchers for PANA_DVD.FRM 64-bit ECB
//! cryptanalysis, TOC inspection, KPA codebook generation, and PDF manual search.

use std::path::Path;

use super::constants::{
    BYTES_PER_KIB_F64, BYTES_PER_MIB_F64, CANDIDATE_FRM_CONTAINER_PATHS,
    DEFAULT_FRM_CONTAINER_PATH, DEFAULT_FRM_ENTROPY_TOP_N, DEFAULT_MANUAL_FALLBACK_NAME,
    DEFAULT_MANUAL_SEARCH_TERM,
};
use super::dto::{
    CodebookReport, FrmEntropyReport, FrmTocDumpReport, ManualSearchResult, resolve_candidate,
};

/// Performs Shannon entropy calculation, cipher frequency analysis, and ECB leak detection.
#[tauri::command]
pub fn analyze_frm_command(frm_path: String) -> Result<FrmEntropyReport, String> {
    let hdd_candidates: Vec<&Path> = panadiga_core::constants::CANDIDATE_HDD_RAW_BACKUP_PATHS
        .iter()
        .map(Path::new)
        .collect();
    let hdd_path = hdd_candidates.iter().find(|p| p.exists()).copied();

    panadiga_core::crypto::analyze_frm_container(
        Path::new(&frm_path),
        hdd_path,
        DEFAULT_FRM_ENTROPY_TOP_N,
        None,
    )
    .map_err(|e| e.to_string())
}

/// Dumps and validates the Table of Contents (TOC) descriptors in PANA_DVD.FRM.
#[tauri::command]
pub fn dump_frm_toc_command(frm_path: String) -> Result<FrmTocDumpReport, String> {
    panadiga_core::crypto::read_and_analyze_frm_toc(Path::new(&frm_path)).map_err(|e| e.to_string())
}

/// Computes the 64-bit ECB Known-Plaintext Attack (KPA) codebook dictionary.
#[tauri::command]
pub fn get_frm_codebook_command(
    frm_path: String,
    hdd_path: Option<String>,
) -> Result<CodebookReport, String> {
    let hdd_p = hdd_path.as_deref().map(Path::new);
    panadiga_core::crypto::print_codebook(Path::new(&frm_path), hdd_p, None)
        .map_err(|e| e.to_string())
}

/// Searches official Panasonic DMR-EH55 service manuals and schematics for technical terms.
#[tauri::command]
pub fn search_manuals_command(term: String) -> Result<Vec<ManualSearchResult>, String> {
    let fallback_dir = panadiga_core::constants::find_workspace_subdir(
        panadiga_core::constants::DEFAULT_MANUALS_DIR,
    );
    let candidate_dirs: Vec<&Path> = panadiga_core::constants::CANDIDATE_MANUALS_DIRS
        .iter()
        .map(Path::new)
        .collect();
    let resolved_dir = candidate_dirs
        .into_iter()
        .find(|p| p.is_dir())
        .map(Path::to_path_buf)
        .or(fallback_dir)
        .ok_or_else(|| "Diretório 'manuals_and_schematics' não encontrado.".to_string())?;
    panadiga_core::manuals::search_manuals(&resolved_dir, Some(vec![term]), None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_service_manuals() -> Result<Vec<super::dto::ServiceManualCatalogItemDto>, String> {
    let manuals_dir = panadiga_core::constants::find_workspace_subdir(
        panadiga_core::constants::DEFAULT_MANUALS_DIR,
    )
    .ok_or_else(|| "Diretório 'manuals_and_schematics' não encontrado.".to_string())?;
    let root = panadiga_core::constants::resolve_workspace_root();
    let pdfs =
        panadiga_core::manuals::list_pdf_files(&manuals_dir, None).map_err(|e| e.to_string())?;
    let mut items = Vec::with_capacity(pdfs.len());
    for p in pdfs {
        let size_bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        let formatted_size = if size_bytes >= BYTES_PER_MIB_F64 as u64 {
            format!("{:.2} MB", size_bytes as f64 / BYTES_PER_MIB_F64)
        } else {
            format!("{:.1} KB", size_bytes as f64 / BYTES_PER_KIB_F64)
        };
        let file_name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_MANUAL_FALLBACK_NAME)
            .to_string();
        let relative_path = p
            .strip_prefix(&root)
            .unwrap_or(&p)
            .to_string_lossy()
            .replace('\\', "/");
        items.push(super::dto::ServiceManualCatalogItemDto {
            file_name,
            relative_path,
            size_bytes,
            formatted_size,
        });
    }
    Ok(items)
}

fn resolve_frm_input_path(frm_path: Option<String>) -> String {
    if let Some(raw) = frm_path
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        let nested = format!("firmware_and_images/{}", raw);
        return resolve_candidate(&[&raw, &nested], &raw);
    }
    resolve_candidate(CANDIDATE_FRM_CONTAINER_PATHS, DEFAULT_FRM_CONTAINER_PATH)
}

/// Frontend `analyze_frm` command.
#[tauri::command]
pub fn analyze_frm(frm_path: Option<String>) -> Result<FrmEntropyReport, String> {
    analyze_frm_command(resolve_frm_input_path(frm_path))
}

/// Frontend `dump_frm_toc` command.
#[tauri::command]
pub fn dump_frm_toc(frm_path: Option<String>) -> Result<FrmTocDumpReport, String> {
    dump_frm_toc_command(resolve_frm_input_path(frm_path))
}

/// Frontend `get_codebook` command.
#[tauri::command]
pub fn get_codebook(
    frm_path: Option<String>,
    hdd_path: Option<String>,
) -> Result<CodebookReport, String> {
    get_frm_codebook_command(resolve_frm_input_path(frm_path), hdd_path)
}

/// Frontend `search_manuals` command accepting either `query` or `term`.
#[tauri::command]
pub fn search_manuals(
    query: Option<String>,
    term: Option<String>,
) -> Result<Vec<ManualSearchResult>, String> {
    let q = query
        .or(term)
        .unwrap_or_else(|| DEFAULT_MANUAL_SEARCH_TERM.to_string());
    search_manuals_command(q)
}
