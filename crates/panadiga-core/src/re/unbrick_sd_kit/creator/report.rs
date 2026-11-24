#![deny(unsafe_code)]

//! Progress DTOs, creator reports, and layperson step-by-step recovery instructions.

use serde::{Deserialize, Serialize};

use super::super::fat16_builder::Fat16PlacedFileReport;
use super::super::inspect_sd::SdDiskInspectionReport;
use crate::re::prewrite_gate::PreWriteGateReport;

/// Discrete stage emitted during SD recovery card synthesis and flashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdCreatorStage {
    Synthesize,
    LockoutVerify,
    DeleteLayout,
    WritePartition,
    ReadbackVerify,
    InspectFat16,
}

/// Real-time progress update yielded to desktop Tauri IPC or CLI handlers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdCreatorProgress {
    pub stage: SdCreatorStage,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub detail: String,
}

/// Card capacity compatibility advisory flagging SDXC `>= 64 GiB` media.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdCardAdvisory {
    pub is_sdxc_warning: bool,
    pub card_class: String,
    pub recommendation: String,
    pub detail: String,
}

/// Complete verified outcome of creating a deterministic FAT16B (`0x06`) SD recovery card or `.img`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdCardCreatorReport {
    pub target: String,
    pub target_kind: String,
    pub model: String,
    pub volume_mb: u32,
    pub cleanroom_only: bool,
    pub bytes_written: usize,
    pub image_sha256: String,
    pub advisory: SdCardAdvisory,
    pub files: Vec<Fat16PlacedFileReport>,
    pub emu_gate: PreWriteGateReport,
    pub readback_verified: bool,
    pub inspection: SdDiskInspectionReport,
    pub next_steps: Vec<String>,
}

/// Generates clear step-by-step instructions tailored for non-specialist hardware recovery operators.
#[must_use]
pub fn build_layperson_sd_steps(
    model: &str,
    volume_mb: u32,
    advisory: &SdCardAdvisory,
    has_pana_sd: bool,
) -> Vec<String> {
    let mut steps = Vec::new();
    if advisory.is_sdxc_warning {
        steps.push(format!("WARNING (SDXC): {}", advisory.detail));
    }
    steps.push(format!(
        "1. Safely eject the SD card from your PC without opening it in Explorer/Finder (prevents OS metadata folders in the clean {volume_mb} MiB FAT16B root)."
    ));
    steps.push(format!(
        "2. Unplug the Panasonic {model} AC mains cord and wait at least 30 seconds until the display is completely dark."
    ));
    let fw_desc = if has_pana_sd {
        "DIGADIAG.FRM + PANA__SD.FRM recovery stage"
    } else {
        "DIGADIAG.FRM diagnostic RAM loader"
    };
    steps.push(format!(
        "3. Insert this SD card into the front SD slot and reconnect AC power; SDDIAG should automatically boot ({fw_desc})."
    ));
    steps.push(
        "4. Wait until the front VFD display shows END/BYE or UART P7501 reports completion before removing power."
            .to_string(),
    );
    steps.push(
        "5. Remove the SD card, power-cycle AC mains once more, and verify normal boot without U99."
            .to_string(),
    );
    steps
}
