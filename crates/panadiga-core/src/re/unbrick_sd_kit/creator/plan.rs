#![deny(unsafe_code)]

//! SD card drive enumeration, SDXC `>= 64 GiB` compatibility advisory, and FAT16B kit planning.

use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::super::compose::SdLoaderProfile;
use super::super::fat16_builder::{
    FAT16_DEFAULT_VOLUME_MB, FAT16_MAX_VOLUME_MB, FAT16_MIN_VOLUME_MB, SynthesizedSdImage,
    synthesize_fat16_sd_image,
};
use super::report::SdCardAdvisory;
use super::shared_kit::{RecoveryPayloadSpec, resolve_recovery_media_payloads};
use crate::hdd::writer::{PhysicalDriveInfo, StorageMediaRole, list_physical_drives};
use crate::re::prewrite_gate::PreWriteGateReport;

/// Exact binary `64 GiB` threshold above which SDXC cards often fail AM34 SD slot initialization.
pub const SDXC_ADVISORY_THRESHOLD_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const TWO_GIB_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn default_cleanroom_only() -> bool {
    true
}

/// Request DTO for synthesizing or flashing a deterministic FAT16B (`0x06`) SD recovery card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdCardCreatorRequest {
    pub device_path: Option<String>,
    pub confirm_token: Option<String>,
    pub output_image_path: Option<PathBuf>,
    pub model: String,
    pub volume_mb: u32,
    pub loader_profile: SdLoaderProfile,
    pub digadiag_path: Option<PathBuf>,
    pub pana_sd_path: Option<PathBuf>,
    pub pana_dvd_path: Option<PathBuf>,
    pub include_pana_sd: bool,
    #[serde(default = "default_cleanroom_only")]
    pub cleanroom_only: bool,
}

impl Default for SdCardCreatorRequest {
    fn default() -> Self {
        Self {
            device_path: None,
            confirm_token: None,
            output_image_path: None,
            model: "DMR-EH55".to_string(),
            volume_mb: FAT16_DEFAULT_VOLUME_MB,
            loader_profile: SdLoaderProfile::Ver061RawSd48,
            digadiag_path: None,
            pana_sd_path: None,
            pana_dvd_path: None,
            include_pana_sd: false,
            cleanroom_only: true,
        }
    }
}

/// Enriched SD card target metadata for UI selectors and pre-flight lockout verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdTargetDriveInfo {
    pub device_path: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub size_gb: f64,
    pub drive_role: StorageMediaRole,
    pub bus_type: String,
    pub removable: bool,
    pub safe_sd_target: bool,
    pub expected_confirm_token: String,
    pub advisory: SdCardAdvisory,
    pub block_reason: Option<String>,
}

/// Planned SD recovery card build metadata prior to image output or physical block writing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdCardCreatorPlan {
    pub model: String,
    pub volume_mb: u32,
    pub cleanroom_only: bool,
    pub has_pana_sd: bool,
    pub digadiag_source: String,
    pub pana_sd_source: Option<String>,
    pub emu_gate: PreWriteGateReport,
}

/// Evaluates whether an SD card's physical capacity triggers an SDXC `>= 64 GiB` advisory warning.
#[must_use]
pub fn assess_sd_card_size_advisory(bytes: u64) -> SdCardAdvisory {
    if bytes == 0 {
        return SdCardAdvisory {
            is_sdxc_warning: false,
            card_class: "Unknown".to_string(),
            recommendation: "Use SDSC (<= 2 GB) or SDHC (4..32 GB) card".to_string(),
            detail:
                "Virtual image mode or unreported media size; verify physical card is SDSC/SDHC."
                    .to_string(),
        };
    }
    if bytes >= SDXC_ADVISORY_THRESHOLD_BYTES {
        SdCardAdvisory {
            is_sdxc_warning: true,
            card_class: "SDXC (>= 64 GiB)".to_string(),
            recommendation: "Recommend SDSC (<= 2 GB) or SDHC (4..32 GB)".to_string(),
            detail: "Card is >= 64 GiB (SDXC). Older Panasonic DIGA IPL SD controllers frequently reject SDXC cards even with a 32 MiB FAT16B partition; SDSC <= 2 GB or SDHC 4..32 GB is strongly recommended.".to_string(),
        }
    } else if bytes <= TWO_GIB_BYTES {
        SdCardAdvisory {
            is_sdxc_warning: false,
            card_class: "SDSC (<= 2 GiB)".to_string(),
            recommendation: "Optimal hardware compatibility".to_string(),
            detail: "Legacy SDSC capacity (<= 2 GiB): ideal for Panasonic DIGA AM34 IPL SDDIAG."
                .to_string(),
        }
    } else {
        SdCardAdvisory {
            is_sdxc_warning: false,
            card_class: "SDHC (4..32 GiB)".to_string(),
            recommendation: "Compatible with 32 MiB FAT16B MBR partition".to_string(),
            detail: "SDHC card: compatible when partitioned as deterministic 32 MiB FAT16B (0x06) at LBA 63.".to_string(),
        }
    }
}

