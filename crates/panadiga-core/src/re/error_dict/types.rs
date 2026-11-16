//! Strongly typed firmware error dictionary models across DRV (`0x0000..0x83FF`),
//! `/BOOT.BIN` FAT16/SDCD/BGO, front-panel VFD codes, ATAPI SENSE, and UART monitors.

use serde::{Deserialize, Serialize};

/// Subsystem domain inside Panasonic DMR-EH55 / Matshita R6-MV4 firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareErrorDomain {
    FrontPanel,
    BootRom,
    BootFat,
    SdBoot,
    AtapiSense,
    UartMonitor,
    DrvMecha,
}

/// Actionable severity level for an identified firmware error occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareErrorSeverity {
    Ok,
    Warn,
    Critical,
}

/// Declarative entry loaded from `firmware_error_dict.json` (`431` DRV + Boot/Panel/Monitor SSOT).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareErrorEntry {
    pub code: String,
    pub aliases: Vec<String>,
    pub domain: FirmwareErrorDomain,
    pub address: String,
    pub severity: FirmwareErrorSeverity,
    pub title: String,
    pub native_text: String,
    pub translation: String,
    pub actions: Vec<String>,
}

/// Matched dictionary occurrence anchored to observed bench evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareCorrelationMatch {
    pub code: String,
    pub domain: FirmwareErrorDomain,
    pub severity: FirmwareErrorSeverity,
    pub title: String,
    pub address: String,
    pub native_text: String,
    pub translation: String,
    pub evidence_source: String,
    pub recommended_actions: Vec<String>,
}

/// Consolidated correlation summary linking live Pico UART + electrical telemetry + VFD cues.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirmwareCorrelationReport {
    pub dictionary_total_entries: usize,
    pub matches: Vec<FirmwareCorrelationMatch>,
    pub prioritized_actions: Vec<String>,
    pub summary: String,
}
