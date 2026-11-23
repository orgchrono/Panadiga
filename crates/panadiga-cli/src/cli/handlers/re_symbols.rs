#![deny(unsafe_code)]

//! CLI handler for `panadiga re index-symbols` generating `profiles/symbols/<train>/<module>.json`.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use panadiga_core::constants::resolve_workspace_root;
use panadiga_core::log_println;
use panadiga_core::re::symbol_index::{
    SymbolAnnotationOverlay, build_symbol_index, builtin_overlays_for,
    resolve_default_symbol_output_path, write_symbol_index,
};
use panadiga_core::ssot::HardwareProfileRegistry;

use super::re::load_binary_and_base;
use super::unbrick::render::emit_json_if_requested;

/// Generates a deterministic train-keyed symbol/caller/side-effect index JSON file.
#[allow(clippy::too_many_arguments)]
pub fn handle_re_index_symbols(
    input: &std::path::Path,
    train: &str,
    module: &str,
    base: Option<&str>,
    annotations: Option<&std::path::Path>,
    write_profile: bool,
    out: Option<&std::path::Path>,
    json: Option<&std::path::Path>,
) -> Result<()> {
    let reg = HardwareProfileRegistry::builtin();
    if !reg.bundle().firmware_trains.contains_key(train.trim()) {
        bail!(
            "Unknown firmware_train_id '{}' (must exist in profiles/firmware_trains/; never use commercial model names)",
            train.trim()
        );
    }

    let (data, base_va) = load_binary_and_base(input, base)?;
    let mut overlays = builtin_overlays_for(train, module);
    if let Some(ann_path) = annotations {
        let raw_ann = fs::read_to_string(ann_path)
            .with_context(|| format!("Reading annotations overlay {}", ann_path.display()))?;
        let extra: Vec<SymbolAnnotationOverlay> = serde_json::from_str(&raw_ann)
            .with_context(|| format!("Parsing annotations overlay {}", ann_path.display()))?;
        overlays.extend(extra);
    }

    let index = build_symbol_index(&data, train.trim(), module.trim(), base_va, &overlays);
    let target_out: Option<PathBuf> = if let Some(explicit) = out {
        Some(explicit.to_path_buf())
    } else if write_profile {
        let root = resolve_workspace_root();
        Some(resolve_default_symbol_output_path(
            &root,
            &index.firmware_train_id,
            &index.module,
        )?)
    } else {
        None
    };

    if let Some(ref dest) = target_out {
        write_symbol_index(&index, dest)?;
    }

    if json.is_none() {
        log_println!(
            "[OK] Indexed {} functions ({} destructive) for {}/{} base={} sha256={}{}",
            index.function_count,
            index.destructive_function_count,
            index.firmware_train_id,
            index.module,
            index.base_va,
            &index.sha256[..16],
            target_out
                .as_ref()
                .map(|p| format!(" -> {}", p.display()))
                .unwrap_or_default()
        );
    }
    emit_json_if_requested(&index, json)
}
