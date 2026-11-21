#![deny(unsafe_code)]

//! Normalized DIGA model resolution engine for [`crate::ProfilesBundle`].

use std::collections::HashSet;

use anyhow::{Context, Result};

use crate::derive::{derived_geometry_label, format_sa_range, sector_range_for_aligned_span};
use crate::gate::CapabilityGate;
use crate::types::{
    CapabilitySummary, FlashChipProfile, NorLayoutProfile, NorRegionKind, ProfilesBundle,
    ResolvedModelProfile, SocProfile,
};

fn find_nor_bus_bases(soc: Option<&SocProfile>) -> (Option<u32>, Option<u32>) {
    let Some(s) = soc else {
        return (None, None);
    };
    let win = s
        .bus_windows
        .iter()
        .find(|w| w.name == "nor_cached" || w.name.contains("nor"));
    match win {
        Some(w) => (Some(w.base), w.uncached_alias_base),
        None => (None, None),
    }
}

fn derive_region_sa(
    chip: Option<&FlashChipProfile>,
    layout: Option<&NorLayoutProfile>,
    kind: NorRegionKind,
) -> Option<String> {
    let (Some(c), Some(l)) = (chip, layout) else {
        return None;
    };
    let reg = l.find_region(kind)?;
    let (sa0, sa1) = sector_range_for_aligned_span(c, reg.start_offset, reg.end_offset).ok()?;
    Some(format_sa_range(sa0, sa1))
}

/// Resolves a model ID or exact alias from `bundle` into its complete normalized tree.
pub fn resolve_model_in_bundle(
    bundle: &ProfilesBundle,
    model_query: &str,
) -> Result<ResolvedModelProfile> {
    let needle = model_query.trim();
    let manifest = bundle
        .models
        .values()
        .find(|m| {
            m.model_id.eq_ignore_ascii_case(needle)
                || m.exact_aliases
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(needle))
        })
        .with_context(|| format!("Unknown DIGA model profile '{model_query}'"))?
        .clone();

    let board = manifest
        .board_ref
        .as_deref()
        .and_then(|id| bundle.boards.get(id))
        .cloned();
    let soc = board
        .as_ref()
        .and_then(|b| b.soc_ref.as_deref())
        .and_then(|id| bundle.socs.get(id))
        .cloned();
    let soc_core = soc
        .as_ref()
        .and_then(|s| bundle.soc_cores.get(&s.core_ref))
        .cloned();
    let flash_chip = board
        .as_ref()
        .and_then(|b| b.flash_ref.as_deref())
        .and_then(|id| bundle.flash_chips.get(id))
        .cloned();
    let eeprom_chips = board
        .as_ref()
        .map(|b| {
            b.eeprom_refs
                .iter()
                .filter_map(|id| bundle.eeprom_chips.get(id).cloned())
                .collect()
        })
        .unwrap_or_default();
    let firmware_train = manifest
        .train_ref
        .as_deref()
        .and_then(|id| bundle.firmware_trains.get(id))
        .cloned();
    let nor_layout = manifest
        .nor_layout_ref
        .as_deref()
        .and_then(|id| bundle.nor_layouts.get(id))
        .cloned();

    let media_formats = firmware_train
        .as_ref()
        .map(|t| {
            let mut seen = HashSet::new();
            t.media_refs
                .iter()
                .filter(|id| seen.insert((*id).clone()))
                .filter_map(|id| bundle.media_formats.get(id).cloned())
                .collect()
        })
        .unwrap_or_default();

    let (nor_cached, nor_uncached) = find_nor_bus_bases(soc.as_ref());
    let eemul = nor_layout
        .as_ref()
        .and_then(|l| l.find_region(NorRegionKind::Eemul));
    let ipl = nor_layout
        .as_ref()
        .and_then(|l| l.find_region(NorRegionKind::Ipl));

    let nvram_cached_va_hex = match (nor_cached, eemul) {
        (Some(base), Some(reg)) => Some(format!("0x{:08X}", base.wrapping_add(reg.start_offset))),
        _ => None,
    };
    let nvram_uncached_va_hex = match (nor_uncached, eemul) {
        (Some(base), Some(reg)) => Some(format!("0x{:08X}", base.wrapping_add(reg.start_offset))),
        _ => None,
    };
    let ipl_protected_range_hex = match (nor_uncached.or(nor_cached), ipl) {
        (Some(base), Some(reg)) => Some(format!(
            "0x{:08X}..0x{:08X}",
            base.wrapping_add(reg.start_offset),
            base.wrapping_add(reg.end_offset)
        )),
        _ => None,
    };

    let bgo_sector_range_derived =
        derive_region_sa(flash_chip.as_ref(), nor_layout.as_ref(), NorRegionKind::Bgo);
    let nvram_sector_range_derived = derive_region_sa(
        flash_chip.as_ref(),
        nor_layout.as_ref(),
        NorRegionKind::Eemul,
    );
    let derived_sector_geometry = flash_chip.as_ref().map(derived_geometry_label);

    let dummy_summary = CapabilitySummary {
        inspect_read_only: true,
        nvram_sanitize_offline_cow: false,
        nvram_write_live: false,
        upper_nor_patch_offline_cow: false,
        upper_nor_write_live: false,
        official_frm_pass_through: false,
        custom_recovery_media: false,
        lab_override_offline_cow_eligible: false,
    };
    let mut resolved = ResolvedModelProfile {
        manifest,
        board,
        soc_core,
        soc,
        flash_chip,
        eeprom_chips,
        firmware_train,
        nor_layout,
        media_formats,
        nvram_cached_va_hex,
        nvram_uncached_va_hex,
        ipl_protected_range_hex,
        bgo_sector_range_derived,
        nvram_sector_range_derived,
        derived_sector_geometry,
        effective_capabilities: dummy_summary,
    };
    resolved.effective_capabilities = CapabilityGate::summarize(&resolved);
    Ok(resolved)
}

/// Resolves every model manifest in deterministic key order.
#[must_use]
pub fn list_resolved_models(bundle: &ProfilesBundle) -> Vec<ResolvedModelProfile> {
    bundle
        .models
        .values()
        .filter_map(|m| resolve_model_in_bundle(bundle, &m.model_id).ok())
        .collect()
}
