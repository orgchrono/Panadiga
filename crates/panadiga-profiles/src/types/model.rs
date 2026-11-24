#![deny(unsafe_code)]

//! Model variant manifest, claim-level provenance matrix, resolved profile DTO, and full bundle table.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::board::BoardProfile;
use super::eeprom::EepromChipProfile;
use super::evidence::ClaimEvidence;
use super::flash::FlashChipProfile;
use super::layout::NorLayoutProfile;
use super::media::MediaFormatProfile;
use super::soc::{SocCoreProfile, SocProfile};
use super::train::FirmwareTrainProfile;
use crate::region::InterpolatedRegionVariant;

/// Per-axis provenance matrix attached to each DIGA model variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelClaims {
    pub board: ClaimEvidence,
    pub flash_chip: ClaimEvidence,
    pub nor_layout: ClaimEvidence,
    pub train: ClaimEvidence,
    pub eeprom: ClaimEvidence,
}

/// Supported claim axes targeted by `panadiga profiles promote`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimAxis {
    Board,
    FlashChip,
    NorLayout,
    Train,
    Eeprom,
}

/// Top-level model manifest pointing strictly to normalized hardware/train/layout references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelProfileManifest {
    pub model_id: String,
    pub family_label: String,
    pub series: String,
    #[serde(default)]
    pub region_suffix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tv_standard: Option<String>,
    #[serde(default)]
    pub hw_code: Option<String>,
    #[serde(default)]
    pub board_ref: Option<String>,
    #[serde(default)]
    pub train_ref: Option<String>,
    #[serde(default)]
    pub nor_layout_ref: Option<String>,
    #[serde(default)]
    pub exact_aliases: Vec<String>,
    #[serde(default)]
    pub fixture_ref: Option<String>,
    pub claims: ModelClaims,
}

/// Policy authorization summary computed statically from claims + geometry for UI/CLI display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySummary {
    pub inspect_read_only: bool,
    pub nvram_sanitize_offline_cow: bool,
    pub nvram_write_live: bool,
    pub upper_nor_patch_offline_cow: bool,
    pub upper_nor_write_live: bool,
    pub official_frm_pass_through: bool,
    pub custom_recovery_media: bool,
    pub lab_override_offline_cow_eligible: bool,
}

/// Fully dereferenced hardware/firmware/layout specification returned by `resolve_model`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedModelProfile {
    pub manifest: ModelProfileManifest,
    pub board: Option<BoardProfile>,
    pub soc_core: Option<SocCoreProfile>,
    pub soc: Option<SocProfile>,
    pub flash_chip: Option<FlashChipProfile>,
    pub eeprom_chips: Vec<EepromChipProfile>,
    pub firmware_train: Option<FirmwareTrainProfile>,
    pub nor_layout: Option<NorLayoutProfile>,
    pub media_formats: Vec<MediaFormatProfile>,
    pub nvram_cached_va_hex: Option<String>,
    pub nvram_uncached_va_hex: Option<String>,
    pub ipl_protected_range_hex: Option<String>,
    pub bgo_sector_range_derived: Option<String>,
    pub nvram_sector_range_derived: Option<String>,
    pub derived_sector_geometry: Option<String>,
    #[serde(default)]
    pub regional_variants: Vec<InterpolatedRegionVariant>,
    #[serde(default)]
    pub active_region: Option<InterpolatedRegionVariant>,
    pub effective_capabilities: CapabilitySummary,
}

/// Audit report emitted by `validate_bundle`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryValidationReport {
    pub valid: bool,
    pub soc_core_count: usize,
    pub soc_count: usize,
    pub flash_chip_count: usize,
    pub eeprom_chip_count: usize,
    pub firmware_train_count: usize,
    pub nor_layout_count: usize,
    pub media_format_count: usize,
    pub board_count: usize,
    pub model_count: usize,
    pub checks_run: usize,
    pub checks_failed: usize,
    pub safety_checks_passed: usize,
    pub errors: Vec<String>,
}

/// Canonical normalized bundle across all 9 relational profile tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfilesBundle {
    pub schema_version: u32,
    pub soc_cores: BTreeMap<String, SocCoreProfile>,
    pub socs: BTreeMap<String, SocProfile>,
    pub flash_chips: BTreeMap<String, FlashChipProfile>,
    pub eeprom_chips: BTreeMap<String, EepromChipProfile>,
    pub firmware_trains: BTreeMap<String, FirmwareTrainProfile>,
    pub nor_layouts: BTreeMap<String, NorLayoutProfile>,
    pub media_formats: BTreeMap<String, MediaFormatProfile>,
    pub boards: BTreeMap<String, BoardProfile>,
    pub models: BTreeMap<String, ModelProfileManifest>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub blobs: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traps: Option<crate::i18n::TrapCatalog>,
}
