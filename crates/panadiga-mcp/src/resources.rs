//! MCP Resources (`resources/list`, `resources/read`) exposing Panadiga SSOT specs and documentation.

use anyhow::{Result, bail};
use panadiga_core::constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_IPL_SIZE, AM34_FLASH_SIZE,
    AM34_FLASH_UNCACHED_BASE, AM34_FORBIDDEN_LETHAL_REG, AM34_NVRAM_BASE,
    AM34_SYSTEM_MEMORY_REGIONS, SPANSION_NVRAM_SIZE_BYTES,
};
use panadiga_core::re::symbol_index::{list_symbol_indexes, load_symbol_index};
use panadiga_core::ssot::{
    HardwareProfileRegistry, build_hardware_ssot_spec, get_diga_model_catalog_dto,
};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

/// Hard payload ceiling (`32 KiB`) for single-shot MCP `resources/read` responses to prevent token exhaustion.
pub const MAX_MCP_RESOURCE_BYTES: usize = 32 * 1024;

pub(crate) fn truncate_mcp_text_to_cap(text: &str, max_bytes: usize, uri: &str) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let notice = format!(
        "\n\n[TRUNCATED AT {} KiB ({} bytes total) for {} — do NOT dump huge logs/pseudocode via resources/read. Use `panadiga_search_code`, `panadiga_search_visuals`, or `panadiga_rag_query` for targeted retrieval.]\n",
        max_bytes / 1024,
        text.len(),
        uri
    );
    let keep_budget = max_bytes.saturating_sub(notice.len());
    let mut end = keep_budget.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &text[..end], notice)
}

