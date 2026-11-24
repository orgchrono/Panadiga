//! Thin CLI handlers exposing `panadiga-core::pico` RSP framing, bench decoders, and A/B harness analytics.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::pico::bench_decoders::{
    AbBootRun, build_ab_boot_command, classify_ab_boot_log, format_ab_csv, parse_busdiag_report,
    parse_panarsp_report, parse_pincheck_report, parse_piologic_report, parse_status_output_report,
    summarize_ab_runs, validate_safe_bench_command,
};
use panadiga_core::pico::describe_boot_fat_error_code;
use panadiga_core::pico::protocol::{parse_boot_class_line, parse_tbus_audit_line};
use panadiga_core::pico::rsp_codec::{
    MAX_M_CHUNK, build_rsp_frame, build_rsp_write_frame, decode_mn103s_g_registers,
    decode_rsp_hex_memory, is_ipl_readonly_address, parse_rsp_reply, rsp_checksum,
    validate_dump_range,
};
use serde::Serialize;

use super::json_io::{emit_json_output, read_cli_input_bytes, read_cli_input_text};

#[derive(Serialize)]
struct TextReport {
    text: String,
}

#[derive(Serialize)]
struct BytesHexReport {
    byte_len: usize,
    data_hex: String,
}

#[derive(Serialize)]
struct CommandValidationReport {
    normalized: String,
}

/// Dispatches `panadiga pico parse-diag` over captured text/codes cleanly via JSON.
pub fn handle_parse_diag(
    kind: &str,
    file: Option<PathBuf>,
    code: Option<i32>,
    json: Option<PathBuf>,
) -> Result<()> {
    if kind == "fat-error" {
        let code_val = code.context("--code is required for fat-error")?;
        return emit_json_output(
            &TextReport {
                text: describe_boot_fat_error_code(code_val).to_string(),
            },
            json.as_deref(),
        );
    }
    let path = file.context("--file is required")?;
    if kind == "raw-log" {
        let raw = read_cli_input_bytes(&path)?;
        let decoded = panadiga_core::pico::decode_captured_log_bytes(&raw);
        return emit_json_output(&TextReport { text: decoded }, json.as_deref());
    }
    let text = read_cli_input_text(&path)?;
    match kind {
        "status" => emit_json_output(&parse_status_output_report(&text), json.as_deref()),
        "pincheck" => emit_json_output(&parse_pincheck_report(&text), json.as_deref()),
        "busdiag" => emit_json_output(&parse_busdiag_report(&text), json.as_deref()),
        "piologic" => emit_json_output(&parse_piologic_report(&text), json.as_deref()),
        "panarsp" => emit_json_output(&parse_panarsp_report(&text), json.as_deref()),
        "bootclass" => {
            let parsed =
                parse_boot_class_line(&text).context("No @P7501_BOOT_CLASS line found in input")?;
            emit_json_output(&parsed, json.as_deref())
        }
        "tbus-audit" => {
            let parsed =
                parse_tbus_audit_line(&text).context("No @P7501_TBUS_AUDIT line found in input")?;
            emit_json_output(&parsed, json.as_deref())
        }
        other => bail!(
            "Unknown diag kind '{other}'. Expected status|pincheck|busdiag|piologic|panarsp|bootclass|tbus-audit|raw-log|fat-error"
        ),
    }
}

/// Validates a non-destructive P7501 bench command against Rust `SAFE_BENCH_COMMANDS`.
pub fn handle_validate_bench_cmd(cmd: &str, json: Option<PathBuf>) -> Result<()> {
    let normalized = validate_safe_bench_command(cmd)?;
    emit_json_output(&CommandValidationReport { normalized }, json.as_deref())
}

