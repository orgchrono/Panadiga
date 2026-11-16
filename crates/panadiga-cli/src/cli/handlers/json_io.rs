//! Shared stdin/file binary readers, hex list parsers, and clean `--json` emission helpers.

use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use panadiga_core::crypto::parse_u32_radix;
use serde::Serialize;

/// Reads raw bytes from `path`, or reads all bytes from `stdin` when `path == "-"`.
pub fn read_cli_input_bytes(path: &Path) -> Result<Vec<u8>> {
    if path == Path::new("-") {
        let mut buf = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buf)
            .context("Failed reading binary input from stdin")?;
        return Ok(buf);
    }
    fs::read(path).with_context(|| format!("Failed reading input file '{}'", path.display()))
}

/// Reads UTF-8 lossy text from `path`, or from `stdin` when `path == "-"`.
pub fn read_cli_input_text(path: &Path) -> Result<String> {
    let bytes = read_cli_input_bytes(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Ensures parent directories exist and writes raw bytes cleanly to `path`.
pub fn write_cli_output_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes).with_context(|| format!("Failed writing output '{}'", path.display()))
}

/// Emits `val` as compact JSON to stdout when `json_path == Some("-")`, or writes pretty JSON to file.
pub fn emit_json_output<T: Serialize>(val: &T, json_path: Option<&Path>) -> Result<()> {
    let Some(p) = json_path else {
        return Ok(());
    };
    if p == Path::new("-") {
        println!("{}", serde_json::to_string(val)?);
        return Ok(());
    }
    let formatted = serde_json::to_string_pretty(val)?;
    write_cli_output_bytes(p, formatted.as_bytes())?;
    println!("JSON report written to {}", p.display());
    Ok(())
}

/// Parses an optional hex/decimal `u32` CLI string argument.
pub fn parse_opt_u32(raw: Option<&str>) -> Result<Option<u32>> {
    raw.map(parse_u32_radix).transpose()
}

/// Parses a comma-separated list of hex/decimal `u32` values into `Vec<u32>`.
pub fn parse_u32_csv(raw: Option<&str>) -> Result<Vec<u32>> {
    let Some(text) = raw else {
        return Ok(Vec::new());
    };
    text.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_u32_radix)
        .collect()
}
