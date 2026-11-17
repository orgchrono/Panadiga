use super::*;

#[test]
fn parses_upgraded_and_legacy_baudscan_telemetry() {
    let raw_legacy = "[BAUDSCAN] edges=1252 min_low_us=8 max_low_us=53 raw_bps=125000 snapped_baud=115200\nBAUDSCAN: OK";
    let b_leg = parse_baudscan_output(raw_legacy);
    let rep = evaluate_guided_bench(
        "COM10",
        Some(3.25),
        None,
        Some(b_leg),
        VfdObservation::ReadyNormal,
        vec![],
    );
    assert!(
        rep.baud_verdict
            .contains("Locked cleanly on GP2: 115200 bps 8N1")
    );
}

#[test]
fn parses_upgraded_baudscan_and_wake_trace_telemetry() {
    use super::super::cdc::reply_has_completed;
    // Opening banner must not prematurely complete before footer `BOOTTRACE: OK`
    assert!(!reply_has_completed(
        "=== BOOTTRACE: window=1500ms boot=0 hold=0ms sticky=NO (8N1 @ 115200) ===\n",
        &["BOOTTRACE: OK", "BOOTTRACE: ERR", "PANADIGA>", "P7501>"]
    ));
    assert!(reply_has_completed(
        "=== BOOTTRACE: window=1500ms ===\nBOOTTRACE: OK\n",
        &["BOOTTRACE: OK", "BOOTTRACE: ERR", "PANADIGA>", "P7501>"]
    ));

    let raw_baud = "[BAUDSCAN] edges=312 min_low_us=9 min_high_us=9 raw_bps=111111 snapped_baud=115200 cluster_hits=84 confidence=72% glitches=1 rx_bytes=42 ascii=40 framing_errs=0\nBAUDSCAN: OK\n";
    let b = parse_baudscan_output(raw_baud);
    assert_eq!(b.edges, 312);
    assert_eq!(b.snapped_baud, 115200);
    assert_eq!(b.cluster_hits, 84);
    assert_eq!(b.confidence_pct, 72);
    assert_eq!(b.glitches, 1);
    assert_eq!(b.rx_bytes, 42);
    assert_eq!(b.framing_errs, 0);
    assert!(!b.no_signal);

    let raw_wake = "[BOOTTRACE] nsw5v_edges=2 rxd_edges=0 txd_edges=0 adc0_min=0.05 adc0_max=3.27\nBOOTTRACE: OK\n";
    let w = parse_wake_trace_output(raw_wake);
    assert_eq!(w.nsw5v_edges, 2);
    assert_eq!(w.rxd_edges, 0);
    assert!(w.vdd_max_volts.unwrap() > 3.2);

    let real_boottrace = "[BOOTTRACE +1820ms] WAKE edge detected (NSW5V=1, VDD=3.25V) -> starting capture\n\
        [BOOTTRACE] UART: bytes=348 first_byte=18400us ferr=0 perr=0 ovf=NO | SW_rel=YES@2500000us(TIMER)\n\
        [BOOTTRACE] ADC0_SWUART: min=0.00V max=3.26V (low=180 hiz=3.25..3.26V n=22) | CtrlEdges: NSW5V=1 TXD=1 XRESET=0 (logged=2)\n\
        BOOTTRACE: OK\n";
    let w_real = parse_wake_trace_output(real_boottrace);
    assert_eq!(w_real.nsw5v_edges, 1);
    assert_eq!(w_real.rxd_edges, 348);
    assert_eq!(w_real.txd_edges, 1);
    assert!(w_real.sw_uart_released);
    assert!(w_real.vdd_max_volts.unwrap() > 3.2);

    assert_eq!(
        BootWakeProfile::PreJigWake.build_pico_command(12000, 3000, 45000),
        "BOOTTRACE 12000 PREJIG:3000 WAKE:45000"
    );
    assert_eq!(
        BootWakeProfile::DiagWatchPreJigWake.build_pico_command(15000, 3000, 45000),
        "DIAGWATCH 15000 PREJIG:3000 NONE RELEASE WAKE:45000"
    );
}

#[test]
fn evaluates_u99_2s_standby_drop_and_panamon1_ghidra_readiness() {
    let wake = WakeTraceSummary {
        nsw5v_edges: 2,
        rxd_edges: 0,
        txd_edges: 0,
        vdd_min_volts: Some(0.05),
        vdd_max_volts: Some(3.26),
        sw_uart_released: false,
        raw_output: String::new(),
    };
    let baud = BaudScanSummary {
        edges: 0,
        no_signal: true,
        ..Default::default()
    };
    let rep = evaluate_guided_bench(
        "COM10",
        Some(0.05),
        Some(wake.clone()),
        Some(baud.clone()),
        VfdObservation::HelloThenU99Under2s,
        vec![],
    );
    assert!(!rep.can_attach_ghidra_now);
    assert_eq!(rep.recommended_gdb_mode, "none_pre_uart_or_u99");
    assert!(rep.baud_verdict.contains("SILENT GP2"));
    assert!(
        rep.firmware_correlation
            .matches
            .iter()
            .any(|m| m.code == "HELLO_U99_STANDBY_WAKE")
    );

    let rep_pw = evaluate_guided_bench(
        "COM10",
        Some(0.05),
        Some(wake),
        Some(baud),
        VfdObservation::PleaseWaitThenU99Under2s,
        vec![],
    );
    assert!(
        rep_pw
            .firmware_correlation
            .matches
            .iter()
            .any(|m| m.code == "PLEASE_WAIT_U99_2S")
    );

    let panamon_step = GuidedStepEvidence {
        step_id: "STEP_4_JIG_BOOT".to_string(),
        title: "JIG".to_string(),
        command_executed: "BOOTTRACE 8000 JIG".to_string(),
        operator_note: "SD card inserted".to_string(),
        status: "PASS".to_string(),
        summary: "PANAMON1 online".to_string(),
        raw_output: "***** serial boot mode *****\nPANAMON1>".to_string(),
    };
    let rep_ready = evaluate_guided_bench(
        "COM10",
        Some(3.25),
        None,
        None,
        VfdObservation::ReadyNormal,
        vec![panamon_step],
    );
    assert!(rep_ready.can_attach_ghidra_now);
    assert_eq!(rep_ready.recommended_gdb_mode, "panamon1");
    assert!(
        rep_ready
            .ghidra_gdb_verdict
            .contains("--target-mode panamon1")
    );
    assert!(format_guided_bench_markdown(&rep_ready).contains("Async GDB / Ghidra Verdict"));
}
