//! Pure data models and deterministic state records for the Matshita `R6-MV4` ATAPI/Serial ODD Emulator.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatshitaBootScenario {
    /// Virtual blank/healthy tray with valid DVD-RAM/FRM disc mounted -> `startup_drive` succeeds.
    HealthyWithDisc,
    /// Virtual optical unlock disc (`PANA_DVD.FRM` in ISO9660 LBA 16+) -> triggers `JMP 0x50400000`.
    UnlockDiscFrm,
    /// Reproduces the physical bench tray-open / 4-pin cable unplugged error (`80045300`).
    TrayCannotClose80045300,
    /// Reproduces laser diode APC `< 0.5mW` failure (`80040904` + vendor `0x83A7`).
    LaserApcFailure80040904,
}

impl FromStr for MatshitaBootScenario {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "healthy" | "healthy_with_disc" => Ok(Self::HealthyWithDisc),
            "unlock" | "unlock_disc" | "unlock_disc_frm" => Ok(Self::UnlockDiscFrm),
            "tray" | "tray_open" | "tray_cannot_close_80045300" | "80045300" => {
                Ok(Self::TrayCannotClose80045300)
            }
            "laser" | "apc" | "laser_apc_failure_80040904" | "80040904" => {
                Ok(Self::LaserApcFailure80040904)
            }
            other => anyhow::bail!(
                "Unsupported Matshita boot scenario '{other}' (expected healthy_with_disc, unlock_disc_frm, tray_cannot_close_80045300, or laser_apc_failure_80040904)"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtapiTransferDirection {
    None,
    DataIn,
    DataOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtapiCommandRecord {
    pub seq: usize,
    pub cdb_hex: String,
    pub opcode: u8,
    pub opcode_name: String,
    pub drv_handler_va: String,
    pub direction: AtapiTransferDirection,
    pub payload_len: usize,
    pub sense_key: u8,
    pub asc: u8,
    pub ascq: u8,
    pub pana_error_hex: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteInjectionRecord {
    pub seq: usize,
    pub source: String,
    pub cdb0_or_cmd: String,
    pub mode: u8,
    pub target_va_or_lba: u32,
    pub len_bytes: usize,
    pub payload_sha256: String,
    pub status: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootSequenceStep {
    pub step_index: usize,
    pub boot_fn_va: String,
    pub boot_fn_name: String,
    pub cdb_hex: String,
    pub sense_hex: String,
    pub boot_log_line: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootSequenceReport {
    pub scenario: MatshitaBootScenario,
    pub inquiry_id: String,
    pub steps: Vec<BootSequenceStep>,
    pub uart_boot_transcript: Vec<String>,
    pub startup_succeeded: bool,
    pub unlock_media_verified: bool,
    pub jump_target_va_hex: Option<String>,
    pub final_boot_code_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrmUnlockVerificationSummary {
    pub container_kind: String,
    pub container_bytes: usize,
    pub sha256: String,
    pub entry_count: usize,
    pub bootloader_jump_va_hex: String,
    pub extrhead_valid: Option<bool>,
    pub verified: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialCliResponse {
    pub command: String,
    pub authorized_flag_cd8: u8,
    pub s211_flag_d78: u8,
    pub slog_enabled: bool,
    pub output_lines: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatshitaEmulatorSnapshot {
    pub inquiry_vendor: String,
    pub inquiry_product: String,
    pub inquiry_revision: String,
    pub tray_closed: bool,
    pub media_present: bool,
    pub sram_0cd8_auth: u8,
    pub sram_0d78_s211: u8,
    pub sram_0242_ed_mode: u8,
    pub mmode: u8,
    pub slog_enabled: bool,
    pub slog_entries: Vec<String>,
    pub buffer_len: usize,
    pub buffer_sha256: String,
    pub disc_lba_sectors: usize,
    pub history_count: usize,
    pub injections: Vec<WriteInjectionRecord>,
}
