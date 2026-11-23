use panadiga_core::constants::{
    AM34_SYSTEM_MEMORY_REGIONS, SPANSION_BOOTLOADER_SIZE_BYTES, SPANSION_FLASH_SIZE_BYTES,
    SPANSION_NVRAM_BASE_OFFSET,
};
use panadiga_mcp::{execute_tool, prompts, resources, run_stdio_with_io, tool_definitions};
use serde_json::{Value, json};
use std::fs;
use std::io::Cursor;
use std::path::Path;
#[test]
fn test_tool_definitions_and_memory_map() {
    let defs = tool_definitions();
    assert_eq!(defs.len(), 26);

    let all = execute_tool("panadiga_memory_map", &json!({}), Path::new(".")).unwrap();
    assert_eq!(all["total"], AM34_SYSTEM_MEMORY_REGIONS.len());

    let nvram = execute_tool(
        "panadiga_memory_map",
        &json!({ "filter_name": "nvram_u99" }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(nvram["total"], 1);
    assert_eq!(nvram["regions"][0]["start"], "0x807F0000");
}

#[test]
fn test_safety_check_invariants() {
    let overvolt = execute_tool(
        "panadiga_safety_check",
        &json!({ "voltage": 5.0 }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(overvolt["safe"], false);
    assert!(
        overvolt["violations"][0]
            .as_str()
            .unwrap()
            .contains("VOLTAGE_VIOLATION")
    );

    let safe_volt = execute_tool(
        "panadiga_safety_check",
        &json!({ "voltage": 3.3 }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(safe_volt["safe"], true);

    let ipl_write = execute_tool(
        "panadiga_safety_check",
        &json!({ "address": "0x80000100" }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(ipl_write["safe"], false);
    assert!(
        ipl_write["violations"][0]
            .as_str()
            .unwrap()
            .contains("FLASH_WRITE_VIOLATION")
    );

    let nvram_write = execute_tool(
        "panadiga_safety_check",
        &json!({ "address": "0x807F0000" }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(nvram_write["safe"], true);
    assert!(
        nvram_write["address_status"]
            .as_str()
            .unwrap()
            .contains("mutable NVRAM")
    );

    let uncached_ipl = execute_tool(
        "panadiga_safety_check",
        &json!({ "address": "0x84000000" }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(uncached_ipl["safe"], false);
    assert!(
        uncached_ipl["violations"][0]
            .as_str()
            .unwrap()
            .contains("FLASH_WRITE_VIOLATION")
    );

    let lethal = execute_tool(
        "panadiga_safety_check",
        &json!({ "address": "0xC000100C" }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(lethal["safe"], false);
    assert!(
        lethal["violations"][0]
            .as_str()
            .unwrap()
            .contains("LETHAL_REGISTER_VIOLATION")
    );

    let neg_volt = execute_tool(
        "panadiga_safety_check",
        &json!({ "voltage": -1.0 }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(neg_volt["safe"], false);
    assert!(
        neg_volt["violations"][0]
            .as_str()
            .unwrap()
            .contains("NEGATIVE_VOLTAGE_VIOLATION")
    );

    let upper_locked = execute_tool(
        "panadiga_safety_check",
        &json!({ "address": "0x84130000" }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(upper_locked["safe"], false);

    let upper_unlocked = execute_tool(
        "panadiga_safety_check",
        &json!({ "address": "0x84130000", "confirm_upper_nor": true }),
        Path::new("."),
    )
    .unwrap();
    assert_eq!(upper_unlocked["safe"], true);
}

#[test]
fn test_clean_nvram_rejects_in_place_and_sanitizes_copy() {
    let tmp = std::env::temp_dir().join(format!("panadiga_mcp_nvram_{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();

    let in_path = tmp.join("flash_in.bin");
    let out_path = tmp.join("flash_out.bin");

    let mut flash = vec![0xA5u8; SPANSION_FLASH_SIZE_BYTES];
    let banner = b"Loader ver0.61 [06/03/06]";
    flash[0x0003_F000..0x0003_F000 + banner.len()].copy_from_slice(banner);
    flash[SPANSION_NVRAM_BASE_OFFSET..SPANSION_NVRAM_BASE_OFFSET + 8]
        .copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0xF1, 0xFF, 0xFF]);
    flash[SPANSION_NVRAM_BASE_OFFSET] = 0x11;
    fs::write(&in_path, &flash).unwrap();

    let same_res = execute_tool(
        "panadiga_clean_nvram",
        &json!({
            "input_file": "flash_in.bin",
            "output_file": "flash_in.bin"
        }),
        &tmp,
    )
    .unwrap();
    assert!(
        same_res["error"]
            .as_str()
            .unwrap()
            .contains("Forensic Invariant Violation")
    );

    let clean_res = execute_tool(
        "panadiga_clean_nvram",
        &json!({
            "input_file": "flash_in.bin",
            "output_file": "flash_out.bin"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(clean_res["status"], "success");
    assert_eq!(clean_res["sanitized_file_created"], true);
    assert_eq!(
        clean_res["ipl_bytes_checked"],
        SPANSION_BOOTLOADER_SIZE_BYTES
    );

    let cleaned_bytes = fs::read(&out_path).unwrap();
    assert_eq!(
        &cleaned_bytes[..SPANSION_BOOTLOADER_SIZE_BYTES],
        &flash[..SPANSION_BOOTLOADER_SIZE_BYTES]
    );
    assert!(
        cleaned_bytes[SPANSION_NVRAM_BASE_OFFSET..SPANSION_FLASH_SIZE_BYTES]
            .iter()
            .all(|&b| b == 0xFF)
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_new_dev_tools_and_resources_prompts() {
    let catalog = execute_tool("panadiga_pico_catalog", &json!({}), Path::new(".")).unwrap();
    assert!(!catalog["pico_commands"].as_array().unwrap().is_empty());
    assert_eq!(catalog["pins"].as_array().unwrap().len(), 6);

    let ghidra = execute_tool(
        "panadiga_ghidra_symbols",
        &json!({ "filter": "nvram" }),
        Path::new("."),
    )
    .unwrap();
    assert!(!ghidra["memory_blocks"].as_array().unwrap().is_empty());

    let res_invariants = resources::read_resource("panadiga://invariants", Path::new(".")).unwrap();
    let text = res_invariants["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains("0x807F0000"));
    assert!(text.contains("p7501_never_power_pico"));
    assert!(text.contains("pin5_nsw5v_gp3_hiz_only"));
    assert!(!text.contains("pin6_never_power_pico"));
    let res_visuals =
        resources::read_resource("panadiga://visual-catalog", Path::new(".")).unwrap();
    let vis_manifest_text = res_visuals["contents"][0]["text"].as_str().unwrap();
    assert!(vis_manifest_text.contains("na_p62_zoom.png"));
    assert!(vis_manifest_text.len() <= resources::MAX_MCP_RESOURCE_BYTES);

    let large_doc = resources::read_resource(
        "panadiga://docs/pseudocode/EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md",
        Path::new("."),
    )
    .unwrap();
    let large_doc_text = large_doc["contents"][0]["text"].as_str().unwrap();
    assert!(large_doc_text.len() <= resources::MAX_MCP_RESOURCE_BYTES);
    assert!(large_doc_text.contains("TRUNCATED AT 32 KiB"));
    let vis_search = execute_tool(
        "panadiga_search_visuals",
        &json!({ "query": "Q7514", "limit": 5 }),
        Path::new("."),
    )
    .unwrap();
    assert!(vis_search["matched"].as_u64().unwrap() >= 1);
    assert!(resources::read_resource("panadiga://docs/../../etc/passwd", Path::new(".")).is_err());

    let prompt =
        prompts::get_prompt("cddiag_p7501_capture", &json!({ "serial_port": "COM10" })).unwrap();
    assert!(
        prompt["messages"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .contains("COM10")
    );

    let p7501_prompt =
        prompts::get_prompt("p7501_uart_diagnostics", &json!({ "serial_port": "COM10" })).unwrap();
    let p7501_text = p7501_prompt["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(p7501_text.contains("COM10"));
    assert!(p7501_text.contains("P7501"));
}

#[test]
fn test_jsonrpc_stdio_protocol_and_capabilities() {
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"panadiga_safety_check\",\"arguments\":{\"voltage\":5.0,\"address\":\"0x80000000\"}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"resources/list\"}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"prompts/list\"}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"ping\"}\n"
    );
    let mut output = Vec::new();
    run_stdio_with_io(Cursor::new(input.as_bytes()), &mut output, Path::new(".")).unwrap();

    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 6);

    let r1: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(r1["result"]["serverInfo"]["name"], "panadiga-mcp-server");
    assert!(r1["result"]["capabilities"]["resources"].is_object());
    assert!(r1["result"]["capabilities"]["prompts"].is_object());

    let r2: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(r2["result"]["tools"].as_array().unwrap().len(), 26);

    let r3: Value = serde_json::from_str(lines[2]).unwrap();
    let body = r3["result"]["content"][0]["text"].as_str().unwrap();
    assert!(body.contains("VOLTAGE_VIOLATION"));
    assert!(body.contains("FLASH_WRITE_VIOLATION"));

    let r4: Value = serde_json::from_str(lines[3]).unwrap();
    assert!(r4["result"]["resources"].as_array().unwrap().len() >= 5);

    let r5: Value = serde_json::from_str(lines[4]).unwrap();
    assert_eq!(r5["result"]["prompts"].as_array().unwrap().len(), 4);

    let r6: Value = serde_json::from_str(lines[5]).unwrap();
    assert_eq!(r6["id"], 6);
}
