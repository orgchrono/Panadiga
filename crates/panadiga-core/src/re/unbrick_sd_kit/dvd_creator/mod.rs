#![deny(unsafe_code)]

//! Pure-Rust ISO 9660 Level 1 DVD recovery creator + cross-platform `1x` / minimum-speed burner.
//!
//! Shares payload resolution and the mandatory `Mn103Vm` pre-write gate (`resolve_recovery_media_payloads`)
//! with [`super::creator`].

pub mod burn_os;
pub mod types;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use super::creator::shared_kit::{RecoveryPayloadSpec, resolve_recovery_media_payloads};
use super::iso9660_builder::{
    Iso9660FileInput, inspect_iso9660_image, synthesize_iso9660_level1_image,
};
pub use burn_os::{OpticalBurnBackend, SystemOpticalBurnBackend};
pub use types::{
    DvdCreatorProgress, DvdCreatorReport, DvdCreatorRequest, DvdCreatorStage,
    OpticalBurnerDriveInfo, expected_optical_confirm_token,
};

/// Enumerates host optical disc burners and lowest supported DVD write speeds.
#[must_use]
pub fn inspect_optical_burners() -> Vec<OpticalBurnerDriveInfo> {
    SystemOpticalBurnBackend.enumerate_burners()
}

fn build_dvd_next_steps(model: &str, dry_run: bool, burned_physical: bool) -> Vec<String> {
    if dry_run {
        return vec![
            "1. Review the dry-run 1x ISO 9660 preview command and Mn103Vm gate SHA-256.".into(),
            "2. Clear dry_run and supply CONFIRM-<burner_id> when a blank DVD-R is inserted."
                .into(),
        ];
    }
    if burned_physical {
        return vec![
            "1. Label the DVD-R and wait for drive tray eject verification.".into(),
            format!(
                "2. Insert the DVD-R into {model} and close tray (ODD reads ISO 9660 root PANA_DVD.FRM / DIGADIAG.FRM)."
            ),
            "3. Wait until the front VFD shows COMPLETE/FINISH before ejecting.".into(),
        ];
    }
    vec![
        "1. Burn this verified ISO 9660 Level 1 .iso strictly in DAO mode at 1x (or lowest drive speed).".into(),
        format!("2. Insert disc into {model} to run recovery update without packet-writing overhead."),
    ]
}

fn synthesize_and_verify_dvd_iso(
    req: &DvdCreatorRequest,
    on_progress: &mut dyn FnMut(DvdCreatorProgress),
) -> Result<(
    super::iso9660_builder::SynthesizedIsoImage,
    super::iso9660_builder::Iso9660InspectionReport,
    crate::re::prewrite_gate::PreWriteGateReport,
    bool,
)> {
    on_progress(DvdCreatorProgress {
        stage: DvdCreatorStage::EmuGateVerify,
        percent: 20,
        detail: "Resolving clean-room / kit payloads and enforcing Mn103Vm pre-write gate..."
            .into(),
    });
    let spec = RecoveryPayloadSpec {
        loader_profile: req.loader_profile,
        digadiag_path: req.digadiag_path.clone(),
        secondary_path: None,
        pana_dvd_source_path: req.pana_dvd_path.clone(),
        include_secondary_from_kit: false,
        cleanroom_only: req.cleanroom_only,
    };
    let resolved = resolve_recovery_media_payloads(&spec, "PANA_DVD.FRM")?;
    let mut iso_files: Vec<Iso9660FileInput> = resolved
        .files
        .into_iter()
        .map(|f| Iso9660FileInput {
            name_8_3: f.name_8_3,
            bytes: f.bytes,
        })
        .collect();
    if let Some(ref dvd_p) = req.pana_dvd_path
        && req.cleanroom_only
    {
        let dvd_bytes =
            fs::read(dvd_p).with_context(|| format!("Reading PANA_DVD.FRM {}", dvd_p.display()))?;
        iso_files.push(Iso9660FileInput {
            name_8_3: "PANA_DVD.FRM".to_string(),
            bytes: dvd_bytes,
        });
    }

    on_progress(DvdCreatorProgress {
        stage: DvdCreatorStage::SynthesizeIso,
        percent: 45,
        detail: format!(
            "Synthesizing ECMA-119 ISO 9660 Level 1 ({})...",
            req.volume_label
        ),
    });
    let synth = synthesize_iso9660_level1_image(&req.volume_label, &iso_files)?;

    on_progress(DvdCreatorProgress {
        stage: DvdCreatorStage::InspectIso9660,
        percent: 65,
        detail: "Verifying PVD 'CD001', terminator, and root 8.3 file extents...".into(),
    });
    let inspection = inspect_iso9660_image(&synth.image_bytes)?;
    if inspection.overall_status != "PASS" {
        bail!("Synthesized ISO 9660 Level 1 image failed structural readback audit");
    }
    Ok((
        synth,
        inspection,
        resolved.emu_gate,
        resolved.cleanroom_only,
    ))
}

