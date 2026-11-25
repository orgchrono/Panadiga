#![deny(unsafe_code)]

//! Fail-closed validator for declarative `DigaOS` SSOT configurations (`DigaOsConfigSpec`).
//!
//! Enforces silicon voltage limits (<= 3300 mV LVTTL), mandatory `lethal_mmio_guard` and
//! `ipl_ro_guard` invariants, strict blocklists against AM34 lethal MMIO ranges and SA0..SA10
//! critical boot IPL regions, and fail-closed `UNVERIFIED_MMIO` isolation across non-AM34 SoCs.

use std::collections::HashSet;

use anyhow::{Result, bail};

use crate::types::DigaOsConfigSpec;
use crate::validate::REQUIRED_LVTTL_MV;

const FORBIDDEN_WRITE_RANGES: &[(u32, u32, &str)] = &[
    (0xC000_100C, 0xC000_100F, "AM34 lethal WDT/bus register"),
    (0xDC0F_0000, 0xDC0F_0FFF, "AM34 SYSCON lethal aperture"),
    (0x8000_0000, 0x8003_FFFF, "Spansion SA0..SA10 IPL RO cached"),
    (
        0x8400_0000,
        0x8403_FFFF,
        "Spansion SA0..SA10 IPL RO uncached",
    ),
];

fn ranges_overlap(start: u32, end: u32, lo: u32, hi: u32) -> bool {
    start <= hi && end >= lo
}

fn validate_allowed_mmio_ranges(
    prof_id: &str,
    ranges: &[crate::types::DigaOsMmioRangeSpec],
) -> Result<()> {
    for r in ranges {
        if r.size == 0 {
            bail!(
                "{prof_id}: allowed_mmio_writes entry at 0x{:08X} must have size > 0",
                r.start
            );
        }
        let Some(end) = r.start.checked_add(r.size - 1) else {
            bail!(
                "{prof_id}: allowed_mmio_writes range 0x{:08X}+0x{:X} overflows u32",
                r.start,
                r.size
            );
        };
        for &(lo, hi, label) in FORBIDDEN_WRITE_RANGES {
            if ranges_overlap(r.start, end, lo, hi) {
                bail!(
                    "{prof_id}: allowed_mmio_writes 0x{:08X}..0x{:08X} overlaps {label} (0x{lo:08X}..0x{hi:08X})",
                    r.start,
                    end
                );
            }
        }
    }
    Ok(())
}

/// Validates a declarative [`DigaOsConfigSpec`] and fails closed on any physical, schema, or SoC isolation violation.
pub fn validate_digaos_config(cfg: &DigaOsConfigSpec) -> Result<()> {
    if cfg.prompt != "DIGAOS>" {
        bail!(
            "DigaOsConfigSpec.prompt must be 'DIGAOS>' (got '{}')",
            cfg.prompt
        );
    }
    if cfg.menus.is_empty()
        || cfg.themes.is_empty()
        || cfg.power_profiles.is_empty()
        || cfg.soc_targets.is_empty()
    {
        bail!(
            "DigaOsConfigSpec must define non-empty menus, themes, power_profiles, and soc_targets"
        );
    }

    let mut seen_menu_ids = HashSet::new();
    let mut seen_item_ids = HashSet::new();
    for menu in &cfg.menus {
        if menu.id.trim().is_empty() || !seen_menu_ids.insert(menu.id.as_str()) {
            bail!("Invalid or duplicate DigaOS menu id '{}'", menu.id);
        }
        if menu.items.is_empty() {
            bail!("DigaOS menu '{}' must define at least one item", menu.id);
        }
        for item in &menu.items {
            if item.id.trim().is_empty() || !seen_item_ids.insert(item.id.as_str()) {
                bail!("Invalid or duplicate DigaOS menu item id '{}'", item.id);
            }
            if item.command.trim().is_empty() {
                bail!("DigaOS menu item '{}' has empty command", item.id);
            }
        }
    }

    let mut seen_themes = HashSet::new();
    for theme in &cfg.themes {
        if theme.id.trim().is_empty() || !seen_themes.insert(theme.id.as_str()) {
            bail!("Invalid or duplicate DigaOS theme id '{}'", theme.id);
        }
        if theme.contrast_level > 100 || theme.overscan_px > 64 {
            bail!(
                "DigaOS theme '{}' bounds invalid (contrast={}, overscan={})",
                theme.id,
                theme.contrast_level,
                theme.overscan_px
            );
        }
    }

    let mut soc_cores = HashSet::new();
    let mut primary_count = 0usize;
    for target in &cfg.soc_targets {
        if target.soc_core_id.trim().is_empty() || !soc_cores.insert(target.soc_core_id.as_str()) {
            bail!(
                "Invalid or duplicate DigaOS soc_core_id '{}'",
                target.soc_core_id
            );
        }
        if target.logic_voltage_mv == 0 || target.logic_voltage_mv > REQUIRED_LVTTL_MV {
            bail!(
                "DigaOS target '{}' logic_voltage_mv={} exceeds {} mV LVTTL invariant",
                target.soc_core_id,
                target.logic_voltage_mv,
                REQUIRED_LVTTL_MV
            );
        }
        if target.is_primary_executable && !target.mmio_verified {
            bail!(
                "DigaOS target '{}' cannot mark is_primary_executable=true when mmio_verified=false (UNVERIFIED_MMIO)",
                target.soc_core_id
            );
        }
        if target.is_primary_executable {
            primary_count += 1;
        }
    }
    if primary_count != 1 {
        bail!(
            "DigaOS config must declare exactly 1 primary executable SoC target (found {primary_count})"
        );
    }

    let mut seen_profiles = HashSet::new();
    for prof in &cfg.power_profiles {
        if prof.id.trim().is_empty() || !seen_profiles.insert(prof.id.as_str()) {
            bail!("Invalid or duplicate DigaOS power profile id '{}'", prof.id);
        }
        if !soc_cores.contains(prof.soc_core_id.as_str()) {
            bail!(
                "DigaOS power profile '{}' references unknown soc_core_id '{}'",
                prof.id,
                prof.soc_core_id
            );
        }
        if !prof.lethal_mmio_guard {
            bail!(
                "DigaOS power profile '{}': lethal_mmio_guard must be true",
                prof.id
            );
        }
        if !prof.ipl_ro_guard {
            bail!(
                "DigaOS power profile '{}': ipl_ro_guard must be true",
                prof.id
            );
        }
        if prof.uart_baud == 0 {
            bail!("DigaOS power profile '{}' uart_baud must be > 0", prof.id);
        }
        validate_allowed_mmio_ranges(&prof.id, &prof.allowed_mmio_writes)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_checked_in_default_digaos_config_and_rejects_lethal_boundaries() {
        let cfg_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../profiles/digaos/default_digaos_config.json");
        let raw = std::fs::read_to_string(&cfg_path).expect("read default_digaos_config.json");
        let cfg: DigaOsConfigSpec =
            serde_json::from_str(&raw).expect("parse default_digaos_config.json");
        validate_digaos_config(&cfg).expect("default DigaOS config must pass validation");

        let mut bad = cfg.clone();
        bad.power_profiles[0].lethal_mmio_guard = false;
        assert!(validate_digaos_config(&bad).is_err());

        let mut bad_mmio = cfg.clone();
        bad_mmio.power_profiles[0]
            .allowed_mmio_writes
            .push(crate::types::DigaOsMmioRangeSpec {
                start: 0xDC0F_0000,
                size: 4,
                note: "forbidden".to_string(),
            });
        assert!(validate_digaos_config(&bad_mmio).is_err());
    }
}
