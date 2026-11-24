//! Path resolution, DTO schemas, and workspace discovery for `.FRM` containers.

use panadiga_core::constants::{
    find_workspace_subdir, resolve_workspace_root, resolve_writable_data_root,
};
use panadiga_core::crypto::ExtrheadHeader;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

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

fn normalize_path_for_guard(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn canonicalize_with_tail(target: &Path) -> IpcResult<String> {
    let mut probe = Some(target);
    while let Some(anc) = probe {
        if anc.exists() {
            let canon = anc.canonicalize().map_err(|e| {
                PanadigaIpcError::io(format!("Falha ao canonicalizar '{}': {}", anc.display(), e))
            })?;
            let tail = target.strip_prefix(anc).unwrap_or(Path::new(""));
            return Ok(normalize_path_for_guard(&canon.join(tail)));
        }
        probe = anc.parent();
    }
    Ok(normalize_path_for_guard(target))
}

/// Validates caller-supplied IPC file paths against NUL bytes, `..` traversal, and raw device namespaces.
pub(crate) fn validate_ipc_path_input(raw: &str) -> IpcResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PanadigaIpcError::validation(FRM_WRITE_EMPTY_PATH_ERR));
    }
    if trimmed.contains('\0') {
        return Err(PanadigaIpcError::validation(
            "Caminho IPC inválido: byte NUL proibido.",
        ));
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("\\\\.\\")
        || lower.starts_with("\\\\?\\")
        || lower.starts_with("\\device\\")
        || lower.starts_with("//./")
        || lower.starts_with("//?/")
        || lower == "/dev"
        || lower.starts_with("/dev/")
    {
        return Err(PanadigaIpcError::validation(format!(
            "Acesso direto a namespace de dispositivo '{}' é bloqueado via IPC.",
            trimmed
        )));
    }
    if Path::new(trimmed)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(PanadigaIpcError::validation(format!(
            "Path traversal '..' não permitido via IPC: '{}'.",
            trimmed
        )));
    }
    if trimmed == "~" || trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
        return Err(PanadigaIpcError::validation(format!(
            "Expansão de tilde '~' fora do root permitido via IPC: '{}'.",
            trimmed
        )));
    }
    Ok(trimmed.to_string())
}

fn push_root_variants(out: &mut Vec<String>, root: &Path) {
    let raw_norm = normalize_path_for_guard(root);
    if !raw_norm.is_empty() && !out.contains(&raw_norm) {
        out.push(raw_norm);
    }
    if let Ok(canon) = root.canonicalize() {
        let canon_norm = normalize_path_for_guard(&canon);
        if !canon_norm.is_empty() && !out.contains(&canon_norm) {
            out.push(canon_norm);
        }
    }
}

pub(crate) fn allowed_ipc_roots() -> Vec<String> {
    let mut roots = Vec::new();
    push_root_variants(&mut roots, &resolve_workspace_root());
    push_root_variants(&mut roots, &resolve_writable_data_root());
    push_root_variants(&mut roots, &std::env::temp_dir());
    roots
}

pub(crate) fn ensure_within_allowed_ipc_roots(norm: &str, display_raw: &str) -> IpcResult<()> {
    let allowed = allowed_ipc_roots();
    let in_bounds = allowed
        .iter()
        .any(|root| norm == root || norm.starts_with(&format!("{root}/")));
    if !in_bounds {
        return Err(PanadigaIpcError::validation(format!(
            "Caminho IPC fora dos diretórios permitidos (workspace / data / temp): '{}'.",
            display_raw
        )));
    }
    Ok(())
}

fn ensure_safe_write_target(resolved: &Path) -> IpcResult<()> {
    let norm = canonicalize_with_tail(resolved)?;
    ensure_within_allowed_ipc_roots(&norm, &resolved.display().to_string())?;
    const FORBIDDEN_PREFIXES: &[&str] = &[
        "c:/windows",
        "/etc",
        "/usr",
        "/bin",
        "/sbin",
        "/system",
        "/dev",
    ];
    for prefix in FORBIDDEN_PREFIXES {
        if norm == *prefix || norm.starts_with(&format!("{prefix}/")) {
            return Err(PanadigaIpcError::validation(format!(
                "Escrita em diretório protegido do sistema bloqueada via IPC: '{}'.",
                resolved.display()
            )));
        }
    }
    if norm.contains("/.git/") || norm.ends_with("/.git") {
        return Err(PanadigaIpcError::validation(format!(
            "Escrita em metadados Git bloqueada via IPC: '{}'.",
            resolved.display()
        )));
    }
    if let Ok(ws_canon) = resolve_workspace_root().canonicalize() {
        let schema_dir = format!("{}/profiles/schema", normalize_path_for_guard(&ws_canon));
        if norm == schema_dir || norm.starts_with(&format!("{schema_dir}/")) {
            return Err(PanadigaIpcError::validation(format!(
                "Escrita direta em schemas SSOT bloqueada via IPC: '{}'.",
                resolved.display()
            )));
        }
    }
    Ok(())
}

fn resolve_read_path(raw: &str) -> PathBuf {
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

/// Validates and resolves a readable regular-file path supplied over Tauri IPC.
pub(crate) fn resolve_safe_read_path(raw: &str) -> IpcResult<PathBuf> {
    let validated = validate_ipc_path_input(raw)?;
    let resolved = resolve_read_path(&validated);
    let norm = canonicalize_with_tail(&resolved)?;
    if norm == "/dev" || norm.starts_with("/dev/") {
        return Err(PanadigaIpcError::validation(format!(
            "Leitura direta de dispositivo '{}' bloqueada via IPC.",
            raw.trim()
        )));
    }
    ensure_within_allowed_ipc_roots(&norm, raw.trim())?;
    Ok(resolved)
}

/// Validates and resolves a writable directory path supplied over Tauri IPC.
pub(crate) fn resolve_safe_write_dir(raw: &str) -> IpcResult<PathBuf> {
    let trimmed = validate_ipc_path_input(raw)?;
    let path = PathBuf::from(&trimmed);
    let resolved = if path.is_absolute() {
        path
    } else {
        resolve_workspace_root().join(path)
    };
    ensure_safe_write_target(&resolved)?;
    Ok(resolved)
}

pub(crate) fn resolve_write_path(raw: &str) -> IpcResult<PathBuf> {
    let trimmed = validate_ipc_path_input(raw)?;
    let path = PathBuf::from(&trimmed);
    let resolved = if path.is_absolute() {
        path
    } else {
        resolve_writable_data_root().join(path)
    };
    ensure_safe_write_target(&resolved)?;
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
