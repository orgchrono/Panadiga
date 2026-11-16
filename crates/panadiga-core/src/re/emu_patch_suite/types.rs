//! Report DTOs for `emu_patch_suite`.

use serde::{Deserialize, Serialize};

/// Individual verification stage outcome produced by [`super::run_nor_emu_patch_verification_suite`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchVerificationCheck {
    pub id: String,
    pub passed: bool,
    pub detail: String,
}

/// Complete report covering `.FRM` packing, Spansion NOR simulation, and MN103S routine execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NorEmuPatchSuiteReport {
    pub source_mode: String,
    pub cfi_manufacturer_id_hex: String,
    pub cfi_device_id_hex: String,
    pub frm_dvd_bytes: usize,
    pub frm_sd_bytes: usize,
    pub prog061_sha256: String,
    pub main_sha256: String,
    pub checks: Vec<PatchVerificationCheck>,
    pub all_passed: bool,
}

pub(super) struct FlashedFrmArtifacts {
    pub(super) prog061: Vec<u8>,
    pub(super) main: Vec<u8>,
    pub(super) dvd_bytes: usize,
    pub(super) sd_bytes: usize,
    pub(super) check: PatchVerificationCheck,
}
