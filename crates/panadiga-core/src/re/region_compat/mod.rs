//! Deterministic Region & Model Compatibility Patcher (`MAIN` `<deviceinfo>`, `DEV.BIN`, `.FRM` TOC).
//!
//! Targets three verified compatibility surfaces across Panasonic DIGA firmware trains:
//! 1. `MAIN` updater (`src/fmupre/pre_update_firm.c` / `FUN_5749859a`) 8-byte `model` + 1-byte `oem`
//!    guards in both `3410PQ` (`VA 0x57498738`) and `3060NP` (`VA 0x574A01AA`).
//! 2. `DEV.BIN` (`512`-byte `AA 55 FF FF` / `PANA_DVD_REC` container device descriptor).
//! 3. `PANA_DVD.FRM` (`48B` Feistel) & `PANA__SD.FRM` (`64B` plaintext) TOC model/family metadata.

mod dev_bin;
mod frm_retarget;
mod main_fmu;
pub mod tv_standard;
mod types;

#[cfg(test)]
mod tests;

pub use dev_bin::{PANA_IFODEV2_SIZE, inspect_dev_bin_bytes, patch_dev_bin_bytes};
pub use frm_retarget::retarget_frm_toc_bytes;
pub use main_fmu::{
    EXPECTED_MAIN_3060NP_SIZE, OFF_FMU_DEVINFO_3060NP, OFF_FMU_DEVINFO_3410PQ,
    ORIG_FMU_DEVINFO_3060NP, ORIG_FMU_DEVINFO_3410PQ, PATCH_FMU_DEVINFO_3060NP,
    PATCH_FMU_DEVINFO_3410PQ, inspect_main_region_compat, patch_main_region_compat_in_place,
};
pub use tv_standard::{
    FrmTocInspectionSummary, Ifodev2TailMetadata, LbsRegionPresetId, MmioWriteOp,
    TvCalibrationBundleReport, TvCalibrationProfile, TvStandardKind,
    build_tv_calibration_mmio_plan, build_tv_calibration_profile, inspect_tv_calibration_bundle,
};
pub use types::{DevBinReport, FrmRetargetReport, MainRegionCompatReport, RegionCompatMode};

#[cfg(test)]
pub(crate) fn make_synthetic_ifodev2(model: &str, oem: u8, tail_prefix: &[u8]) -> Vec<u8> {
    use crate::constants::{PANA_DEV_BIN_MAGIC, PANA_DEV_BIN_SIG};
    use crate::crypto::codec::write_padded_ascii;

    let mut ifodev2 = vec![0xFFu8; PANA_IFODEV2_SIZE];
    ifodev2[0..4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
    write_padded_ascii(&mut ifodev2[0x04..0x0C], model, 0);
    ifodev2[0x0C] = oem;
    ifodev2[0x20..0x2C].copy_from_slice(PANA_DEV_BIN_SIG);
    ifodev2[0x400..0x400 + tail_prefix.len()].copy_from_slice(tail_prefix);
    ifodev2
}
