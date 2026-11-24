#![forbid(unsafe_code)]

use std::path::Path;

use serde_json::{Value, json};

#[test]
fn snapshot_mcp_tool_inventory_and_annotations() {
    let mut summary: Vec<Value> = panadiga_mcp::tool_definitions()
        .into_iter()
        .map(|tool| {
            let mut required: Vec<String> = tool["inputSchema"]["required"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            required.sort();
            json!({
                "name": tool["name"],
                "annotations": tool["annotations"],
                "required": required,
            })
        })
        .collect();
    summary.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    insta::assert_json_snapshot!("mcp_tool_inventory", summary);
}

#[test]
fn snapshot_mcp_invariants_resource() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/read",
        "params": { "uri": "panadiga://invariants" }
    })
    .to_string();
    let resp = panadiga_mcp::handle_jsonrpc_line(&req, &repo_root)
        .expect("resources/read panadiga://invariants must reply");
    let text = resp["result"]["contents"][0]["text"]
        .as_str()
        .expect("invariants text JSON payload");
    let parsed: Value = serde_json::from_str(text).expect("valid invariants JSON");
    insta::assert_json_snapshot!("mcp_invariants_resource", parsed);
}
