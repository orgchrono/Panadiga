//! Data models for Panadiga interactive guided bench triage & error correlation reports.

use serde::{Deserialize, Serialize};

use crate::re::error_dict::FirmwareCorrelationReport;

/// Parsed telemetry from `BAUDSCAN` (dual-polarity clusters + PIO1 framing verification).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaudScanSummary {
    pub edges: u32,
    pub min_low_us: u32,
    pub min_high_us: u32,
    pub raw_bps: u32,
    pub snapped_baud: u32,
    pub cluster_hits: u32,
    pub confidence_pct: u32,
    pub glitches: u32,
    pub rx_bytes: u32,
    pub ascii_bytes: u32,
    pub framing_errs: u32,
    pub no_signal: bool,
    pub raw_output: String,
}

/// Parsed electrical rail & GPIO transition metrics from `PWRTRACE` / `BOOTTRACE`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WakeTraceSummary {
    pub nsw5v_edges: u32,
    pub rxd_edges: u32,
    pub txd_edges: u32,
    pub vdd_min_volts: Option<f32>,
    pub vdd_max_volts: Option<f32>,
    pub sw_uart_released: bool,
    pub raw_output: String,
}

/// Operator visual cue on the DMR-EH55 front-panel VFD display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VfdObservation {
    StaysStandbyClock,
    HelloPleaseWaitU99Under2s,
    ImmediateU99,
    U99AfterLongWait,
    ReadyNormal,
    PowerOffBye,
    BlankDead,
    Custom(String),
}

impl VfdObservation {
    /// Returns a canonical text token string for firmware error correlation.
    #[must_use]
    pub fn as_cue_text(&self) -> &str {
        match self {
            Self::StaysStandbyClock => "0:00 STANDBY",
            Self::HelloPleaseWaitU99Under2s => "HELLO PLEASE WAIT (<=2s) U99",
            Self::ImmediateU99 => "U99 IMMEDIATE",
            Self::U99AfterLongWait => "PLEASE WAIT -> U99 (>10s)",
            Self::ReadyNormal => "NORMAL READY",
            Self::PowerOffBye => "BYE STANDBY",
            Self::BlankDead => "VFD BLANK",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// One captured stage inside the guided interactive bench test battery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuidedStepEvidence {
    pub step_id: String,
    pub title: String,
    pub command_executed: String,
    pub operator_note: String,
    pub status: String,
    pub summary: String,
    pub raw_output: String,
}

/// Consolidated guided bench report combining electrical rails, baud analysis,
/// firmware dictionary correlation (`431` DRV + Boot/VFD), and actionable next steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuidedBenchReport {
    pub port: String,
    pub initial_vdd_volts: Option<f32>,
    pub wake_trace: Option<WakeTraceSummary>,
    pub baud_scan: Option<BaudScanSummary>,
    pub vfd_observation: VfdObservation,
    pub root_cause_verdict: String,
    pub baud_verdict: String,
    pub ghidra_gdb_verdict: String,
    pub can_attach_ghidra_now: bool,
    pub recommended_gdb_mode: String,
    pub firmware_correlation: FirmwareCorrelationReport,
    pub steps: Vec<GuidedStepEvidence>,
    pub next_actions: Vec<String>,
}
