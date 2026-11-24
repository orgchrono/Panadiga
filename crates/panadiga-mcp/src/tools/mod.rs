//! Tool registry, declarative MCP annotations, and policy-enforced dispatcher for Panadiga MCP.

mod defs_core;
mod defs_re;
mod defs_re_ext;
pub mod forensics;
pub mod forensics_re;
pub mod hardware;
pub mod knowledge;
pub mod re_ext;
pub mod re_ext_bench;
pub mod symbols;
pub mod unbrick;
pub mod visuals;

use crate::policy::McpSecurityPolicy;
use anyhow::Result;
use panadiga_core::pico::panamon1::SerialPanamon1Transport;
use serde_json::{Value, json};
use std::path::Path;

#[derive(Clone, Copy)]
struct ToolAnnotationSpec {
    read_only: bool,
    destructive: bool,
    idempotent: bool,
    open_world: bool,
}

const RO_SPEC: ToolAnnotationSpec = ToolAnnotationSpec {
    read_only: true,
    destructive: false,
    idempotent: true,
    open_world: false,
};

const COW_WRITE_SPEC: ToolAnnotationSpec = ToolAnnotationSpec {
    read_only: false,
    destructive: false,
    idempotent: true,
    open_world: false,
};

const LIVE_HW_SPEC: ToolAnnotationSpec = ToolAnnotationSpec {
    read_only: false,
    destructive: true,
    idempotent: false,
    open_world: false,
};

const TOOL_ANNOTATION_MAP: &[(&str, ToolAnnotationSpec)] = &[
    ("panadiga_memory_map", RO_SPEC),
    ("panadiga_inspect_hdd", RO_SPEC),
    ("panadiga_safety_check", RO_SPEC),
    ("panadiga_clean_nvram", COW_WRITE_SPEC),
    ("panadiga_search_manuals", RO_SPEC),
    ("panadiga_rag_query", RO_SPEC),
    ("panadiga_inspect_frm", RO_SPEC),
    ("panadiga_read_hex", RO_SPEC),
    ("panadiga_pico_catalog", RO_SPEC),
    ("panadiga_ghidra_symbols", RO_SPEC),
    ("panadiga_search_code", RO_SPEC),
    ("panadiga_search_visuals", RO_SPEC),
    ("panadiga_project_status", RO_SPEC),
    ("panadiga_hdd_capacity", COW_WRITE_SPEC),
    ("panadiga_nor_dumper_stub", COW_WRITE_SPEC),
    ("panadiga_bgo_unbrick", COW_WRITE_SPEC),
    ("panadiga_main_passthrough", COW_WRITE_SPEC),
    ("panadiga_sd_unbrick_kit", COW_WRITE_SPEC),
    ("panadiga_region_compat", COW_WRITE_SPEC),
    ("panadiga_hdd_flash_inspect", RO_SPEC),
    ("panadiga_tbc_appliance_plan", RO_SPEC),
    ("panadiga_nor_sector_audit", RO_SPEC),
    ("panadiga_eh55_bgo", LIVE_HW_SPEC),
    ("panadiga_davis_probe", LIVE_HW_SPEC),
    ("panadiga_profiles_authorize", RO_SPEC),
    ("panadiga_symbol_lookup", RO_SPEC),
];

fn attach_tool_annotations(mut def: Value) -> Value {
    let spec = def
        .get("name")
        .and_then(|v| v.as_str())
        .and_then(|name| {
            TOOL_ANNOTATION_MAP
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, s)| *s)
        })
        .unwrap_or(LIVE_HW_SPEC);
    if let Some(obj) = def.as_object_mut() {
        obj.insert(
            "annotations".to_string(),
            json!({
                "readOnlyHint": spec.read_only,
                "destructiveHint": spec.destructive,
                "idempotentHint": spec.idempotent,
                "openWorldHint": spec.open_world,
            }),
        );
    }
    def
}

pub fn tool_definitions() -> Vec<Value> {
    let mut defs = defs_core::core_tool_definitions();
    defs.extend(defs_re::re_tool_definitions());
    defs.extend(defs_re_ext::re_ext_tool_definitions());
    defs.into_iter().map(attach_tool_annotations).collect()
}

/// Executes an MCP tool with default fail-closed read-only policy.
pub fn execute_tool(name: &str, args: &Value, workspace_root: &Path) -> Result<Value> {
    execute_tool_with_policy(name, args, workspace_root, &McpSecurityPolicy::read_only())
}

/// Executes an MCP tool enforcing an explicit [`McpSecurityPolicy`].
pub fn execute_tool_with_policy(
    name: &str,
    args: &Value,
    workspace_root: &Path,
    policy: &McpSecurityPolicy,
) -> Result<Value> {
    match name {
        "panadiga_memory_map" => Ok(hardware::handle_memory_map(args)),
        "panadiga_safety_check" => Ok(hardware::handle_safety_check(args)),
        "panadiga_pico_catalog" => Ok(hardware::handle_pico_catalog(args)),
        "panadiga_project_status" => Ok(hardware::handle_project_status(args, workspace_root)),
        "panadiga_inspect_hdd" => Ok(forensics::handle_inspect_hdd(args, workspace_root)),
        "panadiga_clean_nvram" => {
            forensics::handle_clean_nvram_with_policy(args, workspace_root, policy)
        }
        "panadiga_inspect_frm" => forensics::handle_inspect_frm(args, workspace_root),
        "panadiga_read_hex" => forensics::handle_read_hex(args, workspace_root),
        "panadiga_hdd_capacity" => {
            forensics_re::handle_hdd_capacity_with_policy(args, workspace_root, policy)
        }
        "panadiga_nor_dumper_stub" => {
            forensics_re::handle_nor_dumper_stub_with_policy(args, workspace_root, policy)
        }
        "panadiga_bgo_unbrick" => {
            forensics_re::handle_bgo_unbrick_with_policy(args, workspace_root, policy)
        }
        "panadiga_main_passthrough" => {
            forensics_re::handle_main_passthrough_with_policy(args, workspace_root, policy)
        }
        "panadiga_sd_unbrick_kit" => {
            unbrick::handle_sd_unbrick_kit_with_policy(args, workspace_root, policy)
        }
        "panadiga_search_manuals" => knowledge::handle_search_manuals(args, workspace_root),
        "panadiga_rag_query" => knowledge::handle_rag_query(args),
        "panadiga_search_code" => knowledge::handle_search_code(args),
        "panadiga_ghidra_symbols" => Ok(knowledge::handle_ghidra_symbols(args)),
        "panadiga_search_visuals" => Ok(visuals::handle_search_visuals(args, workspace_root)),
        "panadiga_region_compat" => {
            re_ext::handle_region_compat_with_policy(args, workspace_root, policy)
        }
        "panadiga_hdd_flash_inspect" => re_ext::handle_hdd_flash_inspect(args),
        "panadiga_tbc_appliance_plan" => re_ext::handle_tbc_appliance_plan(args),
        "panadiga_nor_sector_audit" => re_ext::handle_nor_sector_audit(args, workspace_root),
        "panadiga_eh55_bgo" => {
            re_ext_bench::handle_eh55_bgo_with_policy::<SerialPanamon1Transport>(
                args,
                workspace_root,
                None,
                policy,
            )
        }
        "panadiga_davis_probe" => re_ext_bench::handle_davis_probe_with_policy::<
            SerialPanamon1Transport,
        >(args, workspace_root, None, policy),
        "panadiga_profiles_authorize" => hardware::handle_profiles_authorize(args),
        "panadiga_symbol_lookup" => symbols::handle_symbol_lookup(args, workspace_root),
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}
