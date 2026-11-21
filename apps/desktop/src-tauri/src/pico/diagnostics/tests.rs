use std::time::Duration;

use super::*;

fn simulate_mock_diagnostic(raw_cmd: &str) -> String {
    let upper = raw_cmd.trim().to_ascii_uppercase();
    if upper == "PINCHECK" {
        return "\
=== P7501 6-PIN ELECTRICAL MATRIX (PINCHECK) ===
Pin               | Hi-Z | Pull-Down | Pull-Up | Electrical State
------------------+------+-----------+---------+----------------------------------------
GP2 (Pin6 RXD   ) |  1   |     0     |    1    | HI-Z / UART RX 115200 8N1 (R7572 Pull-Up)
GP3 (Pin5 NSW5V ) |  1   |     1     |    1    | HI-Z SENSE ONLY (Static rail ~3.29V; IC1506 ~5.15V)
GP4 (Pin3 TXD   ) |  1   |     1     |    1    | IDLE HIGH (R7570 10k / Q7514 TX)
GP5 (Pin2 XRESET) |  1   |     1     |    1    | STRONG HIGH (Open-Drain Control)
GP26(Pin1 SWUART) | 3.28V|   2.69V   |  3.29V  | SW_UART / ADC0 Rail Sense
------------------------------------------------------------------------
While GP5 (XRESET) asserted LOW (0V): GP2=1, GP3=1, GP4=1, GP5=0
XRESET Release Rise Time (Hi-Z)     : 40 us
Post-Reset 250ms Transitions        : GP2=872, GP3=0, GP4=0, GP5=2
PINCHECK: OK"
            .to_string();
    }
    if upper.starts_with("SNIFF") {
        return "\
SNIFF RESULT (250 ms):
  RXD/EXT(GP2) : Final=1, Transitions=0
  NSW5V  (GP3) : Final=1, Transitions=24
  TXD/DAT(GP4) : Final=1, Transitions=68
  XRESET (GP5) : Final=1, Transitions=0
  [STATUS] ACTIVE BUS ACTIVITY DETECTED! (GP2=0, GP3=24, GP4=68, GP5=0)
SNIFF: OK"
            .to_string();
    }
    if upper == "BUSDIAG" {
        return "\
=== P7501 ELECTRICAL CONTENTION & DUAL-CHANNEL DIAGNOSTIC (BUSDIAG) ===
State 0 (SW_UART=HiZ, XRESET=HiZ):
  GP4 (MAX RXD) @ 2mA  -> pad_low=0, release_rise_ticks=12
  GP4 (MAX RXD) @ 12mA -> pad_low=0, release_rise_ticks=8
Boot Transitions (NORMAL SW_UART=HiZ):
  Phase A (0..1300ms)   : GP2=284 GP3=12 GP4=420 GP5=2
  Phase B (1300..3500ms): GP2=96 GP3=0 GP4=140 GP5=0
BUSDIAG: OK"
            .to_string();
    }
    if upper == "CFI" {
        return "CFI: OK (Spansion S29JL064H QRY Validated, Size: 8 MB / 64 Mbit)".to_string();
    }
    if upper.starts_with("DUMP") {
        return "\
[DUMP] Reading 16 words (64 bytes) from 0x807F0000...
807F0000: FFFFFFFF FFFFFFFF 55AA0100 00000001
807F0010: 44494741 35354845 00000000 FFFFFFFF
807F0020: FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF
807F0030: FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF
DUMP: OK"
            .to_string();
    }
    if upper.starts_with("BOOTCAP") || upper.starts_with("UARTMON") || upper.starts_with("JIGBOOT")
    {
        return "\
=== BOOTCAP: PANASONIC 115200 8N1 UART CAPTURE ON GP2 (42 bytes, 7500 ms) ===
AM34 IPL BOOT v1.18 OK | DDR CHECK OK
BOOTCAP: OK"
            .to_string();
    }
    if upper.starts_with("FMU") || upper.starts_with("UART") {
        return "UART/FMU Probe Capture (18 bytes): SERI_READY OK\nFMU: OK".to_string();
    }
    if upper == "IDENTIFY" {
        return "@P7501_IDENT vdd_mv=3280 nsw5v=1 elec=AWAKE_READY loader=\"Loader ver0.61\" jedec=0x0001227E flash=S29JL064H ipl_crc32=0xA5B6C7D8 hw=\"VEP79138\" tv=PAL/NTSC state=READY model=DMR-EH55 tier=VERIFIED_BANNER caps=0x07 ssot=1234ABCD\nIDENTIFY: OK".to_string();
    }
    if upper == "AUTODETECT" {
        return "AUTODETECT: OK (8 MB NOR Flash, EEMUL NVRAM @ 0x807F0000)".to_string();
    }
    if upper.starts_with("LED")
        || upper == "ISOLATE"
        || upper == "ENGAGE"
        || upper == "RESET"
        || upper == "BANNER"
        || upper == "PING"
        || upper == "STATUS"
    {
        return format!(
            "{}: OK VDD=3.28V",
            upper.split_whitespace().next().unwrap_or("CMD")
        );
    }
    format!("{}: OK", upper)
}

