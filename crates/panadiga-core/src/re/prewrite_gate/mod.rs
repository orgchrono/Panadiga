#![deny(unsafe_code)]

//! Mandatory Pre-Write `Mn103Vm` Emulator & Safety Gate (`require_prewrite_gate`).
//!
//! No synthesized SD card, composed HDD stream, patched firmware binary, or CoW NOR repair
//! may be written to physical media or committed to `recompiled/` unless this gate returns `passed = true`.

mod checks;
mod checks_emu;
#[cfg(test)]
mod tests;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::nvram::calculate_hashes;
use crate::re::bgo_unbrick::Prog061BgoPatchMode;
use crate::re::unbrick_sd_kit::SdLoaderProfile;

/// Individual deterministic verification check executed by the `Mn103Vm` pre-write gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreWriteCheckItem {
    pub id: String,
    pub passed: bool,
    pub detail: String,
}

/// Complete audit report emitted before any physical SD, HDD, or NOR write operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreWriteGateReport {
    pub passed: bool,
    pub artifact_kind: String,
    pub artifact_sha256: String,
    pub ipl_untouched: bool,
    pub lethal_mmio_blocked: bool,
    pub executed_steps: u64,
    pub checks: Vec<PreWriteCheckItem>,
    pub notes: Vec<String>,
}

/// Artifact classification passed into [`verify_prewrite_gate`].
#[derive(Debug, Clone, Copy)]
pub enum PreWriteTarget<'a> {
    CleanroomDigadiag {
        bytes: &'a [u8],
        loader_profile: SdLoaderProfile,
    },
    PatchedProg061 {
        bytes: &'a [u8],
        mode: Prog061BgoPatchMode,
    },
    PatchedMain {
        raw_main: &'a [u8],
        expect_passthrough: bool,
        expect_capacity_unlock: bool,
    },
    NorCowRepair {
        original_nor: &'a [u8],
        repaired_nor: &'a [u8],
        allow_upper_nor: bool,
    },
    ComposedHddStream {
        total_sectors: u64,
        main_extrhead: Option<&'a [u8]>,
        drv_extrhead: Option<&'a [u8]>,
    },
}

pub(crate) fn push_check(
    checks: &mut Vec<PreWriteCheckItem>,
    id: &str,
    passed: bool,
    detail: impl Into<String>,
) {
    checks.push(PreWriteCheckItem {
        id: id.to_string(),
        passed,
        detail: detail.into(),
    });
}

/// Executes deterministic `Mn103Vm` and structural safety checks for the target artifact.
pub fn verify_prewrite_gate(target: PreWriteTarget<'_>) -> Result<PreWriteGateReport> {
    match target {
        PreWriteTarget::CleanroomDigadiag {
            bytes,
            loader_profile,
        } => {
            let (_, sha) = calculate_hashes(bytes);
            checks_emu::check_cleanroom_digadiag(bytes, loader_profile, sha)
        }
        PreWriteTarget::PatchedProg061 { bytes, mode } => {
            let (_, sha) = calculate_hashes(bytes);
            checks_emu::check_patched_prog061(bytes, mode, sha)
        }
        PreWriteTarget::PatchedMain {
            raw_main,
            expect_passthrough,
            expect_capacity_unlock,
        } => {
            let (_, sha) = calculate_hashes(raw_main);
            checks_emu::check_patched_main(
                raw_main,
                expect_passthrough,
                expect_capacity_unlock,
                sha,
            )
        }
        PreWriteTarget::NorCowRepair {
            original_nor,
            repaired_nor,
            allow_upper_nor,
        } => {
            let (_, sha) = calculate_hashes(repaired_nor);
            checks::check_nor_cow_repair(original_nor, repaired_nor, allow_upper_nor, sha)
        }
        PreWriteTarget::ComposedHddStream {
            total_sectors,
            main_extrhead,
            drv_extrhead,
        } => checks::check_composed_hdd_stream(total_sectors, main_extrhead, drv_extrhead),
    }
}

/// Fail-closed pre-write barrier: returns an error unless [`verify_prewrite_gate`] passes 100%.
pub fn require_prewrite_gate(target: PreWriteTarget<'_>) -> Result<PreWriteGateReport> {
    let report = verify_prewrite_gate(target)?;
    if !report.passed {
        let failed: Vec<String> = report
            .checks
            .iter()
            .filter(|c| !c.passed)
            .map(|c| format!("{}: {}", c.id, c.detail))
            .collect();
        bail!(
            "PRE-WRITE EMULATOR GATE BLOCKED [{}]: {}",
            report.artifact_kind,
            failed.join(" | ")
        );
    }
    Ok(report)
}
