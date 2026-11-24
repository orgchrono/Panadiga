//! Pure Pico P7501 bench diagnostic decoders, command allowlist guard, and A/B harness analytics.

mod ab_harness;
mod telemetry;
mod types;

pub use ab_harness::{
    build_ab_boot_command, classify_ab_boot_log, format_ab_csv, summarize_ab_runs,
};
pub use telemetry::{
    parse_busdiag_report, parse_panarsp_report, parse_pincheck_report, parse_piologic_report,
    parse_status_output_report, validate_interactive_pico_command, validate_safe_bench_command,
};
pub use types::{
    AbBootRun, AbHarnessSummary, AbModeStats, BenchStatusReport, BusDiagBenchReport,
    PanarspBenchReport, PinCheckBenchRow, PiologicBenchReport, SAFE_BENCH_COMMANDS,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_safe_bench_commands_and_parses_telemetry() {
        assert!(validate_safe_bench_command("PIOLOGIC 4000 JIG:350").is_ok());
        assert!(validate_safe_bench_command("FLASH_ERASE 0x807F0000").is_err());
        assert!(validate_safe_bench_command("STATUS\nERASE").is_err());

        let st = parse_status_output_report("VDD: 2.70V (OK)\r\n");
        assert_eq!(st.vdd_volts, Some(2.70));
        assert!(!st.gp26_sticky_low_suspected);

        let bus = parse_busdiag_report(
            "GP2 (Pin6): 872 trans\nGP3: 0 trans\nXRESET Release Rise Time (Hi-Z) : 20 us",
        );
        assert_eq!(bus.gp2_transitions, Some(872));
        assert!(bus.gp2_sole_active_tx);
        assert_eq!(bus.xreset_rise_us, Some(20));

        let pio = parse_piologic_report(
            "fifo_stall=NO | edges=28 | span=512000us\nNSW5V (GP3) : Transitions=0\nXRESET_rise -> first_RXD_fall: 482 us\nTXD_fall -> RXD_fall turnaround: min=120 us max=120 us",
        );
        assert_eq!(pio.edges_count, 28);
        assert_eq!(pio.reset_to_first_rxd_us, Some(482));

        let rsp = parse_panarsp_report(
            "[PANARSP] Frame: ack=+ nak=no nak0x15=yes | payload=\"DEADBEEF\" | csum=0xAA/0xAA (VALID)\n[PANARSP] PayloadHex: DEADBEEF\nPANARSP: OK (rx=14 bytes, rsp=VALID, fifo_ovf=NO)",
        );
        assert_eq!(rsp.rx_bytes, Some(14));
        assert_eq!(rsp.payload_hex.as_deref(), Some("DEADBEEF"));
        assert_eq!(rsp.checksum_ok, Some(true));
    }

    #[test]
    fn validate_interactive_pico_command_enforces_catalog_and_safety_barriers() {
        assert_eq!(
            validate_interactive_pico_command("  status ").unwrap(),
            "STATUS"
        );
        assert_eq!(
            validate_interactive_pico_command("identify").unwrap(),
            "IDENTIFY"
        );
        assert!(validate_interactive_pico_command("isolate").is_ok());
        assert!(validate_interactive_pico_command("hiz").is_ok());
        assert!(validate_interactive_pico_command("help").is_ok());
        assert!(validate_interactive_pico_command("FAST_READ 84000000 1000").is_ok());
        assert!(
            validate_interactive_pico_command("FAST_WRITE 843F0100 0000 CONFIRM_UPPER_NOR").is_ok()
        );
        assert!(validate_interactive_pico_command("FAST_ERASE 843F0000 CONFIRM_UPPER_NOR").is_ok());

        assert!(validate_interactive_pico_command("").is_err());
        assert!(validate_interactive_pico_command("STATUS\r\nPING").is_err());
        assert!(validate_interactive_pico_command("STATUS\0PING").is_err());
        assert!(validate_interactive_pico_command("RM_RF_SLASH").is_err());
        assert!(validate_interactive_pico_command("PANARSP MEM:C000100C,4").is_err());
        assert!(validate_interactive_pico_command("FAST_READ 0xC000_100C 10").is_err());
        assert!(validate_interactive_pico_command("FAST_WRITE 843F0100 0000").is_err());
        assert!(validate_interactive_pico_command("FAST_ERASE 843F0000").is_err());
        assert!(
            validate_interactive_pico_command("FAST_ERASE 84000000 CONFIRM_UPPER_NOR").is_err()
        );
        assert!(
            validate_interactive_pico_command("FAST_WRITE 84000000 0000 CONFIRM_UPPER_NOR")
                .is_err()
        );
        assert!(validate_interactive_pico_command("ERASE 80001000").is_err());
    }
}
