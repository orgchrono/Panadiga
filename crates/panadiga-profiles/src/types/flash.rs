#![deny(unsafe_code)]

//! Datasheet-verified NOR Flash and OTP PROM geometries (no manual SA label strings).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::evidence::ClaimEvidence;
use crate::hex_serde::{hex_u32, hex_u32_vec};

/// JEDEC manufacturer + device ID sequence read via Autoselect command (`0x90`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FlashJedecId {
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub mfg: u32,
    #[serde(with = "hex_u32_vec")]
    #[schemars(with = "Vec<String>")]
    pub device_cycles: Vec<u32>,
}

/// Contiguous bank of uniform sectors ordered from lowest flash offset (`0x000000`) upwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FlashSectorBank {
    pub name: String,
    pub count: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub sector_size: u32,
}

/// Simultaneous read-while-write (RWW) bank span in physical `SA` indices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FlashRwBank {
    pub start_sa: u32,
    pub end_sa: u32,
}

/// Hardware write-protect pin constraint (e.g. `WP#=VIL` protecting `SA127` on `S29GL128N`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FlashHwProtect {
    pub pin: String,
    pub protects_region: String,
    pub affects_top_sector: bool,
}

/// Physical NOR Flash or OTP PROM part specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FlashChipProfile {
    pub id: String,
    pub vendor: String,
    pub part_number: String,
    pub package: String,
    pub voltage_min_mv: u32,
    pub voltage_max_mv: u32,
    pub nominal_voltage_mv: u32,
    pub writable: bool,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub total_bytes: u32,
    #[serde(default)]
    pub jedec: Option<FlashJedecId>,
    #[serde(default)]
    pub sector_banks: Vec<FlashSectorBank>,
    #[serde(default)]
    pub rw_banks: Vec<FlashRwBank>,
    #[serde(default)]
    pub hw_protect: Option<FlashHwProtect>,
    pub evidence: ClaimEvidence,
}
