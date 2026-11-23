//! Hierarchical multi-modal corpus indexer (Markdown docs, Service Manual PDFs, Source Code & Ghidra).

pub mod bm25;
pub mod code;
pub mod markdown;
pub mod pdf;
pub mod ssot_atlas;
pub mod symbols;
pub mod tokenizer;
pub mod vector;
mod visual;

pub use bm25::BM25Index;
pub use tokenizer::{STOPWORDS, technical_tokenize};
pub use vector::VectorIndex;

use panadiga_core::constants::resolve_workspace_root;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::guardrails::QueryGuard;
use super::models::{Chunk, SubsystemType};

enum IndexedBucket {
    Markdown(Vec<Chunk>),
    Pdf {
        chunks: Vec<Chunk>,
        cache_update: Option<(String, u64, u64)>,
    },
    Code(Vec<Chunk>),
    Ghidra(Vec<Chunk>),
    Symbols(Vec<Chunk>),
    Skipped,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorpusIndexStats {
    pub total_chunks: usize,
    pub markdown_chunks: usize,
    pub pdf_chunks: usize,
    pub pdf_files_indexed: usize,
    pub code_chunks: usize,
    pub ghidra_chunks: usize,
    #[serde(default)]
    pub symbol_chunks: usize,
    pub vocab_size: usize,
    pub corpus_root: String,
}

pub struct HierarchicalIndexer {
    pub guard: QueryGuard,
    pub chunks: Vec<Chunk>,
    pub bm25: BM25Index,
    pub vector: VectorIndex,
    pub stats: CorpusIndexStats,
}

impl Default for HierarchicalIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl HierarchicalIndexer {
    pub fn new() -> Self {
        Self {
            guard: QueryGuard::new(),
            chunks: Vec::new(),
            bm25: BM25Index::default(),
            vector: VectorIndex::default(),
            stats: CorpusIndexStats::default(),
        }
    }

    pub fn corpus_stats(&self) -> CorpusIndexStats {
        let mut stats = self.stats.clone();
        stats.total_chunks = self.chunks.len();
        stats.vocab_size = self.vector.vocab.len();
        stats
    }

    pub fn determine_subsystem(&self, file_path: &Path, content: &str) -> SubsystemType {
        visual::classify_subsystem(file_path, content)
    }

    pub fn chunk_markdown_file(&self, file_path: &Path) -> Vec<Chunk> {
        let preview = fs::read_to_string(file_path).unwrap_or_default();
        let subsystem = self.determine_subsystem(file_path, &preview);
        markdown::chunk_markdown_file(&self.guard, subsystem, file_path)
    }

    pub fn build_index_from_directories(&mut self, directories: Option<&[PathBuf]>) -> usize {
        let root = resolve_workspace_root();
        self.stats = CorpusIndexStats {
            corpus_root: root.to_string_lossy().to_string(),
            ..Default::default()
        };

        let mut seen_canonical = HashSet::new();
        let mut candidate_files = Vec::new();

        if let Some(custom_dirs) = directories {
            for d in custom_dirs {
                Self::collect_entry(d, &mut seen_canonical, &mut candidate_files);
            }
        } else {
            let default_entries = [
                root.join("docs"),
                root.join("manuals_and_schematics"),
                root.join("firmware").join("pico-p7501"),
                root.join("tools").join("ghidra"),
                root.join("profiles").join("symbols"),
                root.join("profiles").join("traps"),
                root.join("profiles").join("forensic_atlas"),
                root.join("profiles").join("firmware_trains"),
                root.join("crates"),
                root.join("apps")
                    .join("desktop")
                    .join("src-tauri")
                    .join("src"),
                root.join("apps").join("desktop").join("src"),
                root.join("scripts").join("re"),
                root.join("scripts").join("bench"),
                root.join("AGENTS.md"),
                root.join("README.md"),
            ];
            for entry in &default_entries {
                Self::collect_entry(entry, &mut seen_canonical, &mut candidate_files);
            }
        }

        let mut pdf_cache = pdf::PdfDiskCache::load();
        let per_file: Vec<IndexedBucket> = candidate_files
            .par_iter()
            .map(|path| self.index_single_file(path, &pdf_cache))
            .collect();

        let mut all_chunks = Vec::new();
        for bucket in per_file {
            match bucket {
                IndexedBucket::Markdown(added) => {
                    self.stats.markdown_chunks += added.len();
                    all_chunks.extend(added);
                }
                IndexedBucket::Pdf {
                    chunks: added,
                    cache_update,
                } => {
                    if let Some((key, len, mtime)) = cache_update {
                        pdf_cache.insert(key, len, mtime, added.clone());
                    }
                    if !added.is_empty() {
                        self.stats.pdf_files_indexed += 1;
                        self.stats.pdf_chunks += added.len();
                        all_chunks.extend(added);
                    }
                }
                IndexedBucket::Code(added) => {
                    self.stats.code_chunks += added.len();
                    all_chunks.extend(added);
                }
                IndexedBucket::Ghidra(added) => {
                    self.stats.ghidra_chunks += added.len();
                    all_chunks.extend(added);
                }
                IndexedBucket::Symbols(added) => {
                    self.stats.symbol_chunks += added.len();
                    all_chunks.extend(added);
                }
                IndexedBucket::Skipped => {}
            }
        }

        pdf_cache.flush_if_dirty();
        self.chunks = all_chunks;
        self.reindex_models();
        self.chunks.len()
    }

