//! DTOs and ASCII validation helpers for region & model compatibility patching.

use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::crypto::DecryptedFrmTocEntry;

/// Patch mode for `MAIN` updater `<deviceinfo>` model/OEM guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionCompatMode {
    BypassModelAndOem,
    FactoryStrict,
}

impl FromStr for RegionCompatMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bypass" | "bypass_model_and_oem" | "unlock" => Ok(Self::BypassModelAndOem),
            "factory" | "factory_strict" | "restore" => Ok(Self::FactoryStrict),
            other => {
                bail!("Unsupported region-compat mode '{other}' (expected bypass|factory_strict)")
            }
        }
    }
}

/// Inspection/patch summary for unpacked `MAIN` (`3410PQ` / `3060NP`) `<deviceinfo>` checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainRegionCompatReport {
    pub build_profile: String,
    pub size_bytes: usize,
    pub va_hex: String,
    pub file_offset_hex: String,
    pub state: String,
    pub actual_hex: String,
    pub sha256: String,
}

/// Decoded/verified `DEV.BIN` (`512`-byte) or NFROM `ifodev2` (`2048`-byte) device descriptor report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevBinReport {
    pub size_bytes: usize,
    pub model: String,
    pub oem: String,
    pub option_code_hex: String,
    pub hw_version_u16: u16,
    pub model_code: u8,
    pub market_code: u8,
    pub market_code_name: String,
    pub main_option_btn: [u16; 4],
    pub main_option_rcmode: u8,
    pub main_option_gui: [u8; 4],
    pub main_option_chmode: u8,
    pub signature: String,
    pub md5: String,
    pub sha256: String,
}

/// Summary of retargeted `.FRM` TOC metadata slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrmRetargetReport {
    pub layout: String,
    pub total_bytes: usize,
    pub updated_entries: usize,
    pub entries: Vec<DecryptedFrmTocEntry>,
    pub sha256: String,
}

pub(super) fn validate_ascii_field(label: &str, text: &str, max_len: usize) -> Result<()> {
    let t = text.trim();
    if t.is_empty() || t.len() > max_len || !t.bytes().all(|b| (0x20..0x7F).contains(&b)) {
        bail!("{label} must be 1..={max_len} printable ASCII characters, got '{text}'");
    }
    Ok(())
}
