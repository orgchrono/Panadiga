#![deny(unsafe_code)]

//! Safe FAT16B (`0x06`) SD recovery card creator (image synthesis, lockout guards, and flashing).

pub mod flash;
pub mod plan;
pub mod report;
pub mod shared_kit;

pub use flash::{
    create_sd_recovery_card_on_device, create_sd_recovery_image, flash_sd_recovery_card,
};
pub use plan::{
    SDXC_ADVISORY_THRESHOLD_BYTES, SdCardCreatorPlan, SdCardCreatorRequest, SdTargetDriveInfo,
    assess_sd_card_size_advisory, inspect_sd_card_targets, plan_sd_recovery_card,
};
pub use report::{
    SdCardAdvisory, SdCardCreatorReport, SdCreatorProgress, SdCreatorStage,
    build_layperson_sd_steps,
};
pub use shared_kit::{RecoveryPayloadSpec, ResolvedRecoveryMedia, resolve_recovery_media_payloads};
