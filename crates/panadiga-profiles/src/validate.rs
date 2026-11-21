#![deny(unsafe_code)]

//! Cross-entity fail-closed validator checking silicon voltage, lethal MMIO deny-lists,
//! datasheet sector sums, alignment, IPL protection floor (>=256 KiB), EEMUL isolation,
//! JEDEC uniqueness, and global model alias uniqueness.

use std::collections::{BTreeMap, HashSet};

use anyhow::{Result, bail};

use crate::derive::{derive_sector_table, sector_range_for_aligned_span};
use crate::hex_serde::parse_hex_bytes;
use crate::types::{
    FlashChipProfile, ModelProfileManifest, NorLayoutProfile, NorRegionKind, NorRegionPolicy,
    ProfilesBundle, RegistryValidationReport,
};

/// Non-negotiable physical constants enforced regardless of any external dictionary overlay.
pub const REQUIRED_LVTTL_MV: u32 = 3300;
pub const AM34_LETHAL_MMIO_ADDR: u32 = 0xC000_100C;
pub const MIN_CRITICAL_IPL_GUARD_BYTES: u32 = 0x0004_0000;

struct CheckTracker {
    runs: usize,
    errors: Vec<String>,
}

impl CheckTracker {
    fn new() -> Self {
        Self {
            runs: 0,
            errors: Vec::new(),
        }
    }

    fn check(&mut self, ok: bool, err_msg: impl FnOnce() -> String) {
        self.runs += 1;
        if !ok {
            self.errors.push(err_msg());
        }
    }

    fn check_fk<T>(&mut self, table: &BTreeMap<String, T>, key: &str, owner: &str, field: &str) {
        self.check(table.contains_key(key), || {
            format!("{owner}: unresolved foreign key {field}='{key}'")
        });
    }
}

fn check_hex_digest(val: Option<&str>) -> bool {
    match val {
        None => true,
        Some(s) => s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()),
    }
}

fn validate_soc_layer(bundle: &ProfilesBundle, tr: &mut CheckTracker) {
    for (map_key, core) in &bundle.soc_cores {
        tr.check(map_key == &core.id, || {
            format!("soc_core key '{map_key}' != id '{}'", core.id)
        });
        tr.check(core.logic_voltage_mv == REQUIRED_LVTTL_MV, || {
            format!(
                "soc_core '{}': logic_voltage_mv={} violates {} mV LVTTL invariant",
                core.id, core.logic_voltage_mv, REQUIRED_LVTTL_MV
            )
        });
    }

    for (map_key, soc) in &bundle.socs {
        tr.check(map_key == &soc.id, || {
            format!("soc key '{map_key}' != id '{}'", soc.id)
        });
        tr.check_fk(&bundle.soc_cores, &soc.core_ref, &soc.id, "core_ref");
        tr.check(soc.p7501_nsw5v_hiz_only, || {
            format!("soc '{}': p7501_nsw5v_hiz_only must be true", soc.id)
        });
        let is_mn103 = bundle
            .soc_cores
            .get(&soc.core_ref)
            .is_some_and(|c| c.isa.to_ascii_uppercase().contains("MN103"));
        if is_mn103 {
            tr.check(soc.lethal_mmio.contains(&AM34_LETHAL_MMIO_ADDR), || {
                format!(
                    "soc '{}': MN103 SoC must block lethal MMIO 0x{AM34_LETHAL_MMIO_ADDR:08X}",
                    soc.id
                )
            });
        }
        for win in &soc.bus_windows {
            tr.check(win.size > 0, || {
                format!("soc '{}' window '{}' has size 0", soc.id, win.name)
            });
        }
    }
}

