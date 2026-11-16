//! Firmware identity detection and TBC (`E-E`) passthrough site constants/patcher.

use std::fs;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::super::bgo_unbrick::ensure_parent_dir;
use super::super::patch_spec::{
    PatchApplyAction, PatchSpec, apply_patch_specs_in_place, classify_patch_site,
};
use super::rsp_plan::format_rsp_write_mem_frame;
use crate::constants::{AM34_MAIN_BASE_VA, EXPECTED_MAIN_3410PQ_SIZE_BYTES};
use crate::nvram::resolve_input_path;

/// Virtual base address where `EH55_MAIN_3410PQ.bin` executes in SDRAM.
pub const MAIN_BASE_VA: u32 = AM34_MAIN_BASE_VA;
/// Exact unpacked byte size of `EH55_MAIN_3410PQ.bin`.
pub const EXPECTED_MAIN_3410PQ_SIZE: usize = EXPECTED_MAIN_3410PQ_SIZE_BYTES;

/// File offset of front-panel status control site (`VA 0x5771ABFB`, 6 bytes).
pub const OFF_FPC_STATUS_CTRL: usize = 0x0071_ABFB;
/// File offset of ODD state branch site (`VA 0x574F7B8C`, 4 bytes).
pub const OFF_ODD_STATE_BRANCH: usize = 0x004F_7B8C;
/// File offset of ODD init status site (`VA 0x574F7C5C`, 7 bytes).
pub const OFF_ODD_INIT_STATUS: usize = 0x004F_7C5C;

pub const ORIG_FPC_STATUS_CTRL: [u8; 6] = [0xCF, 0xE0, 0xF8, 0xFE, 0xD0, 0x82];
pub const PATCH_FPC_STATUS_CTRL: [u8; 6] = [0x80, 0x01, 0xF0, 0xFC, 0xCB, 0xCB];

pub const ORIG_ODD_STATE_BRANCH: [u8; 4] = [0xBF, 0x03, 0xC1, 0x10];
pub const PATCH_ODD_STATE_BRANCH: [u8; 4] = [0xCC, 0xD0, 0x00, 0xCB];

pub const ORIG_ODD_INIT_STATUS: [u8; 7] = [0xDD, 0xCD, 0x3C, 0x00, 0x00, 0x00, 0x00];
pub const PATCH_ODD_INIT_STATUS: [u8; 7] = [0x80, 0x00, 0xCB, 0xCB, 0xCB, 0xCB, 0xCB];

pub(crate) const PASSTHROUGH_SPECS: [PatchSpec<'static>; 3] = [
    PatchSpec {
        name: "fpc_status_ctrl",
        description: "Front-panel status control -> mov 1, d0; ret; nop; nop",
        va: MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32,
        base_va: MAIN_BASE_VA,
        expect: &ORIG_FPC_STATUS_CTRL,
        replace: &PATCH_FPC_STATUS_CTRL,
        required_len: Some(EXPECTED_MAIN_3410PQ_SIZE),
        input_sha256: None,
    },
    PatchSpec {
        name: "odd_state_branch",
        description: "ODD state check branch -> jmp16 +0x00D0 (0x574F7C5C); nop",
        va: MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32,
        base_va: MAIN_BASE_VA,
        expect: &ORIG_ODD_STATE_BRANCH,
        replace: &PATCH_ODD_STATE_BRANCH,
        required_len: Some(EXPECTED_MAIN_3410PQ_SIZE),
        input_sha256: None,
    },
    PatchSpec {
        name: "odd_init_status",
        description: "ODD init return status -> clr d0; nop x5",
        va: MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32,
        base_va: MAIN_BASE_VA,
        expect: &ORIG_ODD_INIT_STATUS,
        replace: &PATCH_ODD_INIT_STATUS,
        required_len: Some(EXPECTED_MAIN_3410PQ_SIZE),
        input_sha256: None,
    },
];

/// Metadata extracted directly from unpacked Panasonic DIGA `MAIN` firmware bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainFirmwareIdentity {
    pub build_id: String,
    pub model_name: Option<String>,
    pub chassis_family: Option<String>,
    pub header_build_name: String,
    pub size_bytes: usize,
    pub supports_3410pq_tbc_patch: bool,
}

fn find_ascii_marker(haystack: &[u8], marker: &[u8]) -> Option<usize> {
    if marker.is_empty() || haystack.len() < marker.len() {
        return None;
    }
    let first = marker[0];
    let mut i = 0;
    let limit = haystack.len() - marker.len();
    while i <= limit {
        if haystack[i] == first && &haystack[i..i + marker.len()] == marker {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn extract_system_version_token(bin: &[u8]) -> Option<String> {
    let marker = b"System Version : ";
    let idx = find_ascii_marker(bin, marker)?;
    let tail = &bin[idx + marker.len()..];
    let tok: String = tail
        .iter()
        .take(16)
        .take_while(|&&b| b.is_ascii_alphanumeric())
        .map(|&b| b as char)
        .collect();
    if tok.is_empty() { None } else { Some(tok) }
}

/// Inspects an unpacked `MAIN` binary directly for true build string, model/chassis markers, & patch compatibility.
pub fn inspect_main_firmware_identity(
    bin: &[u8],
    header_build: Option<&str>,
) -> MainFirmwareIdentity {
    let header_clean = header_build.unwrap_or("").trim().to_string();
    let scanned_build = extract_system_version_token(bin);
    let build_id = scanned_build
        .or_else(|| (!header_clean.is_empty()).then(|| header_clean.clone()))
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let model_name = ["DMR-EH55", "DMR-EH75V", "DMR-EH58"]
        .into_iter()
        .find(|m| find_ascii_marker(bin, m.as_bytes()).is_some())
        .map(str::to_string);
    let chassis_family = ["M6930", "M6931", "M6740"]
        .into_iter()
        .find(|c| find_ascii_marker(bin, c.as_bytes()).is_some())
        .map(str::to_string);

    let supports_3410pq_tbc_patch =
        bin.len() == EXPECTED_MAIN_3410PQ_SIZE && (build_id == "3410PQ" || build_id == "UNKNOWN");

    MainFirmwareIdentity {
        build_id,
        model_name,
        chassis_family,
        header_build_name: header_clean,
        size_bytes: bin.len(),
        supports_3410pq_tbc_patch,
    }
}

/// Supported configuration modes for `MAIN` AV TBC (`E-E`) passthrough sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainPassthroughMode {
    AvPassthrough,
    FactoryOriginal,
}

impl MainPassthroughMode {
    pub fn from_str_mode(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "av_passthrough" | "passthrough" | "ee" | "tbc" => Ok(Self::AvPassthrough),
            "factory_original" | "factory" | "restore" => Ok(Self::FactoryOriginal),
            other => bail!(
                "Unsupported MAIN passthrough mode '{other}': expected av_passthrough or factory_original"
            ),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AvPassthrough => "av_passthrough",
            Self::FactoryOriginal => "factory_original",
        }
    }
}

impl FromStr for MainPassthroughMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_str_mode(s)
    }
}

