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

/// Selectable P7501 hardware boot trigger profile executed from Standby or live target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootWakeProfile {
    /// Hi-Z `SW_UART` & `XRESET`, waiting cleanly for Standby wake (`NSW5V=1` or `VDD>=2.0V`).
    PassiveWake,
    /// Pre-straps `SW_UART = LOW` in Standby WITHOUT pulsing `XRESET`, releasing after `serial boot mode`.
    PreJigWake,
    /// Autonomous `DIAGWATCH` + `PREJIG` wake intercept for `SDDIAG` / `BGO` / `PANAMON1>` telemetry.
    DiagWatchPreJigWake,
    /// Active `SW_UART = LOW` + Open-Drain `XRESET` pulse (`JIG` forced serial boot).
    ActiveJigXreset,
}

impl BootWakeProfile {
    /// Builds the exact Pico CDC command (`BOOTTRACE` or `DIAGWATCH`) for this profile.
    #[must_use]
    pub fn build_pico_command(
        self,
        window_ms: u32,
        jig_hold_ms: u32,
        wake_timeout_ms: u32,
    ) -> String {
        let wake_suffix = if wake_timeout_ms > 0 {
            format!(" WAKE:{wake_timeout_ms}")
        } else {
            String::new()
        };
        match self {
            Self::PassiveWake => format!("BOOTTRACE {window_ms} PASSIVE{wake_suffix}"),
            Self::PreJigWake => format!("BOOTTRACE {window_ms} PREJIG:{jig_hold_ms}{wake_suffix}"),
            Self::DiagWatchPreJigWake => {
                format!("DIAGWATCH {window_ms} PREJIG:{jig_hold_ms} NONE RELEASE{wake_suffix}")
            }
            Self::ActiveJigXreset => format!("BOOTTRACE {window_ms} JIG:{jig_hold_ms}"),
        }
    }

    /// Stable identifier token for report sections.
    #[must_use]
    pub fn step_id(self) -> &'static str {
        match self {
            Self::PassiveWake => "STEP_WAKE_PASSIVE",
            Self::PreJigWake => "STEP_WAKE_PREJIG",
            Self::DiagWatchPreJigWake => "STEP_WAKE_DIAGWATCH",
            Self::ActiveJigXreset => "STEP_BOOT_ACTIVE_JIG",
        }
    }

    /// Human-readable title describing the electrical trigger used.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::PassiveWake => "Passive Standby Wakeup Capture (Hi-Z SW_UART / XRESET)",
            Self::PreJigWake => "Standby PREJIG Serial Boot (SW_UART LOW, No XRESET Pulse)",
            Self::DiagWatchPreJigWake => "Autonomous DIAGWATCH PREJIG Wake Interceptor",
            Self::ActiveJigXreset => "Active SW_UART LOW + Open-Drain XRESET Probe",
        }
    }
}

/// Visual cue observed by the operator on the DMR-EH55 front-panel VFD display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VfdObservation {
    StaysStandbyClock,
    PleaseWaitThenU99Under2s,
    HelloThenU99Under2s,
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
            Self::PleaseWaitThenU99Under2s => "PLEASE WAIT (~2s) -> U99",
            Self::HelloThenU99Under2s => "HELLO (~2s) -> U99",
            Self::ImmediateU99 => "DIRECT U99",
            Self::U99AfterLongWait => "PLEASE WAIT (>10s) -> U99",
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
