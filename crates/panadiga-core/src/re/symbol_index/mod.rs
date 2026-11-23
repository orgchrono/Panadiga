#![deny(unsafe_code)]

//! Deterministic Function, Caller, ABI, Xref, and Side-Effect Dictionary for MN103S Firmware.
//!
//! Indexed by `(firmware_train_id, module, sha256, base_va)` under `profiles/symbols/` and
//! integrated directly into CLI (`panadiga re index-symbols`), MCP (`panadiga_symbol_lookup`,
//! `panadiga://symbols/<train_id>/<module>`), and the Zero-Error RAG Engine safety shield.

mod analyzer;
mod store;
#[cfg(test)]
mod tests;
mod types;

pub use analyzer::{build_symbol_index, format_va_hex, parse_va_hex};
pub use store::{
    builtin_overlays_for, list_symbol_indexes, load_all_destructive_symbols, load_symbol_index,
    lookup_symbols, resolve_default_symbol_output_path, symbols_root, write_symbol_index,
};
pub use types::{
    FirmwareSymbolIndex, FunctionSymbolEntry, SymbolAbiDetail, SymbolAnnotationOverlay,
    SymbolLookupMatch, SymbolLookupQuery, SymbolLookupResponse, SymbolLookupSafetyGate,
    SymbolSideEffect,
};
