#![deny(unsafe_code)]

//! Tool registry and execution dispatcher for the Panadiga MCP Server.

mod defs_core;
mod defs_re;
mod defs_re_ext;
pub mod forensics;
pub mod forensics_re;
pub mod hardware;
pub mod knowledge;
pub mod re_ext;
pub mod re_ext_bench;
pub mod unbrick;
pub mod visuals;

use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub fn tool_definitions() -> Vec<Value> {
    let mut defs = defs_core::core_tool_definitions();
    defs.extend(defs_re::re_tool_definitions());
    defs.extend(defs_re_ext::re_ext_tool_definitions());
    defs
}

pub fn execute_tool(name: &str, args: &Value, workspace_root: &Path) -> Result<Value> {
    match name {
        "panadiga_memory_map" => Ok(hardware::handle_memory_map(args)),
        "panadiga_safety_check" => Ok(hardware::handle_safety_check(args)),
        "panadiga_pico_catalog" => Ok(hardware::handle_pico_catalog(args)),
        "panadiga_project_status" => Ok(hardware::handle_project_status(args, workspace_root)),
        "panadiga_inspect_hdd" => Ok(forensics::handle_inspect_hdd(args, workspace_root)),
        "panadiga_clean_nvram" => forensics::handle_clean_nvram(args, workspace_root),
        "panadiga_inspect_frm" => forensics::handle_inspect_frm(args, workspace_root),
        "panadiga_read_hex" => forensics::handle_read_hex(args, workspace_root),
        "panadiga_hdd_capacity" => forensics_re::handle_hdd_capacity(args, workspace_root),
        "panadiga_nor_dumper_stub" => forensics_re::handle_nor_dumper_stub(args, workspace_root),
        "panadiga_bgo_unbrick" => forensics_re::handle_bgo_unbrick(args, workspace_root),
        "panadiga_main_passthrough" => forensics_re::handle_main_passthrough(args, workspace_root),
        "panadiga_sd_unbrick_kit" => unbrick::handle_sd_unbrick_kit(args, workspace_root),
        "panadiga_search_manuals" => knowledge::handle_search_manuals(args, workspace_root),
        "panadiga_rag_query" => knowledge::handle_rag_query(args),
        "panadiga_search_code" => knowledge::handle_search_code(args),
        "panadiga_ghidra_symbols" => Ok(knowledge::handle_ghidra_symbols(args)),
        "panadiga_search_visuals" => Ok(visuals::handle_search_visuals(args, workspace_root)),
        "panadiga_region_compat" => re_ext::handle_region_compat(args, workspace_root),
        "panadiga_hdd_flash_inspect" => re_ext::handle_hdd_flash_inspect(args),
        "panadiga_tbc_appliance_plan" => re_ext::handle_tbc_appliance_plan(args),
        "panadiga_nor_sector_audit" => re_ext::handle_nor_sector_audit(args, workspace_root),
        "panadiga_eh55_bgo" => re_ext_bench::handle_eh55_bgo(args, workspace_root),
        "panadiga_davis_probe" => re_ext_bench::handle_davis_probe(args, workspace_root),
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}
