//! MCP tool handler for `panadiga_symbol_lookup` (train-keyed Function/Caller/Side-Effect index).

use std::path::Path;

use anyhow::Result;
use panadiga_core::re::symbol_index::{SymbolLookupQuery, SymbolSideEffect, lookup_symbols};
use serde_json::Value;

use crate::workspace::optional_non_empty_str;

/// Queries `profiles/symbols/<train>/<module>.json` with automatic unattended-call safety gating.
pub fn handle_symbol_lookup(args: &Value, workspace_root: &Path) -> Result<Value> {
    let side_effect = optional_non_empty_str(args, "side_effect")
        .map(str::parse::<SymbolSideEffect>)
        .transpose()?;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(1, 200) as usize);

    let query = SymbolLookupQuery {
        train_id: optional_non_empty_str(args, "train_id").map(str::to_string),
        module: optional_non_empty_str(args, "module").map(str::to_string),
        va: optional_non_empty_str(args, "va").map(str::to_string),
        name: optional_non_empty_str(args, "name").map(str::to_string),
        caller_of: optional_non_empty_str(args, "caller_of").map(str::to_string),
        callee_of: optional_non_empty_str(args, "callee_of").map(str::to_string),
        side_effect,
        string_xref: optional_non_empty_str(args, "string_xref").map(str::to_string),
        mmio_xref: optional_non_empty_str(args, "mmio_xref").map(str::to_string),
        limit,
    };

    let effective_root = if workspace_root.join("profiles").is_dir() {
        workspace_root.to_path_buf()
    } else {
        crate::workspace::resolve_workspace_root()
    };
    let response = lookup_symbols(&effective_root, &query)?;
    Ok(serde_json::to_value(response)?)
}
