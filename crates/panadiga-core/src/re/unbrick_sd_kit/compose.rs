//! Pure deterministic artifact builders for `DIGADIAG.FRM`, `PANA__SD.FRM`, and `SHA256SUMS`.

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, build_digadiag_frm, pack_extrhead, pack_frm,
};
use crate::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};
use crate::nvram::calculate_hashes;

use super::super::bgo_unbrick::{PROG061_BASE_VA, Prog061BgoPatchMode, patch_prog061_bgo_in_place};
use super::super::main_passthrough::{
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, MainPassthroughMode, apply_main_passthrough_in_place,
    inspect_main_passthrough_sites,
};
use super::super::unbrick_workbench::{load_main_binary_and_meta, load_prog061_from_bytes};
use super::verify::SdMediaVerificationReport;

#[derive(Debug, Clone)]
pub struct SdUnbrickKitRequest {
    pub output_dir: PathBuf,
    pub pana_dvd_path: Option<PathBuf>,
    pub prog061_path: Option<PathBuf>,
    pub main_path: Option<PathBuf>,
    pub drv_path: Option<PathBuf>,
    pub bgo_mode: Prog061BgoPatchMode,
    pub main_passthrough_mode: MainPassthroughMode,
    pub include_pana_sd: bool,
    pub unlock_hdd_dynamic: bool,
    pub hdd_target_gb: Option<u32>,
    pub write_extracted_sources: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdUnbrickKitReport {
    pub output_dir: String,
    pub prog061_source: String,
    pub main_source: Option<String>,
    pub drv_source: Option<String>,
    pub bgo_mode_applied: String,
    pub main_passthrough_applied: Option<String>,
    pub hdd_capacity_applied: Option<String>,
    pub digadiag_path: String,
    pub digadiag_sha256: String,
    pub pana_sd_path: Option<String>,
    pub pana_sd_sha256: Option<String>,
    pub sha256sums_path: String,
    pub verification: SdMediaVerificationReport,
}

pub(crate) struct ComposedMainArtifact {
    pub container: Vec<u8>,
    pub slot_name: &'static str,
    pub passthrough_applied: String,
    pub hdd_capacity_applied: Option<String>,
}

/// Pure composition of patched `PROG061` and its 128 KiB `DIGADIAG.FRM` wrapper.
pub(crate) fn compose_digadiag_and_prog061(
    prog_raw: &[u8],
    bgo_mode: Prog061BgoPatchMode,
) -> Result<(Vec<u8>, Vec<u8>, String)> {
    let mut prog_patched = load_prog061_from_bytes(prog_raw)?;
    patch_prog061_bgo_in_place(&mut prog_patched, bgo_mode)?;
    let (diag_bytes, _) = build_digadiag_frm(
        &prog_patched,
        PROG061_BASE_VA,
        "FVU ",
        "PROG061",
        4,
        PANA_DIGADIAG_FRM_SIZE,
    )?;
    let (_, sha) = calculate_hashes(&diag_bytes);
    Ok((prog_patched, diag_bytes, sha))
}

/// Pure composition of TBC-passthrough + optional HDD-capacity-unlocked `MAIN` container.
pub(crate) fn compose_patched_main_container(
    raw_main: &[u8],
    passthrough_mode: MainPassthroughMode,
    unlock_hdd_dynamic: bool,
    hdd_target_gb: Option<u32>,
) -> Result<ComposedMainArtifact> {
    let (mut main_bin, build_name, flag, adler_step) = load_main_binary_and_meta(raw_main)?;
    let mut modified = false;

    let passthrough_applied = if main_bin.len() == EXPECTED_MAIN_3410PQ_SIZE
        && inspect_main_passthrough_sites(&main_bin).is_ok()
    {
        apply_main_passthrough_in_place(&mut main_bin, passthrough_mode)?;
        modified = true;
        passthrough_mode.as_str().to_string()
    } else {
        "skipped_non_3410pq_build".to_string()
    };

    let mut hdd_capacity_applied = None;
    if unlock_hdd_dynamic || hdd_target_gb.is_some() {
        let cap_mode = if unlock_hdd_dynamic {
            HddCapacityPatchMode::DynamicUnclamped
        } else {
            HddCapacityPatchMode::FixedCapacity
        };
        if apply_capacity_patch_in_place(&mut main_bin, cap_mode, hdd_target_gb).is_ok() {
            hdd_capacity_applied = Some(if unlock_hdd_dynamic {
                "dynamic".to_string()
            } else {
                format!("cap_{}gb", hdd_target_gb.unwrap_or(0))
            });
            modified = true;
        }
    }

    let container = if modified || !raw_main.starts_with(&[0x51, 0x87, 0x13, 0x8D]) {
        pack_extrhead(
            &main_bin,
            "MAIN",
            MAIN_BASE_VA,
            if build_name.is_empty() {
                "3410PQ"
            } else {
                &build_name
            },
            ExtrheadCryptMode::Main,
            adler_step.max(16),
            flag.max(1),
        )?
    } else {
        raw_main.to_vec()
    };

    let slot_name = if build_name.contains("3060") {
        "MAIN3060NP"
    } else {
        "MAIN3410PQ"
    };

    Ok(ComposedMainArtifact {
        container,
        slot_name,
        passthrough_applied,
        hdd_capacity_applied,
    })
}

/// Pure composition of 64-byte-TOC `PANA__SD.FRM` container from constituent firmware payloads.
pub(crate) fn compose_pana_sd_container(
    drv_bytes: Option<Vec<u8>>,
    prog_patched: Vec<u8>,
    main_container: Vec<u8>,
    main_slot: &str,
) -> Result<(Vec<u8>, String)> {
    let mut sd_modules = Vec::new();
    if let Some(drv) = drv_bytes {
        let mut m = FrmPackModule::new("DRV S211", drv);
        m.version_u16 = 0x0211;
        sd_modules.push(m);
    }
    let mut prog_mod = FrmPackModule::new("PROG061 ", prog_patched);
    prog_mod.version_u16 = 0x0061;
    sd_modules.push(prog_mod);

    let mut main_mod = FrmPackModule::new(main_slot, main_container);
    main_mod.version_u16 = 0x3410;
    sd_modules.push(main_mod);

    let (sd_bytes, _) = pack_frm(&sd_modules, FrmContainerMode::Sd, None)?;
    let (_, sd_sha) = calculate_hashes(&sd_bytes);
    Ok((sd_bytes, sd_sha))
}

/// Pure deterministic `SHA256SUMS.txt` text formatter.
pub(crate) fn format_sha256_manifest(entries: &[(String, &str)]) -> String {
    let mut out = String::new();
    for (sha, name) in entries {
        out.push_str(sha);
        out.push_str("  ");
        out.push_str(name);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::verify::verify_sd_buffers;
    use super::*;
    use crate::re::bgo_unbrick::{
        OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, ORIG_A2CD_HOOK,
        ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PROG061_EXPECTED_SIZE,
    };

    #[test]
    fn compose_and_verify_digadiag_and_manifest_in_memory() {
        let mut prog = vec![0xFFu8; PROG061_EXPECTED_SIZE];
        prog[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
        prog[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2].copy_from_slice(&ORIG_DEVINFO_BEQ);
        prog[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20].copy_from_slice(&ORIG_A2CD_HOOK);

        let (_patched, diag, sha) =
            compose_digadiag_and_prog061(&prog, Prog061BgoPatchMode::AutoFormatBgo).unwrap();
        assert_eq!(diag.len(), PANA_DIGADIAG_FRM_SIZE);
        assert_eq!(sha.len(), 64);

        let rep = verify_sd_buffers("/mem/sd", Some(Ok(&diag)), None);
        assert_eq!(rep.overall_status, "READY_WITH_HW_WARNINGS");
        assert_eq!(rep.digadiag_bgo_mode.as_deref(), Some("AUTO_FORMAT_BGO"));

        let sums = format_sha256_manifest(&[(sha, "DIGADIAG.FRM")]);
        assert!(sums.ends_with("  DIGADIAG.FRM\n"));
    }
}
