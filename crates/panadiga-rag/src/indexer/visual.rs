//! Subsystem classification and curated `visual_manifest.json` chunk indexing.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::super::guardrails::QueryGuard;
use super::super::models::{Chunk, ChunkMetadata, SubsystemType};
use super::tokenizer::technical_tokenize;

/// Classifies a technical corpus document or source file by subsystem domain.
pub fn classify_subsystem(file_path: &Path, content: &str) -> SubsystemType {
    let path_lower = file_path.to_string_lossy().to_lowercase();
    let filename_lower = file_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_lowercase();
    let content_lower = content.to_lowercase();

    if filename_lower.contains("p7501")
        || filename_lower.contains("hardware")
        || filename_lower.contains("esquema")
        || filename_lower.contains("schematic")
        || path_lower.contains("pico-p7501")
    {
        SubsystemType::Hardware
    } else if filename_lower.contains("meihdfs")
        || filename_lower.contains("hdd")
        || filename_lower.contains("alocacao")
    {
        SubsystemType::Storage
    } else if filename_lower.contains("frm")
        || filename_lower.contains("crypto")
        || filename_lower.contains("aes")
    {
        SubsystemType::Crypto
    } else if filename_lower.contains("ghidra")
        || filename_lower.contains("gdb")
        || filename_lower.contains("reverse")
        || path_lower.contains("ghidra")
    {
        SubsystemType::ReverseEngineering
    } else if content_lower.contains("boot")
        || content_lower.contains("ipl")
        || content_lower.contains("flash")
        || filename_lower.contains("nvram")
    {
        SubsystemType::Firmware
    } else {
        SubsystemType::General
    }
}

/// Indexes curated DMR-EH55 schematic & PCB PNG metadata from `docs/assets/visual_manifest.json`.
pub(super) fn chunk_visual_manifest(guard: &QueryGuard, path: &Path) -> Vec<Chunk> {
    #[derive(Deserialize)]
    struct VisualEntry {
        id: String,
        filename: String,
        repo_path: String,
        #[serde(default)]
        aliases: Vec<String>,
        #[serde(default)]
        manual_edition: String,
        page_number: Option<u32>,
        #[serde(default)]
        category: String,
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default)]
        caption: String,
    }
    #[derive(Deserialize)]
    struct VisualManifest {
        #[serde(default)]
        images: Vec<VisualEntry>,
    }

    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_str::<VisualManifest>(&raw) else {
        return Vec::new();
    };
    let path_str = path.to_string_lossy().to_string();
    let mut chunks = Vec::with_capacity(manifest.images.len());
    for (idx, item) in manifest.images.into_iter().enumerate() {
        let page_str = item
            .page_number
            .map(|p| p.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let aliases = if item.aliases.is_empty() {
            String::new()
        } else {
            format!(" Aliases: {}.", item.aliases.join(", "))
        };
        let tags = if item.tags.is_empty() {
            String::new()
        } else {
            format!(" Tags: {}.", item.tags.join(", "))
        };
        let text = format!(
            "[Visual Asset Catalog] {} ({}) — {} p.{}, Categoria: {}.{} {} {} Path: {}",
            item.filename,
            item.id,
            item.manual_edition,
            page_str,
            item.category,
            aliases,
            tags,
            item.caption,
            item.repo_path
        );
        let tokens = technical_tokenize(&text, true);
        let extracted_entities = guard.process(&text).extracted_entities;
        chunks.push(Chunk {
            id: format!("visual_manifest::{}", item.id),
            text,
            tokens,
            metadata: ChunkMetadata {
                doc_title: "Visual Asset Catalog (DMR-EH55 Service Manuals)".to_string(),
                file_path: path_str.clone(),
                section_title: format!("{} (p.{})", item.filename, page_str),
                hierarchy: vec!["Visual Catalog".to_string(), item.category, item.filename],
                line_start: idx + 1,
                line_end: idx + 1,
                subsystem: SubsystemType::Hardware,
                extracted_entities,
            },
        });
    }
    chunks
}
