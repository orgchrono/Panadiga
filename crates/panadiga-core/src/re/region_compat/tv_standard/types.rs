#![deny(unsafe_code)]

//! Domain types for declarative TV Standard (`NTSC` / `PAL-M` / `PAL` / `SECAM`),
//! `LBS` regional chassis calibration, and read-only bundle audit reports.

use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{AIOF_VMODESEL_NTSC, AIOF_VMODESEL_PAL, AIOF_VMODESEL_PAL_M};
use crate::crypto::DecryptedFrmTocEntry;
use crate::nvram::NvramCleanReport;

use super::super::types::{DevBinReport, MainRegionCompatReport};

/// Analog television colorimetric & scanning standard supported by the `AM34` `AIOF`/`AVDD` pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TvStandardKind {
    Ntsc,
    PalM,
    Pal,
    Secam,
}

impl TvStandardKind {
    /// Canonical lowercase slug.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ntsc => "ntsc",
            Self::PalM => "pal_m",
            Self::Pal => "pal",
            Self::Secam => "secam",
        }
    }

    /// Clean-room `AIOF_VMODESEL` (`+0x00`) register value when verified statically.
    pub const fn vmodesel_value(self) -> Option<u16> {
        match self {
            Self::Ntsc => Some(AIOF_VMODESEL_NTSC),
            Self::PalM => Some(AIOF_VMODESEL_PAL_M),
            Self::Pal => Some(AIOF_VMODESEL_PAL),
            Self::Secam => None,
        }
    }
}

impl FromStr for TvStandardKind {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ntsc" | "ntsc-m" | "ntsc_m" | "ntsc-j" | "ntsc_j" => Ok(Self::Ntsc),
            "pal-m" | "palm" | "pal_m" | "br" | "lbs" => Ok(Self::PalM),
            "pal" | "pal-bg" | "pal-b" | "pal-g" | "pal-i" | "pal_eu" => Ok(Self::Pal),
            "secam" => Ok(Self::Secam),
            other => {
                bail!("Unsupported TV standard '{other}': expected ntsc, pal-m, pal, or secam")
            }
        }
    }
}

/// Regional chassis calibration preset (Brazil `LBS`, North America `NA`, Europe `EC`, or inspected).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LbsRegionPresetId {
    BrPalMR4,
    UsNtscR1,
    EuPalR2,
    CustomInspected,
}

impl LbsRegionPresetId {
    /// Canonical lowercase preset identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BrPalMR4 => "br_pal_m_r4",
            Self::UsNtscR1 => "us_ntsc_r1",
            Self::EuPalR2 => "eu_pal_r2",
            Self::CustomInspected => "custom_inspected",
        }
    }

    /// Primary analog TV standard paired with this regional chassis.
    pub const fn default_tv_standard(self) -> TvStandardKind {
        match self {
            Self::BrPalMR4 => TvStandardKind::PalM,
            Self::UsNtscR1 | Self::CustomInspected => TvStandardKind::Ntsc,
            Self::EuPalR2 => TvStandardKind::Pal,
        }
    }

    /// Expected `M6930*` chassis family string in `DEV.BIN` / `ifodev2` / `.FRM` TOC.
    pub const fn expected_chassis_family(self) -> &'static str {
        match self {
            Self::BrPalMR4 => "M6930LB",
            Self::UsNtscR1 | Self::EuPalR2 | Self::CustomInspected => "M6930",
        }
    }

    /// Expected DVD region code for stock regional firmware (`None` for custom/inspected).
    pub const fn dvd_region_code(self) -> Option<u8> {
        match self {
            Self::BrPalMR4 => Some(4),
            Self::UsNtscR1 => Some(1),
            Self::EuPalR2 => Some(2),
            Self::CustomInspected => None,
        }
    }

    /// Expected single-byte `DEV.BIN`/`ifodev2` OEM marker (`0x0C`) when known.
    pub const fn expected_oem_char(self) -> Option<char> {
        match self {
            Self::BrPalMR4 => Some('M'),
            Self::UsNtscR1 => Some('P'),
            Self::EuPalR2 => Some('E'),
            Self::CustomInspected => None,
        }
    }
}

impl FromStr for LbsRegionPresetId {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "br_pal_m_r4" | "br" | "lbs" | "pal-m" | "m6930lb" => Ok(Self::BrPalMR4),
            "us_ntsc_r1" | "us" | "na" | "ntsc" => Ok(Self::UsNtscR1),
            "eu_pal_r2" | "eu" | "ec" | "eg" | "pal" => Ok(Self::EuPalR2),
            "custom" | "custom_inspected" | "inspected" => Ok(Self::CustomInspected),
            other => bail!(
                "Unsupported LBS region preset '{other}': expected br_pal_m_r4, us_ntsc_r1, eu_pal_r2, or custom_inspected"
            ),
        }
    }
}

/// Declarative MMIO register write specification for clean-room TV/TBC calibration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MmioWriteOp {
    pub addr: u32,
    pub width_bits: u8,
    pub value: u32,
    pub register_name: String,
    pub note: String,
}

/// Declarative calibration profile uniting regional metadata and clean-room `AIOF`/`AVDD` MMIO plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvCalibrationProfile {
    pub preset: LbsRegionPresetId,
    pub tv_standard: TvStandardKind,
    pub chassis_family: String,
    pub dvd_region_code: Option<u8>,
    pub expected_oem_char: Option<char>,
    /// Explicitly guards unverified `ifodev2` trailing blocks (`0x400..0x7FF`) from silent byte writes.
    pub metadata_and_dev_oem_only: bool,
    pub mmio_plan: Vec<MmioWriteOp>,
    pub notes: Vec<String>,
}

/// Read-only audit summary of the unverified `ifodev2` (`0x400..0x7FF`) payload blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ifodev2TailMetadata {
    pub offset_range_hex: String,
    pub byte_len: usize,
    pub non_ff_bytes: usize,
    pub prefix_hex: String,
    pub sha256: String,
    pub metadata_and_dev_oem_only: bool,
}

/// Summary of an inspected `.FRM` TOC container inside a TV/region calibration bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrmTocInspectionSummary {
    pub layout: String,
    pub entry_count: usize,
    pub all_list_sums_valid: bool,
    pub entries: Vec<DecryptedFrmTocEntry>,
    pub sha256: String,
}

/// Consolidated read-only inspection report across `DEV.BIN`/`ifodev2`, `MAIN`, `.FRM`, and NVRAM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvCalibrationBundleReport {
    pub profile: TvCalibrationProfile,
    pub dev_bin: Option<DevBinReport>,
    pub ifodev2_tail: Option<Ifodev2TailMetadata>,
    pub metadata_and_dev_oem_only: bool,
    pub main_region: Option<MainRegionCompatReport>,
    pub frm_toc: Option<FrmTocInspectionSummary>,
    pub nvram_sa134_sa141: Option<NvramCleanReport>,
    pub warnings: Vec<String>,
}
