//! Silicon SoC core and chip-level bus/MMIO definitions (strictly decoupled from firmware symbols).

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::evidence::ClaimEvidence;
use crate::hex_serde::{hex_u32, hex_u32_map, hex_u32_vec, opt_hex_u32};

/// Physical bus aperture window exposed by an SoC silicon block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BusWindow {
    pub name: String,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub base: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub size: u32,
    #[serde(default, with = "opt_hex_u32")]
    #[schemars(with = "Option<String>")]
    pub uncached_alias_base: Option<u32>,
}

/// CPU core ISA and baseline electrical logic family (`MN10300/AM34`, `MN10300/AM33-2`, `SPARC V8`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SocCoreProfile {
    pub id: String,
    pub isa: String,
    pub endianness: String,
    pub logic_voltage_mv: u32,
    pub evidence: ClaimEvidence,
}

/// Concrete SoC silicon part number (`MN2DS0009`, `MN2DS0012-H`, `MN2DS00151DB`, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SocProfile {
    pub id: String,
    pub part_number: String,
    pub core_ref: String,
    pub bus_windows: Vec<BusWindow>,
    #[serde(default, with = "hex_u32_map")]
    #[schemars(with = "BTreeMap<String, String>")]
    pub mmio: BTreeMap<String, u32>,
    #[serde(default, with = "hex_u32_vec")]
    #[schemars(with = "Vec<String>")]
    pub lethal_mmio: Vec<u32>,
    pub p7501_uart_baud: u32,
    pub p7501_nsw5v_hiz_only: bool,
    pub evidence: ClaimEvidence,
}
