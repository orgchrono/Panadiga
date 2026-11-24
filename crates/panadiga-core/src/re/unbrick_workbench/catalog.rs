//! Pure immutable DTOs, hardware fact catalog, and pure slug/label helpers for unbrick workbench.

use serde::{Deserialize, Serialize};

use crate::hdd::MainCapacityInspectionReport;

use super::super::bgo_unbrick::{Prog061BgoInspection, Prog061BgoPatchMode};
use super::super::main_passthrough::{MainPassthroughReport, MainPassthroughRspPlan};
use super::super::unbrick_sd_kit::SdMediaVerificationReport;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractedFrmModuleReport {
    pub name: String,
    pub offset_hex: String,
    pub encrypted_size: usize,
    pub decrypted_path: String,
    pub decrypted_sha256: String,
    pub unpacked_path: Option<String>,
    pub unpacked_size: Option<usize>,
    pub dst_addr_hex: Option<String>,
    pub checksum_valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrmModuleExtractionReport {
    pub source_frm_path: String,
    pub container_format: String,
    pub output_dir: String,
    pub modules: Vec<ExtractedFrmModuleReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnbrickHardwareFact {
    pub topic: String,
    pub verdict: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkbenchRegionSummary {
    pub dev_bin_path: Option<String>,
    pub dev_bin_model: Option<String>,
    pub dev_bin_oem: Option<String>,
    pub main_build_profile: Option<String>,
    pub main_devinfo_state: Option<String>,
    pub recommended_tv_standard: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkbenchBgoSummary {
    pub nor_dump_path: String,
    pub healthy: bool,
    pub total_sectors: usize,
    pub p1_active_count: usize,
    pub p2_present_count: usize,
    pub p2_missing_count: usize,
    pub spare_count: usize,
    pub corrupt_count: usize,
    pub nfrom_file_count: usize,
    pub ifodev2_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbrickWorkspaceReport {
    pub workspace_root: String,
    pub discovered_pana_dvd: Option<String>,
    pub discovered_prog061: Option<String>,
    pub discovered_main_3410pq: Option<String>,
    pub discovered_drv: Option<String>,
    pub discovered_sd_dir: Option<String>,
    pub prog061_bgo: Option<Prog061BgoInspection>,
    pub main_passthrough: Option<MainPassthroughReport>,
    pub main_capacity: Option<MainCapacityInspectionReport>,
    pub rsp_plan: MainPassthroughRspPlan,
    pub sd_verification: Option<SdMediaVerificationReport>,
    #[serde(default)]
    pub recommended_recovery_profile: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_compat_summary: Option<WorkbenchRegionSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgo_audit_summary: Option<WorkbenchBgoSummary>,
    pub hardware_facts: Vec<UnbrickHardwareFact>,
    pub digadiag_vs_pana_notes: Vec<String>,
}

pub(crate) fn sanitize_module_slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut prev_us = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_us = false;
        } else if !prev_us && !slug.is_empty() {
            slug.push('_');
            prev_us = true;
        }
    }
    if slug.is_empty() {
        "module".to_string()
    } else {
        slug.trim_end_matches('_').to_string()
    }
}

pub(crate) fn bgo_mode_label(mode: Prog061BgoPatchMode) -> &'static str {
    match mode {
        Prog061BgoPatchMode::BypassChecksOnly => "bypass",
        Prog061BgoPatchMode::AutoFormatBgo => "auto_format",
        Prog061BgoPatchMode::UartMonitor => "uart_monitor",
        Prog061BgoPatchMode::FactoryOriginal => "factory",
    }
}

pub(crate) fn hardware_facts_catalog() -> Vec<UnbrickHardwareFact> {
    vec![
        UnbrickHardwareFact {
            topic: "DIGADIAG.FRM vs PANA__SD.FRM / PANA_DVD.FRM".to_string(),
            verdict: "DISTINCT_DIGADIAG_RAM_VS_FRM_UPDATER_CONTAINERS".to_string(),
            evidence: "DMR-EH55 Loader ver0.61 [06/03/06] loads 256 KiB (0x40000) raw DIGADIAG.FRM directly to 0x90400000 (JMP 0x50400000) and expects 48B TOC (0x1800) PANA__SD.FRM (sd48). Later Loader ver0.63 (DMR-EH58 BOOT.BIN) reads 128 KiB (0x20000) plaintext EXTRHEAD DIGADIAG.FRM and 64B TOC PANA__SD.FRM. PANA_DVD.FRM uses 48B Feistel TOC + full Feistel payload.".to_string(),
        },
        UnbrickHardwareFact {
            topic: "Forcing SD 1-bit Mode via Pico P7501 (-91006 Bypass)".to_string(),
            verdict: "NOT_POSSIBLE_OVER_P7501_UART".to_string(),
            evidence: "BOOT.BIN executes from NOR (0x44000000/0x80000000), passes 1st GetSD_Status @ 0x44027826 in 1-bit mode, then switches to 4-bit via ACMD6 @ 0x44027F79 before 2nd GetSD_Status @ 0x44027F93 (-91006). uart0_getc @ 0x4400479F has no active caller and P7501 lacks OCD clock (DCSR=0xFFFFFFFF). Fix via P6801/P6802 DAT1..3 + <=2GB SDSC FAT16 or TSOP-48 NOR patch.".to_string(),
        },
        UnbrickHardwareFact {
            topic: "Standalone Digital PCB TBC Operation Outside Drive".to_string(),
            verdict: "ODD_HDD_REMOVABLE_MAIN_PCB_REQUIRED".to_string(),
            evidence: "Patching MAIN passthrough sites (0x5771ABFB, 0x574F7B8C, 0x574F7C5C — verified strictly for MAIN 3410PQ) lets TBC (E-E) run without ODD/HDD checks (or with CF/SSD via IDE2SAT2). Digital PCB P59001 still requires Main PCB P7402 (schematics na_p49_blk1, vol2_p125) for IC3001 video clamps/75-ohm drivers, IC7501 Timer MCU TBUS handshake, and 8+ power rails.".to_string(),
        },
        UnbrickHardwareFact {
            topic: "CS1 0x84200000 Window & AutoFormatBgo Alias Risk (D4)".to_string(),
            verdict: "BLOCKED_FAIL_CLOSED_IC5001_ALIAS".to_string(),
            evidence: "Both 0x50404592 and 0x50426F78 probe CS1 0x84200000..0x843FFFFF and return Spansion S29JL064H autoselect IDs (0x0001 / 0x227E / 0x2202 / 0x220A). That window maps offsets +0x200000..+0x3FFFFF of IC5001 (8 MiB NOR) inside the read-only IPL range; nfrom_format (0x50401F3C) must stay fail-closed.".to_string(),
        },
        UnbrickHardwareFact {
            topic: "Orphan nfrom_format (0x50401F3C) Indirect Callback Preconditions (D5)".to_string(),
            verdict: "UNINITIALIZED_RAM_PTRS_0x504F8464".to_string(),
            evidence: "Calling nfrom_format (0x50401F3C) invokes 0x5040248E(2, 0) -> 0x50421EBE, which calls indirect RAM pointers 0x504F8464/68/6C before setting CFI handlers 0x504F88DC..0x504F8900 and never writes *(u8*)0x504802B0=1; jumping there directly from 0x5040A2CD without [nfrom] Case 0 init risks wild calls.".to_string(),
        },
    ]
}

pub(crate) fn digadiag_vs_pana_notes() -> Vec<String> {
    vec![
        "DIGADIAG.FRM -> DMR-EH55 Loader ver0.61 reads 256 KiB (0x40000) raw uncompressed payload to 0x90400000 and executes JMP 0x50400000; Loader ver0.63 reads 128 KiB plaintext EXTRHEAD 'FVU ' to 0x93FC0000 and uncompresses to 0x50400000.".to_string(),
        "PANA__SD.FRM -> Loader ver0.61 / PROG061 reads 48B plaintext TOC slots (sd48, 0x1800 header); Loader ver0.63 BOOT.BIN reads 64B plaintext TOC slots (sd64, 0x2000 header).".to_string(),
        "PANA_DVD.FRM -> Optical update container with 48B Feistel TOC + full Feistel module layer over DRV S211 / PROG061 / MAIN3060NP (verified byte-for-byte source of factory sd48 PANA__SD.FRM).".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_slug_and_facts_catalog_are_deterministic() {
        assert_eq!(sanitize_module_slug("DRV S211 "), "drv_s211");
        assert_eq!(sanitize_module_slug("  MAIN3410PQ"), "main3410pq");
        assert_eq!(sanitize_module_slug("---"), "module");
        assert_eq!(
            bgo_mode_label(Prog061BgoPatchMode::BypassChecksOnly),
            "bypass"
        );
        assert_eq!(hardware_facts_catalog().len(), 5);
        assert_eq!(digadiag_vs_pana_notes().len(), 3);
    }
}
