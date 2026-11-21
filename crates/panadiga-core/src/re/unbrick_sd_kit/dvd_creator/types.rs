#![deny(unsafe_code)]

//! Domain DTOs and progress stages for ISO 9660 Level 1 (`1x` / min speed) DVD recovery creation.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::compose::SdLoaderProfile;
use super::super::iso9660_builder::{Iso9660InspectionReport, Iso9660PlacedFileReport};
use crate::re::prewrite_gate::PreWriteGateReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DvdCreatorStage {
    SynthesizeIso,
    EmuGateVerify,
    InspectIso9660,
    LockoutVerify,
    BurnOptical1x,
    ReadbackVerify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DvdCreatorProgress {
    pub stage: DvdCreatorStage,
    pub percent: u8,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpticalBurnerDriveInfo {
    pub device_id: String,
    pub display_name: String,
    pub os_backend: String,
    pub supported_write_speeds_kbps: Vec<u32>,
    pub min_write_speed_kbps: u32,
    pub min_write_speed_label: String,
    pub safe_optical_target: bool,
    pub expected_confirm_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DvdCreatorRequest {
    pub burner_id: Option<String>,
    pub confirm_token: Option<String>,
    pub output_iso_path: Option<PathBuf>,
    pub model: String,
    pub volume_label: String,
    pub loader_profile: SdLoaderProfile,
    pub digadiag_path: Option<PathBuf>,
    pub pana_dvd_path: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub cleanroom_only: bool,
    #[serde(default)]
    pub dry_run: bool,
}

fn default_true() -> bool {
    true
}

impl Default for DvdCreatorRequest {
    fn default() -> Self {
        Self {
            burner_id: None,
            confirm_token: None,
            output_iso_path: None,
            model: "DMR-EH55".to_string(),
            volume_label: "PANADIGA_REC".to_string(),
            loader_profile: SdLoaderProfile::Ver061RawSd48,
            digadiag_path: None,
            pana_dvd_path: None,
            cleanroom_only: true,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DvdCreatorReport {
    pub target: String,
    pub target_kind: String,
    pub model: String,
    pub volume_label: String,
    pub cleanroom_only: bool,
    pub bytes_written: usize,
    pub iso_sha256: String,
    pub write_speed_policy: String,
    pub burn_command_preview: Option<String>,
    pub files: Vec<Iso9660PlacedFileReport>,
    pub emu_gate: PreWriteGateReport,
    pub inspection: Iso9660InspectionReport,
    pub next_steps: Vec<String>,
}

/// Computes canonical `CONFIRM-<burner_id>` safety token for optical disc burning.
#[must_use]
pub fn expected_optical_confirm_token(device_id: &str) -> String {
    let trimmed = device_id
        .trim()
        .strip_prefix("\\\\.\\")
        .or_else(|| device_id.trim().strip_prefix("/dev/"))
        .unwrap_or(device_id.trim());
    format!("CONFIRM-{trimmed}")
}
