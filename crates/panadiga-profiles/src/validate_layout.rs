#![deny(unsafe_code)]

//! NOR layout alignment, region isolation, train/board foreign-key, and model alias validation
//! helpers for [`crate::validate`].

use std::collections::BTreeMap;

use crate::derive::sector_range_for_aligned_span;
use crate::hex_serde::parse_hex_bytes;
use crate::types::{
    FlashChipProfile, ModelProfileManifest, NorLayoutProfile, NorRegionKind, NorRegionPolicy,
    ProfilesBundle,
};
use crate::validate::{CheckTracker, MIN_CRITICAL_IPL_GUARD_BYTES, check_hex_digest};

fn validate_single_nor_layout(
    layout: &NorLayoutProfile,
    flash: &FlashChipProfile,
    tr: &mut CheckTracker,
) {
    tr.check(flash.writable, || {
        format!(
            "nor_layout '{}': target flash '{}' is non-writable OTP",
            layout.id, flash.id
        )
    });
    let mut regions_sorted = layout.regions.clone();
    regions_sorted.sort_by_key(|r| r.start_offset);

    let mut prev_end: Option<u32> = None;
    let mut ipl_ok = false;
    let mut eemul_count = 0usize;

    for reg in &regions_sorted {
        tr.check(
            reg.start_offset <= reg.end_offset && reg.end_offset < flash.total_bytes,
            || {
                format!(
                    "nor_layout '{}': region {:?} [0x{:X}..=0x{:X}] out of flash bounds (0x{:X})",
                    layout.id, reg.kind, reg.start_offset, reg.end_offset, flash.total_bytes
                )
            },
        );
        if let Some(pe) = prev_end {
            tr.check(reg.start_offset > pe, || {
                format!(
                    "nor_layout '{}': overlapping region {:?} at 0x{:X} <= previous end 0x{:X}",
                    layout.id, reg.kind, reg.start_offset, pe
                )
            });
        }
        prev_end = Some(reg.end_offset);

        let aligned = sector_range_for_aligned_span(flash, reg.start_offset, reg.end_offset);
        tr.check(aligned.is_ok(), || {
            format!(
                "nor_layout '{}': region {:?} [0x{:X}..=0x{:X}] misaligned: {}",
                layout.id,
                reg.kind,
                reg.start_offset,
                reg.end_offset,
                aligned.err().map_or_else(String::new, |e| e.to_string())
            )
        });

        match reg.kind {
            NorRegionKind::Ipl => {
                let span = reg
                    .end_offset
                    .saturating_sub(reg.start_offset)
                    .saturating_add(1);
                if reg.start_offset == 0
                    && reg.policy == NorRegionPolicy::ReadOnlyPermanent
                    && span >= MIN_CRITICAL_IPL_GUARD_BYTES
                {
                    ipl_ok = true;
                }
            }
            NorRegionKind::Eemul => {
                eemul_count += 1;
            }
            NorRegionKind::Bgo => {
                tr.check(
                    matches!(
                        reg.policy,
                        NorRegionPolicy::GatedUpperNor | NorRegionPolicy::ReadOnlyReference
                    ),
                    || {
                        format!(
                            "nor_layout '{}': BGO region must use GatedUpperNor or ReadOnlyReference",
                            layout.id
                        )
                    },
                );
            }
            _ => {}
        }

        if reg.policy == NorRegionPolicy::NvramSanitize {
            tr.check(reg.kind == NorRegionKind::Eemul, || {
                format!(
                    "nor_layout '{}': NvramSanitize policy only permitted on Eemul regions (found {:?})",
                    layout.id, reg.kind
                )
            });
        }
    }

    tr.check(ipl_ok, || {
        format!(
            "nor_layout '{}': must start at 0x000000 with Ipl/ReadOnlyPermanent >= 256 KiB (0x{MIN_CRITICAL_IPL_GUARD_BYTES:X})",
            layout.id
        )
    });
    tr.check(eemul_count <= 1, || {
        format!(
            "nor_layout '{}': expected at most 1 Eemul region, found {eemul_count}",
            layout.id
        )
    });
}

