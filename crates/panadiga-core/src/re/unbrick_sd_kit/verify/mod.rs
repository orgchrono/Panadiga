//! Pure immutable inspection & algebraic validation of SD unbrick media (`DIGADIAG.FRM` + `PANA__SD.FRM`).

pub mod media;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::constants::{
    AM34_CS1_NOR_OFFSET_MASK, BGO_TABLE_WORD_IDX_FLASH_END, CHK_ID_BGO_ALLOWLIST_INVARIANT,
    CHK_ID_BGO_GEOMETRY_MISMATCH, CHK_ID_DIGADIAG_BYPASS_TRAP, CHK_ID_FVU_PANAMON_TRAP,
    CHK_ID_MAIN_BGO_GEOMETRY, CHK_ID_MAIN_EXTRHEAD_CHECKSUM, CHK_ID_MAIN_EXTRHEAD_SRC_ADDR,
    CHK_ID_RTSD_LBA48_GATE, CHK_ID_RTSD_TARGET_CONSISTENCY, CHK_ID_SATA_BRIDGE_RESET_FIX,
    CHK_ID_SATA_BRIDGE_STAGE2_BOOT, LBA28_MAX_BLOCKS_2KB, PANA_EXTRHEAD_MAIN_SRC_ADDR,
    SD_STATUS_INVALID, SD_STATUS_READY, SD_STATUS_READY_WITH_WARNINGS,
};

use super::super::bgo_unbrick::{find_train_by_loader_key, validate_bgo_allowlist_invariant};
use super::super::main_bgo_retarget::{
    MAIN_BGO_STATE_8MB, MAIN_BGO_STATE_FACTORY_4MB, MAIN_BGO_STATE_MISMATCH,
};
use super::super::sata_bridge_fix::{SATA_BRIDGE_STATE_MISMATCH, SATA_BRIDGE_STATE_WAIT_1000MS};
use super::super::sata_bridge_stage2::{
    PROG061_STAGE2_STATE_ENABLED, PROG061_STAGE2_STATE_MISMATCH,
};
use super::compose::SdLoaderProfile;

