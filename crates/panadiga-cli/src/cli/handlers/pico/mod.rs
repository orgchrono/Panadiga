#![deny(unsafe_code)]

//! CLI handlers for RP2040/RP2350 Pico hardware probing, CDDIAG DCSR check, memory DUMP, flashing, and GDB RSP bridge.

mod bench_wizard;
mod error_lookup;
pub mod live_ops;
mod ops;

pub use bench_wizard::{BenchWizardConfig, handle_bench_wizard};
pub use error_lookup::handle_error_lookup;

use super::super::args::{GdbConfigArgs, PicoActions, PicoArgs};
use anyhow::{Result, bail};
use panadiga_core::pico::{
    GdbBridgeTargetMode, list_pico_devices, merge_unified_uf2, ping_pico, run_gdb_bridge_with_mode,
};
use panadiga_core::{log_info, log_println};

pub fn handle_pico(args: PicoArgs) -> Result<()> {
    match args.action {
        PicoActions::List => {
            let devices = list_pico_devices()?;
            log_println!("====================================================================");
            log_println!("   PANADIGA HARDWARE DISCOVERY: SERIAL COM PORTS & PICO CONTROLLERS");
            log_println!("====================================================================");
            if devices.is_empty() {
                log_println!("No USB serial devices detected on system.");
            } else {
                for dev in &devices {
                    log_println!(
                        "Port: {:<10} | {:<32} | VID:{:04X} PID:{:04X} | {}",
                        dev.port_name,
                        dev.architecture.to_string(),
                        dev.vid,
                        dev.pid,
                        dev.description
                    );
                }
            }
            log_println!("--------------------------------------------------------------------");
        }
        PicoActions::Ping { port, baud } => {
            let report = ping_pico(&port, baud)?;
            log_println!("{}", report);
        }
        PicoActions::Status { port, baud } | PicoActions::Dcsr { port, baud } => {
            ops::query_pico_dcsr_flow(&port, baud, "IDENTIFY")?;
        }
        PicoActions::BreakIn(b) => {
            if b.mode > 3 {
                bail!("Invalid Cold Break-In mode {} (expected 0..=3)", b.mode);
            }
            let cmd = format!("BREAK_IN {}", b.mode);
            ops::query_pico_dcsr_flow(&b.port, b.baud, &cmd)?;
        }
        PicoActions::Dump(d) => {
            ops::dump_pico_aperture(&d.port, &d.addr, d.words, d.out, d.baud)?;
        }
        PicoActions::GdbBridge(g) => {
            let parsed_mode =
                GdbBridgeTargetMode::parse_mode(&g.target_mode).map_err(|e| anyhow::anyhow!(e))?;
            run_gdb_bridge_with_mode(&g.port, g.baud, &g.listen, parsed_mode)?;
        }
        PicoActions::BenchWizard(w) => {
            handle_bench_wizard(BenchWizardConfig {
                port_opt: w.port,
                baud: w.baud,
                window_ms: w.window_ms,
                profile: w.profile,
                wake_timeout_ms: w.wake_timeout_ms,
                jig_hold_ms: w.jig_hold_ms,
                non_interactive: w.non_interactive,
                vfd_cue: w.vfd_cue,
                active_jig: w.active_jig,
                out_dir: w.out_dir,
            })?;
        }
        PicoActions::ErrorLookup { query, json } => {
            handle_error_lookup(&query, json)?;
        }
        PicoActions::Flash(f) => {
            ops::flash_pico_firmware(f.port, f.file)?;
        }
        PicoActions::MergeUf2(m) => {
            let (blocks, sha256) = merge_unified_uf2(&m.rp2040, &m.rp2350, &m.out)?;
            log_info!(
                "[OK] Unified Universal UF2 generated: {} ({} blocks, SHA-256: {})",
                m.out.display(),
                blocks,
                sha256
            );
        }
        PicoActions::DecodeLog(d) => {
            ops::decode_log_file(d.file, d.json)?;
        }
        PicoActions::ParseDiag(p) => {
            super::pico_bench::handle_parse_diag(&p.kind, p.file, p.code, p.json)?;
        }
        PicoActions::ValidateBenchCmd(v) => {
            super::pico_bench::handle_validate_bench_cmd(&v.cmd, v.json)?;
        }
        PicoActions::RspCodec(r) => {
            super::pico_bench::handle_rsp_codec(
                &r.kind,
                r.input,
                r.addr,
                r.len,
                r.chunk_size,
                r.max_chunk_size,
                r.expected_len,
                r.allow_write,
                r.binary_x,
                r.json,
            )?;
        }
        PicoActions::AbBoot(a) => {
            super::pico_bench::handle_ab_boot(super::pico_bench::AbBootRequest {
                kind: &a.kind,
                mode: &a.mode,
                window_ms: a.window_ms,
                input: a.input,
                run_index: a.run_index,
                command: &a.command,
                duration_ms: a.duration_ms,
                json: a.json,
            })?;
        }
        PicoActions::CaptureNor(c) => live_ops::handle_capture_nor(c)?,
        PicoActions::DavisProbe(d) => live_ops::handle_davis_probe(d)?,
    }
    Ok(())
}

pub fn handle_gdb_config(args: GdbConfigArgs) -> Result<()> {
    log_println!("====================================================================");
    log_println!("      Panadiga GDB RSP Bridge Configuration (P7501 Async UART / GDB RSP)");
    log_println!("====================================================================");
    log_println!("• Serial Port:   {}", args.port);
    log_println!("• Listen Socket: {}:{}", args.host, args.tcp_port);
    log_println!();
    log_println!("GDB Command to Connect:");
    log_println!("  (gdb) target remote {}:{}", args.host, args.tcp_port);
    log_println!("  (gdb) set architecture mn10300");
    log_println!("  (gdb) info registers");
    log_println!("====================================================================");
    Ok(())
}