#[test]
fn compose_command_validates_modes_and_blocks_injection() {
    assert!(compose_diagnostic_command("NOPE", None).is_err());
    assert!(compose_diagnostic_command("BREAK_IN", Some("9")).is_err());
    assert!(compose_diagnostic_command("FLASH_SIZE", Some("32")).is_err());
    assert!(compose_diagnostic_command("SNIFF", Some("250\nRESET")).is_err());
    let (cmd, dur) = compose_diagnostic_command("BOOTCAP", Some("3000")).unwrap();
    assert_eq!(cmd, "BOOTCAP 3000");
    assert_eq!(dur, Duration::from_millis(5500));

    assert!(compose_diagnostic_command("LED", Some("OFF")).is_ok());
    assert!(compose_diagnostic_command("LED", Some("AUTO")).is_ok());
    let (stream_cmd, stream_dur) =
        compose_diagnostic_command("UARTSTREAM", Some("8000 JIG:350")).unwrap();
    assert_eq!(stream_cmd, "UARTSTREAM 8000 JIG:350");
    assert_eq!(stream_dur, Duration::from_millis(10500));

    let (baud_cmd, baud_dur) = compose_diagnostic_command("BAUD", Some("115200")).unwrap();
    assert_eq!(baud_cmd, "BAUD 115200");
    assert_eq!(baud_dur, Duration::from_millis(3000));

    let (sd_cmd, sd_dur) = compose_diagnostic_command("SDSNIFF", Some("6000 JIG:350")).unwrap();
    assert_eq!(sd_cmd, "SDSNIFF 6000 JIG:350");
    assert_eq!(sd_dur, Duration::from_millis(8500));

    let (pwrosc_cmd, pwrosc_dur) = compose_diagnostic_command("PWROSC", Some("2000")).unwrap();
    assert_eq!(pwrosc_cmd, "PWROSC 2000");
    assert_eq!(pwrosc_dur, Duration::from_millis(4500));

    let (ka_cmd, ka_dur) = compose_diagnostic_command("M0KEEPALIVE", Some("ON 10000")).unwrap();
    assert_eq!(ka_cmd, "M0KEEPALIVE ON 10000");
    assert!(ka_dur <= Duration::from_millis(4000));
}

#[test]
fn explicit_ok_prevents_target_uart_error_strings_from_failing_result() {
    let log = "\
=== UARTSTREAM PIO1 (115200 bps, 120 bytes, 8000 ms) ===
BGO driver init failed[1].
[SDCD] Set SD Bus Width fail ERROR:-91006
UARTSTREAM: OK";
    let res = parse_diagnostic_output("UARTSTREAM", "UARTSTREAM 8000 JIG:350", log, 8100);
    assert!(res.success);

    let err_res = parse_diagnostic_output(
        "JOB",
        "JOB NVRAM_CLEAN CONFIRM",
        "JOB: ERROR code=E05 reason=CONFIRM_REQUIRED",
        100,
    );
    assert!(!err_res.success);
}

#[test]
fn parses_pincheck_dump_and_sniff_outputs_cleanly() {
    let backend = crate::state::PanadigaBackendState::new();
    assert!(execute_diagnostic_on_port(&backend, "MOCK_RP2040", "PINCHECK", None).is_err());

    let pincheck = parse_diagnostic_output(
        "PINCHECK",
        "PINCHECK",
        &simulate_mock_diagnostic("PINCHECK"),
        45,
    );
    assert!(pincheck.success);
    assert_eq!(pincheck.pincheck_rows.len(), 5);
    assert_eq!(pincheck.xreset_rise_us.as_deref(), Some("40 us"));
    assert!((pincheck.vdd_volts.unwrap() - 3.28).abs() < 0.01);

    let dump = parse_diagnostic_output(
        "DUMP",
        "DUMP 0x807F0000 16",
        &simulate_mock_diagnostic("DUMP 0x807F0000 16"),
        45,
    );
    assert!(dump.success);
    assert_eq!(dump.dump_rows.len(), 4);
    assert_eq!(dump.dump_rows[1].address, "0x807F0010");

    let sniff = parse_diagnostic_output(
        "SNIFF",
        "SNIFF 250",
        &simulate_mock_diagnostic("SNIFF 250"),
        45,
    );
    assert!(sniff.success);
    assert_eq!(sniff.transition_rows.len(), 4);
    assert_eq!(sniff.transition_rows[2].transitions, 68);
}

