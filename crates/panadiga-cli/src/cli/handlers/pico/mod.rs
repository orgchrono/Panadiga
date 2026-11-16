//! CLI handlers for RP2040/RP2350 Pico hardware probing, CDDIAG DCSR check, memory DUMP, flashing, and GDB RSP bridge.

mod bench_wizard;
mod ops;

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
            ops::query_pico_dcsr_flow(&port, baud, "HANDSHAKE")?;
        }
        PicoActions::BreakIn { port, mode, baud } => {
            if mode > 3 {
                bail!("Invalid Cold Break-In mode {} (expected 0..=3)", mode);
            }
            let cmd = format!("BREAK_IN {}", mode);
            ops::query_pico_dcsr_flow(&port, baud, &cmd)?;
        }
        PicoActions::Dump {
            port,
            addr,
            words,
            out,
            baud,
        } => {
            ops::dump_pico_aperture(&port, &addr, words, out, baud)?;
        }
        PicoActions::GdbBridge {
            port,
            baud,
            listen,
            target_mode,
        } => {
            let parsed_mode =
                GdbBridgeTargetMode::parse_mode(&target_mode).map_err(|e| anyhow::anyhow!(e))?;
            run_gdb_bridge_with_mode(&port, baud, &listen, parsed_mode)?;
        }
        PicoActions::BenchWizard {
            port,
            baud,
            window_ms,
            non_interactive,
            vfd_cue,
            active_jig,
            out_dir,
        } => {
            bench_wizard::run_bench_wizard(
                port,
                baud,
                window_ms,
                non_interactive,
                vfd_cue,
                active_jig,
                out_dir,
            )?;
        }
        PicoActions::ErrorLookup { query, json } => {
            bench_wizard::run_error_lookup(&query, json)?;
        }
        PicoActions::Flash { port, file } => {
            ops::flash_pico_firmware(port, file)?;
        }
        PicoActions::MergeUf2 {
            rp2040,
            rp2350,
            out,
        } => {
            let (blocks, sha256) = merge_unified_uf2(&rp2040, &rp2350, &out)?;
            log_info!(
                "[OK] Unified Universal UF2 generated: {} ({} blocks, SHA-256: {})",
                out.display(),
                blocks,
                sha256
            );
        }
        PicoActions::DecodeLog { file, json } => {
            ops::decode_log_file(file, json)?;
        }
        PicoActions::ParseDiag {
            kind,
            file,
            code,
            json,
        } => {
            super::pico_bench::handle_parse_diag(&kind, file, code, json)?;
        }
        PicoActions::ValidateBenchCmd { cmd, json } => {
            super::pico_bench::handle_validate_bench_cmd(&cmd, json)?;
        }
        PicoActions::RspCodec {
            kind,
            input,
            addr,
            len,
            chunk_size,
            max_chunk_size,
            expected_len,
            allow_write,
            binary_x,
            json,
        } => {
            super::pico_bench::handle_rsp_codec(
                &kind,
                input,
                addr,
                len,
                chunk_size,
                max_chunk_size,
                expected_len,
                allow_write,
                binary_x,
                json,
            )?;
        }
        PicoActions::AbBoot {
            kind,
            mode,
            window_ms,
            input,
            run_index,
            command,
            duration_ms,
            json,
        } => {
            super::pico_bench::handle_ab_boot(super::pico_bench::AbBootRequest {
                kind: &kind,
                mode: &mode,
                window_ms,
                input,
                run_index,
                command: &command,
                duration_ms,
                json,
            })?;
        }
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
