//! Workspace root-jail confinement and forensic input helpers shared across MCP handlers.

use anyhow::{Context, Result, bail};
pub use panadiga_core::constants::resolve_workspace_root;
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Resolves `raw` strictly inside canonicalized `root`, blocking traversal, NULs, and symlink escapes.
pub fn resolve_workspace_path(root: &Path, raw: &str) -> Result<PathBuf> {
    if raw.contains('\0') {
        bail!("Path containment violation: embedded NUL byte is forbidden");
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("Path containment violation: empty path is forbidden");
    }

    let candidate = Path::new(trimmed);
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        bail!("Path containment violation: '..' traversal is forbidden in '{trimmed}'");
    }

    let canon_root = fs::canonicalize(root)
        .with_context(|| format!("Invalid MCP workspace root '{}'", root.display()))?;

    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        if candidate
            .components()
            .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
        {
            bail!(
                "Path containment violation: rooted or drive-relative path '{trimmed}' is forbidden"
            );
        }
        canon_root.join(candidate)
    };

    canonicalize_within(&canon_root, &joined, trimmed)
}

fn canonicalize_within(canon_root: &Path, target: &Path, raw_label: &str) -> Result<PathBuf> {
    let mut cur = target.to_path_buf();
    let mut tail_segments: Vec<OsString> = Vec::new();

    while !cur.exists() {
        if cur.symlink_metadata().is_ok() {
            bail!("Path containment violation: dangling symlink is not permitted at '{raw_label}'");
        }
        let Some(name) = cur.file_name().map(|n| n.to_os_string()) else {
            bail!("Path containment violation: '{raw_label}' has no ancestor inside workspace");
        };
        tail_segments.push(name);
        let Some(parent) = cur.parent() else {
            bail!("Path containment violation: '{raw_label}' escapes workspace root");
        };
        cur = parent.to_path_buf();
    }

    let canon_existing = fs::canonicalize(&cur)
        .with_context(|| format!("Canonicalizing workspace path '{raw_label}'"))?;
    if !canon_existing.starts_with(canon_root) {
        bail!(
            "Path containment violation: '{raw_label}' resolves outside workspace root '{}'",
            canon_root.display()
        );
    }

    Ok(tail_segments
        .into_iter()
        .rev()
        .fold(canon_existing, |acc, seg| acc.join(seg)))
}

/// Ensures parent directories exist and writes a Copy-on-Write output artifact cleanly.
pub fn write_cow_artifact(out_path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, bytes)?;
    Ok(())
}

/// Returns `workspace_root` when it contains `marker_dir`, falling back to canonical repo root if `.`.
pub fn resolve_effective_workspace_for(workspace_root: &Path, marker_dir: &str) -> PathBuf {
    if workspace_root.join(marker_dir).is_dir() {
        workspace_root.to_path_buf()
    } else if workspace_root == Path::new(".") {
        resolve_workspace_root()
    } else {
        workspace_root.to_path_buf()
    }
}

pub fn paths_refer_to_same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    if let (Ok(ca), Ok(cb)) = (fs::canonicalize(a), fs::canonicalize(b)) {
        return ca == cb;
    }
    if let (Ok(ca), Some(b_parent), Some(b_name)) = (fs::canonicalize(a), b.parent(), b.file_name())
        && let Ok(cp) = fs::canonicalize(b_parent)
    {
        return ca == cp.join(b_name);
    }
    false
}

pub fn parse_u64_hex_or_dec(raw: &str) -> std::result::Result<u64, String> {
    let s = raw.trim();
    if let Some(hex_body) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex_body, 16)
            .map_err(|_| format!("Invalid hexadecimal value format: {}", raw))
    } else {
        s.parse::<u64>()
            .map_err(|_| format!("Invalid integer/hex format: {}", raw))
    }
}

pub fn require_non_empty_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Missing required argument: {}", key))
}

pub fn optional_non_empty_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
