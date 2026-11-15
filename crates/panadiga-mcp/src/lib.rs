//! Native Rust Model Context Protocol (MCP) Server for Panadiga.
//!
//! Modularized into protocol transport, resources, prompts, hardware tools,
//! forensic tools, and knowledge/RAG search tools over JSON-RPC 2.0 stdio.

pub mod prompts;
pub mod protocol;
pub mod resources;
pub mod tools;
pub mod workspace;

pub use protocol::handle_jsonrpc_line;
pub use tools::{execute_tool, tool_definitions};
pub use workspace::resolve_workspace_root;

use anyhow::Result;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Runs the JSON-RPC 2.0 MCP loop against generic buffered streams (used in stdio & tests).
pub fn run_stdio_with_io<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
    workspace_root: &Path,
) -> Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        if let Some(resp) = handle_jsonrpc_line(&line, workspace_root) {
            writeln!(writer, "{}", serde_json::to_string(&resp)?)?;
            writer.flush()?;
        }
    }
    Ok(())
}

/// Starts the native Rust MCP CLI/stdio server while isolating logger output to stderr.
pub fn run_mcp_server(show_tools: bool, ping_only: bool) -> Result<()> {
    // CRITICAL: Keep stdout strictly clean for JSON-RPC 2.0 protocol framing.
    panadiga_core::logger::set_sink(|level, msg| {
        eprintln!("[{}] {}", level.as_str(), msg);
    });

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
    run_stdio_with_io(stdin, stdout, &root)
}
