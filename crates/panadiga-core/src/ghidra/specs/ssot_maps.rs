//! Embedded Ghidra JSON SSOT loader and per-model memory map projection builder.

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};

use crate::ssot::registry::{HardwareProfileRegistry, ResolvedModelProfile};

use super::regions::memory_region_specs_for_model;

const RAW_GHIDRA_MAPS_SSOT_JSON: &str = include_str!("ghidra_maps_ssot.json");

/// Returns the verbatim embedded base Ghidra SSOT JSON manifest (`ghidra_maps_ssot.json`).
pub fn ghidra_maps_ssot_json() -> &'static str {
    RAW_GHIDRA_MAPS_SSOT_JSON
}

fn opt_hex_u32(val: Option<u32>) -> Value {
    match val {
        Some(v) => Value::String(format!("0x{v:08X}")),
        None => Value::Null,
    }
}

fn build_model_projection(resolved: &ResolvedModelProfile) -> Result<Value> {
    let regions = memory_region_specs_for_model(&resolved.manifest.model_id)?;
    let region_values: Vec<Value> = regions
        .into_iter()
        .map(|r| {
            json!({
                "name": r.name,
                "start": format!("0x{:08X}", r.start),
                "size": format!("0x{:08X}", r.size),
                "read": r.read,
                "write": r.write,
                "execute": r.execute,
                "is_volatile": r.is_volatile,
            })
        })
        .collect();

    Ok(json!({
        "model_id": resolved.manifest.model_id,
        "series": resolved.manifest.series,
        "soc_isa": resolved.soc_core.as_ref().map(|c| c.isa.as_str()),
        "soc_part": resolved.soc.as_ref().map(|s| s.part_number.as_str()),
        "prog_load_va": opt_hex_u32(resolved.firmware_train.as_ref().and_then(|t| t.prog_load_va)),
        "main_load_va": opt_hex_u32(resolved.firmware_train.as_ref().and_then(|t| t.main_load_va)),
        "drv_load_va": opt_hex_u32(resolved.firmware_train.as_ref().and_then(|t| t.drv_load_va)),
        "nvram_cached_va": resolved.nvram_cached_va_hex,
        "nvram_uncached_va": resolved.nvram_uncached_va_hex,
        "regions": region_values,
    }))
}

/// Generates the full enriched Ghidra SSOT JSON manifest including dynamic per-model
/// memory maps projected from [`HardwareProfileRegistry::builtin()`].
pub fn generate_ghidra_ssot_maps_json() -> Result<String> {
    let mut root: Value = serde_json::from_str(RAW_GHIDRA_MAPS_SSOT_JSON)
        .context("failed to parse embedded ghidra_maps_ssot.json")?;
    let obj = root
        .as_object_mut()
        .context("ghidra_maps_ssot.json root must be a JSON object")?;

    let mut models = HardwareProfileRegistry::builtin().list_models();
    models.sort_by(|a, b| a.manifest.model_id.cmp(&b.manifest.model_id));

    let mut model_maps = Map::with_capacity(models.len());
    for resolved in &models {
        let projected = build_model_projection(resolved)?;
        model_maps.insert(resolved.manifest.model_id.clone(), projected);
    }
    obj.insert("model_memory_maps".to_string(), Value::Object(model_maps));

    let mut rendered = serde_json::to_string_pretty(&root)
        .context("failed to serialize enriched ghidra_ssot_maps.json")?;
    rendered.push('\n');
    Ok(rendered)
}
