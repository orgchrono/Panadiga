//! Hardware & Firmware Single Source of Truth (SSOT)
//!
//! Shared across CLI, Desktop/Android UI (`apps/desktop/src-tauri`), MCP server, and RAG engine.

pub mod catalog;
pub mod fingerprint;
pub mod pico_header;
pub mod registry;
pub mod spec;
pub mod ts_codegen;

pub use catalog::{
    DigaModelProfileDto, detect_diga_model_from_slice, get_diga_model_catalog_dto,
    is_valid_nor_flash_size,
};
pub use fingerprint::{
    FingerprintCandidate, NorFingerprintReport, detect_diga_model_from_slice_safe,
    fingerprint_nor_or_firmware,
};
pub use pico_header::{
    DEV_SERIAL_NAK_BYTE, export_pico_rsp_ssot_header, export_pico_ssot_files,
    generate_c11_ssot_header, generate_c11_ssot_tables, generate_pana_rsp_ssot_header,
};
pub use ts_codegen::{export_ts_ssot_module, generate_ts_ssot_module};

/// Regenerates all deterministic multi-language SSOT projections (`C11`, `TypeScript`, `JSON Schema`, `Ghidra`) under `root`.
pub fn export_all_ssot_projections(root: &std::path::Path) -> anyhow::Result<()> {
    export_pico_ssot_files(&root.join("firmware/pico-p7501"))?;
    export_ts_ssot_module(&root.join("apps/desktop/src/models/generated/ssot.ts"))?;
    export_schemas_to_dir(&root.join("profiles/schema"))?;
    crate::ghidra::export_all_ghidra_scripts(&root.join("tools/ghidra"))?;
    Ok(())
}
pub use registry::{
    AddressReloc, Authorization, AuthorizationMode, BoardProfile, BytePatchEntry, Capability,
    CapabilityGate, CapabilitySummary, ClaimAxis, ClaimEvidence, Denial, DerivedSector,
    DigadiagSpec, EepromChipProfile, EvidenceSource, EvidenceTier, FirmwareTrainProfile,
    FlashChipProfile, FlashHwProtect, FlashJedecId, FlashRwBank, FlashSectorBank, FrmContainerSpec,
    HardwareProfileRegistry, LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride, LoaderFingerprint,
    MediaFormatProfile, ModelClaims, ModelProfileManifest, NorLayoutProfile, NorRegion,
    NorRegionKind, NorRegionPolicy, OffsetRange, PanaRebuildRecipe, ProfilesBundle,
    ProgBgoRetargetSpec, PromoteClaimRequest, RegionSignature, RegistryValidationReport,
    ResolvedModelProfile, RuntimeFingerprint, SocCoreProfile, SocProfile, decode_base64,
    derive_sector_table, derived_geometry_label, encode_base64, export_schemas_to_dir,
    format_sa_range, generate_all_schemas, promote_model_claim, sector_of_offset,
    sector_range_for_aligned_span, verify_schemas_match_dir,
};
pub use spec::{
    BreakInModeSpecDto, FlashGeometrySpecDto, HardwareSsotSpecDto, HardwareThresholdsDto,
    MemoryRegionDto, P7501PinMappingDto, PicoCommandArgOptionDto, PicoCommandSpecDto,
    PicoUsbIdentityDto, build_hardware_ssot_spec,
};
