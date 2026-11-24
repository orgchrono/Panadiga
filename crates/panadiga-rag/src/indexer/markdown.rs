//! Shared line-buffered chunk emitter and hierarchical Markdown/text document chunker.

use std::fs;
use std::path::Path;

use super::super::guardrails::QueryGuard;
use super::super::models::{Chunk, ChunkMetadata, SubsystemType};
use super::tokenizer::technical_tokenize;

pub(crate) struct ChunkFlushTarget<'a> {
    pub file_path: &'a Path,
    pub doc_title: &'a str,
    pub subsystem: SubsystemType,
    pub section_title: &'a str,
    pub hierarchy: &'a [String],
    pub page: Option<usize>,
}

pub(crate) fn flush_lines_chunk_with_page(
    chunks: &mut Vec<Chunk>,
    guard: &QueryGuard,
    target: &ChunkFlushTarget<'_>,
    lines_buf: &mut Vec<(usize, String)>,
    end_line: usize,
) {
    if lines_buf.is_empty() {
        return;
    }
    let text_block = lines_buf
        .iter()
        .map(|(_, l)| l.as_str())
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_string();

    if text_block.len() >= 30 {
        let start_line = lines_buf[0].0;
        let stem = target
            .file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("doc");
        let cid = match target.page {
            Some(page_num) => format!("{stem}::P{page_num}::L{start_line}-L{end_line}"),
            None => format!("{stem}::L{start_line}-L{end_line}"),
        };

        let meta_query = guard.process(&text_block);

        let meta = ChunkMetadata {
            doc_title: target.doc_title.to_string(),
            file_path: target.file_path.to_string_lossy().to_string(),
            section_title: target.section_title.to_string(),
            hierarchy: target.hierarchy.to_vec(),
            line_start: start_line,
            line_end: end_line,
            page: target.page,
            subsystem: target.subsystem,
            extracted_entities: meta_query.extracted_entities,
        };

        let tokens = technical_tokenize(&text_block, true);
        chunks.push(Chunk {
            id: cid,
            text: text_block,
            metadata: meta,
            tokens,
        });
    }
    lines_buf.clear();
}

#[allow(clippy::too_many_arguments)]
pub fn flush_lines_chunk(
    chunks: &mut Vec<Chunk>,
    guard: &QueryGuard,
    file_path: &Path,
    doc_title: &str,
    subsystem: SubsystemType,
    current_section: &str,
    current_hierarchy: &[String],
    lines_buf: &mut Vec<(usize, String)>,
    end_line: usize,
) {
    let target = ChunkFlushTarget {
        file_path,
        doc_title,
        subsystem,
        section_title: current_section,
        hierarchy: current_hierarchy,
        page: None,
    };
    flush_lines_chunk_with_page(chunks, guard, &target, lines_buf, end_line);
}

pub fn chunk_markdown_file(
    guard: &QueryGuard,
    subsystem: SubsystemType,
    file_path: &Path,
) -> Vec<Chunk> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let doc_title = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Document")
        .replace('_', " ");

    let mut current_hierarchy = vec![doc_title.clone()];
    let mut current_section = "Introduction".to_string();
    let mut current_lines: Vec<(usize, String)> = Vec::new();

    for (line_idx_zero, &line) in lines.iter().enumerate() {
        let line_idx = line_idx_zero + 1;
        let stripped = line.trim();

        if stripped.starts_with('#') {
            if !current_lines.is_empty() {
                flush_lines_chunk(
                    &mut chunks,
                    guard,
                    file_path,
                    &doc_title,
                    subsystem,
                    &current_section,
                    &current_hierarchy,
                    &mut current_lines,
                    line_idx - 1,
                );
            }

            let level = stripped.chars().take_while(|&c| c == '#').count();
            let heading_text = stripped.trim_start_matches('#').trim().to_string();
            current_section = heading_text.clone();

            if level == 1 {
                current_hierarchy = vec![heading_text];
            } else if level == 2 {
                let first = current_hierarchy
                    .first()
                    .cloned()
                    .unwrap_or_else(|| doc_title.clone());
                current_hierarchy = vec![first, heading_text];
            } else if current_hierarchy.len() >= 2 {
                current_hierarchy = vec![
                    current_hierarchy[0].clone(),
                    current_hierarchy[1].clone(),
                    heading_text,
                ];
            } else {
                current_hierarchy.push(heading_text);
            }

            current_lines.push((line_idx, line.to_string()));
        } else {
            current_lines.push((line_idx, line.to_string()));
            if current_lines.len() >= 80 {
                flush_lines_chunk(
                    &mut chunks,
                    guard,
                    file_path,
                    &doc_title,
                    subsystem,
                    &current_section,
                    &current_hierarchy,
                    &mut current_lines,
                    line_idx,
                );
            }
        }
    }

    if !current_lines.is_empty() {
        flush_lines_chunk(
            &mut chunks,
            guard,
            file_path,
            &doc_title,
            subsystem,
            &current_section,
            &current_hierarchy,
            &mut current_lines,
            lines.len(),
        );
    }

    chunks
}
