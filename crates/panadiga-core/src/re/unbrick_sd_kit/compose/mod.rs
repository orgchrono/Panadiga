#![deny(unsafe_code)]

//! Pure deterministic artifact builders for `DIGADIAG.FRM`, `PANA__SD.FRM`, and `SHA256SUMS`.

mod builders;
mod hardened;
#[cfg(test)]
mod tests;
mod types;

#[cfg(test)]
pub(crate) use builders::compose_digadiag_and_prog061;
pub(crate) use builders::{
    compose_digadiag_and_prog061_with_tier_policy, compose_pana_sd_container,
    compose_pana_sd_prog061_with_tier_policy, compose_patched_main_container,
    format_sha256_manifest,
};
pub use hardened::compose_hardened_panamon1_kit;
pub use types::{
    HardenedPanamon1KitArtifacts, SdKitBuildPreset, SdLoaderProfile, SdUnbrickKitReport,
    SdUnbrickKitRequest,
};