    fn collect_entry(
        entry: &Path,
        seen_canonical: &mut HashSet<PathBuf>,
        files: &mut Vec<PathBuf>,
    ) {
        if !entry.exists() {
            return;
        }
        if let Ok(canon) = fs::canonicalize(entry)
            && !seen_canonical.insert(canon)
        {
            return;
        }
        if entry.is_file() {
            files.push(entry.to_path_buf());
        } else if entry.is_dir() {
            Self::collect_recursive(entry, files);
        }
    }

    fn index_single_file(&self, path: &Path, pdf_cache: &pdf::PdfDiskCache) -> IndexedBucket {
        let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let path_lower = path.to_string_lossy().to_lowercase();

        if fname.starts_with("AI_GUIDELINES") || fname.starts_with("AUDIT_DOSSIER") {
            return IndexedBucket::Skipped;
        }

        if fname.ends_with(".md") || fname.ends_with(".txt") {
            IndexedBucket::Markdown(self.chunk_markdown_file(path))
        } else if fname == "visual_manifest.json" {
            IndexedBucket::Markdown(self.chunk_visual_manifest(path))
        } else if fname.ends_with(".json")
            && (path_lower.contains("\\symbols\\") || path_lower.contains("/symbols/"))
        {
            IndexedBucket::Symbols(symbols::chunk_symbol_index(&self.guard, path))
        } else if matches!(
            fname,
            "geometry_and_binary_traps.json" | "firmware_forensic_atlas.json"
        ) || (fname.ends_with(".json")
            && (path_lower.contains("\\firmware_trains\\")
                || path_lower.contains("/firmware_trains/")))
        {
            IndexedBucket::Symbols(ssot_atlas::chunk_ssot_traps_or_atlas(&self.guard, path))
        } else if fname.to_lowercase().ends_with(".pdf") {
            let subsystem = self.determine_subsystem(path, "");
            let parsed = pdf::chunk_pdf_file(&self.guard, subsystem, path, pdf_cache);
            IndexedBucket::Pdf {
                chunks: parsed.chunks,
                cache_update: parsed.cache_update,
            }
        } else if code::is_supported_source_file(path)
            && !path_lower.contains("\\tests\\")
            && !path_lower.contains("/tests/")
        {
            let subsystem = self.determine_subsystem(path, "");
            let added = code::chunk_source_file(&self.guard, subsystem, path);
            if path_lower.contains("ghidra") {
                IndexedBucket::Ghidra(added)
            } else {
                IndexedBucket::Code(added)
            }
        } else {
            IndexedBucket::Skipped
        }
    }

    fn reindex_models(&mut self) {
        let doc_refs: Vec<&[String]> = self.chunks.iter().map(|c| c.tokens.as_slice()).collect();
        let bm25 = &mut self.bm25;
        let vector = &mut self.vector;
        rayon::join(|| bm25.fit(&doc_refs), || vector.fit(&doc_refs));
        self.stats.total_chunks = self.chunks.len();
        self.stats.vocab_size = self.vector.vocab.len();
    }

    fn collect_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
        let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(
            dir_name,
            "target"
                | "node_modules"
                | "dist"
                | ".git"
                | "tests"
                | "test"
                | "mocks"
                | "fixtures"
                | "e2e"
                | "__pycache__"
        ) {
            return;
        }

        if let Ok(entries) = fs::read_dir(dir) {
            let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();

            for path in paths {
                if path.is_dir() {
                    Self::collect_recursive(&path, files);
                } else if path.is_file() {
                    files.push(path);
                }
            }
        }
    }

    /// Indexes curated DMR-EH55 schematic & PCB PNG metadata from `docs/assets/visual_manifest.json`.
    fn chunk_visual_manifest(&self, path: &Path) -> Vec<Chunk> {
        visual::chunk_visual_manifest(&self.guard, path)
    }

    pub fn add_custom_chunks(&mut self, chunks: Vec<Chunk>) {
        self.chunks.extend(chunks);
        self.reindex_models();
    }
}
