#![forbid(unsafe_code)]
//! Native Rust Model Context Protocol (MCP) Server for Panadiga.
//!
//! Modularized into protocol transport, fail-closed security policy, resources,
//! prompts, hardware tools, forensic tools, and knowledge/RAG search tools over JSON-RPC 2.0 stdio.

#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]

pub mod policy;
pub mod prompts;
pub mod protocol;
pub mod resources;
pub mod tools;
pub mod workspace;

pub use policy::McpSecurityPolicy;
pub use protocol::{handle_jsonrpc_line, handle_jsonrpc_line_with_policy};
pub use tools::{execute_tool, execute_tool_with_policy, tool_definitions};
pub use workspace::resolve_workspace_root;

use anyhow::Result;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Runs the JSON-RPC 2.0 MCP loop under an explicit [`McpSecurityPolicy`].
pub fn run_stdio_with_policy<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
    workspace_root: &Path,
    policy: &McpSecurityPolicy,
) -> Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        if let Some(resp) = handle_jsonrpc_line_with_policy(&line, workspace_root, policy) {
            writeln!(writer, "{}", serde_json::to_string(&resp)?)?;
            writer.flush()?;
        }
    }
    Ok(())
}

/// Runs the JSON-RPC 2.0 MCP loop against generic buffered streams with default read-only policy.
pub fn run_stdio_with_io<R: BufRead, W: Write>(
    reader: R,
    writer: W,
    workspace_root: &Path,
) -> Result<()> {
    run_stdio_with_policy(
        reader,
        writer,
        workspace_root,
        &McpSecurityPolicy::read_only(),
    )
}

/// Starts the native Rust MCP CLI/stdio server with an explicit security policy.
pub fn run_mcp_server_with_policy(
    show_tools: bool,
    ping_only: bool,
    policy: &McpSecurityPolicy,
) -> Result<()> {
    // CRITICAL: Keep stdout strictly clean for JSON-RPC 2.0 protocol framing.
    panadiga_core::logger::init_telemetry(panadiga_core::LogLevel::Info);
    panadiga_core::logger::set_raw_output_enabled(false);

    let mut stdout = io::stdout().lock();
    if show_tools {
        writeln!(
            stdout,
            "{}",
            serde_json::to_string_pretty(&tool_definitions())?
        )?;
        stdout.flush()?;
        return Ok(());
    }
    if ping_only {
        writeln!(stdout, "PONG - Panadiga Native Rust MCP Server OK")?;
        stdout.flush()?;
        return Ok(());
    }

    let stdin = io::stdin().lock();
    let root = resolve_workspace_root();
    run_stdio_with_policy(stdin, stdout, &root, policy)
}

/// Starts the native Rust MCP CLI/stdio server in default fail-closed read-only mode.
pub fn run_mcp_server(show_tools: bool, ping_only: bool) -> Result<()> {
    run_mcp_server_with_policy(show_tools, ping_only, &McpSecurityPolicy::read_only())
}