fn validate_model_manifest(
    bundle: &ProfilesBundle,
    map_key: &str,
    model: &ModelProfileManifest,
    seen_names: &mut BTreeMap<String, String>,
    tr: &mut CheckTracker,
) {
    let expected_key = model.model_id.to_ascii_lowercase().replace('-', "_");
    tr.check(map_key == expected_key || map_key == model.model_id, || {
        format!(
            "model key '{map_key}' inconsistent with model_id '{}'",
            model.model_id
        )
    });
    if let Some(bref) = &model.board_ref {
        tr.check_fk(&bundle.boards, bref, &model.model_id, "board_ref");
    }
    if let Some(tref) = &model.train_ref {
        tr.check_fk(&bundle.firmware_trains, tref, &model.model_id, "train_ref");
    }
    if let Some(lref) = &model.nor_layout_ref {
        tr.check_fk(&bundle.nor_layouts, lref, &model.model_id, "nor_layout_ref");
        if let (Some(layout), Some(bref)) = (bundle.nor_layouts.get(lref), model.board_ref.as_ref())
            && let Some(board) = bundle.boards.get(bref)
            && let Some(board_flash) = board.flash_ref.as_ref()
        {
            tr.check(&layout.flash_ref == board_flash, || {
                format!(
                    "model '{}': nor_layout '{}' uses flash '{}' != board flash '{board_flash}'",
                    model.model_id, layout.id, layout.flash_ref
                )
            });
        }
        if let (Some(layout), Some(tref)) = (bundle.nor_layouts.get(lref), model.train_ref.as_ref())
        {
            tr.check(&layout.train_ref == tref, || {
                format!(
                    "model '{}': nor_layout '{}' uses train '{}' != model train '{tref}'",
                    model.model_id, layout.id, layout.train_ref
                )
            });
        }
    }

    let names_iter = std::iter::once(&model.model_id).chain(model.exact_aliases.iter());
    for name in names_iter {
        let folded = name.trim().to_ascii_uppercase();
        tr.check(!folded.is_empty(), || {
            format!("model '{}': empty model_id or alias", model.model_id)
        });
        if let Some(prev_owner) = seen_names.insert(folded.clone(), model.model_id.clone()) {
            tr.check(false, || {
                format!(
                    "duplicate model identifier/alias '{folded}': claimed by both '{prev_owner}' and '{}'",
                    model.model_id
                )
            });
        }
    }
}

pub(super) fn validate_train_board_layout_model(bundle: &ProfilesBundle, tr: &mut CheckTracker) {
    for (k, mf) in &bundle.media_formats {
        tr.check(k == &mf.id, || {
            format!("media_format key '{k}' != id '{}'", mf.id)
        });
        tr.check(parse_hex_bytes(&mf.magic_hex).is_ok(), || {
            format!("media '{}' has invalid magic_hex '{}'", mf.id, mf.magic_hex)
        });
    }

    for (k, trn) in &bundle.firmware_trains {
        tr.check(k == &trn.id, || {
            format!("firmware_train key '{k}' != id '{}'", trn.id)
        });
        for mref in &trn.media_refs {
            tr.check_fk(&bundle.media_formats, mref, &trn.id, "media_refs");
        }
        if let Some(frm) = &trn.frm {
            tr.check(check_hex_digest(frm.official_frm_sha256.as_deref()), || {
                format!("train '{}': invalid official_frm_sha256", trn.id)
            });
        }
    }

    for (k, bd) in &bundle.boards {
        tr.check(k == &bd.id, || format!("board key '{k}' != id '{}'", bd.id));
        if let Some(sref) = &bd.soc_ref {
            tr.check_fk(&bundle.socs, sref, &bd.id, "soc_ref");
        }
        if let Some(fref) = &bd.flash_ref {
            tr.check_fk(&bundle.flash_chips, fref, &bd.id, "flash_ref");
        }
        for eref in &bd.eeprom_refs {
            tr.check_fk(&bundle.eeprom_chips, eref, &bd.id, "eeprom_refs");
        }
    }

    for (k, layout) in &bundle.nor_layouts {
        tr.check(k == &layout.id, || {
            format!("nor_layout key '{k}' != id '{}'", layout.id)
        });
        tr.check_fk(
            &bundle.flash_chips,
            &layout.flash_ref,
            &layout.id,
            "flash_ref",
        );
        tr.check_fk(
            &bundle.firmware_trains,
            &layout.train_ref,
            &layout.id,
            "train_ref",
        );
        if let Some(flash) = bundle.flash_chips.get(&layout.flash_ref) {
            validate_single_nor_layout(layout, flash, tr);
        }
    }

    let mut seen_names: BTreeMap<String, String> = BTreeMap::new();
    for (k, model) in &bundle.models {
        validate_model_manifest(bundle, k, model, &mut seen_names, tr);
    }
}
