#![deny(unsafe_code)]

//! Dual-family `Loader ver0.61` `sd_find_firm_entry` (`0x4400C0BA`) parity validation,
//! verification strictness profiles, and shared SD container checksum helpers.

use std::str::FromStr;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::constants::PANA_FRM_TOC_ENTRY_SIZE;
use crate::crypto::codec::compute_fvu_byte_sum;
use crate::crypto::{DecryptedFrmTocEntry, TocLayout, parse_sd_frm_toc_with_layout};

pub(super) const SD48_FAMILY_PRIMARY: &str = "M6930LB";
pub(super) const SD48_FAMILY_ALIAS: &str = "M6930";
const SD48_TARGET_MODEL_PREFIX_M: u8 = b'M';
pub(super) const FACTORY_PROG061_DATE_MARKER: &[u8; 8] = b"05/11/01";

/// Verification strictness profile for SD recovery media (`Standard` vs `HardenedRecovery`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdKitAuditProfile {
    #[default]
    Standard,
    HardenedRecovery,
}

impl FromStr for SdKitAuditProfile {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        let hardened = super::super::super::parse_sd_hardened_profile(
            raw,
            "audit profile",
            &["hardened", "hardened_recovery", "hardened_panamon1"],
        )?;
        Ok(if hardened {
            Self::HardenedRecovery
        } else {
            Self::Standard
        })
    }
}

/// Resolution details for a single `sd_find_firm_entry` (`0x4400C0BA`) lookup in a 48B `sd48` TOC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DualFamilySlotResolution {
    pub hardware_family: String,
    pub tag_prefix: String,
    pub slot_index: usize,
    pub model_prefix_byte_hex: String,
    pub model_prefix_is_m: bool,
    pub list_sum_valid: bool,
    pub fvu_sum_valid: bool,
    pub module_offset: u32,
    pub module_size: u32,
    pub fvu_checksum_u32: u32,
}

/// Dual-family `Loader ver0.61` `sd_find_firm_entry` parity report (`M6930LB` + `M6930`, `[0x18] == 0x4D`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DualFamilyParityReport {
    pub is_sd48_layout: bool,
    pub m6930lb_prog061: Option<DualFamilySlotResolution>,
    pub m6930lb_main: Option<DualFamilySlotResolution>,
    pub m6930_prog061: Option<DualFamilySlotResolution>,
    pub m6930_main: Option<DualFamilySlotResolution>,
    pub m6930lb_drv: Option<DualFamilySlotResolution>,
    pub m6930_drv: Option<DualFamilySlotResolution>,
    pub dual_parity_ok: bool,
    pub issues: Vec<String>,
}

pub(crate) fn is_factory_unpatched_prog061(buf: &[u8], bgo_mode: Option<&str>) -> bool {
    if bgo_mode == Some("FACTORY_ORIGINAL") {
        return true;
    }
    if matches!(
        bgo_mode,
        Some("UART_MONITOR_PANAMON1" | "BYPASS_CHECKS_ONLY" | "AUTO_FORMAT_BGO")
    ) {
        return false;
    }
    buf.windows(FACTORY_PROG061_DATE_MARKER.len())
        .any(|win| win == FACTORY_PROG061_DATE_MARKER)
}

pub(super) fn slice_module_and_check_fvu<'a>(
    sd_bytes: &'a [u8],
    entry: &DecryptedFrmTocEntry,
) -> Option<(&'a [u8], bool)> {
    let start = entry.module_offset as usize;
    let end = start.checked_add(entry.module_size as usize)?;
    let mod_slice = sd_bytes.get(start..end)?;
    let fvu_ok = compute_fvu_byte_sum(mod_slice) == entry.fvu_checksum_u32;
    Some((mod_slice, fvu_ok))
}

fn resolve_sd48_firm_entry(
    toc_bytes: &[u8],
    entries: &[DecryptedFrmTocEntry],
    family: &str,
    tag_prefix: &str,
) -> Option<DualFamilySlotResolution> {
    let tag_upper = tag_prefix.trim().to_ascii_uppercase();
    let entry = entries.iter().find(|e| {
        e.hardware_family.trim().eq_ignore_ascii_case(family)
            && e.module_name
                .trim()
                .to_ascii_uppercase()
                .starts_with(&tag_upper)
    })?;
    let prefix_byte = entry
        .index
        .checked_mul(PANA_FRM_TOC_ENTRY_SIZE)
        .and_then(|off| toc_bytes.get(off + 0x18))
        .copied()
        .unwrap_or(0);
    let fvu_sum_valid = slice_module_and_check_fvu(toc_bytes, entry).is_some_and(|(_, ok)| ok);
    Some(DualFamilySlotResolution {
        hardware_family: family.to_string(),
        tag_prefix: tag_prefix.to_string(),
        slot_index: entry.index,
        model_prefix_byte_hex: format!("0x{prefix_byte:02X}"),
        model_prefix_is_m: prefix_byte == SD48_TARGET_MODEL_PREFIX_M,
        list_sum_valid: entry.list_sum_valid,
        fvu_sum_valid,
        module_offset: entry.module_offset,
        module_size: entry.module_size,
        fvu_checksum_u32: entry.fvu_checksum_u32,
    })
}

