//! Pure immutable inspection & algebraic validation of SD unbrick media (`DIGADIAG.FRM` + `PANA__SD.FRM`).

pub mod media;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use super::super::bgo_unbrick::find_retarget_spec_by_loader_key;
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
    #[serde(default)]
    pub factory_unpatched_prog061_detected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dual_family_parity: Option<DualFamilyParityReport>,
    pub checks: Vec<SdUnbrickCheck>,
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

    let retarget_spec = diag
        .detected_profile
        .and_then(|p| find_retarget_spec_by_loader_key(p.as_str()))
        .or_else(|| {
            sd.detected_profile
                .and_then(|p| find_retarget_spec_by_loader_key(p.as_str()))
        });
    if let Some(spec) = retarget_spec {
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
                "bgo_geometry_mismatch",
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
        checks.push(check(
            "fvu_panamon_trap",
            "warn",
            "PANA__SD.FRM Contains UART_MONITOR_PANAMON1 (Halts Instead of Flashing MAIN)",
            "In Loader ver0.61 normal SD update mode, PROG061 inside PANA__SD.FRM must use BYPASS_CHECKS_ONLY so FVU proceeds to flash MAIN onto HDD; UART_MONITOR_PANAMON1 will halt at PANAMON1> instead.",
        ));
    }

    if let Some(cap_label) = sd.main_hdd_capacity_label.as_deref() {
        checks.push(check(
            "rtsd_target_consistency",
            "pass",
            format!("MAIN HDD Capacity Policy Inspected ({cap_label})"),
            format!(
                "MAIN src/hdd_cmd.c RTSD limit resolves to '{cap_label}' (dynamic_unclamped={}).",
                sd.main_hdd_dynamic.unwrap_or(false),
            ),
        ));
    }

    let overall_status = if checks.iter().any(|c| c.severity == "fail") {
        "INCOMPLETE_OR_INVALID"
    } else if checks.iter().any(|c| c.severity == "warn") {
        "READY_WITH_HW_WARNINGS"
    } else {
        "READY"
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
        factory_unpatched_prog061_detected,
        dual_family_parity,
        checks,
    }
}