fn collect_markdown_docs(docs_dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(docs_dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();

    for p in paths {
        if p.is_dir() {
            collect_markdown_docs(&p, out);
        } else if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("md") {
            out.push(p);
        }
    }
}

pub fn list_resources(workspace_root: &Path) -> Vec<Value> {
    let mut resources = vec![
        json!({
            "uri": "panadiga://invariants",
            "name": "Physical & Forensic Safety Invariants",
            "description": "Voltage caps (3.3V max LVTTL / NSW5V GP3 Hi-Z), P7501 power rules, Spansion Flash/IPL lockouts, MEIHFS raw image immutability.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "panadiga://memory-map",
            "name": "Matsushita AM34 / MN103S Physical Memory Map",
            "description": "Authoritative bus regions for SRAM, SDRAM, Spansion NOR Flash, IPL bootloader, NVRAM U99, ASIC & PATA MMIO.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "panadiga://pinout-p7501-uart",
            "name": "P7501 (TO WRITE JIGU) Async UART 115200 8N1 & RP2040/RP2350 Wiring",
            "description": "Primary DMR-EH55 P7501 discrete buffer mapping (SW_UART, XRESET, UARTM2P_T/Q7514, GND, NSW5V Hi-Z ONLY, UARTP2M_T) plus voltage limits.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "panadiga://visual-catalog",
            "name": "DMR-EH55 Curated Visual Catalog (238 PNGs)",
            "description": "Structured JSON index (docs/assets/visual_manifest.json) of all 238 curated schematic, PCB foil, and vector CAD PNG crops.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "panadiga://models-catalog",
            "name": "Panasonic DIGA Hardware Catalog",
            "description": "Supported DIGA models, Flash chips, NVRAM offsets, and SoC profiles.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "panadiga://pico-commands",
            "name": "PicoFW P7501 Async UART Serial Protocol Command Catalog",
            "description": "Complete command set implemented by firmware/picofw (PING, STATUS, PINCHECK, BUSDIAG, BOOTCAP, JIGBOOT, UARTMON, UARTSEND, ISOLATE, LIVE_CAPTURE, BRIDGE, GDB RSP).",
            "mimeType": "application/json"
        }),
    ];

    let effective_root = crate::workspace::resolve_effective_workspace_for(workspace_root, "docs");
    let docs_root = effective_root.join("docs");
    let mut md_files = Vec::new();
    for resolved in HardwareProfileRegistry::builtin().list_models() {
        resources.push(json!({
            "uri": format!("panadiga://profiles/{}", resolved.manifest.model_id),
            "name": format!("DIGA Profile: {}", resolved.manifest.model_id),
            "description": format!(
                "Resolved hardware/firmware/layout manifest for {} ({})",
                resolved.manifest.model_id, resolved.manifest.series
            ),
            "mimeType": "application/json"
        }));
    }
    for (train_id, module, _) in list_symbol_indexes(&effective_root) {
        resources.push(json!({
            "uri": format!("panadiga://symbols/{train_id}/{module}"),
            "name": format!("Symbols: {train_id}/{module}"),
            "description": format!(
                "Train-keyed function, ABI, caller/callee, xref & side-effect dictionary for {train_id} ({module})."
            ),
            "mimeType": "application/json"
        }));
    }
    collect_markdown_docs(&docs_root, &mut md_files);

    for path in md_files.into_iter().take(250) {
        if let Ok(rel) = path.strip_prefix(&effective_root) {
            let rel_posix = rel.to_string_lossy().replace('\\', "/");
            let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("doc")
                .replace('_', " ");
            let size_note = if size_bytes > MAX_MCP_RESOURCE_BYTES as u64 {
                format!(" ({size_bytes} B; truncated @ 32 KiB — prefer rag/search)")
            } else {
                format!(" ({size_bytes} B)")
            };
            resources.push(json!({
                "uri": format!("panadiga://{}", rel_posix),
                "name": title,
                "description": format!("Technical dossier: {}{}", rel_posix, size_note),
                "mimeType": "text/markdown"
            }));
        }
    }

    resources
}

pub fn read_resource(uri: &str, workspace_root: &Path) -> Result<Value> {
    match uri {
        "panadiga://invariants" => {
            let spec = build_hardware_ssot_spec();
            let payload = json!({
                "max_nominal_voltage_v": spec.thresholds.nominal_vdd_volts,
                "max_safe_voltage_v": spec.thresholds.max_vdd_ceiling_volts,
                "min_safe_voltage_v": spec.thresholds.safe_operating_min_volts,
                "min_vdd_threshold_v": spec.thresholds.min_vdd_threshold_volts,
                "p7501_never_power_pico": true,
                "pin1_sw_uart_gp26_max_ma": 10,
                "pin2_xreset_gp5_open_drain_only": true,
                "pin5_nsw5v_gp3_hiz_only": true,
                "pin6_uartp2m_rx_gp2": true,
                "spansion_bus_base": format!("0x{:08X}", AM34_FLASH_BASE),
                "spansion_uncached_bus_base": format!("0x{:08X}", AM34_FLASH_UNCACHED_BASE),
                "spansion_total_bytes": AM34_FLASH_SIZE,
                "ipl_layer1_ro_bytes": AM34_FLASH_CRITICAL_IPL_SIZE,
                "ipl_layer1_ro_sectors": "SA0..SA10 (8x8K + 3x64K = 256 KiB permanent E05)",
                "upper_nor_layer2_default_ro_bytes": AM34_FLASH_IPL_SIZE - AM34_FLASH_CRITICAL_IPL_SIZE,
                "upper_nor_layer2_unlock_token": "CONFIRM_UPPER_NOR",
                "nvram_mutable_bus_base": format!("0x{:08X}", AM34_NVRAM_BASE),
                "nvram_mutable_size_bytes": SPANSION_NVRAM_SIZE_BYTES,
                "forbidden_lethal_register": format!("0x{:08X}", AM34_FORBIDDEN_LETHAL_REG),
                "forensic_raw_dumps_immutable": true,
            });
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&payload)?
                }]
            }))
        }
        "panadiga://memory-map" => {
            let regions: Vec<Value> = AM34_SYSTEM_MEMORY_REGIONS
                .iter()
                .map(|r| r.to_json_value())
                .collect();
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&regions)?
                }]
            }))
        }
        "panadiga://pinout-p7501-uart" => {
            let spec = build_hardware_ssot_spec();
            let payload = json!({
                "connector": "P7501 (JST B6B-ZR-3R4 TO JIGU) Async UART 115200 8N1",
                "pinout": spec.pinout,
                "thresholds": spec.thresholds,
                "breakin_modes": spec.breakin_modes,
            });
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&payload)?
                }]
            }))
        }
        "panadiga://visual-catalog" => {
            let summary = crate::tools::visuals::load_visual_catalog_summary_json(
                workspace_root,
                MAX_MCP_RESOURCE_BYTES,
            )
            .map_err(|e| anyhow::anyhow!(e))?;
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": summary
                }]
            }))
        }
        "panadiga://models-catalog" => {
            let catalog = get_diga_model_catalog_dto();
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&catalog)?
                }]
            }))
        }
        "panadiga://pico-commands" => {
            let spec = build_hardware_ssot_spec();
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&spec.pico_commands)?
                }]
            }))
        }
        other if other.starts_with("panadiga://profiles/") => {
            let model_id = other.trim_start_matches("panadiga://profiles/");
            let resolved = HardwareProfileRegistry::builtin().resolve_model(model_id)?;
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&resolved)?
                }]
            }))
        }
        other if other.starts_with("panadiga://symbols/") => {
            let tail = other.trim_start_matches("panadiga://symbols/");
            let Some((train_id, module)) = tail.split_once('/') else {
                bail!(
                    "Invalid symbol resource URI '{uri}': expected panadiga://symbols/<train_id>/<module>"
                );
            };
            let sym_idx = load_symbol_index(workspace_root, train_id, module)?;
            let raw = serde_json::to_string_pretty(&sym_idx)?;
            let text = truncate_mcp_text_to_cap(&raw, MAX_MCP_RESOURCE_BYTES, uri);
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": text
                }]
            }))
        }
        other if other.starts_with("panadiga://docs/") => {
            let tail = other.trim_start_matches("panadiga://docs/");
            let effective_root =
                crate::workspace::resolve_effective_workspace_for(workspace_root, "docs");
            let docs_root = effective_root.join("docs");
            let full_path = crate::workspace::resolve_workspace_path(&docs_root, tail)?;
            if !full_path.is_file() {
                bail!("Document resource not found: {}", uri);
            }
            let raw = fs::read_to_string(&full_path)?;
            let text = truncate_mcp_text_to_cap(&raw, MAX_MCP_RESOURCE_BYTES, uri);
            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "text/markdown",
                    "text": text
                }]
            }))
        }
        _ => bail!("Unknown Panadiga resource URI: {}", uri),
    }
}
