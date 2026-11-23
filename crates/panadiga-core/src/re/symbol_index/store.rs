#![deny(unsafe_code)]

//! Curated overlays, disk persistence (`profiles/symbols/<train_id>/<module>.json`),
//! and safety-gated query engine for [`super::types::FirmwareSymbolIndex`].

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::analyzer::{format_va_hex, parse_va_hex};
use super::types::{
    FirmwareSymbolIndex, FunctionSymbolEntry, SymbolAnnotationOverlay, SymbolLookupMatch,
    SymbolLookupQuery, SymbolLookupResponse, SymbolLookupSafetyGate, SymbolSideEffect,
};

fn validate_safe_slug(raw: &str, field: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        bail!("Invalid symbol {field} identifier '{raw}'");
    }
    Ok(trimmed.to_string())
}

/// Canonical folder containing checked-in train/module symbol JSON dictionaries.
#[must_use]
pub fn symbols_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join("profiles").join("symbols")
}

/// Resolves `profiles/symbols/<train_id>/<module>.json` safely with traversal rejection.
pub fn resolve_default_symbol_output_path(
    workspace_root: &Path,
    firmware_train_id: &str,
    module: &str,
) -> Result<PathBuf> {
    let train = validate_safe_slug(firmware_train_id, "firmware_train_id")?;
    let mod_slug = validate_safe_slug(module, "module")?;
    Ok(symbols_root(workspace_root)
        .join(train)
        .join(format!("{mod_slug}.json")))
}

/// Writes a [`FirmwareSymbolIndex`] deterministically with parent directory creation.
pub fn write_symbol_index(index: &FirmwareSymbolIndex, out_path: &Path) -> Result<()> {
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Creating symbol index dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(index)?;
    fs::write(out_path, format!("{json}\n"))
        .with_context(|| format!("Writing symbol index {}", out_path.display()))
}

