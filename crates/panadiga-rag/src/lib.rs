//! Panadiga Zero-Error RAG Engine (Native Rust Implementation)
//!
//! Replaces legacy Python RAG with a deterministic, memory-safe, zero-copy,
//! 6-layer defense engine executing in sub-milliseconds without external runtime dependencies.

pub mod config;
pub mod critic;
pub mod generator;
pub mod guardrails;
pub mod indexer;
pub mod models;
pub mod pipeline;
pub mod retriever;
pub mod safety;

#[cfg(test)]
mod tests;

pub use config::*;
pub use critic::*;
pub use generator::*;
pub use guardrails::*;
pub use indexer::*;
pub use models::*;
pub use pipeline::*;
pub use retriever::*;
pub use safety::*;

use std::sync::LazyLock;

static GLOBAL_ENGINE: LazyLock<ZeroErrorRagEngine> = LazyLock::new(|| {
    let mut engine = ZeroErrorRagEngine::new();
    engine.initialize();
    engine
});

/// Scoped immutable access helper over the shared lock-free Zero-Error RAG Engine.
pub fn with_global_engine<R>(f: impl FnOnce(&ZeroErrorRagEngine) -> R) -> R {
    f(&GLOBAL_ENGINE)
}

/// Convenience helper querying the lock-free shared Zero-Error RAG Engine.
pub fn query(text: &str) -> RagAnswer {
    with_global_engine(|engine| engine.query(text))
}

/// Returns corpus index statistics across Markdown docs, PDF manuals, source code, and Ghidra scripts.
pub fn corpus_stats() -> CorpusIndexStats {
    with_global_engine(ZeroErrorRagEngine::corpus_stats)
}

/// Performs filtered hybrid retrieval across specific source classes (PDF manuals, source code, Ghidra, docs).
pub fn retrieve_filtered(
    text: &str,
    filter: ChunkSourceFilter,
    top_k: usize,
) -> Vec<RetrievalResult> {
    with_global_engine(|engine| engine.retrieve_filtered(text, filter, top_k))
}
