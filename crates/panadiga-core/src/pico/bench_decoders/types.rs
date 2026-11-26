//! Domain models and command allowlist constants for P7501 bench decoders & A/B harness.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Read-only / non-destructive bench command allowlist for P7501 UART diagnostics.
pub const SAFE_BENCH_COMMANDS: &[&str] = &[
    "PING",
    "HELP",
    "BANNER",
    "STATUS",
    "VDD",
    "LED",
    "PINCHECK",
    "BUSDIAG",
    "SNIFF",
    "ISOLATE",
    "FAULTACK",
    "UARTMON",
    "BOOTCAP",
    "JIGBOOT",
    "UARTPROBE",
    "UARTSEND",
    "BAUD",
    "FRAMING",
    "BAUDSCAN",
    "UARTSTREAM",
    "HEXCAP",
    "UARTTXHEX",
    "WAITTX",
    "PWRTRACE",
    "BOOTTRACE",
    "BOOTFOLLOW",
    "PIOLOGIC",
    "SDSNIFF",
    "JIGSWEEP",
    "DIAGWATCH",
    "PANARSP",
    "DRVPANA",
    "IRTX",
    "GPS",
    "ROUTE",
    "XRAY",
];

pub const FORBIDDEN_BENCH_TOKENS: &[&str] = &[
    "ERASE", "WRITE", "FLASH", "PROGRAM", "WRITEMEM", "PATCH", "UNLOCK", "JEDEC", "POKE", "WMEM",
    "WRITE32", "RAW:M", "RAW:X", "VFLASH", "C000100C", "DC0F0000",
];
pub(super) const ALLOWED_AB_MODES: [&str; 2] = ["NORMAL", "JIG"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchStatusReport {
    pub vdd_volts: Option<f32>,
    pub vdd_status: String,
    pub gp26_sticky_low_suspected: bool,
    pub nsw5v_high: Option<bool>,
    pub fault_latch: Option<bool>,
    pub dropouts: Option<u32>,
    pub raw_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinCheckBenchRow {
    pub pin_label: String,
    pub hiz_volts: Option<f32>,
    pub pull_down_volts: Option<f32>,
    pub pull_up_volts: Option<f32>,
    pub electrical_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusDiagBenchReport {
    pub gp2_transitions: Option<u32>,
    pub gp3_transitions: Option<u32>,
    pub gp4_transitions: Option<u32>,
    pub xreset_rise_us: Option<u32>,
    pub gp2_sole_active_tx: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiologicBenchReport {
    pub fifo_stall: bool,
    pub timing_skew: bool,
    pub edges_count: u32,
    pub edges_truncated: bool,
    pub span_us: Option<u32>,
    pub nsw5v_transitions: Option<u32>,
    pub reset_to_first_rxd_us: Option<u32>,
    pub min_tx_to_rx_us: Option<u32>,
    pub max_tx_to_rx_us: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanarspBenchReport {
    pub rx_bytes: Option<u32>,
    pub ack_plus: bool,
    pub nak_minus: bool,
    pub nak_0x15: bool,
    pub payload: Option<String>,
    pub payload_hex: Option<String>,
    pub checksum_ok: Option<bool>,
    pub rsp_status: String,
    pub fifo_overflow: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbBootRun {
    pub run_index: usize,
    pub mode: String,
    pub command: String,
    pub ddr_check_ok: bool,
    pub boot_mode: Option<String>,
    pub bgo_failed_code: Option<i32>,
    pub sd_91006: bool,
    pub sd_90002: bool,
    pub sd_90001: bool,
    pub odd_83a7: bool,
    pub duration_ms: f64,
    pub sd_hw_ok: bool,
    pub sd_diag_load_fail: bool,
    pub sd_uncompress_fail: bool,
    pub odd_tray_fail: bool,
    pub diagnosis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbModeStats {
    pub mode: String,
    pub runs: usize,
    pub ddr_ok_count: usize,
    pub sd_91006_count: usize,
    pub sd_90002_count: usize,
    pub sd_90001_count: usize,
    pub odd_83a7_count: usize,
    pub bgo_fail_count: usize,
    pub sd_hw_ok_count: usize,
    pub sd_diag_load_fail_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbHarnessSummary {
    pub runs: Vec<AbBootRun>,
    pub by_mode: BTreeMap<String, AbModeStats>,
}