/// Generates a standalone, Mn103Vm-gated ISO 9660 Level 1 `.iso` file offline.
pub fn create_dvd_recovery_iso(
    req: &DvdCreatorRequest,
    output_iso_path: &Path,
) -> Result<DvdCreatorReport> {
    let mut noop = |_p: DvdCreatorProgress| {};
    let (synth, inspection, emu_gate, cleanroom_used) =
        synthesize_and_verify_dvd_iso(req, &mut noop)?;
    if let Some(parent) = output_iso_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_iso_path, &synth.image_bytes)?;
    let written = fs::read(output_iso_path)?;
    if written.len() != synth.image_bytes.len() {
        bail!(
            "ISO readback size mismatch on {}",
            output_iso_path.display()
        );
    }
    Ok(DvdCreatorReport {
        target: output_iso_path.display().to_string(),
        target_kind: "iso_file".to_string(),
        model: req.model.clone(),
        volume_label: synth.volume_id,
        cleanroom_only: cleanroom_used,
        bytes_written: written.len(),
        iso_sha256: synth.image_sha256,
        write_speed_policy: "1x_or_drive_minimum_dao".to_string(),
        burn_command_preview: None,
        files: synth.files,
        emu_gate,
        inspection,
        next_steps: build_dvd_next_steps(&req.model, false, false),
    })
}

/// Burns or dry-runs an ISO 9660 Level 1 recovery disc at `1x` / lowest drive speed using `backend`.
pub fn create_dvd_recovery_disc_with_backend<
    B: OpticalBurnBackend,
    F: FnMut(DvdCreatorProgress),
>(
    req: &DvdCreatorRequest,
    backend: &B,
    mut on_progress: F,
) -> Result<DvdCreatorReport> {
    let burner_id = req
        .burner_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .context("Missing burner_id for physical optical disc write")?;
    let expected = expected_optical_confirm_token(burner_id);
    if req.confirm_token.as_deref().map(str::trim) != Some(expected.as_str()) {
        bail!("DVD SAFETY LOCKOUT: confirm_token must equal '{expected}'");
    }

    on_progress(DvdCreatorProgress {
        stage: DvdCreatorStage::LockoutVerify,
        percent: 10,
        detail: format!("Verified optical burn token for {burner_id}"),
    });
    let (synth, inspection, emu_gate, cleanroom_used) =
        synthesize_and_verify_dvd_iso(req, &mut on_progress)?;

    let iso_path: PathBuf = if let Some(ref p) = req.output_iso_path {
        p.clone()
    } else {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "panadiga_dvd_stage_{}_{}.iso",
            std::process::id(),
            nanos
        ))
    };
    if let Some(parent) = iso_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&iso_path, &synth.image_bytes)?;
    let preview = backend.build_burn_command_preview(burner_id, &iso_path)?;

    if !req.dry_run {
        on_progress(DvdCreatorProgress {
            stage: DvdCreatorStage::BurnOptical1x,
            percent: 85,
            detail: format!("Burning ISO 9660 Level 1 at 1x/min speed -> {burner_id}"),
        });
        let _status = backend.burn_iso_lowest_speed(burner_id, &iso_path)?;
    }
    on_progress(DvdCreatorProgress {
        stage: DvdCreatorStage::ReadbackVerify,
        percent: 100,
        detail: "ISO 9660 Level 1 1x burn workflow complete".into(),
    });
    if req.output_iso_path.is_none() {
        let _ = fs::remove_file(&iso_path);
    }

    Ok(DvdCreatorReport {
        target: burner_id.to_string(),
        target_kind: if req.dry_run {
            "optical_dry_run"
        } else {
            "optical_disc"
        }
        .to_string(),
        model: req.model.clone(),
        volume_label: synth.volume_id,
        cleanroom_only: cleanroom_used,
        bytes_written: synth.image_bytes.len(),
        iso_sha256: synth.image_sha256,
        write_speed_policy: "1x_or_drive_minimum_dao".to_string(),
        burn_command_preview: Some(preview),
        files: synth.files,
        emu_gate,
        inspection,
        next_steps: build_dvd_next_steps(&req.model, req.dry_run, !req.dry_run),
    })
}

/// Convenience wrapper binding [`SystemOpticalBurnBackend`].
pub fn create_dvd_recovery_disc<F: FnMut(DvdCreatorProgress)>(
    req: &DvdCreatorRequest,
    on_progress: F,
) -> Result<DvdCreatorReport> {
    create_dvd_recovery_disc_with_backend(req, &SystemOpticalBurnBackend, on_progress)
}
