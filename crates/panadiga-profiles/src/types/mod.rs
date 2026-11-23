#![deny(unsafe_code)]

//! Domain types for normalized hardware, firmware train, NOR layout, and policy-gated models.

pub mod board;
pub mod eeprom;
pub mod evidence;
pub mod flash;
pub mod layout;
pub mod media;
pub mod model;
pub mod patch_specs;
pub mod soc;
pub mod train;

pub use board::BoardProfile;
pub use eeprom::{EepromChipProfile, EepromFunction};
pub use evidence::{
    Authorization, AuthorizationMode, Capability, ClaimEvidence, Denial, EvidenceSource,
    EvidenceTier, LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride, RuntimeFingerprint,
};
pub use flash::{FlashChipProfile, FlashHwProtect, FlashJedecId, FlashRwBank, FlashSectorBank};
pub use layout::{NorLayoutProfile, NorRegion, NorRegionKind, NorRegionPolicy, RegionSignature};
pub use media::MediaFormatProfile;
pub use model::{
    CapabilitySummary, ClaimAxis, ModelClaims, ModelProfileManifest, ProfilesBundle,
    RegistryValidationReport, ResolvedModelProfile,
};
pub use patch_specs::{
    BinaryPatchSiteSpec, MainBuildSpec, SataBridgeResetSpec, Stage2BootSpec, Stage2BootSymbolsSpec,
};
pub use soc::{BusWindow, SocCoreProfile, SocProfile};
pub use train::{
    AddressReloc, BytePatchEntry, DigadiagSpec, DigadiagTrustModel, FirmwareTrainProfile,
    FrmContainerSpec, LoaderFingerprint, OffsetRange, PanaRebuildRecipe, ProgBgoRetargetSpec,
};
