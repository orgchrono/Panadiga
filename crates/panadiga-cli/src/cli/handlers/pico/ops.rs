//! Hardware operations for Pico USB CDC probing, aperture dumping, UF2 flashing, and log decoding.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, bail};
use panadiga_core::constants::AM34_NVRAM_BASE;
use panadiga_core::headers::hex_dump;
use panadiga_core::log_println;
use panadiga_core::nvram::inspect_partial_nvram_capture;
use panadiga_core::pico::cdc::{
    open_cdc, parse_pico_dump_hex_to_bytes, parse_vdd_reply, send_and_read_until,
};
use panadiga_core::pico::{detect_bootsel_drives, flash_uf2_to_drive, trigger_1200_baud_bootsel};

pub(super) fn parse_hex_u32(raw: &str) -> Result<u32> {
    let clean = raw.trim().trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(clean, 16)
        .map_err(|_| anyhow::anyhow!("Invalid 32-bit hex address: '{}'", raw))
}

fn extract_dcsr_value(text: &str) -> Option<u32> {
    for token in text.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| !c.is_ascii_hexdigit() && c != 'x' && c != 'X');
        if let Some(hex_body) = cleaned
            .strip_prefix("0x")
            .or_else(|| cleaned.strip_prefix("0X"))
            && hex_body.len() == 8
            && let Ok(val) = u32::from_str_radix(hex_body, 16)
        {
            return Some(val);
        }
    }
    None
}

fn describe_dcsr_state(dcsr: u32) -> &'static str {
    if dcsr == 0xFFFF_FFFF {
        "FLOATING HIGH (0xFFFFFFFF) - Target powered off or UART lines disconnected"
    } else if dcsr == 0x0000_0000 {
        "ACTIVE BUS / NO HALT BIT (0x00000000) - Typical during CDDIAG/factory loop"
    } else if (dcsr & 0x0000_0001) != 0 {
        "HALTED (Bit 0 = 1) - AM34 CPU frozen under debug control"
    } else {
        "ACTIVE BUS RESPONSE - Target responding on debug interface, Halt bit unset"
    }
}

pub(super) fn query_pico_dcsr_flow(port: &str, baud: u32, cmd: &str) -> Result<()> {
    let mut ser = open_cdc(port, baud, Duration::from_millis(1200))?;
    let prompt_stops = ["PANADIGA>", "P7501>"];
    let ping = send_and_read_until(&mut *ser, "PING", &prompt_stops, Duration::from_millis(800));
    let vdd_raw = send_and_read_until(
        &mut *ser,
        "STATUS",
        &prompt_stops,
        Duration::from_millis(800),
    );
    let vdd = parse_vdd_reply(&vdd_raw);
    let reply = send_and_read_until(&mut *ser, cmd, &prompt_stops, Duration::from_millis(2500));

    log_println!("====================================================================");
    log_println!("   PANADIGA PICO P7501 ASYNC UART TELEMETRY ({})", port);
    log_println!("====================================================================");
    log_println!(
        "PING:   {}",
        if ping.contains("PONG") {
            "PONG OK"
        } else {
            ping.trim()
        }
    );
    if let Some(v) = vdd {
        log_println!("VDD:    {:.2} V (3.3V LVTTL target sense)", v);
    } else {
        log_println!("VDD:    {}", vdd_raw.trim());
    }
    if let Some(dcsr) = extract_dcsr_value(&reply) {
        log_println!("DCSR:   0x{:08X} -> {}", dcsr, describe_dcsr_state(dcsr));
    } else if let Some(ident) = panadiga_core::pico::protocol::parse_ident_line(&reply) {
        log_println!(
            "IDENT:  model={} flash={} jedec=0x{:08X} ipl_crc32=0x{:08X} state={}",
            ident.model,
            ident.flash,
            ident.jedec,
            ident.ipl_crc32,
            ident.state
        );
    }
    log_println!("RAW:    {}", reply.trim());
    log_println!("====================================================================");
    Ok(())
}

pub(super) fn dump_pico_aperture(
    port: &str,
    addr: &str,
    words: usize,
    out: Option<PathBuf>,
    baud: u32,
) -> Result<()> {
    if words == 0 || words > 16384 {
        bail!(
            "Word count {} out of supported range (1..=16384 words)",
            words
        );
    }
    let base_addr = parse_hex_u32(addr)?;
    let mut ser = open_cdc(port, baud, Duration::from_millis(2000))?;
    let mut captured: Vec<u8> = Vec::with_capacity(words * 4);
    let mut remaining = words;
    let mut cursor_addr = base_addr;

    while remaining > 0 {
        let chunk_words = remaining.min(1024);
        let cmd = format!("DUMP 0x{:08X} {}", cursor_addr, chunk_words);
        let reply = send_and_read_until(
            &mut *ser,
            &cmd,
            &["PANADIGA>", "P7501>", "ERR"],
            Duration::from_millis(5000),
        );
        let chunk_bytes = parse_pico_dump_hex_to_bytes(&reply);
        if chunk_bytes.is_empty() {
            bail!(
                "Pico returned no word data for '{}' (raw response: {})",
                cmd,
                reply.trim()
            );
        }
        cursor_addr = cursor_addr.wrapping_add(chunk_bytes.len() as u32);
        remaining = remaining.saturating_sub(chunk_bytes.len() / 4);
        captured.extend_from_slice(&chunk_bytes);
    }

    log_println!("====================================================================");
    log_println!(
        "   LIVE MONITOR APERTURE DUMP @ 0x{:08X} ({} bytes captured)",
        base_addr,
        captured.len()
    );
    log_println!("====================================================================");
    log_println!(
        "{}",
        hex_dump(&captured, base_addr as usize, captured.len().min(512))
    );

    if (AM34_NVRAM_BASE..AM34_NVRAM_BASE + 0x0001_0000).contains(&base_addr)
        && let Ok(report) =
            inspect_partial_nvram_capture(&captured, base_addr, &format!("pico:{}", port))
    {
        log_println!(
            "NVRAM Audit: non-0xFF={}/{} bytes (sectors={}) | SHA-256={}",
            report.nvram_non_ff_bytes,
            captured.len(),
            report.microsectors.len(),
            report.original_sha256
        );
    }

    if let Some(out_path) = out {
        if let Some(parent) = out_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out_path, &captured)?;
        log_println!("[OK] Captured bytes saved to {}", out_path.display());
    }
    Ok(())
}

