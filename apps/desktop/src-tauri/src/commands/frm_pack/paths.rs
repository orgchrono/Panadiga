//! Path resolution, DTO schemas, and workspace discovery for `.FRM` containers.

use panadiga_core::constants::{find_workspace_subdir, resolve_workspace_root};
use panadiga_core::crypto::ExtrheadHeader;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::commands::constants::{
    FIRMWARE_IMAGES_DIR_NAME, FRM_COLLECTION_MAX_DEPTH, FRM_FILE_EXTENSION, FRM_IGNORED_SUBDIR,
    FRM_WRITE_EMPTY_PATH_ERR,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};

/// Module input item passed from the desktop UI for `dvd` / `sd` container packing.
#[derive(Debug, Clone, Deserialize)]
pub struct FrmPackModuleInputDto {
    pub name: String,
    pub path: String,
}

/// Parameters accepted by `pack_frm_command`.
#[derive(Debug, Clone, Deserialize)]
pub struct PackFrmRequestDto {
    pub mode: String,
    pub out_path: String,
    #[serde(default)]
    pub modules: Vec<FrmPackModuleInputDto>,
    pub input_path: Option<String>,
    pub dst_addr: Option<String>,
    pub tag: Option<String>,
    pub build_name: Option<String>,
    pub hw_family: Option<String>,
    pub model_group: Option<String>,
    pub version_u16: Option<String>,
    pub target_flag: Option<u8>,
}

/// Result returned by `unpack_extrhead_command` with verified header and checksum details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtrheadUnpackReportDto {
    pub file_path: String,
    pub header: ExtrheadHeader,
    pub decompressed_bytes: usize,
    pub calculated_checksum_hex: String,
    pub header_checksum_hex: String,
    pub checksum_valid: bool,
    pub footer_present: bool,
    pub written_output_path: Option<String>,
}

pub(crate) fn resolve_read_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    let direct = PathBuf::from(trimmed);
    if direct.exists() || direct.is_absolute() {
        return direct;
    }
    let ws_root = resolve_workspace_root();
    let ws_rel = ws_root.join(trimmed);
    if ws_rel.exists() {
        return ws_rel;
    }
    let fw_rel = ws_root
        .join(crate::commands::constants::FIRMWARE_IMAGES_DIR_NAME)
        .join(trimmed);
    if fw_rel.exists() {
        return fw_rel;
    }
    ws_rel
}

pub(crate) fn resolve_write_path(raw: &str) -> IpcResult<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PanadigaIpcError::validation(FRM_WRITE_EMPTY_PATH_ERR));
    }
    let path = PathBuf::from(trimmed);
    let resolved = if path.is_absolute() {
        path
    } else {
        resolve_workspace_root().join(path)
    };
    if let Some(parent) = resolved.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            PanadigaIpcError::io(format!(
                "Falha criando diretório '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }
    Ok(resolved)
}

fn collect_frm_files(dir: &Path, root: &Path, out: &mut Vec<String>, depth: usize) {
    if depth > FRM_COLLECTION_MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if name != FRM_IGNORED_SUBDIR {
                collect_frm_files(&p, root, out, depth + 1);
            }
            continue;
        }
        let is_frm = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case(FRM_FILE_EXTENSION))
            .unwrap_or(false);
        if is_frm {
            let rel = p.strip_prefix(root).unwrap_or(&p);
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Lists `.FRM` containers available under workspace `firmware_and_images`.
#[tauri::command]
pub fn list_frm_containers_command() -> IpcResult<Vec<String>> {
    let ws_root = resolve_workspace_root();
    let Some(fw_dir) = find_workspace_subdir(FIRMWARE_IMAGES_DIR_NAME) else {
        return Ok(Vec::new());
    };
    let mut found = Vec::new();
    collect_frm_files(&fw_dir, &ws_root, &mut found, 0);
    found.sort();
    found.dedup();
    Ok(found)
}
