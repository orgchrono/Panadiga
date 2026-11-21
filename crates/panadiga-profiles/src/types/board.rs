#![deny(unsafe_code)]

//! Physical Digital / Main PCB board assembly declarations (`VEPxxxxx` / `RFKxxxxx`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::evidence::ClaimEvidence;

/// Hardware PCB assembly uniting SoC silicon, Flash chip, optional EEPROMs, and P7501 debug availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoardProfile {
    pub id: String,
    pub pcb_code: String,
    #[serde(default)]
    pub service_module_code: Option<String>,
    #[serde(default)]
    pub soc_ref: Option<String>,
    #[serde(default)]
    pub flash_ref: Option<String>,
    #[serde(default)]
    pub eeprom_refs: Vec<String>,
    pub p7501_header_present: bool,
    pub has_hdd_interface: bool,
    pub has_sd_slot: bool,
    pub evidence: ClaimEvidence,
}