/// Lists all available `(firmware_train_id, module, path)` symbol dictionaries under `profiles/symbols/`.
#[must_use]
pub fn list_symbol_indexes(workspace_root: &Path) -> Vec<(String, String, PathBuf)> {
    let root = symbols_root(workspace_root);
    let Ok(train_dirs) = fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in train_dirs.flatten() {
        let train_path = entry.path();
        if !train_path.is_dir() {
            continue;
        }
        let Some(train_name) = train_path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if validate_safe_slug(train_name, "firmware_train_id").is_err() {
            continue;
        }
        let Ok(mod_files) = fs::read_dir(&train_path) else {
            continue;
        };
        for m_entry in mod_files.flatten() {
            let m_path = m_entry.path();
            if m_path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = m_path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if validate_safe_slug(stem, "module").is_ok() {
                out.push((train_name.to_string(), stem.to_string(), m_path));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    out
}

/// Loads one [`FirmwareSymbolIndex`] by `(firmware_train_id, module)`.
pub fn load_symbol_index(
    workspace_root: &Path,
    train_id: &str,
    module: &str,
) -> Result<FirmwareSymbolIndex> {
    let path = resolve_default_symbol_output_path(workspace_root, train_id, module)?;
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Reading symbol dictionary {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("Parsing symbol dictionary {}", path.display()))
}

fn norm_opt_va(raw: Option<&String>) -> Result<Option<String>> {
    raw.map(|s| parse_va_hex(s).map(format_va_hex)).transpose()
}

fn entry_matches(
    entry: &FunctionSymbolEntry,
    query: &SymbolLookupQuery,
    va_f: Option<&str>,
    caller_of_f: Option<&str>,
    callee_of_f: Option<&str>,
) -> bool {
    if let Some(target_va) = va_f
        && entry.va != target_va
    {
        return false;
    }
    if let Some(name_q) = &query.name {
        let needle = name_q.trim().to_ascii_lowercase();
        if !needle.is_empty() && !entry.name.to_ascii_lowercase().contains(&needle) {
            return false;
        }
    }
    if let Some(c_target) = caller_of_f
        && !entry.callees.iter().any(|c| c == c_target)
    {
        return false;
    }
    if let Some(c_source) = callee_of_f
        && !entry.callers.iter().any(|c| c == c_source)
    {
        return false;
    }
    if let Some(eff) = query.side_effect
        && entry.side_effects != eff
    {
        return false;
    }
    if let Some(str_q) = &query.string_xref {
        let needle = str_q.trim().to_ascii_lowercase();
        if !needle.is_empty()
            && !entry
                .string_xrefs
                .iter()
                .any(|s| s.to_ascii_lowercase().contains(&needle))
        {
            return false;
        }
    }
    if let Some(mmio_q) = &query.mmio_xref {
        let needle = mmio_q.trim().to_ascii_uppercase();
        if !needle.is_empty()
            && !entry
                .mmio_xrefs
                .iter()
                .any(|m| m.to_ascii_uppercase().contains(&needle))
        {
            return false;
        }
    }
    true
}

/// Queries checked-in `profiles/symbols/` dictionaries and computes an unattended-call safety gate.
pub fn lookup_symbols(
    workspace_root: &Path,
    query: &SymbolLookupQuery,
) -> Result<SymbolLookupResponse> {
    let va_f = norm_opt_va(query.va.as_ref())?;
    let caller_of_f = norm_opt_va(query.caller_of.as_ref())?;
    let callee_of_f = norm_opt_va(query.callee_of.as_ref())?;
    let train_filter = query
        .train_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let mod_filter = query
        .module
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let mut matches = Vec::new();
    for (train, module, path) in list_symbol_indexes(workspace_root) {
        if let Some(tf) = train_filter
            && !train.eq_ignore_ascii_case(tf)
        {
            continue;
        }
        if let Some(mf) = mod_filter
            && !module.eq_ignore_ascii_case(mf)
        {
            continue;
        }
        let Some(idx) = load_symbol_index_file(&path) else {
            continue;
        };
        for func in idx.functions {
            if entry_matches(
                &func,
                query,
                va_f.as_deref(),
                caller_of_f.as_deref(),
                callee_of_f.as_deref(),
            ) {
                matches.push(SymbolLookupMatch {
                    firmware_train_id: idx.firmware_train_id.clone(),
                    module: idx.module.clone(),
                    base_va: idx.base_va.clone(),
                    symbol: func,
                });
            }
        }
    }

    let total_matches = matches.len();
    let destructive_matches = matches
        .iter()
        .filter(|m| m.symbol.side_effects.is_destructive())
        .count();
    let blocked = destructive_matches > 0;
    let warning = blocked.then(|| {
        format!(
            "SAFETY GATE BLOCKED FOR UNATTENDED CALL: {destructive_matches} matched function(s) mutate persistent NOR/BGO, NVRAM, or HDD state (`writes_nor_bgo` / `writes_nvram` / `writes_hdd`). Use guarded `panadiga_bgo_unbrick` / CoW verification instead of invoking directly."
        )
    });
    matches.truncate(limit);

    Ok(SymbolLookupResponse {
        total_matches,
        safety_gate: SymbolLookupSafetyGate {
            blocked_for_unattended_call: blocked,
            destructive_matches,
            warning,
        },
        matches,
    })
}

fn load_symbol_index_file(path: &Path) -> Option<FirmwareSymbolIndex> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<FirmwareSymbolIndex>(&raw).ok()
}

/// Loads all destructive symbol entries across checked-in train/module indexes for RAG safety checks.
#[must_use]
pub fn load_all_destructive_symbols(
    workspace_root: &Path,
) -> Vec<(String, String, FunctionSymbolEntry)> {
    let mut out = Vec::new();
    for (train, module, path) in list_symbol_indexes(workspace_root) {
        let Some(idx) = load_symbol_index_file(&path) else {
            continue;
        };
        for func in idx.functions {
            if func.side_effects.is_destructive() {
                out.push((train.clone(), module.clone(), func));
            }
        }
    }
    out
}

/// Returns built-in verified annotations grounded in Ghidra pseudocode & bench logs.
#[must_use]
pub fn builtin_overlays_for(train_id: &str, module: &str) -> Vec<SymbolAnnotationOverlay> {
    if train_id.eq_ignore_ascii_case("loader_ver061_toc48_main57")
        && module.eq_ignore_ascii_case("PROG061")
    {
        return prog061_ver061_overlays();
    }
    Vec::new()
}

fn overlay_entry(
    spec: (
        &str,
        Option<u32>,
        &str,
        &str,
        &str,
        SymbolSideEffect,
        &str,
        &str,
    ),
) -> SymbolAnnotationOverlay {
    let (va, size_bytes, name, abi, ret, side_effects, evidence_tier, notes) = spec;
    SymbolAnnotationOverlay {
        va: va.to_string(),
        size_bytes,
        name: Some(name.to_string()),
        abi: Some(abi.to_string()),
        ret: Some(ret.to_string()),
        side_effects: Some(side_effects),
        evidence_tier: Some(evidence_tier.to_string()),
        notes: Some(notes.to_string()),
    }
}

fn prog061_ver061_overlays() -> Vec<SymbolAnnotationOverlay> {
    [
        (
            "0x50401F3C",
            Some(483),
            "p61_nfrom_format_orphan_1f3c",
            "D0/D1",
            "D0 (u32)",
            SymbolSideEffect::WritesNorBgo,
            "ghidra_verified",
            "BGO NFROM format routine @ 0x50401F3C (size=483 B in doc_prog061).",
        ),
        (
            "0x50404592",
            None,
            "p61_bgo_nor_driver_init_4592",
            "D0/D1/(0x0C,SP)",
            "D0 (u32)",
            SymbolSideEffect::WritesNorBgo,
            "ghidra_verified",
            "Initializes Spansion Upper NOR BGO FTL driver against 0x84380000 partition.",
        ),
        (
            "0x50404FA7",
            Some(1126),
            "p61_main_menu_4fa7",
            "void",
            "void",
            SymbolSideEffect::WritesNorBgo,
            "bench_verified",
            "DIGADIAG PROG061 main orchestrator: calls 0x5040A294 (BGO init) and flag clearers 0x50409DF2 / 0x50409E65.",
        ),
        (
            "0x5040689D",
            Some(32),
            "p61_u99_fatal_halt_689d",
            "void",
            "void",
            SymbolSideEffect::ReadOnly,
            "bench_verified",
            "Enters U99 fatal wait loop (while iRam504f00f4 == 0) when BGO/TBC init fails.",
        ),
        (
            "0x50409DF2",
            None,
            "p61_bgo_flag_clear_9df2",
            "void",
            "void",
            SymbolSideEffect::WritesNorBgo,
            "bench_verified",
            "Mutates persistent BGO state / NOR flags via 0x504800a6..0x5048009d (requires iRam504802b0 == 1); block unattended invocation.",
        ),
        (
            "0x50409E65",
            None,
            "p61_bgo_flag_clear_9e65",
            "void",
            "void",
            SymbolSideEffect::WritesNorBgo,
            "bench_verified",
            "Companion BGO NOR flag mutation cleaner invoked after 0x50409DF2; block unattended invocation.",
        ),
        (
            "0x5040A294",
            Some(130),
            "p61_bgo_init_hook_a294",
            "void",
            "D0 (u32)",
            SymbolSideEffect::WritesNorBgo,
            "bench_verified",
            "Calls 0x50404592 at 0x84380000; fails U99 unless patched by panadiga_bgo_unbrick / CoW BGO plan.",
        ),
        (
            "0x5040A3F6",
            None,
            "p61_bgo_guard_a3f6",
            "D0/D1/(0x0C,SP)",
            "D0 (u32)",
            SymbolSideEffect::WritesNorBgo,
            "ghidra_verified",
            "Guards BGO driver initialization flag iRam504802b0 ('BGO driver no init.').",
        ),
        (
            "0x5040A90B",
            None,
            "p61_bgo_guard_a90b",
            "D0/D1/(0x0C,SP)",
            "D0 (u32)",
            SymbolSideEffect::WritesNorBgo,
            "ghidra_verified",
            "Secondary BGO NOR write wrapper checking iRam504802b0 ('BGO driver no init.').",
        ),
    ]
    .into_iter()
    .map(overlay_entry)
    .collect()
}