fn validate_flash_and_eeprom_layer(bundle: &ProfilesBundle, tr: &mut CheckTracker) {
    let mut seen_jedec: HashSet<(u32, Vec<u32>)> = HashSet::new();
    for (map_key, chip) in &bundle.flash_chips {
        tr.check(map_key == &chip.id, || {
            format!("flash_chip key '{map_key}' != id '{}'", chip.id)
        });
        tr.check(chip.nominal_voltage_mv == REQUIRED_LVTTL_MV, || {
            format!(
                "flash '{}': nominal_voltage_mv={} violates {} mV invariant",
                chip.id, chip.nominal_voltage_mv, REQUIRED_LVTTL_MV
            )
        });
        tr.check(chip.total_bytes > 0, || {
            format!("flash '{}': total_bytes must be > 0", chip.id)
        });

        if chip.writable {
            let sector_sum: u64 = chip
                .sector_banks
                .iter()
                .map(|b| u64::from(b.count) * u64::from(b.sector_size))
                .sum();
            tr.check(
                !chip.sector_banks.is_empty() && sector_sum == u64::from(chip.total_bytes),
                || {
                    format!(
                        "flash '{}': sector_banks sum ({sector_sum}) != total_bytes ({})",
                        chip.id, chip.total_bytes
                    )
                },
            );
            let derived = derive_sector_table(chip);
            let sa_count = derived.as_ref().map_or(0, Vec::len) as u32;
            for rw in &chip.rw_banks {
                tr.check(rw.start_sa <= rw.end_sa && rw.end_sa < sa_count, || {
                    format!(
                        "flash '{}': invalid RWW bank SA{}..SA{} (sector_count={sa_count})",
                        chip.id, rw.start_sa, rw.end_sa
                    )
                });
            }
        } else {
            tr.check(chip.sector_banks.is_empty(), || {
                format!(
                    "flash '{}' is OTP (writable=false) and must not declare sector_banks",
                    chip.id
                )
            });
        }

        if let Some(jedec) = &chip.jedec {
            let key = (jedec.mfg, jedec.device_cycles.clone());
            tr.check(seen_jedec.insert(key), || {
                format!(
                    "flash '{}': duplicate JEDEC ID (mfg=0x{:02X}, dev={:04X?})",
                    chip.id, jedec.mfg, jedec.device_cycles
                )
            });
        }
    }

    for (map_key, eep) in &bundle.eeprom_chips {
        tr.check(map_key == &eep.id, || {
            format!("eeprom key '{map_key}' != id '{}'", eep.id)
        });
        tr.check(eep.voltage_mv <= REQUIRED_LVTTL_MV, || {
            format!(
                "eeprom '{}': voltage_mv={} exceeds {} mV",
                eep.id, eep.voltage_mv, REQUIRED_LVTTL_MV
            )
        });
    }
}

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

fn validate_train_board_layout_model(bundle: &ProfilesBundle, tr: &mut CheckTracker) {
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

/// Validates an entire [`ProfilesBundle`] and returns an accurate [`RegistryValidationReport`].
pub fn validate_bundle(bundle: &ProfilesBundle) -> RegistryValidationReport {
    let mut tr = CheckTracker::new();
    validate_soc_layer(bundle, &mut tr);
    validate_flash_and_eeprom_layer(bundle, &mut tr);
    validate_train_board_layout_model(bundle, &mut tr);

    let failed = tr.errors.len();
    let passed = tr.runs.saturating_sub(failed);
    RegistryValidationReport {
        valid: tr.errors.is_empty(),
        soc_core_count: bundle.soc_cores.len(),
        soc_count: bundle.socs.len(),
        flash_chip_count: bundle.flash_chips.len(),
        eeprom_chip_count: bundle.eeprom_chips.len(),
        firmware_train_count: bundle.firmware_trains.len(),
        nor_layout_count: bundle.nor_layouts.len(),
        media_format_count: bundle.media_formats.len(),
        board_count: bundle.boards.len(),
        model_count: bundle.models.len(),
        checks_run: tr.runs,
        checks_failed: failed,
        safety_checks_passed: passed,
        errors: tr.errors,
    }
}

/// Validates and fails closed (`Err`) on any invariant breach.
pub fn ensure_bundle_valid(bundle: &ProfilesBundle) -> Result<RegistryValidationReport> {
    let rep = validate_bundle(bundle);
    if !rep.valid {
        bail!(
            "ProfilesBundle failed validation: {}",
            rep.errors.join("; ")
        );
    }
    Ok(rep)
}
