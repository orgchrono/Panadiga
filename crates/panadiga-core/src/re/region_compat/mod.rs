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
mod types;

#[cfg(test)]
mod tests;

pub use dev_bin::{inspect_dev_bin_bytes, patch_dev_bin_bytes};
pub use frm_retarget::retarget_frm_toc_bytes;
pub use main_fmu::{
    EXPECTED_MAIN_3060NP_SIZE, OFF_FMU_DEVINFO_3060NP, OFF_FMU_DEVINFO_3410PQ,
    ORIG_FMU_DEVINFO_3060NP, ORIG_FMU_DEVINFO_3410PQ, PATCH_FMU_DEVINFO_3060NP,
    PATCH_FMU_DEVINFO_3410PQ, inspect_main_region_compat, patch_main_region_compat_in_place,
};
pub use types::{DevBinReport, FrmRetargetReport, MainRegionCompatReport, RegionCompatMode};
