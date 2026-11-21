#![deny(unsafe_code)]

//! Directory tree loader, deterministic bundle serializer, fail-closed overlay merger,
//! and model resolution engine.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;

use crate::derive::{derived_geometry_label, format_sa_range, sector_range_for_aligned_span};
use crate::gate::CapabilityGate;
use crate::types::{
    BoardProfile, CapabilitySummary, ClaimEvidence, EepromChipProfile, EvidenceTier,
    FirmwareTrainProfile, FlashChipProfile, MediaFormatProfile, ModelProfileManifest,
    NorLayoutProfile, NorRegionKind, ProfilesBundle, ResolvedModelProfile, SocCoreProfile,
    SocProfile,
};
use crate::validate::ensure_bundle_valid;

/// Current normalized manifest bundle schema version.
pub const PROFILES_SCHEMA_VERSION: u32 = 2;

/// Canonical normalized entity subdirectories under `profiles/`.
pub const ENTITY_SUBDIRS: &[&str] = &[
    "soc_cores",
    "socs",
    "flash_chips",
    "eeprom_chips",
    "firmware_trains",
    "nor_layouts",
    "media_formats",
    "boards",
    "models",
];

/// Converts a commercial model ID (`DMR-EH55`) into canonical bundle key (`dmr_eh55`).
#[must_use]
pub fn model_storage_key(model_id: &str) -> String {
    model_id.trim().to_ascii_lowercase().replace('-', "_")
}

fn load_entity_dir<T: DeserializeOwned>(
    root: &Path,
    subdir: &str,
    required: bool,
    key_fn: fn(&T) -> String,
    out: &mut BTreeMap<String, T>,
) -> Result<()> {
    let dir = root.join(subdir);
    if !dir.is_dir() {
        if required {
            bail!("Missing required profiles directory {}", dir.display());
        }
        return Ok(());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("Reading {}", dir.display()))? {
        let p = entry?.path();
        if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("json") {
            files.push(p);
        }
    }
    files.sort();
    for path in files {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Reading profile JSON {}", path.display()))?;
        let item: T = serde_json::from_str(&raw)
            .with_context(|| format!("Parsing {subdir} JSON {}", path.display()))?;
        let key = key_fn(&item);
        if out.insert(key.clone(), item).is_some() {
            bail!(
                "Duplicate identifier '{key}' inside {subdir} ({})",
                path.display()
            );
        }
    }
    Ok(())
}

pub fn empty_bundle() -> ProfilesBundle {
    ProfilesBundle {
        schema_version: PROFILES_SCHEMA_VERSION,
        soc_cores: BTreeMap::new(),
        socs: BTreeMap::new(),
        flash_chips: BTreeMap::new(),
        eeprom_chips: BTreeMap::new(),
        firmware_trains: BTreeMap::new(),
        nor_layouts: BTreeMap::new(),
        media_formats: BTreeMap::new(),
        boards: BTreeMap::new(),
        models: BTreeMap::new(),
    }
}

fn load_tree_raw(root: &Path, required: bool) -> Result<ProfilesBundle> {
    let mut b = empty_bundle();
    load_entity_dir(
        root,
        "soc_cores",
        required,
        |s: &SocCoreProfile| s.id.clone(),
        &mut b.soc_cores,
    )?;
    load_entity_dir(
        root,
        "socs",
        required,
        |s: &SocProfile| s.id.clone(),
        &mut b.socs,
    )?;
    load_entity_dir(
        root,
        "flash_chips",
        required,
        |f: &FlashChipProfile| f.id.clone(),
        &mut b.flash_chips,
    )?;
    load_entity_dir(
        root,
        "eeprom_chips",
        required,
        |e: &EepromChipProfile| e.id.clone(),
        &mut b.eeprom_chips,
    )?;
    load_entity_dir(
        root,
        "firmware_trains",
        required,
        |t: &FirmwareTrainProfile| t.id.clone(),
        &mut b.firmware_trains,
    )?;
    load_entity_dir(
        root,
        "nor_layouts",
        required,
        |l: &NorLayoutProfile| l.id.clone(),
        &mut b.nor_layouts,
    )?;
    load_entity_dir(
        root,
        "media_formats",
        required,
        |m: &MediaFormatProfile| m.id.clone(),
        &mut b.media_formats,
    )?;
    load_entity_dir(
        root,
        "boards",
        required,
        |bd: &BoardProfile| bd.id.clone(),
        &mut b.boards,
    )?;
    load_entity_dir(
        root,
        "models",
        required,
        |m: &ModelProfileManifest| model_storage_key(&m.model_id),
        &mut b.models,
    )?;
    Ok(b)
}

