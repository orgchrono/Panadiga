//! Engineering Documentation, Raw Pseudocode, Bench Logs & Workspace Source Reference IPC Commands
//!
//! Single Responsibility: Strictly confined access to `<workspace>/docs/` (`.md`, `.txt`, `.log`
//! including UTF-16LE captures), allowlisted root Markdown files (`repo:<file>`), and allowlisted
//! workspace source files (`src:<path>`) for interactive cross-reference navigation.

use std::path::{Component, Path, PathBuf};

use panadiga_core::pico::{Am34BootLogReport, decode_captured_log_bytes, parse_bench_log_text};

use super::dto::EngineeringDocEntryDto;

const ROOT_DOC_ALLOWLIST: &[&str] = &[
    "README.md",
    "README.pt-BR.md",
    "AUDITORIA_COMPLETA.md",
    "COMPLETE_AUDIT.md",
    "AGENTS.md",
];

const SRC_TOP_DIRS: &[&str] = &["crates", "firmware", "apps", "scripts", "tools"];

const SRC_EXTENSIONS: &[&str] = &[
    "rs", "c", "h", "pio", "py", "ts", "tsx", "css", "json", "toml", "yml", "yaml", "md", "txt",
    "log", "cmd", "sh",
];

const DOC_EXTENSIONS: &[&str] = &["md", "txt", "log"];

const MAX_SRC_PREVIEW_BYTES: usize = 512 * 1024;

fn resolve_workspace() -> PathBuf {
    panadiga_core::constants::resolve_workspace_root()
}

fn resolve_docs_root() -> Result<PathBuf, String> {
    if let Some(dir) = panadiga_core::constants::find_workspace_subdir("docs") {
        return Ok(dir);
    }
    let fallback = resolve_workspace().join("docs");
    if fallback.is_dir() {
        return Ok(fallback);
    }
    Err("Diretório 'docs' não encontrado no workspace.".to_string())
}

fn is_normal_rel_path(rel: &str) -> bool {
    let p = Path::new(rel);
    !rel.is_empty() && !p.is_absolute() && p.components().all(|c| matches!(c, Component::Normal(_)))
}

fn has_allowed_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| allowed.iter().any(|a| ext.eq_ignore_ascii_case(a)))
        .unwrap_or(false)
}

fn read_decoded_text(path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("Falha ao ler '{}': {e}", path.display()))?;
    Ok(decode_captured_log_bytes(&bytes))
}

fn extract_doc_title(path: &Path, file_name: &str) -> String {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("log") {
        return format!("COM10/P7501 Capture: {file_name}");
    }
    if ext.eq_ignore_ascii_case("txt") {
        return format!("TXT Reference: {file_name}");
    }
    if let Ok(content) = read_decoded_text(path) {
        for line in content.lines().take(12) {
            let trimmed = line.trim();
            if let Some(heading) = trimmed.strip_prefix("# ") {
                let h = heading.trim();
                if !h.is_empty() {
                    return h.to_string();
                }
            }
        }
    }
    file_name.trim_end_matches(".md").replace('_', " ")
}

fn collect_dir_entries(
    base_dir: &Path,
    locale: &str,
    rel_prefix: &str,
    entries: &mut Vec<EngineeringDocEntryDto>,
) -> Result<(), String> {
    if !base_dir.is_dir() {
        return Ok(());
    }
    let mut found = Vec::new();
    let read_dir = std::fs::read_dir(base_dir)
        .map_err(|e| format!("Falha listando 'docs/{rel_prefix}': {e}"))?;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() || !has_allowed_extension(&path, DOC_EXTENSIONS) {
            continue;
        }
        let Some(file_name) = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        found.push(EngineeringDocEntryDto {
            locale: locale.to_string(),
            relative_path: format!("{rel_prefix}/{file_name}"),
            title: extract_doc_title(&path, &file_name),
            file_name,
            size_bytes,
        });
    }
    found.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    entries.extend(found);
    Ok(())
}

/// Lists bilingual markdown docs (`INDEX.md`, root docs, `en-US`, `pt-BR`, `audits`),
/// Ghidra pseudocode/raw `.txt` dumps, and deduplicated `COM10` `.log` captures.
#[tauri::command]
pub fn list_engineering_docs() -> Result<Vec<EngineeringDocEntryDto>, String> {
    let docs_root = resolve_docs_root()?;
    let ws_root = resolve_workspace();
    let mut entries = Vec::new();

    let index_path = docs_root.join("INDEX.md");
    if index_path.is_file() {
        let size_bytes = std::fs::metadata(&index_path).map(|m| m.len()).unwrap_or(0);
        entries.push(EngineeringDocEntryDto {
            locale: "root".to_string(),
            file_name: "INDEX.md".to_string(),
            relative_path: "INDEX.md".to_string(),
            title: extract_doc_title(&index_path, "INDEX.md"),
            size_bytes,
        });
    }

    for &root_name in ROOT_DOC_ALLOWLIST {
        let p = ws_root.join(root_name);
        if p.is_file() {
            let size_bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            entries.push(EngineeringDocEntryDto {
                locale: "root".to_string(),
                file_name: root_name.to_string(),
                relative_path: format!("repo:{root_name}"),
                title: extract_doc_title(&p, root_name),
                size_bytes,
            });
        }
    }

    for locale in ["en-US", "pt-BR", "audits", "pseudocode", "debug_logs"] {
        collect_dir_entries(&docs_root.join(locale), locale, locale, &mut entries)?;
    }
    collect_dir_entries(
        &docs_root.join("pseudocode").join("raw"),
        "raw_txt",
        "pseudocode/raw",
        &mut entries,
    )?;

    Ok(entries)
}

