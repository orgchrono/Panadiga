//! CLI sub-handlers for NOR dumper stubs, dump stream decoding, and Matshita ODD emulation.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::nvram::calculate_hashes;
use panadiga_core::re::matshita_emu::{
    MatshitaBootScenario, MatshitaOddEmulator, build_sample_unlock_frm, parse_hex_payload,
    parse_unlock_container_mode, verify_unlock_frm_container,
};
use panadiga_core::re::{
    build_nor_dumper_artifact, decode_nor_dump_stream, resolve_nor_stub_load_addr,
};
use serde::Serialize;

use super::super::json_io::{read_cli_input_bytes, write_cli_output_bytes};
use super::render::{
    emit_json_if_requested, format_frm_unlock_verification_lines, format_matshita_boot_lines,
    format_matshita_injection_lines, print_lines,
};
use crate::cli::args::unbrick::MatshitaEmuAction;

#[derive(Serialize)]
struct NorStubJsonReport {
    start_addr: u32,
    total_len: u32,
    block_size: u32,
    load_addr: u32,
    wrapped: bool,
    raw_stub_len: usize,
    payload_len: usize,
    sha256: String,
    data_hex: String,
}

#[derive(Serialize)]
struct NorDumpJsonReport {
    start_addr: u32,
    total_len: u32,
    block_size: u32,
    sha256: String,
    block_checksums: Vec<u32>,
    data_hex: String,
}

pub(super) fn handle_nor_stub(
    start: &str,
    len: &str,
    block_size: u32,
    load_addr: Option<&str>,
    out: Option<PathBuf>,
    wrap_digadiag: bool,
    json: Option<&std::path::Path>,
) -> Result<()> {
    let start_u32 = parse_u32_radix(start)?;
    let len_u32 = parse_u32_radix(len)?;
    let explicit_load = load_addr.map(parse_u32_radix).transpose()?;
    let load_u32 = resolve_nor_stub_load_addr(explicit_load, wrap_digadiag);
    let (raw_stub_len, payload) =
        build_nor_dumper_artifact(start_u32, len_u32, block_size, load_u32, wrap_digadiag)?;
    if let Some(p) = out {
        write_cli_output_bytes(&p, &payload)?;
        panadiga_core::log_println!(
            "Wrote MN103S NOR dumper ({} B, wrapped={}, load=0x{:08X}) -> {}",
            payload.len(),
            wrap_digadiag,
            load_u32,
            p.display()
        );
    } else {
        panadiga_core::log_println!(
            "Assembled MN103S NOR dumper ({} B, start=0x{:08X}, len=0x{:X}, load=0x{:08X})",
            payload.len(),
            start_u32,
            len_u32,
            load_u32
        );
    }
    let (_, sha256) = calculate_hashes(&payload);
    let report = NorStubJsonReport {
        start_addr: start_u32,
        total_len: len_u32,
        block_size,
        load_addr: load_u32,
        wrapped: wrap_digadiag,
        raw_stub_len,
        payload_len: payload.len(),
        sha256,
        data_hex: hex::encode(&payload),
    };
    emit_json_if_requested(&report, json)
}

pub(super) fn handle_decode_nor_dump(
    input: &std::path::Path,
    out: Option<PathBuf>,
    json: Option<&std::path::Path>,
) -> Result<()> {
    let stream = read_cli_input_bytes(input)?;
    let decoded = decode_nor_dump_stream(&stream)?;
    if let Some(p) = out {
        write_cli_output_bytes(&p, &decoded.data)?;
    }
    panadiga_core::log_println!(
        "Decoded verified NOR dump: 0x{:08X} ({} bytes, blocks={}, SHA-256={})",
        decoded.start_addr,
        decoded.total_len,
        decoded.block_checksums.len(),
        decoded.sha256
    );
    let report = NorDumpJsonReport {
        start_addr: decoded.start_addr,
        total_len: decoded.total_len,
        block_size: decoded.block_size,
        sha256: decoded.sha256,
        block_checksums: decoded.block_checksums,
        data_hex: hex::encode(decoded.data),
    };
    emit_json_if_requested(&report, json)
}

pub(super) fn handle_matshita_emu(action: MatshitaEmuAction) -> Result<()> {
    match action {
        MatshitaEmuAction::Boot {
            scenario,
            frm,
            json,
        } => {
            let parsed: MatshitaBootScenario = scenario.parse()?;
            let mut emu = MatshitaOddEmulator::new(parsed);
            if let Some(frm_path) = frm {
                let bytes = fs::read(&frm_path)
                    .with_context(|| format!("Reading {}", frm_path.display()))?;
                let _ = emu.mount_unlock_frm("dvd", &bytes)?;
            }
            let report = emu.run_boot_sequence()?;
            print_lines(&format_matshita_boot_lines(&report));
            emit_json_if_requested(&report, json.as_deref())
        }
        MatshitaEmuAction::InjectWrite {
            offset,
            mode,
            vendor_ed,
            va,
            payload_hex,
            payload_file,
            json,
        } => {
            let payload = match (payload_hex, payload_file) {
                (Some(h), None) => parse_hex_payload(&h)?,
                (None, Some(p)) => {
                    fs::read(&p).with_context(|| format!("Reading {}", p.display()))?
                }
                (Some(_), Some(_)) => {
                    bail!("Pass either --payload-hex or --payload-file, not both")
                }
                (None, None) => b"PANADIGA_INJECT".to_vec(),
            };
            let target_va = va.as_deref().map(parse_u32_radix).transpose()?;
            let mut emu = MatshitaOddEmulator::default();
            let (rec, snap) = emu.run_inject_write(target_va, offset, mode, vendor_ed, &payload)?;
            print_lines(&format_matshita_injection_lines(&rec, &snap));
            emit_json_if_requested(
                &serde_json::json!({ "record": rec, "snapshot": snap }),
                json.as_deref(),
            )
        }
        MatshitaEmuAction::SerialCmd {
            unlock_pass,
            cmd,
            json,
        } => {
            let mut emu = MatshitaOddEmulator::default();
            if unlock_pass {
                let _ = emu.execute_drv_serial_command("PASS 0x4d415453 0x48495441")?;
            }
            let resp = emu.execute_drv_serial_command(&cmd)?;
            println!("DRV CLI [{}]: {}", resp.status, resp.command);
            print_lines(&resp.output_lines);
            emit_json_if_requested(&resp, json.as_deref())
        }
        MatshitaEmuAction::VerifyFrm { kind, input, json } => {
            let bytes = fs::read(&input).with_context(|| format!("Reading {}", input.display()))?;
            let summary = verify_unlock_frm_container(&kind, &bytes)?;
            print_lines(&format_frm_unlock_verification_lines(&summary));
            emit_json_if_requested(&summary, json.as_deref())
        }
        MatshitaEmuAction::PackUnlockFrm {
            kind,
            payload_file,
            out,
            json,
        } => {
            let mode = parse_unlock_container_mode(&kind)?;
            let prog_payload = match payload_file {
                Some(p) => fs::read(&p).with_context(|| format!("Reading {}", p.display()))?,
                None => vec![0xCBu8; 1024],
            };
            let (bytes, summary) = build_sample_unlock_frm(mode, &prog_payload)?;
            if let Some(parent) = out.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent)?;
            }
            fs::write(&out, &bytes)?;
            println!(
                "Wrote unlock {} -> {}",
                summary.container_kind,
                out.display()
            );
            print_lines(&format_frm_unlock_verification_lines(&summary));
            emit_json_if_requested(&summary, json.as_deref())
        }
    }
}