/// Inspection status and live RSP `$M` write frame for a single `MAIN` site.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainPassthroughSiteReport {
    pub name: String,
    pub description: String,
    pub va_hex: String,
    pub file_offset_hex: String,
    pub length: usize,
    pub state: String,
    pub current_bytes_hex: String,
    pub rsp_passthrough_frame: String,
    pub rsp_factory_frame: String,
}

/// Complete report of `MAIN` AV TBC passthrough sites (used for inspect & patch).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainPassthroughReport {
    pub file_size: usize,
    pub expected_size_match: bool,
    pub detected_mode: String,
    pub sites: Vec<MainPassthroughSiteReport>,
    pub output_path: Option<String>,
    pub applied_mode: Option<String>,
}

/// Inspect the 3 AV TBC passthrough sites inside an unpacked `MAIN` (`3410PQ`, `13,003,052` B) buffer.
pub fn inspect_main_passthrough_sites(buf: &[u8]) -> Result<MainPassthroughReport> {
    let mut sites = Vec::with_capacity(PASSTHROUGH_SPECS.len());
    let mut all_factory = true;
    let mut all_patched = true;

    for spec in &PASSTHROUGH_SPECS {
        let cls = classify_patch_site(buf, spec, "AV_PASSTHROUGH")?;
        if cls.state == "FACTORY" {
            all_patched = false;
        } else if cls.state == "AV_PASSTHROUGH" {
            all_factory = false;
        } else {
            all_factory = false;
            all_patched = false;
        }
        let state_label = if cls.state == "FACTORY" {
            "FACTORY_ORIGINAL"
        } else {
            cls.state
        };

        sites.push(MainPassthroughSiteReport {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            va_hex: format!("0x{:08X}", spec.va),
            file_offset_hex: format!("0x{:06X}", cls.file_offset),
            length: cls.actual_bytes.len(),
            state: state_label.to_string(),
            current_bytes_hex: hex::encode(&cls.actual_bytes),
            rsp_passthrough_frame: format_rsp_write_mem_frame(spec.va, spec.replace),
            rsp_factory_frame: format_rsp_write_mem_frame(spec.va, spec.expect),
        });
    }

    let detected_mode = if all_factory {
        "FACTORY_ORIGINAL"
    } else if all_patched {
        "AV_PASSTHROUGH"
    } else {
        "MIXED_OR_UNKNOWN"
    };

    Ok(MainPassthroughReport {
        file_size: buf.len(),
        expected_size_match: buf.len() == EXPECTED_MAIN_3410PQ_SIZE,
        detected_mode: detected_mode.to_string(),
        sites,
        output_path: None,
        applied_mode: None,
    })
}

/// Apply the requested `MainPassthroughMode` atomically across all 3 verified `3410PQ` sites in `buf`.
pub fn apply_main_passthrough_in_place(
    buf: &mut [u8],
    mode: MainPassthroughMode,
) -> Result<MainPassthroughReport> {
    let action = match mode {
        MainPassthroughMode::AvPassthrough => PatchApplyAction::ApplyReplace,
        MainPassthroughMode::FactoryOriginal => PatchApplyAction::RestoreExpect,
    };
    let ops: Vec<(&PatchSpec<'_>, PatchApplyAction)> =
        PASSTHROUGH_SPECS.iter().map(|s| (s, action)).collect();
    apply_patch_specs_in_place(buf, &ops)?;

    let mut report = inspect_main_passthrough_sites(buf)?;
    report.applied_mode = Some(mode.as_str().to_string());
    Ok(report)
}

/// Read an unpacked `MAIN` binary, apply `mode` non-destructively, and write `output_path`.
pub fn patch_main_passthrough_file(
    input_path: &Path,
    output_path: &Path,
    mode: MainPassthroughMode,
) -> Result<MainPassthroughReport> {
    let resolved = resolve_input_path(input_path)?;
    let mut buf =
        fs::read(&resolved).with_context(|| format!("Failed to read {}", resolved.display()))?;
    let mut report = apply_main_passthrough_in_place(&mut buf, mode)?;
    ensure_parent_dir(output_path)?;
    fs::write(output_path, &buf)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;
    report.output_path = Some(output_path.to_string_lossy().into_owned());
    Ok(report)
}
