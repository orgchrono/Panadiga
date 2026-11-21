//! Physical NOR Flash partition layouts as functions of `(FirmwareTrain × FlashChip)`.

use super::evidence::ClaimEvidence;
use crate::hex_serde::hex_u32;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Semantic role of a physical byte span in NOR Flash (`IC5001`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NorRegionKind {
    Ipl,
    Erased,
    Extrhead,
    MainXip,
    Bgo,
    NorFs,
    Eemul,
    VendorNvram,
    Unknown,
}

/// Fail-closed write policy attached to a NOR region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NorRegionPolicy {
    ReadOnlyPermanent,
    GatedUpperNor,
    NvramSanitize,
    ReadOnlyReference,
}

/// Structural header marker expected at a region sector head (e.g., byte `[5] == 0xF1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegionSignature {
    pub marker_byte_offset: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub marker_value: u32,
}

/// Contiguous sector-aligned region inside a [`NorLayoutProfile`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NorRegion {
    pub kind: NorRegionKind,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub start_offset: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub end_offset: u32,
    pub policy: NorRegionPolicy,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub signature: Option<RegionSignature>,
}

/// Explicit Flash NOR partitioning tying a [`crate::FirmwareTrainProfile`] to a [`crate::FlashChipProfile`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NorLayoutProfile {
    pub id: String,
    pub flash_ref: String,
    pub train_ref: String,
    pub regions: Vec<NorRegion>,
    pub evidence: ClaimEvidence,
}

impl NorLayoutProfile {
    /// Returns the first region matching `kind`, if declared.
    #[must_use]
    pub fn find_region(&self, kind: NorRegionKind) -> Option<&NorRegion> {
        self.regions.iter().find(|r| r.kind == kind)
    }
}
