#![deny(unsafe_code)]

//! Profiles, request builders, and output artifact descriptors for SD unbrick kits.

use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::super::super::bgo_unbrick::Prog061BgoPatchMode;
use super::super::super::main_passthrough::MainPassthroughMode;
use super::super::verify::{DualFamilyParityReport, SdMediaVerificationReport};

/// Target Panasonic DIGA bootloader media profile for SD recovery kits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SdLoaderProfile {
    /// DMR-EH55 `Loader ver0.61 [06/03/06]`: 256 KiB (`0x40000`) raw `DIGADIAG.FRM` + 48B `sd48` TOC `PANA__SD.FRM`.
    #[default]
    Ver061RawSd48,
    /// Later `Loader ver0.63` (e.g. DMR-EH58): 128 KiB (`0x20000`) plaintext `EXTRHEAD` `DIGADIAG.FRM` + 64B `sd64` TOC.
    Ver063ExtrheadSd64,
}

impl SdLoaderProfile {
    /// Canonical lowercase profile slug.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ver061RawSd48 => "ver061",
            Self::Ver063ExtrheadSd64 => "ver063",
        }
    }

    /// Human-readable description of the container format.
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Ver061RawSd48 => "Loader ver0.61 (256 KiB raw DIGADIAG.FRM + 48B sd48 TOC)",
            Self::Ver063ExtrheadSd64 => {
                "Loader ver0.63 (128 KiB EXTRHEAD DIGADIAG.FRM + 64B sd64 TOC)"
            }
        }
    }
}

impl std::str::FromStr for SdLoaderProfile {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ver061" | "0.61" | "ver0.61" | "eh55" | "raw_sd48" | "sd48" => Ok(Self::Ver061RawSd48),
            "ver063" | "0.63" | "ver0.63" | "extrhead_sd64" | "sd64" => {
                Ok(Self::Ver063ExtrheadSd64)
            }
            other => bail!(
                "Unsupported SD loader profile '{other}': expected ver061 (DMR-EH55 raw 256 KiB + sd48) or ver063 (128 KiB EXTRHEAD + sd64)"
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SdUnbrickKitRequest {
    pub output_dir: PathBuf,
    pub pana_dvd_path: Option<PathBuf>,
    pub prog061_path: Option<PathBuf>,
    pub main_path: Option<PathBuf>,
    pub drv_path: Option<PathBuf>,
    pub bgo_mode: Prog061BgoPatchMode,
    pub main_passthrough_mode: MainPassthroughMode,
    pub loader_profile: SdLoaderProfile,
    pub include_pana_sd: bool,
    pub unlock_hdd_dynamic: bool,
    pub hdd_target_gb: Option<u32>,
    pub write_extracted_sources: bool,
    pub allow_unsafe_auto_format: bool,
    pub expected_main_build: Option<String>,
    pub allow_target_mismatch: bool,
    pub sata_bridge_reset_fix: bool,
    pub sata_bridge_stage2_boot: bool,
    pub main_bgo_retarget: bool,
}

/// High-level SD recovery kit preset distinguishing standard builds from the hardened `PANAMON1` `ver0.61` kit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdKitBuildPreset {
    #[default]
    Standard,
    HardenedPanamon1,
}

impl std::str::FromStr for SdKitBuildPreset {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        let hardened = super::super::parse_sd_hardened_profile(
            raw,
            "build preset",
            &[
                "hardened_panamon1",
                "panamon1",
                "ver061_hardened",
                "hardened",
            ],
        )?;
        Ok(if hardened {
            Self::HardenedPanamon1
        } else {
            Self::Standard
        })
    }
}

impl SdUnbrickKitRequest {
    /// Constructs a pre-hardened DMR-EH55 `Loader ver0.61` SD unbrick request using
    /// `Prog061BgoPatchMode::UartMonitor` (`PANAMON1`), dual-slot `sd48` `PANA__SD.FRM`,
    /// `AV_PASSTHROUGH`, and dynamic HDD capacity unlock.
    #[must_use]
    pub fn hardened_panamon1(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            pana_dvd_path: None,
            prog061_path: None,
            main_path: None,
            drv_path: None,
            bgo_mode: Prog061BgoPatchMode::UartMonitor,
            main_passthrough_mode: MainPassthroughMode::AvPassthrough,
            loader_profile: SdLoaderProfile::Ver061RawSd48,
            include_pana_sd: true,
            unlock_hdd_dynamic: true,
            hdd_target_gb: None,
            write_extracted_sources: false,
            allow_unsafe_auto_format: false,
            expected_main_build: None,
            allow_target_mismatch: false,
            sata_bridge_reset_fix: false,
            sata_bridge_stage2_boot: false,
            main_bgo_retarget: false,
        }
    }
}

/// Verified in-memory output of `compose_hardened_panamon1_kit`.
#[derive(Debug, Clone)]
pub struct HardenedPanamon1KitArtifacts {
    pub digadiag_bytes: Vec<u8>,
    pub digadiag_sha256: String,
    pub pana_sd_bytes: Vec<u8>,
    pub pana_sd_sha256: String,
    pub dual_parity: DualFamilyParityReport,
    pub verification: SdMediaVerificationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdUnbrickKitReport {
    pub output_dir: String,
    pub prog061_source: String,
    pub main_source: Option<String>,
    pub drv_source: Option<String>,
    pub bgo_mode_applied: String,
    pub loader_profile_applied: String,
    pub main_passthrough_applied: Option<String>,
    pub hdd_capacity_applied: Option<String>,
    #[serde(default)]
    pub sata_bridge_reset_fix_applied: bool,
    #[serde(default)]
    pub sata_bridge_stage2_boot_applied: bool,
    #[serde(default)]
    pub main_bgo_retarget_applied: bool,
    pub digadiag_path: String,
    pub digadiag_sha256: String,
    pub pana_sd_path: Option<String>,
    pub pana_sd_sha256: Option<String>,
    pub sha256sums_path: String,
    pub verification: SdMediaVerificationReport,
}

pub(crate) struct ComposedMainArtifact {
    pub container: Vec<u8>,
    pub slot_name: String,
    pub build_version_u16: u16,
    pub passthrough_applied: String,
    pub hdd_capacity_applied: Option<String>,
    pub sata_bridge_reset_fix_applied: bool,
    pub main_bgo_retarget_applied: bool,
}
