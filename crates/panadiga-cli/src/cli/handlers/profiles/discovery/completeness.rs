#![deny(unsafe_code)]

//! Per-collection and per-tier SSOT completeness reporting for `panadiga profiles completeness`.

use std::collections::BTreeMap;

use panadiga_core::ssot::{
    BoardProfile, EepromChipProfile, EvidenceTier, FirmwareTrainProfile, FlashChipProfile,
    ModelClaims, ModelProfileManifest, ProfilesBundle, SocProfile,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(in super::super) struct CollectionCompletenessSummary {
    pub total: usize,
    pub missing_fields: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(in super::super) struct ModelMissingEntry {
    pub model_id: String,
    pub max_tier: String,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(in super::super) struct ProfilesCompletenessReport {
    pub total_models: usize,
    pub min_tier_filter: Option<String>,
    pub filtered_models_count: usize,
    pub models_by_max_tier: BTreeMap<String, usize>,
    pub collections: BTreeMap<String, CollectionCompletenessSummary>,
    pub incomplete_models: Vec<ModelMissingEntry>,
}

fn max_claim_tier(claims: &ModelClaims) -> EvidenceTier {
    [
        claims.board.tier,
        claims.flash_chip.tier,
        claims.nor_layout.tier,
        claims.train.tier,
        claims.eeprom.tier,
    ]
    .into_iter()
    .max()
    .unwrap_or(EvidenceTier::CommunityUnverified)
}

type FieldMissingCheck<T> = (&'static str, fn(&T) -> bool);

fn count_missing<I, T>(items: I, checks: &[FieldMissingCheck<T>]) -> CollectionCompletenessSummary
where
    I: IntoIterator<Item = T>,
{
    let mut total = 0usize;
    let mut missing: BTreeMap<String, usize> = checks
        .iter()
        .map(|(name, _)| ((*name).to_string(), 0usize))
        .collect();
    for item in items {
        total += 1;
        for (name, pred) in checks {
            if pred(&item)
                && let Some(c) = missing.get_mut(*name)
            {
                *c += 1;
            }
        }
    }
    CollectionCompletenessSummary {
        total,
        missing_fields: missing,
    }
}

/// Computes per-collection and per-tier completeness counts from a loaded `ProfilesBundle`.
pub(in super::super) fn build_completeness_report(
    bundle: &ProfilesBundle,
    min_tier: Option<EvidenceTier>,
) -> ProfilesCompletenessReport {
    let mut by_tier: BTreeMap<String, usize> = BTreeMap::new();
    let mut incomplete_models = Vec::new();
    let mut filtered_count = 0usize;

    for model in bundle.models.values() {
        let top_tier = max_claim_tier(&model.claims);
        *by_tier.entry(top_tier.short_tag().to_string()).or_insert(0) += 1;
        if min_tier.is_some_and(|req| top_tier < req) {
            continue;
        }
        filtered_count += 1;
        let mut miss = Vec::new();
        for (field, is_missing) in [
            ("hw_code", model.hw_code.is_none()),
            ("board_ref", model.board_ref.is_none()),
            ("train_ref", model.train_ref.is_none()),
            ("nor_layout_ref", model.nor_layout_ref.is_none()),
            ("fixture_ref", model.fixture_ref.is_none()),
            ("exact_aliases", model.exact_aliases.is_empty()),
        ] {
            if is_missing {
                miss.push(field.to_string());
            }
        }
        if !miss.is_empty() {
            incomplete_models.push(ModelMissingEntry {
                model_id: model.model_id.clone(),
                max_tier: top_tier.short_tag().to_string(),
                missing: miss,
            });
        }
    }

    let mut collections = BTreeMap::new();
    collections.insert(
        "models".to_string(),
        count_missing(
            bundle.models.values(),
            &[
                ("hw_code", |m: &&ModelProfileManifest| m.hw_code.is_none()),
                ("board_ref", |m: &&ModelProfileManifest| {
                    m.board_ref.is_none()
                }),
                ("train_ref", |m: &&ModelProfileManifest| {
                    m.train_ref.is_none()
                }),
                ("nor_layout_ref", |m: &&ModelProfileManifest| {
                    m.nor_layout_ref.is_none()
                }),
                ("fixture_ref", |m: &&ModelProfileManifest| {
                    m.fixture_ref.is_none()
                }),
                ("exact_aliases", |m: &&ModelProfileManifest| {
                    m.exact_aliases.is_empty()
                }),
            ],
        ),
    );
    collections.insert(
        "boards".to_string(),
        count_missing(
            bundle.boards.values(),
            &[
                ("service_module_code", |b: &&BoardProfile| {
                    b.service_module_code.is_none()
                }),
                ("soc_ref", |b: &&BoardProfile| b.soc_ref.is_none()),
                ("flash_ref", |b: &&BoardProfile| b.flash_ref.is_none()),
                ("eeprom_refs", |b: &&BoardProfile| b.eeprom_refs.is_empty()),
            ],
        ),
    );
    collections.insert(
        "socs".to_string(),
        count_missing(
            bundle.socs.values(),
            &[
                ("bus_windows", |s: &&SocProfile| s.bus_windows.is_empty()),
                ("lethal_mmio", |s: &&SocProfile| s.lethal_mmio.is_empty()),
            ],
        ),
    );
    collections.insert(
        "flash_chips".to_string(),
        count_missing(
            bundle.flash_chips.values(),
            &[
                ("jedec", |f: &&FlashChipProfile| f.jedec.is_none()),
                ("sector_banks", |f: &&FlashChipProfile| {
                    f.sector_banks.is_empty()
                }),
                ("rw_banks", |f: &&FlashChipProfile| f.rw_banks.is_empty()),
                ("hw_protect", |f: &&FlashChipProfile| f.hw_protect.is_none()),
            ],
        ),
    );
    collections.insert(
        "eeprom_chips".to_string(),
        count_missing(
            bundle.eeprom_chips.values(),
            &[("service_replacement_part", |e: &&EepromChipProfile| {
                e.service_replacement_part.is_none()
            })],
        ),
    );
    collections.insert(
        "firmware_trains".to_string(),
        count_missing(
            bundle.firmware_trains.values(),
            &[
                ("loader_date", |t: &&FirmwareTrainProfile| {
                    t.loader_date.is_none()
                }),
                ("loader_fingerprint", |t: &&FirmwareTrainProfile| {
                    t.loader_fingerprint.is_none()
                }),
                ("frm", |t: &&FirmwareTrainProfile| t.frm.is_none()),
                ("prog_load_va", |t: &&FirmwareTrainProfile| {
                    t.prog_load_va.is_none()
                }),
                ("main_load_va", |t: &&FirmwareTrainProfile| {
                    t.main_load_va.is_none()
                }),
                ("drv_load_va", |t: &&FirmwareTrainProfile| {
                    t.drv_load_va.is_none()
                }),
                ("symbols", |t: &&FirmwareTrainProfile| t.symbols.is_empty()),
                ("sd_boot_filenames", |t: &&FirmwareTrainProfile| {
                    t.sd_boot_filenames.is_empty()
                }),
                ("digadiag_raw_bytes", |t: &&FirmwareTrainProfile| {
                    t.digadiag_raw_bytes.is_none()
                }),
                ("digadiag_spec", |t: &&FirmwareTrainProfile| {
                    t.digadiag_spec.is_none()
                }),
                ("pana_rebuild_recipe", |t: &&FirmwareTrainProfile| {
                    t.pana_rebuild_recipe.is_none()
                }),
            ],
        ),
    );

    let regions_total = bundle.nor_layouts.values().map(|l| l.regions.len()).sum();
    let regions_missing_sig = bundle
        .nor_layouts
        .values()
        .flat_map(|l| l.regions.iter())
        .filter(|r| r.signature.is_none())
        .count();
    collections.insert(
        "nor_layouts".to_string(),
        CollectionCompletenessSummary {
            total: bundle.nor_layouts.len(),
            missing_fields: BTreeMap::from([
                ("regions_total".to_string(), regions_total),
                ("regions_without_signature".to_string(), regions_missing_sig),
            ]),
        },
    );

    ProfilesCompletenessReport {
        total_models: bundle.models.len(),
        min_tier_filter: min_tier.map(|t| t.short_tag().to_string()),
        filtered_models_count: filtered_count,
        models_by_max_tier: by_tier,
        collections,
        incomplete_models,
    }
}
