#![deny(unsafe_code)]

//! Declarative TV Standard (`NTSC` / `PAL-M` / `PAL` / `SECAM`), `LBS` Region Calibration,
//! and Clean-Room `AIOF`/`AVDD` MMIO Plan Builder for Panasonic DIGA (`DMR-EH55` / `AM34`).
//!
//! Strictly read-only / copy-on-write inspection: unverified `ifodev2` trailing bytes
//! (`0x400..0x7FF`) are explicitly tagged `metadata_and_dev_oem_only = true` to prohibit
//! silent binary mutation.

mod inspect;
mod mmio;
#[cfg(test)]
mod tests;
mod types;

pub use inspect::inspect_tv_calibration_bundle;
pub use mmio::{
    AVDD_NPALCTL0_PRESERVED_HI_WORD, build_tv_calibration_mmio_plan, build_tv_calibration_profile,
};
pub use types::{
    FrmTocInspectionSummary, Ifodev2TailMetadata, LbsRegionPresetId, MmioWriteOp,
    TvCalibrationBundleReport, TvCalibrationProfile, TvStandardKind,
};
