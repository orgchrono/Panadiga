//! Service manual & datasheet PDF page extractor and cache for the Panadiga RAG Engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::super::guardrails::QueryGuard;
use super::super::models::{Chunk, SubsystemType};
use super::markdown::{ChunkFlushTarget, flush_lines_chunk_with_page};

const PDF_CACHE_SUBDIR: &str = "panadiga";
const PDF_CACHE_FILENAME: &str = "rag_pdf_cache_v2.json";
const MAX_PDF_LINES_PER_CHUNK: usize = 45;
#[cfg(unix)]
const PRIVATE_DIR_MODE: u32 = 0o700;
#[cfg(unix)]
const PRIVATE_FILE_MODE: u32 = 0o600;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedPdfEntry {
    file_len: u64,
    modified_secs: u64,
    chunks: Vec<Chunk>,
}

/// Scoped disk-backed PDF parse cache (read-only across Rayon threads, mutated on collector thread).
#[derive(Debug, Default)]
pub struct PdfDiskCache {
    entries: HashMap<String, CachedPdfEntry>,
    dirty: bool,
}

pub struct ParsedPdfResult {
    pub chunks: Vec<Chunk>,
    pub cache_update: Option<(String, u64, u64)>,
}

impl PdfDiskCache {
    pub fn load() -> Self {
        Self {
            entries: load_disk_cache(),
            dirty: false,
        }
    }

    pub fn lookup(&self, key: &str, len: u64, mtime: u64) -> Option<Vec<Chunk>> {
        self.entries
            .get(key)
            .filter(|entry| entry.file_len == len && entry.modified_secs == mtime)
            .map(|entry| entry.chunks.clone())
    }

    pub fn insert(&mut self, key: String, file_len: u64, modified_secs: u64, chunks: Vec<Chunk>) {
        self.entries.insert(
            key,
            CachedPdfEntry {
                file_len,
                modified_secs,
                chunks,
            },
        );
        self.dirty = true;
    }

    pub fn flush_if_dirty(&mut self) {
        if self.dirty {
            persist_disk_cache(&self.entries);
            self.dirty = false;
        }
    }
}

pub(crate) fn cache_file_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(PDF_CACHE_SUBDIR)
        .join(PDF_CACHE_FILENAME)
}

fn load_disk_cache() -> HashMap<String, CachedPdfEntry> {
    let path = cache_file_path();
    let Ok(bytes) = fs::read(path) else {
        return HashMap::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn persist_disk_cache(cache: &HashMap<String, CachedPdfEntry>) {
    let Ok(json) = serde_json::to_vec(cache) else {
        return;
    };
    let path = cache_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(PRIVATE_DIR_MODE));
        }
    }
    let tmp_path = path.with_file_name(format!(".{PDF_CACHE_FILENAME}.{}.tmp", std::process::id()));
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(PRIVATE_FILE_MODE);
    }
    if let Ok(mut file) = opts.open(&tmp_path) {
        use std::io::Write;
        if file.write_all(&json).is_ok() && file.sync_all().is_ok() {
            let _ = fs::rename(&tmp_path, &path);
            return;
        }
    }
    let _ = fs::remove_file(&tmp_path);
}

fn file_signature(path: &Path) -> (String, u64, u64) {
    let canon = fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string();
    let meta = fs::metadata(path).ok();
    let len = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (canon, len, mtime)
}

fn is_readable_technical_text(text: &str) -> bool {
    let mut alnum = 0usize;
    let mut printable = 0usize;
    let mut total = 0usize;

    for ch in text.chars() {
        total += 1;
        if ch.is_ascii_alphanumeric() {
            alnum += 1;
        }
        if !ch.is_control() || ch == '\n' || ch == '\r' || ch == '\t' {
            printable += 1;
        }
    }

    alnum >= 40 && total > 0 && (printable * 100 / total) >= 80
}

pub fn chunk_pdf_file(
    guard: &QueryGuard,
    subsystem: SubsystemType,
    file_path: &Path,
    cache: &PdfDiskCache,
) -> ParsedPdfResult {
    let (key, len, mtime) = file_signature(file_path);

    if let Some(chunks) = cache.lookup(&key, len, mtime) {
        return ParsedPdfResult {
            chunks,
            cache_update: None,
        };
    }

    let Ok(doc) = lopdf::Document::load(file_path) else {
        return ParsedPdfResult {
            chunks: Vec::new(),
            cache_update: None,
        };
    };

    let doc_title = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Service Manual PDF")
        .replace('_', " ");

    let pages = doc.get_pages();
    let mut page_nums: Vec<u32> = pages.keys().copied().collect();
    page_nums.sort_unstable();

    let mut chunks = Vec::new();
    for page_num in page_nums {
        let Ok(raw_text) = doc.extract_text(&[page_num]) else {
            continue;
        };
        if !is_readable_technical_text(&raw_text) {
            continue;
        }

        let section = format!("Page {}", page_num);
        let hierarchy = vec![doc_title.clone(), section.clone()];
        let target = ChunkFlushTarget {
            file_path,
            doc_title: &doc_title,
            subsystem,
            section_title: &section,
            hierarchy: &hierarchy,
            page: Some(page_num as usize),
        };
        let mut line_buf: Vec<(usize, String)> = Vec::new();
        let mut page_line_num = 0usize;

        for raw_line in raw_text.lines() {
            let trimmed = raw_line.split_whitespace().collect::<Vec<&str>>().join(" ");
            if trimmed.len() < 3 {
                continue;
            }
            page_line_num += 1;
            line_buf.push((page_line_num, trimmed));
            if line_buf.len() >= MAX_PDF_LINES_PER_CHUNK {
                flush_lines_chunk_with_page(
                    &mut chunks,
                    guard,
                    &target,
                    &mut line_buf,
                    page_line_num,
                );
            }
        }

        if !line_buf.is_empty() {
            flush_lines_chunk_with_page(&mut chunks, guard, &target, &mut line_buf, page_line_num);
        }
    }

    ParsedPdfResult {
        chunks,
        cache_update: Some((key, len, mtime)),
    }
}