pub(crate) use self::media::{DigadiagInspection, verify_digadiag_buffer, verify_pana_sd_buffer};
pub use self::media::{
    DualFamilyParityReport, DualFamilySlotResolution, SdKitAuditProfile,
    simulate_sd_find_firm_entry_dual,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdUnbrickCheck {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdMediaVerificationReport {
    pub sd_root_dir: String,
    pub overall_status: String,
    pub digadiag_present: bool,
    pub digadiag_exact_size_ok: bool,
    pub digadiag_dst_addr_hex: Option<String>,
    pub digadiag_bgo_mode: Option<String>,
    pub pana_sd_present: bool,
    pub pana_sd_toc_entries: usize,
    pub pana_sd_list_sums_ok: bool,
    pub pana_sd_prog061_mode: Option<String>,
    pub pana_sd_main_build: Option<String>,
    pub pana_sd_main_passthrough_mode: Option<String>,
    pub pana_sd_main_hdd_dynamic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digadiag_bgo_geometry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pana_sd_prog061_bgo_geometry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pana_sd_main_hdd_capacity_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pana_sd_prog061_sata_bridge_fix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pana_sd_prog061_stage2_boot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pana_sd_main_sata_bridge_fix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pana_sd_main_bgo_geometry: Option<String>,
    #[serde(default)]
    pub factory_unpatched_prog061_detected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dual_family_parity: Option<DualFamilyParityReport>,
    pub checks: Vec<SdUnbrickCheck>,
}

fn format_main_label(main_build: Option<&str>) -> String {
    match main_build.map(str::trim).filter(|s| !s.is_empty()) {
        Some(b) if b.to_ascii_uppercase().starts_with("MAIN") => b.to_ascii_uppercase(),
        Some(b) => format!("MAIN{}", b.to_ascii_uppercase()),
        None => "MAIN".to_string(),
    }
}

fn push_sata_bridge_fix_check(
    checks: &mut Vec<SdUnbrickCheck>,
    sd_present: bool,
    main_build: Option<&str>,
    prog_state: Option<&str>,
    main_state: Option<&str>,
    audit_profile: SdKitAuditProfile,
) {
    let main_build_present = main_build.is_some();
    let main_label = format_main_label(main_build);
    if prog_state == Some(SATA_BRIDGE_STATE_MISMATCH)
        || main_state == Some(SATA_BRIDGE_STATE_MISMATCH)
    {
        checks.push(check(
            CHK_ID_SATA_BRIDGE_RESET_FIX,
            "fail",
            "Corrupt/Unexpected Bytes at SATA-Bridge Hard-Reset Timing Site",
            format!(
                "PANA__SD.FRM SATA-bridge timing site failed fail-closed match (prog061={prog_state:?}, {main_label}={main_state:?})."
            ),
        ));
        return;
    }

    let prog_patched = prog_state == Some(SATA_BRIDGE_STATE_WAIT_1000MS);
    let main_patched = main_state == Some(SATA_BRIDGE_STATE_WAIT_1000MS);
    if prog_patched && main_patched {
        checks.push(check(
            CHK_ID_SATA_BRIDGE_RESET_FIX,
            "pass",
            "SATA-PATA Bridge Reset Wait Fix Verified (1000 ms)",
            format!(
                "Both PANA__SD.FRM PROG061 and {main_label} apply verified 1000 ms post-RESET- wait (100 x 10 ms RTOS ticks)."
            ),
        ));
    } else if (prog_patched && (main_build_present || main_state.is_some()))
        || (main_patched && sd_present)
    {
        let sev = match audit_profile {
            SdKitAuditProfile::HardenedRecovery => "fail",
            SdKitAuditProfile::Standard => "warn",
        };
        checks.push(check(
            CHK_ID_SATA_BRIDGE_RESET_FIX,
            sev,
            "Partial SATA-PATA Bridge Reset Wait Fix (PROG061 vs MAIN Mismatch)",
            format!(
                "Partial SataBridgeResetFix in PANA__SD.FRM (prog061={prog_state:?}, {main_label}={main_state:?}); both PROG061 FVU and {main_label} must use 1000 ms or factory 2 ms consistently."
            ),
        ));
    }
}

fn push_stage2_boot_check(
    checks: &mut Vec<SdUnbrickCheck>,
    stage2_state: Option<&str>,
    prog_reset_state: Option<&str>,
    main_reset_state: Option<&str>,
    audit_profile: SdKitAuditProfile,
) {
    match stage2_state {
        Some(PROG061_STAGE2_STATE_MISMATCH) => {
            checks.push(check(
                CHK_ID_SATA_BRIDGE_STAGE2_BOOT,
                "fail",
                "Corrupt/Mismatched PROG061 Stage-2 HDD Boot Hook or Payload",
                "PANA__SD.FRM PROG061 hook 0x504050D3 and tail payload 0x50439600 failed fail-closed consistency.",
            ));
        }
        Some(PROG061_STAGE2_STATE_ENABLED) => {
            let prog_reset_ok = prog_reset_state == Some(SATA_BRIDGE_STATE_WAIT_1000MS);
            let main_reset_ok = main_reset_state == Some(SATA_BRIDGE_STATE_WAIT_1000MS);
            if prog_reset_ok && main_reset_ok {
                checks.push(check(
                    CHK_ID_SATA_BRIDGE_STAGE2_BOOT,
                    "pass",
                    "PROG061 Stage-2 SATA HDD Boot Hook Verified (Pre-FVU)",
                    "PANA__SD.FRM PROG061 embeds verified Stage-2 HDD boot hook at 0x504050D3 -> 0x50439600 paired with 1000 ms SataBridgeResetFix and escape hatches.",
                ));
            } else if !prog_reset_ok {
                checks.push(check(
                    CHK_ID_SATA_BRIDGE_STAGE2_BOOT,
                    "fail",
                    "PROG061 Stage-2 Hook Present Without 1000 ms SataBridgeResetFix",
                    format!(
                        "Stage-2 HDD boot hook requires PROG061 1000 ms SATA-bridge reset wait (prog_reset={prog_reset_state:?}, main_reset={main_reset_state:?})."
                    ),
                ));
            } else {
                let sev = match audit_profile {
                    SdKitAuditProfile::HardenedRecovery => "fail",
                    SdKitAuditProfile::Standard => "warn",
                };
                checks.push(check(
                    CHK_ID_SATA_BRIDGE_STAGE2_BOOT,
                    sev,
                    "PROG061 Stage-2 Paired With Unpatched MAIN SATA-Bridge Wait",
                    format!(
                        "PROG061 has Stage-2 + 1000 ms wait, but MAIN SATA-bridge timing is {main_reset_state:?} (expected SATA_BRIDGE_1000MS)."
                    ),
                ));
            }
        }
        _ => {}
    }
}

fn push_main_bgo_geometry_check(
    checks: &mut Vec<SdUnbrickCheck>,
    main_build: Option<&str>,
    main_bgo_geo: Option<&str>,
    loader_profile: Option<SdLoaderProfile>,
    stage2_state: Option<&str>,
) {
    let Some(geo) = main_bgo_geo else {
        return;
    };
    let main_label = format_main_label(main_build);
    let is_ver061 = loader_profile == Some(SdLoaderProfile::Ver061RawSd48);
    let is_stage2 = stage2_state == Some(PROG061_STAGE2_STATE_ENABLED);
    match geo {
        MAIN_BGO_STATE_8MB => checks.push(check(
            CHK_ID_MAIN_BGO_GEOMETRY,
            "pass",
            format!("{main_label} BGO/NFROM NOR Geometry Verified (8 MiB 0x84130000, 45 Sectors)"),
            format!(
                "PANA__SD.FRM {main_label} targets 8 MiB Uniform BGO (0x84130000..0x84400000, 45 sectors)."
            ),
        )),
        MAIN_BGO_STATE_MISMATCH => checks.push(check(
            CHK_ID_MAIN_BGO_GEOMETRY,
            "fail",
            format!("Corrupt/Partial {main_label} BGO/NFROM NOR Geometry Sites"),
            format!(
                "PANA__SD.FRM {main_label} BGO verification failed fail-closed validation (mixed 4 MiB/8 MiB tables)."
            ),
        )),
        MAIN_BGO_STATE_FACTORY_4MB => {
            let (sev, title, detail) = if is_ver061 && is_stage2 {
                (
                    "fail",
                    format!("Stock 4 MiB {main_label} BGO Geometry Incompatible With ver061 Stage-2 (8 MiB EH55)"),
                    format!(
                        "Stage-2 SATA HDD boot on 8 MiB DMR-EH55 NOR halts in '[nfrom] BGO driver init failed[1].' (src/nfd_api.c:228) unless {main_label} is retargeted via --main-bgo-retarget."
                    ),
                )
            } else if is_ver061 {
                (
                    "warn",
                    format!("Factory 4 MiB {main_label} BGO Geometry Detected (Requires 8 MiB Retarget for EH55 NOR)"),
                    format!(
                        "PANA__SD.FRM {main_label} targets 4 MiB Top-Boot BGO (0x84200000, 35 sectors); pass --main-bgo-retarget when targeting 8 MiB DMR-EH55 NOR."
                    ),
                )
            } else {
                (
                    "pass",
                    format!("Factory 4 MiB {main_label} BGO Geometry Matches Non-ver061 Target"),
                    format!(
                        "PANA__SD.FRM {main_label} uses 4 MiB Top-Boot BGO geometry (0x84200000, 35 sectors)."
                    ),
                )
            };
            checks.push(check(CHK_ID_MAIN_BGO_GEOMETRY, sev, title, detail));
        }
        other => checks.push(check(
            CHK_ID_MAIN_BGO_GEOMETRY,
            "fail",
            format!("Unexpected {main_label} BGO/NFROM Geometry State"),
            format!("Unexpected MAIN BGO geometry state '{other}' in PANA__SD.FRM."),
        )),
    }
}

pub(crate) fn check(
    id: &str,
    severity: &str,
    title: impl Into<String>,
    detail: impl Into<String>,
) -> SdUnbrickCheck {
    SdUnbrickCheck {
        id: id.to_string(),
        severity: severity.to_string(),
        title: title.into(),
        detail: detail.into(),
    }
}

/// Pure function verifying in-memory `DIGADIAG.FRM` and optional `PANA__SD.FRM` buffers
/// under [`SdKitAuditProfile::Standard`].
pub fn verify_sd_buffers(
    sd_root_label: &str,
    digadiag_opt: Option<Result<&[u8], &str>>,
    pana_sd_opt: Option<&[u8]>,
) -> SdMediaVerificationReport {
    verify_sd_buffers_with_profile(
        sd_root_label,
        digadiag_opt,
        pana_sd_opt,
        SdKitAuditProfile::Standard,
    )
}

/// Pure function verifying in-memory `DIGADIAG.FRM` and optional `PANA__SD.FRM` buffers
/// under the selected [`SdKitAuditProfile`].
pub fn verify_sd_buffers_with_profile(
    sd_root_label: &str,
    digadiag_opt: Option<Result<&[u8], &str>>,
    pana_sd_opt: Option<&[u8]>,
    audit_profile: SdKitAuditProfile,
) -> SdMediaVerificationReport {
    let diag: DigadiagInspection = verify_digadiag_buffer(digadiag_opt);
    let sd = verify_pana_sd_buffer(pana_sd_opt);
    let factory_unpatched_prog061_detected =
        diag.factory_unpatched_prog061 || sd.factory_unpatched_prog061;
    let mut checks = vec![diag.check, sd.check];

    if let (Some(diag_prof), Some(sd_prof)) = (diag.detected_profile, sd.detected_profile)
        && diag_prof != sd_prof
    {
        checks.push(check(
            "loader_profile_mismatch",
            "fail",
            "Incompatible Loader Profiles (DIGADIAG.FRM vs PANA__SD.FRM)",
            format!(
                "DIGADIAG.FRM uses {} while PANA__SD.FRM uses {}; both must match Loader ver0.61 (256 KiB raw + sd48) or ver0.63 (128 KiB EXTRHEAD + sd64).",
                diag_prof.describe(),
                sd_prof.describe(),
            ),
        ));
    }

    if factory_unpatched_prog061_detected {
        let sev = match audit_profile {
            SdKitAuditProfile::HardenedRecovery => "fail",
            SdKitAuditProfile::Standard => "warn",
        };
        checks.push(check(
            "factory_unpatched_prog061",
            sev,
            "FactoryUnpatchedProg061Detected ([05/11/01] without BGO bypass / PANAMON1)",
            "Unpatched stock PROG061 [05/11/01] stalls in 'BGO driver init failed[1].' on corrupt NOR; harden with Prog061BgoPatchMode::UartMonitor (PANAMON1).",
        ));
    }

    let dual_family_parity = if audit_profile == SdKitAuditProfile::HardenedRecovery {
        if diag.present && diag.detected_profile != Some(SdLoaderProfile::Ver061RawSd48) {
            checks.push(check(
                "hardened_ver061_required",
                "fail",
                "Hardened recovery requires Loader ver0.61 256 KiB raw DIGADIAG.FRM",
                "Expected 262,144 B raw DIGADIAG.FRM for DMR-EH55 Loader ver0.61.",
            ));
        }
        pana_sd_opt.map(|sd_bytes| {
            let parity = simulate_sd_find_firm_entry_dual(sd_bytes);
            let parity_chk = if parity.dual_parity_ok {
                check(
                    "sd48_dual_family_parity",
                    "pass",
                    "sd_find_firm_entry Dual-Family Parity (M6930LB + M6930)",
                    "Both M6930LB and M6930 resolve PROG061 and MAIN in sd48 TOC with [0x18]==0x4D and valid FVU sums.",
                )
            } else {
                check(
                    "sd48_dual_family_parity",
                    "fail",
                    "sd_find_firm_entry Dual-Family Parity Failed",
                    parity.issues.join("; "),
                )
            };
            checks.push(parity_chk);
            parity
        })
    } else {
        None
    };

    let is_ver063_detected = diag.detected_profile == Some(SdLoaderProfile::Ver063ExtrheadSd64)
        || sd.detected_profile == Some(SdLoaderProfile::Ver063ExtrheadSd64);
    let hw_warn = if is_ver063_detected {
        check(
            "sd_hw_91006",
            "warn",
            "DMR-EH55 Loader ver0.61 Compatibility & SD-91006 Hardware Gate",
            "Kit uses Loader ver0.63 (128 KiB EXTRHEAD / sd64). Real DMR-EH55 Loader ver0.61 [06/03/06] rejects 128 KiB EXTRHEAD with 'SD diag load failed[0].' and requires ver061 (262,144 B raw + sd48) on <=2 GiB FAT16 SDSC with intact DAT1..DAT3.",
        )
    } else {
        check(
            "sd_hw_91006",
            "warn",
            "Loader ver0.61 & SD-91006 Hardware Gate (1-bit -> 4-bit ACMD6)",
            "Requires legacy SDSC card (<= 2 GiB, FAT16/FAT12) AND intact SD DAT1..DAT3 traces (P6801/P6802 <-> P53501). Bootloader has no UART RX console so Pico cannot force 1-bit mode over P7501.",
        )
    };
    checks.push(hw_warn);

    let active_train = diag
        .detected_profile
        .and_then(|p| find_train_by_loader_key(p.as_str()))
        .or_else(|| {
            sd.detected_profile
                .and_then(|p| find_train_by_loader_key(p.as_str()))
        });
    if let Some(train) = active_train
        && let Some(spec) = train.bgo_retarget_spec.as_ref()
    {
        match validate_bgo_allowlist_invariant(train, spec) {
            Ok(()) => {
                let start_off = spec.detect_flash_base & AM34_CS1_NOR_OFFSET_MASK;
                let end_incl = spec
                    .bgo_table_words
                    .get(BGO_TABLE_WORD_IDX_FLASH_END)
                    .map_or(start_off, |&w| {
                        (w & AM34_CS1_NOR_OFFSET_MASK).saturating_sub(1)
                    });
                checks.push(check(
                    CHK_ID_BGO_ALLOWLIST_INVARIANT,
                    "pass",
                    "BGO Positive Allowlist & Physical Sector Alignment Verified",
                    format!(
                        "SSOT BGO span [0x{start_off:06X}..=0x{end_incl:06X}] is strictly contained inside '{}' BGO allowlist, sector-aligned, and disjoint from IPL/DRV/EEMUL.",
                        train.id
                    ),
                ));
            }
            Err(err) => {
                checks.push(check(
                    CHK_ID_BGO_ALLOWLIST_INVARIANT,
                    "fail",
                    "BGO Positive Allowlist Invariant Failed",
                    err.to_string(),
                ));
            }
        }

        let expected_geo = spec.target_geometry_id.as_str();
        let mismatches: Vec<String> = [
            ("DIGADIAG.FRM", diag.bgo_geometry.as_deref()),
            ("PANA__SD.FRM:PROG061", sd.prog061_bgo_geometry.as_deref()),
        ]
        .into_iter()
        .filter_map(|(src, geo_opt)| {
            geo_opt
                .filter(|&geo| geo != expected_geo)
                .map(|geo| format!("{src}='{geo}'"))
        })
        .collect();
        if !mismatches.is_empty() {
            let sev = match audit_profile {
                SdKitAuditProfile::HardenedRecovery => "fail",
                SdKitAuditProfile::Standard => "warn",
            };
            checks.push(check(
                CHK_ID_BGO_GEOMETRY_MISMATCH,
                sev,
                "BGO Target Geometry Mismatch for Loader ver0.61 (DMR-EH55 8 MiB NOR)",
                format!(
                    "Expected SSOT geometry '{expected_geo}' for 8 MiB Spansion S29JL064H (0x84130000..0x84400000, 45 sectors), found {}. Using un-retargeted EH50 4 MiB geometry risks U99/SA54-SA70 corruption.",
                    mismatches.join(", "),
                ),
            ));
        }
    }

    if sd.detected_profile == Some(SdLoaderProfile::Ver061RawSd48)
        && sd.prog061_mode.as_deref() == Some("UART_MONITOR_PANAMON1")
    {
        let sev = match audit_profile {
            SdKitAuditProfile::HardenedRecovery => "fail",
            SdKitAuditProfile::Standard => "warn",
        };
        checks.push(check(
            CHK_ID_FVU_PANAMON_TRAP,
            sev,
            "PANA__SD.FRM Contains UART_MONITOR_PANAMON1 (Halts Instead of Flashing MAIN)",
            "In Loader ver0.61 normal SD update mode, PROG061 inside PANA__SD.FRM must use BYPASS_CHECKS_ONLY so FVU proceeds to flash MAIN onto HDD; UART_MONITOR_PANAMON1 will halt at PANAMON1> instead.",
        ));
    }

    if diag.detected_profile == Some(SdLoaderProfile::Ver061RawSd48)
        && diag.bgo_mode.as_deref() == Some("BYPASS_CHECKS_ONLY")
        && (audit_profile == SdKitAuditProfile::HardenedRecovery || sd.present)
    {
        let sev = match audit_profile {
            SdKitAuditProfile::HardenedRecovery => "fail",
            SdKitAuditProfile::Standard => "warn",
        };
        checks.push(check(
            CHK_ID_DIGADIAG_BYPASS_TRAP,
            sev,
            "DIGADIAG.FRM Uses BYPASS_CHECKS_ONLY Instead of Interactive PANAMON1 Monitor",
            "JIG diagnostic boot DIGADIAG.FRM expects UART_MONITOR_PANAMON1 (or bare-metal PANAMON2); BYPASS_CHECKS_ONLY will attempt FVU installation instead of dropping to serial prompt.",
        ));
    }

    if sd.main_extrhead_checksum_ok == Some(false) {
        checks.push(check(
            CHK_ID_MAIN_EXTRHEAD_CHECKSUM,
            "fail",
            "PANA__SD.FRM MAIN EXTRHEAD Checksum/Footer Corrupted",
            "MAIN EXTRHEAD header, Adler-32 strided checksum, or EXTRFOOT trailer failed validation.",
        ));
    }

    if let Some(src_addr) = sd.main_extrhead_src_addr {
        let valid_src = src_addr == PANA_EXTRHEAD_MAIN_SRC_ADDR;
        checks.push(check(
            CHK_ID_MAIN_EXTRHEAD_SRC_ADDR,
            if valid_src { "pass" } else { "warn" },
            if valid_src {
                "MAIN EXTRHEAD Staging Base Valid (0x94000000)"
            } else {
                "MAIN EXTRHEAD Staging Base (+0x1C) Is Null/Non-Standard"
            },
            format!(
                "PANA__SD.FRM MAIN EXTRHEAD+0x1C src_addr={src_addr:#010X} (expected {PANA_EXTRHEAD_MAIN_SRC_ADDR:#010X} for direct Loader/Stage-2 HDD boot)."
            ),
        ));
    }

    if let Some(cap_label) = sd.main_hdd_capacity_label.as_deref() {
        let dynamic = sd.main_hdd_dynamic.unwrap_or(false);
        checks.push(check(
            CHK_ID_RTSD_TARGET_CONSISTENCY,
            "pass",
            format!("MAIN HDD Capacity Policy Inspected ({cap_label})"),
            format!("MAIN src/hdd_cmd.c RTSD limit resolves to '{cap_label}' (dynamic_unclamped={dynamic})."),
        ));

        let exceeds_lba28 = dynamic
            || sd
                .main_effective_blocks_2kb
                .is_some_and(|b| b > LBA28_MAX_BLOCKS_2KB);
        if exceeds_lba28 {
            let is_known_lba48_build = sd.detected_profile == Some(SdLoaderProfile::Ver061RawSd48)
                || sd.main_build.as_deref().is_some_and(|b| {
                    let up = b.to_ascii_uppercase();
                    up.contains("3410") || up.contains("3060")
                });
            let sev = if is_known_lba48_build {
                "pass"
            } else if audit_profile == SdKitAuditProfile::HardenedRecovery {
                "fail"
            } else {
                "warn"
            };
            checks.push(check(
                CHK_ID_RTSD_LBA48_GATE,
                sev,
                format!("MAIN >128 GiB LBA48 Addressing Boundary Check ({cap_label})"),
                format!(
                    "Target exceeds LBA28 ceiling ({LBA28_MAX_BLOCKS_2KB:#010X} 2KiB blocks = 128 GiB); LBA48 path verified={} for build '{}'.",
                    is_known_lba48_build,
                    sd.main_build.as_deref().unwrap_or("unknown"),
                ),
            ));
        }
    }

    push_sata_bridge_fix_check(
        &mut checks,
        sd.present,
        sd.main_build.as_deref(),
        sd.prog061_sata_bridge_fix.as_deref(),
        sd.main_sata_bridge_fix.as_deref(),
        audit_profile,
    );
    push_stage2_boot_check(
        &mut checks,
        sd.prog061_stage2_boot.as_deref(),
        sd.prog061_sata_bridge_fix.as_deref(),
        sd.main_sata_bridge_fix.as_deref(),
        audit_profile,
    );
    push_main_bgo_geometry_check(
        &mut checks,
        sd.main_build.as_deref(),
        sd.main_bgo_geometry.as_deref(),
        sd.detected_profile.or(diag.detected_profile),
        sd.prog061_stage2_boot.as_deref(),
    );

    let overall_status = if checks.iter().any(|c| c.severity == "fail") {
        SD_STATUS_INVALID
    } else if checks.iter().any(|c| c.severity == "warn") {
        SD_STATUS_READY_WITH_WARNINGS
    } else {
        SD_STATUS_READY
    }
    .to_string();

    SdMediaVerificationReport {
        sd_root_dir: sd_root_label.to_string(),
        overall_status,
        digadiag_present: diag.present,
        digadiag_exact_size_ok: diag.exact_size_ok,
        digadiag_dst_addr_hex: diag.dst_addr_hex,
        digadiag_bgo_mode: diag.bgo_mode,
        pana_sd_present: sd.present,
        pana_sd_toc_entries: sd.toc_entries,
        pana_sd_list_sums_ok: sd.list_sums_ok,
        pana_sd_prog061_mode: sd.prog061_mode,
        pana_sd_main_build: sd.main_build,
        pana_sd_main_passthrough_mode: sd.main_passthrough_mode,
        pana_sd_main_hdd_dynamic: sd.main_hdd_dynamic,
        digadiag_bgo_geometry: diag.bgo_geometry,
        pana_sd_prog061_bgo_geometry: sd.prog061_bgo_geometry,
        pana_sd_main_hdd_capacity_label: sd.main_hdd_capacity_label,
        pana_sd_prog061_sata_bridge_fix: sd.prog061_sata_bridge_fix,
        pana_sd_prog061_stage2_boot: sd.prog061_stage2_boot,
        pana_sd_main_sata_bridge_fix: sd.main_sata_bridge_fix,
        pana_sd_main_bgo_geometry: sd.main_bgo_geometry,
        factory_unpatched_prog061_detected,
        dual_family_parity,
        checks,
    }
}