#[test]
fn pincheck_vdd_rejects_floating_pulldown_even_when_hiz_is_high() {
    let floating = "\
=== P7501 6-PIN ELECTRICAL MATRIX (PINCHECK) ===
GP26(Pin1 SWUART) | 3.21V|   0.00V   |  3.28V  | P7501 SW_UART / Target Sense
PINCHECK: OK";
    let res = parse_diagnostic_output("PINCHECK", "PINCHECK", floating, 40);
    assert_eq!(res.vdd_volts, Some(0.0));

    assert!(compose_diagnostic_command("WDT", None).is_ok());
    let wdt_res = parse_diagnostic_output(
        "WDT",
        "WDT",
        "[WDT] wdt=ACTIVE(4000ms,NON_OVERRIDABLE) seq=2 wdt_recoveries=1 reboot_src=WDT_RECOVERY fault_slot=5 fault_stage=PROBE_SWEEP last_cmd=IDENTIFY\nWDT: OK",
        20,
    );
    assert!(wdt_res.success);
    let wdt_finding = wdt_res
        .findings
        .iter()
        .find(|f| f.code == "WDT_ACTIVE")
        .expect("WDT_ACTIVE finding missing");
    assert!(wdt_finding.detail.contains("fault_stage=PROBE_SWEEP"));

    let funnel_log =
        "... SD init end\nSD diag load failed[0].\nBGO driver init failed[1].\nDIAGWATCH: OK";
    let funnel_res =
        parse_diagnostic_output("DIAGWATCH", "DIAGWATCH 6000 JIG NONE", funnel_log, 6000);
    assert!(funnel_res.success);
    assert!(funnel_res.findings.iter().any(|f| f.code == "SD_HW_OK"));
    assert!(
        funnel_res
            .findings
            .iter()
            .any(|f| f.code == "SD_DIAG_LOAD_FAIL")
    );
    assert!(
        funnel_res
            .findings
            .iter()
            .any(|f| f.code == "BGO_INIT_HALT")
    );
}

#[test]
fn parses_wire_protocol_ident_prog_evt_and_res_authoritatively() {
    let ident_raw = "@P7501_IDENT vdd_mv=3290 nsw5v=1 elec=AWAKE_READY loader=\"Loader ver0.61\" jedec=0x0001227E flash=S29JL064H ipl_crc32=0xA5B6C7D8 hw=\"VEP79138\" tv=PAL/NTSC state=READY model=DMR-EH55 tier=VERIFIED_BANNER caps=0x07 ssot=1234ABCD\nIDENTIFY: OK (model=DMR-EH55 flash=S29JL064H)\n@P7501_RES cmd=IDENTIFY status=OK code=0 elapsed_us=1840";
    let ident_res = parse_diagnostic_output("IDENTIFY", "IDENTIFY", ident_raw, 50);
    assert!(ident_res.success);
    assert_eq!(ident_res.protocol_code.as_deref(), Some("0"));
    assert_eq!(ident_res.protocol_elapsed_us, Some(1840));
    assert_eq!(ident_res.ident_model.as_deref(), Some("DMR-EH55"));
    assert_eq!(ident_res.ident_flash.as_deref(), Some("S29JL064H"));
    assert_eq!(ident_res.ident_loader.as_deref(), Some("Loader ver0.61"));
    assert_eq!(ident_res.detected_flash_mb, Some(8));
    assert!((ident_res.vdd_volts.unwrap() - 3.29).abs() < 0.01);
    assert!(ident_res.findings.iter().any(|f| f.code == "P7501_IDENT"));

    let job_err_raw = "@PROG step=1/3 msg=READ_TARGET_NVRAM\n@EVT kind=SDS_TRIG trig=91006 clk=12840\n@P7501_RES cmd=JOB status=ERR code=E05 elapsed_us=412";
    let job_err = parse_diagnostic_output("JOB", "JOB NVRAM_CLEAN CONFIRM", job_err_raw, 80);
    assert!(!job_err.success);
    assert_eq!(job_err.protocol_code.as_deref(), Some("E05"));
    assert_eq!(job_err.progress_steps, vec!["1/3 READ_TARGET_NVRAM"]);
    assert_eq!(job_err.wire_events, vec!["SDS_TRIG (clk=12840 trig=91006)"]);
    assert!(job_err.findings.iter().any(|f| f.code == "EVT_SDS_TRIG"));
    assert!(job_err.findings.iter().any(|f| f.code == "P7501_RES_E05"));

    let mmio_ok = parse_diagnostic_output(
        "MMIO_SNAP",
        "MMIO_SNAP",
        "[MMIO_SNAP] count=8 diff_mask=0x00 (0xC000100C excluded)\nMMIO_SNAP: OK\n@P7501_RES cmd=MMIO_SNAP status=OK code=0 elapsed_us=920",
        25,
    );
    assert!(mmio_ok.success);

    let i2c_ok = parse_diagnostic_output(
        "I2C_SCAN",
        "I2C_SCAN",
        "[I2C_SCAN] Scanning...\nI2C_SCAN: OK (devices=1)\n@P7501_RES cmd=I2C_SCAN status=OK code=0 elapsed_us=1100",
        30,
    );
    assert!(i2c_ok.success);
}
