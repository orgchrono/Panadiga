#![deny(unsafe_code)]

//! Shared payload resolution and mandatory `Mn103Vm` pre-write gate helper
//! used identically by FAT16B `sd_creator` and ISO 9660 `dvd_creator` (`0` duplicated logic).

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use super::super::cleanroom_digadiag::{CLEANROOM_DIGADIAG_MAGIC, build_cleanroom_digadiag};
use super::super::compose::{SdLoaderProfile, SdUnbrickKitRequest};
use super::super::fat16_builder::Fat16SdFileInput;
use super::super::io::prepare_sd_unbrick_kit;
use crate::constants::PANA_DIGADIAG_RAW_VER061_SIZE;
use crate::re::bgo_unbrick::{Prog061BgoPatchMode, inspect_prog061_bgo_sites};
use crate::re::prewrite_gate::{PreWriteGateReport, PreWriteTarget, require_prewrite_gate};

/// Shared input specification for resolving `DIAG_PROG_RAM` / `PANA__SD` DIGA recovery payloads.
#[derive(Debug, Clone)]
pub struct RecoveryPayloadSpec {
    pub loader_profile: SdLoaderProfile,
    pub digadiag_path: Option<PathBuf>,
    pub secondary_path: Option<PathBuf>,
    pub pana_dvd_source_path: Option<PathBuf>,
    pub include_secondary_from_kit: bool,
    pub cleanroom_only: bool,
}

/// Verified payloads + `Mn103Vm` pre-write gate report ready for SD FAT16B or DVD ISO 9660 synthesis.
#[derive(Debug, Clone)]
pub struct ResolvedRecoveryMedia {
    pub files: Vec<Fat16SdFileInput>,
    pub diag_source: String,
    pub secondary_source: Option<String>,
    pub emu_gate: PreWriteGateReport,
    pub cleanroom_only: bool,
}

pub(crate) fn run_digadiag_prewrite_gate(
    diag_bytes: &[u8],
    loader_profile: SdLoaderProfile,
) -> Result<PreWriteGateReport> {
    let is_cleanroom_raw = diag_bytes.len() == PANA_DIGADIAG_RAW_VER061_SIZE
        && diag_bytes.get(0x10..0x10 + CLEANROOM_DIGADIAG_MAGIC.len())
            == Some(CLEANROOM_DIGADIAG_MAGIC);
    if is_cleanroom_raw || diag_bytes.starts_with(b"EXTRHEAD") {
        return require_prewrite_gate(PreWriteTarget::CleanroomDigadiag {
            bytes: diag_bytes,
            loader_profile,
        });
    }
    let site_mode = inspect_prog061_bgo_sites(diag_bytes)
        .map(|rep| match rep.detected_mode.as_str() {
            "UART_MONITOR_PANAMON1" => Prog061BgoPatchMode::UartMonitor,
            "BYPASS_CHECKS_ONLY" => Prog061BgoPatchMode::BypassChecksOnly,
            _ => Prog061BgoPatchMode::FactoryOriginal,
        })
        .unwrap_or(Prog061BgoPatchMode::FactoryOriginal);
    require_prewrite_gate(PreWriteTarget::PatchedProg061 {
        bytes: diag_bytes,
        mode: site_mode,
    })
}

fn assemble_media_files(
    diag_bytes: Vec<u8>,
    secondary_filename: &str,
    secondary_bytes: Option<Vec<u8>>,
) -> Vec<Fat16SdFileInput> {
    let mut files = vec![Fat16SdFileInput {
        name_8_3: "DIGADIAG.FRM".to_string(),
        bytes: diag_bytes,
    }];
    if let Some(bytes) = secondary_bytes {
        files.push(Fat16SdFileInput {
            name_8_3: secondary_filename.to_string(),
            bytes,
        });
    }
    files
}

fn read_optional_secondary(
    path_opt: Option<&std::path::Path>,
    secondary_filename: &str,
) -> Result<(Option<Vec<u8>>, Option<String>)> {
    let bytes = path_opt
        .map(|p| {
            fs::read(p).with_context(|| format!("Reading {secondary_filename} {}", p.display()))
        })
        .transpose()?;
    let label = path_opt.map(|p| p.display().to_string());
    Ok((bytes, label))
}

/// Resolves clean-room or user-supplied DIGA recovery payloads and enforces `require_prewrite_gate`.
pub fn resolve_recovery_media_payloads(
    spec: &RecoveryPayloadSpec,
    secondary_filename: &str,
) -> Result<ResolvedRecoveryMedia> {
    let use_cleanroom_default = spec.digadiag_path.is_none()
        && spec.secondary_path.is_none()
        && (spec.cleanroom_only
            || (!spec.include_secondary_from_kit && spec.pana_dvd_source_path.is_none()));

    if let Some(ref diag_p) = spec.digadiag_path {
        let diag_bytes = fs::read(diag_p)
            .with_context(|| format!("Reading DIGADIAG.FRM {}", diag_p.display()))?;
        let emu_gate = run_digadiag_prewrite_gate(&diag_bytes, spec.loader_profile)?;
        let (sec_bytes, sec_label) =
            read_optional_secondary(spec.secondary_path.as_deref(), secondary_filename)?;
        return Ok(ResolvedRecoveryMedia {
            files: assemble_media_files(diag_bytes, secondary_filename, sec_bytes),
            diag_source: diag_p.display().to_string(),
            secondary_source: sec_label,
            emu_gate,
            cleanroom_only: false,
        });
    }

    if use_cleanroom_default {
        let art = build_cleanroom_digadiag(spec.loader_profile)?;
        let (sec_bytes, sec_label) =
            read_optional_secondary(spec.secondary_path.as_deref(), secondary_filename)?;
        return Ok(ResolvedRecoveryMedia {
            files: assemble_media_files(art.bytes, secondary_filename, sec_bytes),
            diag_source: format!(
                "cleanroom:{} ({})",
                spec.loader_profile.as_str(),
                art.sha256
            ),
            secondary_source: sec_label,
            emu_gate: art.emu_gate,
            cleanroom_only: true,
        });
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let stage_dir = std::env::temp_dir().join(format!(
        "panadiga_media_stage_{}_{}",
        std::process::id(),
        nanos
    ));
    let mut kit_req = SdUnbrickKitRequest::hardened_panamon1(stage_dir.clone());
    kit_req.pana_dvd_path = spec.pana_dvd_source_path.clone();
    kit_req.loader_profile = spec.loader_profile;
    kit_req.include_pana_sd = spec.include_secondary_from_kit;
    let kit_rep = prepare_sd_unbrick_kit(&kit_req)?;
    let diag_bytes = fs::read(&kit_rep.digadiag_path)?;
    let emu_gate = run_digadiag_prewrite_gate(&diag_bytes, spec.loader_profile)?;
    let sec_bytes = if let Some(ref sp) = spec.secondary_path {
        Some(fs::read(sp)?)
    } else {
        kit_rep.pana_sd_path.as_ref().map(fs::read).transpose()?
    };
    let sec_label = spec
        .secondary_path
        .as_ref()
        .map(|p| p.display().to_string())
        .or(kit_rep.pana_sd_path);
    let _ = fs::remove_dir_all(&stage_dir);

    Ok(ResolvedRecoveryMedia {
        files: assemble_media_files(diag_bytes, secondary_filename, sec_bytes),
        diag_source: kit_rep.digadiag_path,
        secondary_source: sec_label,
        emu_gate,
        cleanroom_only: false,
    })
}
