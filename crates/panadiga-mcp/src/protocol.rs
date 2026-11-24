//! JSON-RPC 2.0 protocol router implementing MCP tools, resources, and prompts.

use serde_json::{Value, json};
use std::path::Path;

use crate::policy::{McpSecurityPolicy, append_audit_entry, truncate_tool_output};
use crate::prompts::{get_prompt, list_prompts};
use crate::resources::{list_resources, read_resource};
use crate::tools::{execute_tool_with_policy, tool_definitions};

fn rpc_error(id: Value, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message.into() }
    })
}

/// Handles a single JSON-RPC 2.0 line in default fail-closed read-only policy mode.
pub fn handle_jsonrpc_line(line: &str, workspace_root: &Path) -> Option<Value> {
    handle_jsonrpc_line_with_policy(line, workspace_root, &McpSecurityPolicy::read_only())
}

/// Handles a single JSON-RPC 2.0 line under an explicit [`McpSecurityPolicy`].
pub fn handle_jsonrpc_line_with_policy(
    line: &str,
    workspace_root: &Path,
    policy: &McpSecurityPolicy,
) -> Option<Value> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let req: Value = serde_json::from_str(trimmed).ok()?;
    let req_id = req.get("id").cloned();
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    if method == "notifications/initialized" || req_id.as_ref().is_none_or(|v| v.is_null()) {
        return None;
    }
    let id = req_id?;

    let resp = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "panadiga-mcp-server",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": tool_definitions() }
        }),
        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = params
                .get("arguments")
                .filter(|v| v.is_object())
                .cloned()
                .unwrap_or_else(|| json!({}));

            match execute_tool_with_policy(tool_name, &tool_args, workspace_root, policy) {
                Ok(out) => {
                    append_audit_entry(
                        policy.audit_log_path.as_deref(),
                        tool_name,
                        &tool_args,
                        "ok",
                        None,
                    );
                    let pretty = serde_json::to_string_pretty(&out).unwrap_or_default();
                    let bounded = truncate_tool_output(&pretty, policy.max_output_bytes);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": bounded }]
                        }
                    })
                }
                Err(e) => {
                    let msg = e.to_string();
                    append_audit_entry(
                        policy.audit_log_path.as_deref(),
                        tool_name,
                        &tool_args,
                        "error",
                        Some(&msg),
                    );
                    rpc_error(id, -32000, msg)
                }
            }
        }
        "resources/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "resources": list_resources(workspace_root) }
        }),
        "resources/read" => {
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            match read_resource(uri, workspace_root) {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err(e) => rpc_error(id, -32002, e.to_string()),
            }
        }
        "prompts/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "prompts": list_prompts() }
        }),
        "prompts/get" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params
                .get("arguments")
                .filter(|v| v.is_object())
                .cloned()
                .unwrap_or_else(|| json!({}));
            match get_prompt(name, &args) {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err(e) => rpc_error(id, -32602, e.to_string()),
            }
        }
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        other => rpc_error(id, -32601, format!("Method not found: {}", other)),
    };

    Some(resp)
}