/// Loads and validates a complete v2 `profiles/` directory tree.
pub fn load_bundle_from_dir(root: &Path) -> Result<ProfilesBundle> {
    let bundle = load_tree_raw(root, true)?;
    ensure_bundle_valid(&bundle)?;
    Ok(bundle)
}

/// Loads and validates a pre-compiled `profiles.bundle.json` payload.
pub fn load_bundle_from_str(json: &str) -> Result<ProfilesBundle> {
    let bundle: ProfilesBundle =
        serde_json::from_str(json).context("Parsing embedded ProfilesBundle JSON")?;
    ensure_bundle_valid(&bundle)?;
    Ok(bundle)
}

fn downgrade_evidence(ev: &mut ClaimEvidence) {
    ev.tier = EvidenceTier::CommunityUnverified;
}

fn merge_table<T>(
    base: &mut BTreeMap<String, T>,
    incoming: BTreeMap<String, T>,
    downgrade: fn(&mut T),
    kind: &str,
) -> Result<()> {
    for (k, mut v) in incoming {
        if base.contains_key(&k) {
            bail!(
                "Security violation: runtime overlay attempted to overwrite built-in {kind} '{k}'"
            );
        }
        downgrade(&mut v);
        base.insert(k, v);
    }
    Ok(())
}

/// Safely merges an external runtime overlay directory onto a validated built-in bundle:
/// - rejects overwriting ANY built-in ID or alias;
/// - forcefully downgrades every overlay claim/evidence tier to `T0` (`CommunityUnverified`);
/// - runs complete cross-table validation before returning.
pub fn merge_overlay_dir(mut base: ProfilesBundle, overlay_root: &Path) -> Result<ProfilesBundle> {
    let overlay = load_tree_raw(overlay_root, false)?;
    merge_table(
        &mut base.soc_cores,
        overlay.soc_cores,
        |v| downgrade_evidence(&mut v.evidence),
        "soc_core",
    )?;
    merge_table(
        &mut base.socs,
        overlay.socs,
        |v| downgrade_evidence(&mut v.evidence),
        "soc",
    )?;
    merge_table(
        &mut base.flash_chips,
        overlay.flash_chips,
        |v| downgrade_evidence(&mut v.evidence),
        "flash_chip",
    )?;
    merge_table(
        &mut base.eeprom_chips,
        overlay.eeprom_chips,
        |v| downgrade_evidence(&mut v.evidence),
        "eeprom_chip",
    )?;
    merge_table(
        &mut base.firmware_trains,
        overlay.firmware_trains,
        |v| downgrade_evidence(&mut v.evidence),
        "firmware_train",
    )?;
    merge_table(
        &mut base.nor_layouts,
        overlay.nor_layouts,
        |v| downgrade_evidence(&mut v.evidence),
        "nor_layout",
    )?;
    merge_table(
        &mut base.media_formats,
        overlay.media_formats,
        |v| downgrade_evidence(&mut v.evidence),
        "media_format",
    )?;
    merge_table(
        &mut base.boards,
        overlay.boards,
        |v| downgrade_evidence(&mut v.evidence),
        "board",
    )?;
    merge_table(
        &mut base.models,
        overlay.models,
        |m| {
            downgrade_evidence(&mut m.claims.board);
            downgrade_evidence(&mut m.claims.flash_chip);
            downgrade_evidence(&mut m.claims.nor_layout);
            downgrade_evidence(&mut m.claims.train);
            downgrade_evidence(&mut m.claims.eeprom);
        },
        "model",
    )?;
    ensure_bundle_valid(&base)?;
    Ok(base)
}

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
