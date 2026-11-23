#![deny(unsafe_code)]

//! BGO retarget window, DIGADIAG binary spec, Flash geometry, and NFROM blob validation
//! for firmware train profiles (`validate_bgo.rs`).

use crate::types::{FirmwareTrainProfile, NorRegionKind, ProfilesBundle};
use crate::validate::{CheckTracker, MIN_CRITICAL_IPL_GUARD_BYTES, check_hex_digest};

/// Parses declared flash capacity bytes (`_4MB_`/`_8MB_`/`_16MB_`) and uncached base VA (`_0X...`)
/// from a declarative geometry identifier such as `BGO_45SEC_8MB_0X84130000`.
#[must_use]
pub(crate) fn parse_bgo_geometry_tag(tag: &str) -> (Option<u32>, Option<u32>) {
    let mut cap_bytes: Option<u32> = None;
    let mut base_va: Option<u32> = None;
    for raw_tok in tag.trim().split('_') {
        let tok = raw_tok.to_ascii_uppercase();
        match tok.as_str() {
            "4MB" => cap_bytes = Some(0x0040_0000),
            "8MB" => cap_bytes = Some(0x0080_0000),
            "16MB" => cap_bytes = Some(0x0100_0000),
            _ => {
                if let Some(hex_part) = tok.strip_prefix("0X")
                    && hex_part.len() == 8
                    && let Ok(parsed) = u32::from_str_radix(hex_part, 16)
                {
                    base_va = Some(parsed);
                }
            }
        }
    }
    (cap_bytes, base_va)
}

fn check_layouts_for_train_bgo(
    bundle: &ProfilesBundle,
    train_id: &str,
    label: &str,
    geom_tag: &str,
    expected_span: Option<(u32, u32)>,
    tr: &mut CheckTracker,
) {
    let (tag_cap, tag_va) = parse_bgo_geometry_tag(geom_tag);
    for layout in bundle
        .nor_layouts
        .values()
        .filter(|l| l.train_ref == train_id)
    {
        if let (Some(exp_cap), Some(flash)) = (tag_cap, bundle.flash_chips.get(&layout.flash_ref)) {
            tr.check(flash.total_bytes == exp_cap, || {
                format!(
                    "train '{train_id}' {label} '{geom_tag}' declares flash capacity 0x{exp_cap:X} != nor_layout '{}' flash '{}' capacity 0x{:X}",
                    layout.id, flash.id, flash.total_bytes
                )
            });
        }
        if let Some((start_off, end_excl)) = expected_span {
            let exact_bgo = layout.find_region(NorRegionKind::Bgo).is_some_and(|bgo| {
                bgo.start_offset == start_off && bgo.end_offset == end_excl.saturating_sub(1)
            });
            tr.check(exact_bgo, || {
                format!(
                    "train '{train_id}' {label} [0x{start_off:06X}..=0x{:06X}] mismatches nor_layout '{}' BGO span",
                    end_excl.saturating_sub(1),
                    layout.id
                )
            });
        } else if let Some(va) = tag_va {
            let expected_start = va & 0x00FF_FFFF;
            let bgo_ok = layout
                .find_region(NorRegionKind::Bgo)
                .is_some_and(|bgo| bgo.start_offset == expected_start);
            tr.check(bgo_ok, || {
                format!(
                    "train '{train_id}' {label} '{geom_tag}' expects BGO start 0x{expected_start:06X} in nor_layout '{}'",
                    layout.id
                )
            });
        }
    }
}

