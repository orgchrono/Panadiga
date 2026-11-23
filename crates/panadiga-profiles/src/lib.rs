#![deny(unsafe_code)]

//! Panadiga Manifest-Driven, Policy-Gated Hardware Profile SSOT Crate.

pub mod blob_b64;
pub mod derive;
pub mod gate;
pub mod hex_serde;
pub mod loader;
pub mod loader_resolve;
pub mod promote;
pub mod region;
pub mod schema;
pub mod types;
pub mod validate;
mod validate_bgo;
mod validate_layout;
mod validate_patches;

#[cfg(test)]
mod tests;

pub use blob_b64::{decode_base64, encode_base64};

pub use derive::{
    DerivedSector, derive_sector_table, derived_geometry_label, format_sa_range, sector_of_offset,
    sector_range_for_aligned_span,
};
pub use gate::CapabilityGate;
pub use hex_serde::{parse_hex_bytes, parse_prefixed_u64};
pub use loader::{
    ENTITY_SUBDIRS, PROFILES_SCHEMA_VERSION, list_resolved_models, load_bundle_from_dir,
    load_bundle_from_str, merge_overlay_dir, model_storage_key, resolve_model_in_bundle,
};
pub use promote::{PromoteClaimRequest, promote_model_claim};
pub use region::{
    InterpolatedRegionVariant, interpolate_composite_tv_standard, interpolate_model_regions,
    interpolate_region_variant, match_queried_region_variant,
};
pub use schema::{export_schemas_to_dir, generate_all_schemas, verify_schemas_match_dir};
pub use types::*;
pub use validate::{
    AM34_LETHAL_MMIO_ADDR, MIN_CRITICAL_IPL_GUARD_BYTES, REQUIRED_LVTTL_MV, ensure_bundle_valid,
    validate_bundle,
};
