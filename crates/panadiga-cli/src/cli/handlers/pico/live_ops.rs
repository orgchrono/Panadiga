#![deny(unsafe_code)]

//! Live Pico PANAMON1> handlers for `capture-nor` and `davis-probe` (transport-generic for offline tests).

use std::time::Duration;

use anyhow::{Context, Result, bail};
use panadiga_core::constants::{
    AM34_EH55_FLASH_BGO_BASE, AM34_EH55_FLASH_BGO_SIZE, DEFAULT_BAUD_RATE,
};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::log_println;
use panadiga_core::pico::ndb::{NdbSession, SerialNdbTransport, replay_ndb_evidence_log};
use panadiga_core::pico::panamon1::{Panamon1Client, Panamon1Transport, SerialPanamon1Transport};
use panadiga_core::re::davis_probe::{DavisProbeConfig, DavisProbeEngine, DavisProbeReport};
use panadiga_core::re::nor_dumper::{NorCaptureOrchestrator, NorCaptureResult, NorSlicePreset};

use super::super::json_io::{emit_json_output, write_cli_output_bytes};
use crate::cli::args::pico::{
    CaptureNorArgs, DavisProbeArgs, SiliconReplayArgs, SiliconSnapshotArgs,
};

pub(crate) fn parse_nor_slice_preset(
    slice: &str,
    start: Option<&str>,
    len: Option<&str>,
) -> Result<NorSlicePreset> {
    match slice.trim().to_ascii_lowercase().as_str() {
        "full" | "full8mb" | "8mb" => Ok(NorSlicePreset::Full8Mb),
        "bgo" => Ok(NorSlicePreset::Custom {
            start_va: AM34_EH55_FLASH_BGO_BASE,
            len: AM34_EH55_FLASH_BGO_SIZE,
        }),
        "nvram" | "nvram64k" => Ok(NorSlicePreset::Nvram64k),
        "loader" | "loader256k" | "ipl" => Ok(NorSlicePreset::Loader256k),
        "custom" => {
            let s_raw = start.context("--start <HEX> is required when --slice custom")?;
            let l_raw = len.context("--len <HEX> is required when --slice custom")?;
            Ok(NorSlicePreset::Custom {
                start_va: parse_u32_radix(s_raw)?,
                len: parse_u32_radix(l_raw)?,
            })
        }
        other => {
            bail!("Unsupported NOR slice '{other}': expected full, bgo, nvram, loader, or custom")
        }
    }
}

pub(crate) fn parse_davis_drive_sel(drive: &str) -> Result<u8> {
    match drive.trim().to_ascii_lowercase().as_str() {
        "master" | "both" => Ok(0xA0),
        "slave" => Ok(0xB0),
        other => bail!("Unsupported --drive '{other}': expected master, slave, or both"),
    }
}

pub fn run_capture_nor_with_transport<T: Panamon1Transport>(
    transport: T,
    args: &CaptureNorArgs,
) -> Result<NorCaptureResult> {
    let preset = parse_nor_slice_preset(&args.slice, args.start.as_deref(), args.len.as_deref())?;
    let orch = NorCaptureOrchestrator::try_default()?;
    let mut client = Panamon1Client::new(transport);
    let cap = orch.capture_preset(&mut client, preset, &mut |stage, cur, tot| {
        log_println!("[NOR {cur}/{tot}] {stage}");
    })?;
    let _ = client.close_bridge_and_isolate()?;
    write_cli_output_bytes(&args.out, &cap.data)?;
    log_println!(
        "[OK] Captured NOR 0x{:08X} ({} B, sha256={}) -> {}",
        cap.start_va,
        cap.total_len,
        cap.sha256,
        args.out.display()
    );
    emit_json_output(&cap, args.json.as_deref())?;
    Ok(cap)
}

pub(crate) fn handle_capture_nor(args: CaptureNorArgs) -> Result<()> {
    let transport =
        SerialPanamon1Transport::open(&args.port, DEFAULT_BAUD_RATE, Duration::from_millis(1_500))?;
    let _ = run_capture_nor_with_transport(transport, &args)?;
    Ok(())
}

pub fn run_davis_probe_with_transport<T: Panamon1Transport>(
    transport: T,
    args: &DavisProbeArgs,
) -> Result<DavisProbeReport> {
    let drive_sel = parse_davis_drive_sel(&args.drive)?;
    let cfg = DavisProbeConfig {
        drive_sel,
        run_identify: !args.no_identify,
        ..DavisProbeConfig::default()
    };
    let mut client = Panamon1Client::new(transport);
    let report = {
        let mut engine = DavisProbeEngine::new(&mut client, cfg);
        engine.run_probe()?
    };
    let _ = client.close_bridge_and_isolate()?;
    if !args.no_identify
        && let Some(ref out_ec) = args.out_ec
        && let Some(ref ec_512) = report.identify_raw_512
    {
        write_cli_output_bytes(out_ec, ec_512)?;
        log_println!(
            "[OK] Wrote 512-byte ATA IDENTIFY DEVICE payload -> {}",
            out_ec.display()
        );
    }
    log_println!(
        "[DAVIS] class={:?} fe_released={} C0001008=0x{:04X}->0x{:04X} model={:?}",
        report.classification,
        report.fe_released,
        report.c0001008_before,
        report.c0001008_after,
        report.identify_info.as_ref().map(|i| i.model.as_str())
    );
    emit_json_output(&report, args.json.as_deref())?;
    Ok(report)
}

pub(crate) fn handle_davis_probe(args: DavisProbeArgs) -> Result<()> {
    let transport =
        SerialPanamon1Transport::open(&args.port, DEFAULT_BAUD_RATE, Duration::from_millis(1_500))?;
    let _ = run_davis_probe_with_transport(transport, &args)?;
    Ok(())
}

pub(crate) fn handle_silicon_replay(args: SiliconReplayArgs) -> Result<()> {
    let raw_bytes = super::super::json_io::read_cli_input_bytes(&args.log)?;
    let raw_text = panadiga_core::pico::decode_captured_log_bytes(&raw_bytes);
    let label = args
        .log
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("log");
    let summary = replay_ndb_evidence_log(&raw_text, label)?;
    log_println!(
        "[SILICON REPLAY] {} | dd_reads={} write_echoes={} standby_gate={} bcu_fault={:?}",
        summary.source_name,
        summary.dd_reads.len(),
        summary.write_echoes.len(),
        summary.standby_gate_tripped,
        summary
            .bcu_fault
            .as_ref()
            .map(|f| (&f.bcberr_hex, f.fault_address))
    );
    if let Some(ref caveat) = summary.main3060np_osd_caveat {
        log_println!("Note: {caveat}");
    }
    emit_json_output(&summary, args.json.as_deref())?;
    Ok(())
}

pub(crate) fn handle_silicon_snapshot(args: SiliconSnapshotArgs) -> Result<()> {
    let va = parse_u32_radix(&args.addr)?;
    let transport = SerialNdbTransport::connect_and_enter_bridge(&args.port)?;
    let mut session = NdbSession::new(transport);
    let snap = session.read_words(va, args.len.max(4))?;
    let mut tr = session.into_transport();
    let _ = tr.close_safely();
    log_println!(
        "[SILICON DD] 0x{:08X} ({} B): {:02X?}",
        snap.base_va,
        snap.bytes.len(),
        snap.bytes
    );
    emit_json_output(&snap, args.json.as_deref())?;
    Ok(())
}