fn read_docs_confined_text(rel: &str, allowed_exts: &[&str]) -> Result<String, String> {
    if !is_normal_rel_path(rel) || !has_allowed_extension(Path::new(rel), allowed_exts) {
        return Err("Caminho de documento inválido (bloqueado fora de docs/).".to_string());
    }
    let docs_root = resolve_docs_root()?;
    let canon_docs = std::fs::canonicalize(&docs_root)
        .map_err(|e| format!("Falha ao resolver diretório docs/: {e}"))?;
    let canon_target = std::fs::canonicalize(docs_root.join(rel))
        .map_err(|e| format!("Documento 'docs/{rel}' não encontrado: {e}"))?;
    if !canon_target.starts_with(&canon_docs) || !canon_target.is_file() {
        return Err("Acesso bloqueado fora do diretório docs/.".to_string());
    }
    read_decoded_text(&canon_target)
}

fn read_workspace_src_preview(sub: &str) -> Result<String, String> {
    if !is_normal_rel_path(sub) || !has_allowed_extension(Path::new(sub), SRC_EXTENSIONS) {
        return Err("Referência de código inválida no workspace.".to_string());
    }
    let first = sub.split('/').next().unwrap_or("");
    if !SRC_TOP_DIRS.contains(&first) {
        return Err(
            "Referência permitida apenas em crates/, firmware/, apps/, scripts/ ou tools/."
                .to_string(),
        );
    }
    let ws_root = resolve_workspace();
    let canon_ws =
        std::fs::canonicalize(&ws_root).map_err(|e| format!("Falha ao resolver workspace: {e}"))?;
    let canon_target = std::fs::canonicalize(ws_root.join(sub))
        .map_err(|e| format!("Arquivo de referência '{sub}' não encontrado: {e}"))?;
    if !canon_target.starts_with(&canon_ws) || !canon_target.is_file() {
        return Err("Acesso bloqueado fora do workspace.".to_string());
    }
    let bytes = std::fs::read(&canon_target)
        .map_err(|e| format!("Falha ao ler referência '{sub}': {e}"))?;
    if bytes.len() > MAX_SRC_PREVIEW_BYTES {
        let mut s = decode_captured_log_bytes(&bytes[..MAX_SRC_PREVIEW_BYTES]);
        s.push_str("\n\n... [truncado em 512 KiB para visualização rápida]");
        return Ok(s);
    }
    Ok(decode_captured_log_bytes(&bytes))
}

/// Reads an engineering doc (`.md`/`.txt`/`.log` in `docs/`), allowlisted root doc (`repo:<name>`),
/// or workspace source reference (`src:<rel_path>`) with strict path-traversal confinement.
#[tauri::command]
pub fn read_engineering_doc(relative_path: String) -> Result<String, String> {
    let rel = relative_path.trim().replace('\\', "/");
    if let Some(root_name) = rel.strip_prefix("repo:") {
        if !ROOT_DOC_ALLOWLIST.contains(&root_name) {
            return Err("Documento raiz não listado na allowlist.".to_string());
        }
        let ws_root = resolve_workspace();
        let target = ws_root.join(root_name);
        if !target.is_file() {
            return Err(format!("Documento raiz '{root_name}' não encontrado."));
        }
        return read_decoded_text(&target);
    }
    if let Some(sub) = rel.strip_prefix("src:") {
        return read_workspace_src_preview(sub);
    }
    read_docs_confined_text(&rel, DOC_EXTENSIONS)
}

/// Decodes a `.log` or `.txt` bench capture confined within `docs/` via `panadiga_core`.
#[tauri::command]
pub fn decode_engineering_log(relative_path: String) -> Result<Am34BootLogReport, String> {
    let rel = relative_path.trim().replace('\\', "/");
    let decoded = read_docs_confined_text(&rel, &["log", "txt"])?;
    Ok(parse_bench_log_text(&decoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engineering_docs_catalog_and_traversal_guard_work() {
        let docs = list_engineering_docs().unwrap();
        assert!(!docs.is_empty());
        assert!(docs.iter().any(|d| d.relative_path == "INDEX.md"));
        assert!(
            docs.iter()
                .any(|d| d.relative_path == "repo:AUDITORIA_COMPLETA.md")
        );
        assert!(
            docs.iter()
                .any(|d| d.relative_path.starts_with("pseudocode/raw/"))
        );
        assert!(docs.iter().any(|d| d.relative_path.ends_with(".log")));

        let idx = read_engineering_doc("INDEX.md".to_string()).unwrap();
        assert!(!idx.trim().is_empty());
        let src_preview =
            read_engineering_doc("src:crates/panadiga-core/src/pico/log_decoder.rs".to_string())
                .unwrap();
        assert!(src_preview.contains("decode_captured_log_bytes"));

        assert!(read_engineering_doc("../Cargo.toml".to_string()).is_err());
        assert!(read_engineering_doc("../README.md".to_string()).is_err());
        assert!(read_engineering_doc("src:.env".to_string()).is_err());
        assert!(read_engineering_doc("src:../Cargo.toml".to_string()).is_err());
    }

    #[test]
    fn decodes_real_debug_log_capture_cleanly() {
        let report = decode_engineering_log(
            "debug_logs/captura_sd_flat_fix_20261115_025903.log".to_string(),
        )
        .unwrap();
        assert_eq!(report.boot_mode.as_deref(), Some("cd_diag"));
        assert!(report.sdcd_errors.contains(&-91006));
        assert!(report.sdcd_errors.contains(&-90002));
        assert!(report.drive_error_codes.contains(&"80040904".to_string()));
    }
}
