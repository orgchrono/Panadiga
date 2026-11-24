//! Zero-Error RAG chunk indexer for `profiles/symbols/<train_id>/<module>.json`.

use std::fs;
use std::path::Path;

use panadiga_core::re::symbol_index::{FirmwareSymbolIndex, FunctionSymbolEntry, SymbolSideEffect};

use super::super::guardrails::QueryGuard;
use super::super::models::{Chunk, ChunkMetadata, SubsystemType};
use super::tokenizer::technical_tokenize;

fn format_symbol_line(f: &FunctionSymbolEntry) -> String {
    let callers = if f.callers.is_empty() {
        "-".to_string()
    } else {
        f.callers.join(", ")
    };
    let callees = if f.callees.is_empty() {
        "-".to_string()
    } else {
        f.callees.join(", ")
    };
    let strings = if f.string_xrefs.is_empty() {
        "-".to_string()
    } else {
        f.string_xrefs.join(" | ")
    };
    let mmio = if f.mmio_xrefs.is_empty() {
        "-".to_string()
    } else {
        f.mmio_xrefs.join(", ")
    };
    let reasons = if f.side_effect_reasons.is_empty() {
        String::new()
    } else {
        format!(" Reasons: {}.", f.side_effect_reasons.join("; "))
    };
    let notes = f
        .notes
        .as_deref()
        .map(|n| format!(" Notes: {n}"))
        .unwrap_or_default();

    format!(
        "Function {} ({}) abi={} ret={} side_effects={} callers=[{}] callees=[{}] mmio_xrefs=[{}] strings=[{}].{}{}",
        f.va,
        f.name,
        f.abi,
        f.ret,
        f.side_effects.as_str(),
        callers,
        callees,
        mmio,
        strings,
        reasons,
        notes
    )
}

/// Parses one `FirmwareSymbolIndex` JSON into searchable BM25 + dense vector chunks.
pub(super) fn chunk_symbol_index(guard: &QueryGuard, path: &Path) -> Vec<Chunk> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(idx) = serde_json::from_str::<FirmwareSymbolIndex>(&raw) else {
        return Vec::new();
    };
    let path_str = path.to_string_lossy().to_string();
    let mut chunks = Vec::new();
    let mut readonly_lines = Vec::new();

    for (pos, func) in idx.functions.iter().enumerate() {
        let is_high_signal = func.side_effects != SymbolSideEffect::ReadOnly
            || func.evidence_tier != "static_cfg_inferred"
            || !func.string_xrefs.is_empty();
        let line = format_symbol_line(func);

        if is_high_signal {
            let text = format!(
                "[Firmware Symbol Dictionary] train={} module={} base={} sha256={} :: {}",
                idx.firmware_train_id, idx.module, idx.base_va, idx.sha256, line
            );
            let tokens = technical_tokenize(&text, true);
            let extracted_entities = guard.process(&text).extracted_entities;
            chunks.push(Chunk {
                id: format!(
                    "sym::{}::{}::{}",
                    idx.firmware_train_id, idx.module, func.va
                ),
                text,
                metadata: ChunkMetadata {
                    doc_title: format!("Symbols {}/{}", idx.firmware_train_id, idx.module),
                    file_path: path_str.clone(),
                    section_title: format!("{} @ {}", func.name, func.va),
                    hierarchy: vec![
                        "Profiles".to_string(),
                        "Symbols".to_string(),
                        idx.firmware_train_id.clone(),
                        idx.module.clone(),
                    ],
                    line_start: pos + 1,
                    line_end: pos + 1,
                    page: None,
                    subsystem: SubsystemType::ReverseEngineering,
                    extracted_entities,
                },
                tokens,
            });
        } else {
            readonly_lines.push(line);
        }
    }

    for (batch_idx, batch) in readonly_lines.chunks(16).enumerate() {
        let text = format!(
            "[Firmware Symbol Batch #{}] train={} module={} base={}\n{}",
            batch_idx + 1,
            idx.firmware_train_id,
            idx.module,
            idx.base_va,
            batch.join("\n")
        );
        let tokens = technical_tokenize(&text, true);
        let extracted_entities = guard.process(&text).extracted_entities;
        chunks.push(Chunk {
            id: format!(
                "sym_batch::{}::{}::{}",
                idx.firmware_train_id, idx.module, batch_idx
            ),
            text,
            metadata: ChunkMetadata {
                doc_title: format!("Symbols {}/{}", idx.firmware_train_id, idx.module),
                file_path: path_str.clone(),
                section_title: format!("Read-only symbols batch {}", batch_idx + 1),
                hierarchy: vec![
                    "Profiles".to_string(),
                    "Symbols".to_string(),
                    idx.firmware_train_id.clone(),
                    idx.module.clone(),
                ],
                line_start: 1,
                line_end: idx.functions.len().max(1),
                page: None,
                subsystem: SubsystemType::ReverseEngineering,
                extracted_entities,
            },
            tokens,
        });
    }
    chunks
}
