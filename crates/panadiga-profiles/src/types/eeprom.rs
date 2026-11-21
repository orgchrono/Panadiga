//! Serial EEPROM specifications explicitly classifying device role so optical calibration chips
//! or timer MCU EEPROMs are never confused with NOR EEMUL NVRAM.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::evidence::ClaimEvidence;
use crate::hex_serde::hex_u32;

/// Concrete serial EEPROM IC profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EepromChipProfile {
    pub id: String,
    pub vendor: String,
    pub part_number: String,
    #[serde(default)]
    pub service_replacement_part: Option<String>,
    pub bus: String,
    pub function: EepromFunction,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub i2c_addr_7bit: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub total_bytes: u32,
    pub page_size: u32,
    pub voltage_mv: u32,
    pub evidence: ClaimEvidence,
}

/// Architectural purpose of an external serial EEPROM on a Panasonic board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EepromFunction {
    /// Optical pickup laser power / strategy calibration (`@RITEK`, `@MBIPG101R05`).
    OpticalCalibration,
    /// Timer MCU settings/clock/model EEPROM on Main PCB (e.g., `IC7404` `C3EBFC000053`/`RFKFM6016KT`).
    TimerMainConfig,
    /// Non-U99 system settings store.
    SystemSettings,
    /// Unconfirmed function in schematics/dumps.
    Unknown,
}
