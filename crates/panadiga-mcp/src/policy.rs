#![forbid(unsafe_code)]

//! Fail-closed MCP execution policy: read-only defaults, HITL gating for live hardware,
//! UTF-8 safe output truncation, and append-only JSONL forensic audit logging.

use anyhow::{Result, bail};
use panadiga_core::nvram::calculate_hashes;
use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum serialized tool payload size emitted over JSON-RPC stdio (64 KiB).
pub const DEFAULT_MCP_MAX_OUTPUT_BYTES: usize = 65_536;

/// Environment variable providing the Human-In-The-Loop authorization token for live bench tools.
pub const ENV_MCP_HITL_TOKEN: &str = "PANADIGA_MCP_HITL_TOKEN";

/// Environment variable specifying an optional JSONL audit log path when CLI `--audit-log` is unset.
pub const ENV_MCP_AUDIT_LOG: &str = "PANADIGA_MCP_AUDIT_LOG";

const TRUNCATION_NOTICE: &str = "\n... [MCP OUTPUT TRUNCATED AT 64 KiB LIMIT]";

/// Declarative fail-closed capability policy governing MCP tool execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSecurityPolicy {
    pub allow_file_write: bool,
    pub allow_hardware_write: bool,
    pub hitl_token: Option<String>,
    pub max_output_bytes: usize,
    pub audit_log_path: Option<PathBuf>,
}

impl Default for McpSecurityPolicy {
    fn default() -> Self {
        Self::read_only()
    }
}

impl McpSecurityPolicy {
    /// Strict read-only baseline policy: rejects all filesystem writes and live hardware access.
    #[must_use]
    pub const fn read_only() -> Self {
        Self {
            allow_file_write: false,
            allow_hardware_write: false,
            hitl_token: None,
            max_output_bytes: DEFAULT_MCP_MAX_OUTPUT_BYTES,
            audit_log_path: None,
        }
    }

    /// Constructs runtime server policy from explicit operator CLI flags and environment variables.
    #[must_use]
    pub fn from_cli_and_env(
        allow_file_write: bool,
        allow_hardware_write: bool,
        cli_audit_log: Option<&Path>,
        workspace_root: &Path,
    ) -> Self {
        let hitl_token = std::env::var(ENV_MCP_HITL_TOKEN)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let env_audit = std::env::var_os(ENV_MCP_AUDIT_LOG).and_then(|raw| {
            let trimmed = raw.to_string_lossy().trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        });

        let audit_log_path = cli_audit_log.map(Path::to_path_buf).or(env_audit).map(|p| {
            if p.is_absolute() {
                p
            } else {
                workspace_root.join(p)
            }
        });

        Self {
            allow_file_write,
            allow_hardware_write,
            hitl_token,
            max_output_bytes: DEFAULT_MCP_MAX_OUTPUT_BYTES,
            audit_log_path,
        }
    }

    /// Pure guard verifying whether workspace CoW / artifact file writes are permitted.
    pub fn require_file_write(&self, tool_name: &str, target: Option<&Path>) -> Result<()> {
        if self.allow_file_write {
            return Ok(());
        }
        let suffix = target
            .map(|p| format!(" (target '{}')", p.display()))
            .unwrap_or_default();
        bail!(
            "MCP read-only policy blocked file write for '{tool_name}'{suffix}; restart MCP server with --allow-file-write to permit artifact output."
        )
    }

    /// Pure guard enforcing `--allow-hardware-write` AND constant-time `PANADIGA_MCP_HITL_TOKEN` verification.
    pub fn require_hardware_hitl(
        &self,
        tool_name: &str,
        supplied_token: Option<&str>,
    ) -> Result<()> {
        if !self.allow_hardware_write {
            bail!(
                "MCP policy blocked live hardware operation for '{tool_name}'; restart MCP server with --allow-hardware-write and configure {ENV_MCP_HITL_TOKEN}."
            );
        }
        let expected = self
            .hitl_token
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "MCP HITL gate blocked '{tool_name}': {ENV_MCP_HITL_TOKEN} is not configured on the MCP server."
                )
            })?;
        let provided = supplied_token
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "MCP HITL gate blocked '{tool_name}': missing required 'hitl_token' argument."
                )
            })?;

        if !constant_time_eq_bytes(expected.as_bytes(), provided.as_bytes()) {
            bail!("MCP HITL gate blocked '{tool_name}': invalid 'hitl_token'.");
        }
        Ok(())
    }
}

/// Pure constant-time byte-slice comparison folding across maximum length without early return.
fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    let max_len = a.len().max(b.len());
    let diff = (0..max_len).fold(a.len() ^ b.len(), |acc, i| {
        let lhs = a.get(i).copied().unwrap_or(0);
        let rhs = b.get(i).copied().unwrap_or(0);
        acc | usize::from(lhs ^ rhs)
    });
    diff == 0
}

/// Truncates UTF-8 JSON tool responses cleanly at or below `max_bytes`.
#[must_use]
pub fn truncate_tool_output(text: &str, max_bytes: usize) -> String {
    if max_bytes == 0 || text.len() <= max_bytes {
        return text.to_string();
    }
    if max_bytes <= TRUNCATION_NOTICE.len() {
        let end = floor_utf8_boundary(text, max_bytes);
        return text[..end].to_string();
    }
    let body_budget = max_bytes - TRUNCATION_NOTICE.len();
    let end = floor_utf8_boundary(text, body_budget);
    format!("{}{}", &text[..end], TRUNCATION_NOTICE)
}

fn floor_utf8_boundary(text: &str, limit: usize) -> usize {
    let clamped = limit.min(text.len());
    (0..=clamped)
        .rev()
        .find(|&idx| text.is_char_boundary(idx))
        .unwrap_or(0)
}

/// Appends one tamper-evident JSONL audit line per `tools/call` without exposing plaintext secrets.
pub fn append_audit_entry(
    log_path: Option<&Path>,
    tool_name: &str,
    args: &Value,
    status: &str,
    detail: Option<&str>,
) {
    let Some(path) = log_path else {
        return;
    };
    let canonical_args = serde_json::to_vec(args).unwrap_or_default();
    let (_, args_sha256) = calculate_hashes(&canonical_args);
    let ts_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let record = json!({
        "ts_unix_ms": ts_unix_ms,
        "tool": tool_name,
        "args_sha256": args_sha256,
        "status": status,
        "detail": detail,
    });
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{record}");
    }
}
