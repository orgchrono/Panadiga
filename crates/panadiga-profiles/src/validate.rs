#![deny(unsafe_code)]

//! Cross-entity fail-closed validator checking silicon voltage, lethal MMIO deny-lists,
//! datasheet sector sums, alignment, IPL protection floor (>=256 KiB), EEMUL isolation,
//! JEDEC uniqueness, and global model alias uniqueness.

use std::collections::{BTreeMap, HashSet};

use anyhow::{Result, bail};

use crate::derive::derive_sector_table;
use crate::types::{ProfilesBundle, RegistryValidationReport};
use crate::validate_layout::validate_train_board_layout_model;

/// Non-negotiable physical constants enforced regardless of any external dictionary overlay.
pub const REQUIRED_LVTTL_MV: u32 = 3300;
pub const AM34_LETHAL_MMIO_ADDR: u32 = 0xC000_100C;
pub const MIN_CRITICAL_IPL_GUARD_BYTES: u32 = 0x0004_0000;

pub(super) struct CheckTracker {
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

    pub(super) fn check(&mut self, ok: bool, err_msg: impl FnOnce() -> String) {
        self.runs += 1;
        if !ok {
            self.errors.push(err_msg());
        }
    }

    pub(super) fn check_fk<T>(
        &mut self,
        table: &BTreeMap<String, T>,
        key: &str,
        owner: &str,
        field: &str,
    ) {
        self.check(table.contains_key(key), || {
            format!("{owner}: unresolved foreign key {field}='{key}'")
        });
    }
}

pub(super) fn check_hex_digest(val: Option<&str>) -> bool {
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

/// Validates an entire [`ProfilesBundle`] and returns an accurate [`RegistryValidationReport`].
pub fn validate_bundle(bundle: &ProfilesBundle) -> RegistryValidationReport {
    let mut tr = CheckTracker::new();
    tr.check(
        bundle.schema_version == crate::loader::PROFILES_SCHEMA_VERSION,
        || {
            format!(
                "Unsupported ProfilesBundle schema_version {} (expected {})",
                bundle.schema_version,
                crate::loader::PROFILES_SCHEMA_VERSION
            )
        },
    );
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
