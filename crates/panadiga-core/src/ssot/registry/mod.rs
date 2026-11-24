#![deny(unsafe_code)]

//! Thin facade over `panadiga_profiles` embedding the build-time validated `profiles/` v2 bundle.
//!
//! All relational hardware definitions (`soc_cores`, `socs`, `flash_chips`, `eeprom_chips`,
//! `firmware_trains`, `nor_layouts`, `media_formats`, `boards`, `models`) are validated during
//! `panadiga-core` compilation (`build.rs`) and enforced by [`CapabilityGate`] at runtime.

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;

pub use panadiga_profiles::{
    AM34_LETHAL_MMIO_ADDR, AddressReloc, Authorization, AuthorizationMode, BinaryPatchSiteSpec,
    BoardProfile, BytePatchEntry, Capability, CapabilityGate, CapabilitySummary, ClaimAxis,
    ClaimEvidence, Denial, DerivedSector, DigadiagSpec, DigadiagTrustModel, EepromChipProfile,
    EepromFunction, EvidenceSource, EvidenceTier, FirmwareTrainProfile, FlashChipProfile,
    FlashHwProtect, FlashJedecId, FlashRwBank, FlashSectorBank, FrmContainerSpec,
    InterpolatedRegionVariant, LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride, LoaderFingerprint,
    MANUAL_PROPOSALS_REL_DIR, MIN_CRITICAL_IPL_GUARD_BYTES, MainBgoRetargetAnchors, MainBuildSpec,
    ManualClaimProposal, MediaFormatProfile, ModelClaims, ModelProfileManifest, NorLayoutProfile,
    NorRegion, NorRegionKind, NorRegionPolicy, OffsetRange, PROFILES_SCHEMA_VERSION,
    PanaRebuildRecipe, ProfilesBundle, ProgBgoRetargetSpec, PromoteClaimRequest,
    ProposalReviewReport, REQUIRED_LVTTL_MV, RegionSignature, RegistryValidationReport,
    ResolvedModelProfile, RuntimeFingerprint, SataBridgeResetSpec, SocCoreProfile, SocProfile,
    Stage2BootSpec, Stage2BootSymbolsSpec, compute_file_sha256_hex, decode_base64,
    derive_sector_table, derived_geometry_label, encode_base64, ensure_bundle_valid,
    export_schemas_to_dir, format_sa_range, generate_all_schemas,
    interpolate_composite_tv_standard, interpolate_model_regions, interpolate_region_variant,
    list_manual_proposals, list_resolved_models, load_bundle_from_dir, load_bundle_from_str,
    match_queried_region_variant, merge_overlay_dir, model_storage_key, parse_hex_bytes,
    parse_prefixed_u64, promote_model_claim, resolve_model_in_bundle, review_manual_proposal,
    sector_of_offset, sector_range_for_aligned_span, validate_bundle, verify_schemas_match_dir,
};

const EMBEDDED_PROFILES_BUNDLE_JSON: &str =
    include_str!(concat!(env!("OUT_DIR"), "/profiles.bundle.json"));

/// Registry holding the build-time verified (or overlay-merged) relational v2 profile bundle.
#[derive(Debug, Clone)]
pub struct HardwareProfileRegistry {
    bundle: ProfilesBundle,
}

static BUILTIN_REGISTRY: OnceLock<HardwareProfileRegistry> = OnceLock::new();

impl HardwareProfileRegistry {
    /// Loads the compile-time verified built-in v2 hardware profile bundle.
    pub fn load_builtin() -> Result<Self> {
        let bundle = load_bundle_from_str(EMBEDDED_PROFILES_BUNDLE_JSON)?;
        Ok(Self { bundle })
    }

    /// Loads built-in profiles merged with an optional runtime directory of v2 `.json` overlays.
    /// Overlays are strictly monotonic (cannot downgrade claim tiers or relax safety invariants)
    /// and fail closed on any schema, reference, voltage, or geometry error.
    pub fn load_with_overlay(overlay_root: Option<&Path>) -> Result<Self> {
        let base = load_bundle_from_str(EMBEDDED_PROFILES_BUNDLE_JSON)?;
        let merged = match overlay_root {
            Some(dir) => merge_overlay_dir(base, dir)?,
            None => base,
        };
        Ok(Self { bundle: merged })
    }

    /// Returns the cached built-in singleton registry.
    pub fn builtin() -> &'static Self {
        BUILTIN_REGISTRY.get_or_init(|| {
            Self::load_builtin().expect("Built-in Panadiga v2 hardware profiles must be valid")
        })
    }

    /// Returns a reference to the underlying validated bundle.
    #[must_use]
    pub fn bundle(&self) -> &ProfilesBundle {
        &self.bundle
    }

    /// Resolves a model manifest by primary ID or exact alias (case-insensitive).
    pub fn resolve_model(&self, model_id: &str) -> Result<ResolvedModelProfile> {
        resolve_model_in_bundle(&self.bundle, model_id)
    }

    /// Lists all resolved primary model profiles in deterministic key order.
    #[must_use]
    pub fn list_models(&self) -> Vec<ResolvedModelProfile> {
        list_resolved_models(&self.bundle)
    }

    /// Evaluates full relational, claim provenance, and silicon safety validation on the bundle.
    pub fn validate(&self) -> Result<RegistryValidationReport> {
        ensure_bundle_valid(&self.bundle)
    }

    /// Authorizes a capability on a resolved model using runtime hardware/dump fingerprints and
    /// an optional audited [`LabOverride`].
    pub fn authorize(
        &self,
        model_id: &str,
        capability: Capability,
        fingerprint: &RuntimeFingerprint,
        lab_override: Option<&LabOverride>,
    ) -> Result<std::result::Result<Authorization, Denial>> {
        let resolved = self.resolve_model(model_id)?;
        Ok(CapabilityGate::authorize(
            &resolved,
            capability,
            fingerprint,
            lab_override,
        ))
    }
}