/// Exposes pure Rust GDB RSP framing, E05 write guard, reply scanner, and `$g` register decoding.
#[allow(clippy::too_many_arguments)]
pub fn handle_rsp_codec(
    kind: &str,
    input: Option<PathBuf>,
    addr: Option<String>,
    len: usize,
    chunk_size: usize,
    max_chunk_size: Option<usize>,
    expected_len: Option<usize>,
    allow_write: bool,
    binary_x: bool,
    json: Option<PathBuf>,
) -> Result<()> {
    match kind {
        "is-ipl-readonly" => {
            let addr_u32 = parse_u32_radix(addr.as_deref().context("--addr is required")?)?;
            emit_json_output(&is_ipl_readonly_address(addr_u32, len), json.as_deref())
        }
        "validate-dump" => {
            let addr_u32 = parse_u32_radix(addr.as_deref().context("--addr is required")?)?;
            validate_dump_range(
                addr_u32,
                len,
                chunk_size,
                max_chunk_size.unwrap_or(MAX_M_CHUNK),
            )?;
            emit_json_output(&true, json.as_deref())
        }
        "checksum" => {
            let raw = read_cli_input_bytes(&input.context("--input is required")?)?;
            emit_json_output(&rsp_checksum(&raw), json.as_deref())
        }
        "build-frame" => {
            let raw = read_cli_input_bytes(&input.context("--input is required")?)?;
            let frame = build_rsp_frame(&raw, allow_write)?;
            emit_json_output(
                &BytesHexReport {
                    byte_len: frame.len(),
                    data_hex: hex::encode(frame),
                },
                json.as_deref(),
            )
        }
        "build-write" => {
            let addr_u32 = parse_u32_radix(addr.as_deref().context("--addr is required")?)?;
            let raw = read_cli_input_bytes(&input.context("--input is required")?)?;
            let frame = build_rsp_write_frame(addr_u32, &raw, binary_x)?;
            emit_json_output(
                &BytesHexReport {
                    byte_len: frame.len(),
                    data_hex: hex::encode(frame),
                },
                json.as_deref(),
            )
        }
        "parse-reply" => {
            let raw = read_cli_input_bytes(&input.context("--input is required")?)?;
            emit_json_output(&parse_rsp_reply(&raw, expected_len), json.as_deref())
        }
        "decode-hex" => {
            let raw = read_cli_input_bytes(&input.context("--input is required")?)?;
            let decoded = decode_rsp_hex_memory(&raw)?;
            emit_json_output(
                &BytesHexReport {
                    byte_len: decoded.len(),
                    data_hex: hex::encode(decoded),
                },
                json.as_deref(),
            )
        }
        "decode-g" => {
            let raw = read_cli_input_bytes(&input.context("--input is required")?)?;
            emit_json_output(&decode_mn103s_g_registers(&raw)?, json.as_deref())
        }
        other => bail!(
            "Unknown rsp-codec kind '{other}'. Expected checksum|is-ipl-readonly|build-frame|build-write|parse-reply|decode-hex|decode-g|validate-dump"
        ),
    }
}

/// Parameter bundle for `panadiga pico ab-boot` operations.
pub struct AbBootRequest<'a> {
    pub kind: &'a str,
    pub mode: &'a str,
    pub window_ms: u32,
    pub input: Option<PathBuf>,
    pub run_index: usize,
    pub command: &'a str,
    pub duration_ms: f64,
    pub json: Option<PathBuf>,
}

/// Exposes A/B cold-boot harness classification, summary statistics, and CSV serialization.
pub fn handle_ab_boot(req: AbBootRequest<'_>) -> Result<()> {
    match req.kind {
        "build-cmd" => {
            let cmd = build_ab_boot_command(req.mode, req.window_ms)?;
            emit_json_output(&TextReport { text: cmd }, req.json.as_deref())
        }
        "classify" => {
            let raw_log = read_cli_input_text(&req.input.context("--input is required")?)?;
            let run = classify_ab_boot_log(
                req.run_index,
                req.mode,
                req.command,
                &raw_log,
                req.duration_ms,
            );
            emit_json_output(&run, req.json.as_deref())
        }
        "summarize" => {
            let raw_json = read_cli_input_text(&req.input.context("--input is required")?)?;
            let runs: Vec<AbBootRun> = serde_json::from_str(&raw_json)?;
            emit_json_output(&summarize_ab_runs(&runs), req.json.as_deref())
        }
        "csv" => {
            let raw_json = read_cli_input_text(&req.input.context("--input is required")?)?;
            let runs: Vec<AbBootRun> = serde_json::from_str(&raw_json)?;
            emit_json_output(
                &TextReport {
                    text: format_ab_csv(&runs),
                },
                req.json.as_deref(),
            )
        }
        other => bail!("Unknown ab-boot kind '{other}'. Expected build-cmd|classify|summarize|csv"),
    }
}