fn check_required_slot(
    slot: Option<&DualFamilySlotResolution>,
    family: &str,
    tag: &str,
    issues: &mut Vec<String>,
) -> bool {
    let Some(res) = slot else {
        issues.push(format!(
            "Missing sd48 TOC entry for hardware_family='{family}' tag='{tag}'"
        ));
        return false;
    };
    let mut ok = true;
    if !res.model_prefix_is_m {
        issues.push(format!(
            "Slot {} ({family}/{tag}) has [0x18]={} (expected 0x4D 'M')",
            res.slot_index, res.model_prefix_byte_hex
        ));
        ok = false;
    }
    if !res.list_sum_valid {
        issues.push(format!(
            "Slot {} ({family}/{tag}) failed 16-bit TOC list_sum",
            res.slot_index
        ));
        ok = false;
    }
    if !res.fvu_sum_valid {
        issues.push(format!(
            "Slot {} ({family}/{tag}) failed 32-bit FVU payload sum or bounds check",
            res.slot_index
        ));
        ok = false;
    }
    ok
}

/// Simulates `Loader ver0.61` `sd_find_firm_entry` (`0x4400C0BA`) across BOTH `M6930LB` and `M6930`.
///
/// Requires `Sd48B` layout where both families resolve `PROG061` and `MAIN` with `[0x18] == 0x4D`
/// (`'M'`), valid 16-bit TOC `list_sum`, and valid 32-bit `compute_fvu_byte_sum` over payload bytes.
pub fn simulate_sd_find_firm_entry_dual(toc_bytes: &[u8]) -> DualFamilyParityReport {
    let (layout, entries) = parse_sd_frm_toc_with_layout(toc_bytes);
    let is_sd48_layout = layout == Some(TocLayout::Sd48B);
    let mut issues = Vec::new();
    if !is_sd48_layout {
        issues.push(format!(
            "Expected Sd48B (48B plaintext TOC) for Loader ver0.61 parity, detected {:?}",
            layout
        ));
        return DualFamilyParityReport {
            is_sd48_layout: false,
            m6930lb_prog061: None,
            m6930lb_main: None,
            m6930_prog061: None,
            m6930_main: None,
            m6930lb_drv: None,
            m6930_drv: None,
            dual_parity_ok: false,
            issues,
        };
    }

    let m6930lb_prog061 = resolve_sd48_firm_entry(toc_bytes, &entries, SD48_FAMILY_PRIMARY, "PROG");
    let m6930lb_main = resolve_sd48_firm_entry(toc_bytes, &entries, SD48_FAMILY_PRIMARY, "MAIN");
    let m6930_prog061 = resolve_sd48_firm_entry(toc_bytes, &entries, SD48_FAMILY_ALIAS, "PROG");
    let m6930_main = resolve_sd48_firm_entry(toc_bytes, &entries, SD48_FAMILY_ALIAS, "MAIN");
    let m6930lb_drv = resolve_sd48_firm_entry(toc_bytes, &entries, SD48_FAMILY_PRIMARY, "DRV");
    let m6930_drv = resolve_sd48_firm_entry(toc_bytes, &entries, SD48_FAMILY_ALIAS, "DRV");

    let mut all_ok = true;
    all_ok &= check_required_slot(
        m6930lb_prog061.as_ref(),
        SD48_FAMILY_PRIMARY,
        "PROG",
        &mut issues,
    );
    all_ok &= check_required_slot(
        m6930lb_main.as_ref(),
        SD48_FAMILY_PRIMARY,
        "MAIN",
        &mut issues,
    );
    all_ok &= check_required_slot(
        m6930_prog061.as_ref(),
        SD48_FAMILY_ALIAS,
        "PROG",
        &mut issues,
    );
    all_ok &= check_required_slot(m6930_main.as_ref(), SD48_FAMILY_ALIAS, "MAIN", &mut issues);

    for (family, drv_opt) in [
        (SD48_FAMILY_PRIMARY, m6930lb_drv.as_ref()),
        (SD48_FAMILY_ALIAS, m6930_drv.as_ref()),
    ] {
        if drv_opt.is_some() {
            all_ok &= check_required_slot(drv_opt, family, "DRV", &mut issues);
        }
    }

    DualFamilyParityReport {
        is_sd48_layout,
        m6930lb_prog061,
        m6930lb_main,
        m6930_prog061,
        m6930_main,
        m6930lb_drv,
        m6930_drv,
        dual_parity_ok: all_ok,
        issues,
    }
}