fn validate_digadiag_and_layout_pairing(
    bundle: &ProfilesBundle,
    trn: &FirmwareTrainProfile,
    tr: &mut CheckTracker,
) {
    let Some(diag) = &trn.digadiag_spec else {
        return;
    };
    let expected_exact = match diag.format.as_str() {
        "raw_256k_prog061" => Some(262_144_u32),
        "extrhead_128k" => Some(131_072_u32),
        _ => None,
    };
    if let Some(exp) = expected_exact {
        tr.check(diag.exact_bytes == exp, || {
            format!(
                "train '{}': digadiag_spec.format '{}' requires exact_bytes={exp} (got {})",
                trn.id, diag.format, diag.exact_bytes
            )
        });
    } else {
        tr.check(diag.exact_bytes > 0, || {
            format!("train '{}': digadiag_spec.exact_bytes must be > 0", trn.id)
        });
    }
    check_layouts_for_train_bgo(
        bundle,
        &trn.id,
        "digadiag_spec target",
        &diag.bgo_geometry_target,
        None,
        tr,
    );
}

pub(super) fn validate_bgo_retarget_spec(
    bundle: &ProfilesBundle,
    trn: &FirmwareTrainProfile,
    tr: &mut CheckTracker,
) {
    validate_digadiag_and_layout_pairing(bundle, trn, tr);

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

    if spec.bgo_table_words.len() == 64 {
        let w12 = spec.bgo_table_words[12];
        let w13 = spec.bgo_table_words[13];
        let w15 = spec.bgo_table_words[15];
        let w31 = spec.bgo_table_words[31];
        let start_off = w12 & 0x00FF_FFFF;
        let end_excl_off = w13 & 0x00FF_FFFF;

        tr.check(w12 == spec.detect_flash_base, || {
            format!(
                "train '{}': bgo_table_words[12]=0x{w12:08X} != detect_flash_base=0x{:08X}",
                trn.id, spec.detect_flash_base
            )
        });
        tr.check(w13 > w12 && end_excl_off > start_off, || {
            format!(
                "train '{}': bgo_table_words[13]=0x{w13:08X} must exceed word[12]=0x{w12:08X}",
                trn.id
            )
        });
        tr.check(start_off >= MIN_CRITICAL_IPL_GUARD_BYTES, || {
            format!(
                "train '{}': BGO start 0x{start_off:06X} enters 256 KiB IPL guard",
                trn.id
            )
        });
        tr.check(w31 > 0, || {
            format!(
                "train '{}': bgo_table_words[31] (total sectors) must be > 0",
                trn.id
            )
        });
        tr.check(
            spec.detect_logical_sectors > 0 && spec.detect_logical_sectors < w31,
            || {
                format!(
                    "train '{}': detect_logical_sectors ({}) must satisfy 0 < n < word[31] ({w31})",
                    trn.id, spec.detect_logical_sectors
                )
            },
        );
        if w15 == 0xFFFF_FFFF {
            let expected_span = w31.saturating_mul(0x1_0000);
            let actual_span = end_excl_off.saturating_sub(start_off);
            tr.check(actual_span == expected_span, || {
                format!(
                    "train '{}': Bank 2 disabled (word[15]=0xFFFFFFFF) requires [0x{start_off:06X}..0x{end_excl_off:06X}) span 0x{actual_span:X} == word[31]*64KiB (0x{expected_span:X})",
                    trn.id
                )
            });
        }

        if let Some(diag) = &trn.digadiag_spec {
            tr.check(diag.bgo_geometry_target == spec.target_geometry_id, || {
                format!(
                    "train '{}': digadiag_spec.bgo_geometry_target '{}' != bgo_retarget_spec.target_geometry_id '{}'",
                    trn.id, diag.bgo_geometry_target, spec.target_geometry_id
                )
            });
        }

        let (_, tag_va) = parse_bgo_geometry_tag(&spec.target_geometry_id);
        if let Some(va) = tag_va {
            tr.check((va & 0x00FF_FFFF) == start_off, || {
                format!(
                    "train '{}': geometry id '{}' base 0x{:06X} != word[12] offset 0x{start_off:06X}",
                    trn.id,
                    spec.target_geometry_id,
                    va & 0x00FF_FFFF
                )
            });
        }

        check_layouts_for_train_bgo(
            bundle,
            &trn.id,
            "bgo_retarget_spec",
            &spec.target_geometry_id,
            Some((start_off, end_excl_off)),
            tr,
        );
    }

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
