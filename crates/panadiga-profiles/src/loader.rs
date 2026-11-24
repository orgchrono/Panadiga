#![deny(unsafe_code)]

//! Directory tree loader, deterministic bundle serializer, fail-closed overlay merger,
//! and model resolution engine.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;

pub use crate::loader_resolve::{list_resolved_models, resolve_model_in_bundle};
use crate::types::{
    BoardProfile, ClaimEvidence, EepromChipProfile, EvidenceTier, FirmwareTrainProfile,
    FlashChipProfile, MediaFormatProfile, ModelProfileManifest, NorLayoutProfile, ProfilesBundle,
    SocCoreProfile, SocProfile,
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
        blobs: BTreeMap::new(),
        traps: None,
    }
}

fn validate_blob_relpath(relpath: &str) -> Result<()> {
    if !relpath.starts_with("blobs/")
        || relpath.starts_with('/')
        || relpath.starts_with('\\')
        || relpath.contains('\\')
        || relpath
            .split('/')
            .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        bail!(
            "Security violation: invalid nfrom_drv_blob_relpath '{relpath}' (must stay strictly under blobs/ without traversal)"
        );
    }
    Ok(())
}

fn load_referenced_blobs(root: &Path, bundle: &mut ProfilesBundle, required: bool) -> Result<()> {
    let relpaths: Vec<String> = bundle
        .firmware_trains
        .values()
        .filter_map(|t| t.bgo_retarget_spec.as_ref())
        .map(|s| s.nfrom_drv_blob_relpath.clone())
        .collect();
    for rel in relpaths {
        validate_blob_relpath(&rel)?;
        let full = root.join(&rel);
        if full.is_file() {
            let bytes = fs::read(&full)
                .with_context(|| format!("Reading referenced profile blob {}", full.display()))?;
            let encoded = crate::blob_b64::encode_base64(&bytes);
            if let Some(prev) = bundle.blobs.get(&rel)
                && prev != &encoded
            {
                bail!("Conflicting blob contents for '{rel}'");
            }
            bundle.blobs.insert(rel, encoded);
        } else if required {
            bail!("Missing referenced profile blob {}", full.display());
        }
    }
    Ok(())
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
    load_referenced_blobs(root, &mut b, required)?;
    b.traps = crate::i18n::load_optional_traps_from_dir(root)?;
    Ok(b)
}

/// Loads and validates a complete v2 `profiles/` directory tree.
pub fn load_bundle_from_dir(root: &Path) -> Result<ProfilesBundle> {
    let bundle = load_tree_raw(root, true)?;
    ensure_bundle_valid(&bundle)?;
    if root.join("locales").is_dir() {
        let locales = crate::i18n::load_locales_from_dir(root)?;
        if let Err(failures) = crate::i18n::validate_locale_catalog(&locales, &bundle) {
            let summary: Vec<String> = failures.into_iter().map(|f| f.to_string()).collect();
            bail!("SSOT locale validation failed: {}", summary.join("; "));
        }
    }
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
    for (k, v) in overlay.blobs {
        if base.blobs.contains_key(&k) {
            bail!("Security violation: runtime overlay attempted to overwrite built-in blob '{k}'");
        }
        base.blobs.insert(k, v);
    }
    ensure_bundle_valid(&base)?;
    Ok(base)
}
