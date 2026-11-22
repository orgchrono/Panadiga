#![deny(unsafe_code)]

//! NOR layout alignment, region isolation, train/board foreign-key, and model alias validation
//! helpers for [`crate::validate`].

use std::collections::BTreeMap;

use crate::derive::sector_range_for_aligned_span;
use crate::hex_serde::parse_hex_bytes;
use crate::types::{
    FirmwareTrainProfile, FlashChipProfile, ModelProfileManifest, NorLayoutProfile, NorRegionKind,
    NorRegionPolicy, ProfilesBundle,
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
    tr.check(model.tv_standard.is_none(), || {
        format!(
            "model '{}': tv_standard must not be hardcoded in manifest; declare region_suffix for dynamic interpolation",
            model.model_id
        )
    });
    let interpolated_regions = crate::region::interpolate_model_regions(
        model.region_suffix.as_deref(),
        &model.model_id,
        &model.series,
        model.hw_code.as_deref(),
    );
    tr.check(!interpolated_regions.is_empty(), || {
        format!(
            "model '{}': region_suffix must declare at least one region for TV/hardware interpolation",
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
            if let Some(train) = bundle.firmware_trains.get(tref)
                && let Some(spec) = &train.bgo_retarget_spec
                && let Some(bgo_reg) = layout.regions.iter().find(|r| r.kind == NorRegionKind::Bgo)
            {
                let spec_flash_offset = spec.detect_flash_base & 0x00FF_FFFF;
                tr.check(spec_flash_offset == bgo_reg.start_offset, || {
                    format!(
                        "model '{}': train '{tref}' bgo_retarget_spec flash offset 0x{spec_flash_offset:06X} != nor_layout '{}' BGO start 0x{:06X}",
                        model.model_id, layout.id, bgo_reg.start_offset
                    )
                });
            }
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

fn validate_bgo_retarget_spec(
    bundle: &ProfilesBundle,
    trn: &FirmwareTrainProfile,
    tr: &mut CheckTracker,
) {
    let Some(spec) = &trn.bgo_retarget_spec else {
        return;
    };
    tr.check(!spec.target_geometry_id.trim().is_empty(), || {
        format!(
            "train '{}': bgo_retarget_spec.target_geometry_id cannot be empty",
            trn.id
        )
    });
    tr.check(spec.expected_prog_size > 0, || {
        format!(
            "train '{}': bgo_retarget_spec.expected_prog_size must be > 0",
            trn.id
        )
    });
    tr.check(spec.bgo_table_words.len() == 64, || {
        format!(
            "train '{}': bgo_retarget_spec.bgo_table_words must have 64 words (got {})",
            trn.id,
            spec.bgo_table_words.len()
        )
    });
    let prog_len = usize::try_from(spec.expected_prog_size).unwrap_or(0);
    let table_end = (spec.bgo_table_offset as usize).saturating_add(spec.bgo_table_words.len() * 4);
    tr.check(table_end <= prog_len, || {
        format!(
            "train '{}': bgo_table range [0x{:X}..0x{table_end:X}) exceeds expected_prog_size 0x{prog_len:X}",
            trn.id, spec.bgo_table_offset
        )
    });
    tr.check(
        spec.nfrom_drv_start < spec.nfrom_drv_end && spec.nfrom_drv_end <= spec.expected_prog_size,
        || {
            format!(
                "train '{}': invalid nfrom_drv window [0x{:X}..0x{:X}) within 0x{prog_len:X}",
                trn.id, spec.nfrom_drv_start, spec.nfrom_drv_end
            )
        },
    );
    tr.check(
        spec.nfrom_drv_uncompressed_bytes > 0
            && spec
                .nfrom_drv_start
                .saturating_add(spec.nfrom_drv_uncompressed_bytes)
                <= spec.nfrom_drv_end,
        || {
            format!(
                "train '{}': nfrom_drv_uncompressed_bytes ({}) does not fit in [0x{:X}..0x{:X})",
                trn.id, spec.nfrom_drv_uncompressed_bytes, spec.nfrom_drv_start, spec.nfrom_drv_end
            )
        },
    );
    tr.check(
        spec.bgo_sum_beq_offset >= spec.nfrom_drv_start
            && spec.bgo_sum_beq_offset.saturating_add(2) <= spec.nfrom_drv_end,
        || {
            format!(
                "train '{}': bgo_sum_beq_offset 0x{:X} outside [0x{:X}..0x{:X})",
                trn.id, spec.bgo_sum_beq_offset, spec.nfrom_drv_start, spec.nfrom_drv_end
            )
        },
    );
    tr.check(
        spec.bss_zero_end_offset.saturating_add(4) <= spec.expected_prog_size,
        || {
            format!(
                "train '{}': bss_zero_end_offset 0x{:X} out of bounds",
                trn.id, spec.bss_zero_end_offset
            )
        },
    );
    tr.check(
        spec.init2_call_offset.saturating_add(5) <= spec.expected_prog_size,
        || {
            format!(
                "train '{}': init2_call_offset 0x{:X} out of bounds",
                trn.id, spec.init2_call_offset
            )
        },
    );
    tr.check(
        check_hex_digest(Some(&spec.nfrom_drv_uncompressed_sha256)),
        || {
            format!(
                "train '{}': invalid nfrom_drv_uncompressed_sha256 '{}'",
                trn.id, spec.nfrom_drv_uncompressed_sha256
            )
        },
    );
    for rng in &spec.hi_ram_ranges {
        tr.check(
            rng.start < rng.end && rng.end <= spec.expected_prog_size,
            || {
                format!(
                    "train '{}': hi_ram_range [0x{:X}..0x{:X}) out of bounds",
                    trn.id, rng.start, rng.end
                )
            },
        );
    }
    for bp in &spec.byte_patches {
        tr.check(bp.offset < spec.expected_prog_size, || {
            format!(
                "train '{}': byte_patch offset 0x{:X} out of bounds",
                trn.id, bp.offset
            )
        });
    }
    let relpath_ok = spec.nfrom_drv_blob_relpath.starts_with("blobs/")
        && !spec.nfrom_drv_blob_relpath.contains("..");
    tr.check(relpath_ok, || {
        format!(
            "train '{}': invalid nfrom_drv_blob_relpath '{}'",
            trn.id, spec.nfrom_drv_blob_relpath
        )
    });
    let b64_opt = bundle.blobs.get(&spec.nfrom_drv_blob_relpath);
    tr.check(b64_opt.is_some(), || {
        format!(
            "train '{}': missing embedded blob '{}' in bundle.blobs",
            trn.id, spec.nfrom_drv_blob_relpath
        )
    });
    if let Some(b64) = b64_opt {
        tr.check(crate::blob_b64::decode_base64(b64).is_ok(), || {
            format!(
                "train '{}': embedded blob '{}' contains malformed base64",
                trn.id, spec.nfrom_drv_blob_relpath
            )
        });
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
        if let Some(cref) = &trn.soc_core_ref {
            tr.check_fk(&bundle.soc_cores, cref, &trn.id, "soc_core_ref");
        }
        for mref in &trn.media_refs {
            tr.check_fk(&bundle.media_formats, mref, &trn.id, "media_refs");
        }
        if let Some(frm) = &trn.frm {
            tr.check(check_hex_digest(frm.official_frm_sha256.as_deref()), || {
                format!("train '{}': invalid official_frm_sha256", trn.id)
            });
        }
        validate_bgo_retarget_spec(bundle, trn, tr);
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