pub(crate) fn expected_confirm_token_for(device_path: &str) -> String {
    format!("CONFIRM-{}", device_path.trim_start_matches(r"\\.\"))
}

fn enrich_target(d: &PhysicalDriveInfo) -> SdTargetDriveInfo {
    let is_sd = d.drive_role == StorageMediaRole::SdCard && !d.is_system_drive && d.safe_to_write;
    let block_reason = if d.is_system_drive {
        Some(
            d.lockout_reason
                .clone()
                .unwrap_or_else(|| "Primary OS system disk".into()),
        )
    } else if d.drive_role != StorageMediaRole::SdCard {
        Some(format!(
            "Blocked by role {:?}: SD Creator accepts ONLY StorageMediaRole::SdCard",
            d.drive_role
        ))
    } else if !d.safe_to_write {
        Some(
            d.lockout_reason
                .clone()
                .unwrap_or_else(|| "Drive safety lockout active".into()),
        )
    } else {
        None
    };

    SdTargetDriveInfo {
        device_path: d.device_path.clone(),
        display_name: d.display_name.clone(),
        size_bytes: d.size_bytes,
        size_gb: d.size_gb,
        drive_role: d.drive_role,
        bus_type: d.bus_type.clone(),
        removable: d.removable,
        safe_sd_target: is_sd,
        expected_confirm_token: expected_confirm_token_for(&d.device_path),
        advisory: assess_sd_card_size_advisory(d.size_bytes),
        block_reason,
    }
}

/// Enumerates all host drives tagged with SD-only eligibility and SDXC `>= 64 GiB` advisories.
#[must_use]
pub fn inspect_sd_card_targets() -> Vec<SdTargetDriveInfo> {
    list_physical_drives()
        .unwrap_or_default()
        .iter()
        .map(enrich_target)
        .collect()
}

/// Resolves kit files (clean-room `DIGADIAG.FRM`, explicit paths, or [`prepare_sd_unbrick_kit`]) and validates in `Mn103Vm`.
pub fn plan_sd_recovery_card(
    req: &SdCardCreatorRequest,
) -> Result<(SdCardCreatorPlan, SynthesizedSdImage)> {
    if !(FAT16_MIN_VOLUME_MB..=FAT16_MAX_VOLUME_MB).contains(&req.volume_mb) {
        bail!(
            "SD FAT16B volume_mb={} outside supported {}..={} MiB",
            req.volume_mb,
            FAT16_MIN_VOLUME_MB,
            FAT16_MAX_VOLUME_MB
        );
    }

    let spec = RecoveryPayloadSpec {
        loader_profile: req.loader_profile,
        digadiag_path: req.digadiag_path.clone(),
        secondary_path: req.pana_sd_path.clone(),
        pana_dvd_source_path: req.pana_dvd_path.clone(),
        include_secondary_from_kit: req.include_pana_sd,
        cleanroom_only: req.cleanroom_only,
    };
    let resolved = resolve_recovery_media_payloads(&spec, "PANA__SD.FRM")?;
    let synth = synthesize_fat16_sd_image(req.volume_mb, &resolved.files)?;
    let plan = SdCardCreatorPlan {
        model: req.model.clone(),
        volume_mb: req.volume_mb,
        cleanroom_only: resolved.cleanroom_only,
        has_pana_sd: resolved.secondary_source.is_some(),
        digadiag_source: resolved.diag_source,
        pana_sd_source: resolved.secondary_source,
        emu_gate: resolved.emu_gate,
    };
    Ok((plan, synth))
}