pub(super) fn flash_pico_firmware(port: Option<String>, file: Option<PathBuf>) -> Result<()> {
    log_println!("====================================================================");
    log_println!("   PANADIGA AUTOMATED PICO FIRMWARE FLASHER (P7501 UART + OCD)");
    log_println!("====================================================================");
    if let Some(ref p) = port {
        log_println!("Initiating 1200-baud touch reset on port {}...", p);
        let _ = trigger_1200_baud_bootsel(p);
    }
    let bootsel_drives = detect_bootsel_drives();
    if bootsel_drives.is_empty() {
        bail!(
            "No Raspberry Pi BOOTSEL drives (RPI-RP2 / RP2350) detected! Hold BOOTSEL button while plugging USB."
        );
    }
    let target_drive = &bootsel_drives[0];
    log_println!("Found active BOOTSEL drive: {}", target_drive.display());

    let (uf2_source, uf2_data) = if let Some(ref custom_path) = file {
        (
            custom_path.display().to_string(),
            std::fs::read(custom_path)?,
        )
    } else {
        let candidates = [
            "crates/panadiga-core/src/pico/firmware_bins/panasonic_p7501_uart_unified.uf2",
            "target/pico-build/panasonic_p7501_uart_unified.uf2",
            "firmware/pico-p7501/panasonic_p7501_uart_unified.uf2",
            "dist/panasonic_p7501_uart_unified.uf2",
        ];
        candidates
            .iter()
            .map(PathBuf::from)
            .find_map(|c| {
                std::fs::read(&c)
                    .ok()
                    .map(|data| (c.display().to_string(), data))
            })
            .unwrap_or_else(|| {
                (
                    "embedded(EMBEDDED_UNIFIED_UF2)".to_string(),
                    panadiga_core::pico::flasher::EMBEDDED_UNIFIED_UF2.to_vec(),
                )
            })
    };
    log_println!(
        "Using UF2 source: {} ({} bytes)",
        uf2_source,
        uf2_data.len()
    );
    let report = flash_uf2_to_drive(target_drive, &uf2_data)?;
    log_println!(
        "[SUCCESS] {} ({} bytes, family={}, sha256={})",
        report.message,
        report.uf2_size_bytes,
        report.board_family,
        report.sha256_hash
    );
    Ok(())
}

pub(super) fn decode_log_file(file: PathBuf, json: Option<PathBuf>) -> Result<()> {
    let raw = super::super::json_io::read_cli_input_bytes(&file)?;
    let text = panadiga_core::pico::decode_captured_log_bytes(&raw);
    let report = panadiga_core::pico::parse_bench_log_text(&text);
    log_println!("=== PANADIGA OFFLINE AM34 / PICO LOG DECODER ===");
    log_println!("File           : {}", file.display());
    log_println!(
        "Loader         : {}",
        report.loader_version.as_deref().unwrap_or("unknown")
    );
    log_println!(
        "Boot Mode      : {}",
        report.boot_mode.as_deref().unwrap_or("unknown")
    );
    if let Some(code) = report.bgo_failed_code {
        log_println!("BGO Failed     : [{code}]");
    }
    if let Some(lba) = report.sd_capacity_lba {
        log_println!("SD LBA Cap     : 0x{lba:X}");
    }
    if !report.sdcd_errors.is_empty() {
        log_println!("SD Errors      : {:?}", report.sdcd_errors);
    }
    if !report.drive_error_codes.is_empty() {
        log_println!("Drive Errors   : {:?}", report.drive_error_codes);
    }
    for s in &report.sense_packets {
        log_println!(
            "SENSE          : {} -> {}",
            s.mecha_error_code.as_deref().unwrap_or("no-mecha-code"),
            s.interpretation
        );
    }
    if let Some(ref bc) = report.boot_class {
        log_println!(
            "BootClass      : verdict={} build={} avout_ok={} fpc_err={} bgo_err={} op7f={:?}",
            bc.verdict,
            bc.build,
            bc.avout_ok,
            bc.fpc_err,
            bc.bgo_err,
            bc.op7f
        );
    }
    if let Some(ref ta) = report.tbus_audit {
        log_println!(
            "TbusAudit      : frames={} br_rejected={} nak7f={} ops={:?}",
            ta.frames,
            ta.br_rejected,
            ta.nak7f,
            ta.ops
        );
    }
    if let Some(v) = report.vdd_volts {
        log_println!("VDD Telemetry  : {v:.2} V");
    }
    for n in &report.notes {
        log_println!("Note           : {n}");
    }
    super::super::unbrick::render::emit_json_if_requested(&report, json.as_deref())?;
    Ok(())
}
