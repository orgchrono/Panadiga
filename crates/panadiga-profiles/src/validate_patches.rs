#![deny(unsafe_code)]

//! Declarative patch-site, SATA-bridge reset, Stage-2 boot ABI, and `MAIN` build validation
//! for firmware train manifests (`validate_patches.rs`).

use std::collections::HashSet;

use crate::hex_serde::parse_hex_bytes;
use crate::types::{
    BinaryPatchSiteSpec, FirmwareTrainProfile, MainBuildSpec, SataBridgeResetSpec, Stage2BootSpec,
};
use crate::validate::{CheckTracker, MIN_CRITICAL_IPL_GUARD_BYTES, check_hex_digest};
use crate::validate_bgo::parse_bgo_geometry_tag;

fn validate_patch_site(
    owner: &str,
    base_va: u32,
    expected_size: u32,
    site: &BinaryPatchSiteSpec,
    tr: &mut CheckTracker,
) {
    tr.check(!site.name.trim().is_empty(), || {
        format!("{owner}: patch site name cannot be empty")
    });
    let expect_bytes = parse_hex_bytes(&site.expect_hex);
    let replace_bytes = parse_hex_bytes(&site.replace_hex);
    let raw_in_bounds = site
        .va
        .checked_sub(base_va)
        .is_some_and(|off| off < expected_size);
    tr.check(raw_in_bounds, || {
        format!(
            "{owner}: site '{}' VA 0x{:08X} out of bounds for base 0x{base_va:08X} / size {expected_size}",
            site.name, site.va
        )
    });
    tr.check(expect_bytes.is_ok() && replace_bytes.is_ok(), || {
        format!("{owner}: site '{}' contains invalid hex bytes", site.name)
    });
    if let (Ok(exp), Ok(rep)) = (expect_bytes, replace_bytes) {
        tr.check(exp.len() == rep.len(), || {
            format!(
                "{owner}: site '{}' expect length ({}) != replace length ({})",
                site.name,
                exp.len(),
                rep.len()
            )
        });
        let offset = site.va.checked_sub(base_va);
        let in_bounds = offset
            .and_then(|off| off.checked_add(exp.len() as u32))
            .is_some_and(|end| end <= expected_size);
        tr.check(in_bounds, || {
            format!(
                "{owner}: site '{}' VA 0x{:08X} (+{} B) out of bounds for base 0x{base_va:08X} / size {expected_size}",
                site.name, site.va, exp.len()
            )
        });
    }
}

fn validate_sata_reset_spec(owner: &str, spec: &SataBridgeResetSpec, tr: &mut CheckTracker) {
    tr.check(spec.expected_size > 0, || {
        format!("{owner}: sata_bridge_reset_spec.expected_size must be > 0")
    });
    tr.check(spec.patched_wait_ms > spec.factory_wait_ms, || {
        format!(
            "{owner}: patched_wait_ms ({}) must exceed factory_wait_ms ({})",
            spec.patched_wait_ms, spec.factory_wait_ms
        )
    });
    tr.check(spec.sleep_callee_va != 0, || {
        format!("{owner}: sleep_callee_va must be non-zero")
    });
    validate_patch_site(owner, spec.base_va, spec.expected_size, &spec.site, tr);
}

fn validate_stage2_boot_spec(train_id: &str, spec: &Stage2BootSpec, tr: &mut CheckTracker) {
    let owner = format!("train '{train_id}' stage2_boot_spec");
    let hook_bytes = parse_hex_bytes(&spec.hook_expect_hex);
    tr.check(hook_bytes.as_ref().is_ok_and(|b| b.len() == 7), || {
        format!("{owner}: hook_expect_hex must decode to exactly 7 bytes")
    });
    tr.check(
        spec.hook_offset.saturating_add(7) <= spec.expected_size
            && spec.payload_offset < spec.expected_size,
        || {
            format!(
                "{owner}: hook_offset (0x{:X}) or payload_offset (0x{:X}) exceeds expected_size {}",
                spec.hook_offset, spec.payload_offset, spec.expected_size
            )
        },
    );
    tr.check(spec.main_va_min < spec.main_va_max, || {
        format!(
            "{owner}: main_va_min (0x{:08X}) must be < main_va_max (0x{:08X})",
            spec.main_va_min, spec.main_va_max
        )
    });
    let norfrom_off = spec.norfrom_fvu_base_va & 0x00FF_FFFF;
    tr.check(norfrom_off >= MIN_CRITICAL_IPL_GUARD_BYTES, || {
        format!(
            "{owner}: norfrom_fvu_base_va 0x{:08X} overlaps immutable IPL guard < 0x{:X}",
            spec.norfrom_fvu_base_va, MIN_CRITICAL_IPL_GUARD_BYTES
        )
    });
    let s = &spec.symbols;
    let symbol_vas = [
        s.dcache_purge_va,
        s.uart0_getc_nb_va,
        s.uart0_printf_va,
        s.check_update_flag_va,
        s.hdd_max_lba_va,
        s.hdd_read_va,
        s.sd_init_va,
        s.stop_timer_irq_va,
        s.feistel_decrypt_va,
        s.extrhead_get_size_va,
        s.extrhead_check_hdr_va,
        s.extrhead_is_comp_va,
        s.extrhead_get_src_va,
        s.extrhead_get_entry_va,
        s.extrhead_uncomp_va,
        s.fat_mount_va,
        s.fat_open_va,
        s.fat_close_va,
        s.disable_irq_va,
        s.memcpy_va,
        s.sleep_10ms_va,
    ];
    tr.check(symbol_vas.iter().all(|&va| va != 0), || {
        format!("{owner}: all 21 stage2_boot_spec.symbols VAs must be non-zero")
    });
}

fn validate_main_build_spec(train_id: &str, build: &MainBuildSpec, tr: &mut CheckTracker) {
    let owner = format!("train '{train_id}' main_build '{}'", build.build_id);
    tr.check(!build.build_id.trim().is_empty(), || {
        format!("train '{train_id}': main_build.build_id cannot be empty")
    });
    tr.check(
        (1..=10).contains(&build.slot_name.len()) && build.slot_name.is_ascii(),
        || {
            format!(
                "{owner}: slot_name '{}' must be 1..=10 ASCII chars",
                build.slot_name
            )
        },
    );
    tr.check(build.unpacked_size > 0, || {
        format!("{owner}: unpacked_size must be > 0")
    });
    tr.check(check_hex_digest(build.unpacked_sha256.as_deref()), || {
        format!("{owner}: invalid unpacked_sha256")
    });
    if let Some(detect_va) = build.native_bgo_detect_va {
        let in_bounds = detect_va
            .checked_sub(build.base_va)
            .and_then(|off| off.checked_add(4))
            .is_some_and(|end| end <= build.unpacked_size);
        tr.check(in_bounds, || {
            format!("{owner}: native_bgo_detect_va 0x{detect_va:08X} out of MAIN image bounds")
        });
    }
    if let Some(flash_base) = build.native_bgo_flash_base_va
        && let (_, Some(tag_va)) = parse_bgo_geometry_tag(&build.native_bgo_geometry_id)
    {
        tr.check(flash_base == tag_va, || {
            format!(
                "{owner}: native_bgo_flash_base_va 0x{flash_base:08X} != tag '{}' base 0x{tag_va:08X}",
                build.native_bgo_geometry_id
            )
        });
    }
    if let Some(reset_spec) = &build.sata_bridge_reset_spec {
        tr.check(
            reset_spec.base_va == build.base_va && reset_spec.expected_size == build.unpacked_size,
            || {
                format!(
                    "{owner}: sata_bridge_reset_spec (base=0x{:08X}, size={}) must match MAIN (base=0x{:08X}, size={})",
                    reset_spec.base_va,
                    reset_spec.expected_size,
                    build.base_va,
                    build.unpacked_size
                )
            },
        );
        validate_sata_reset_spec(&owner, reset_spec, tr);
    }
    for site in &build.av_passthrough_sites {
        validate_patch_site(&owner, build.base_va, build.unpacked_size, site, tr);
    }
}

pub(super) fn validate_train_patch_specs(trn: &FirmwareTrainProfile, tr: &mut CheckTracker) {
    if let Some(reset_spec) = &trn.prog_sata_bridge_reset_spec {
        let owner = format!("train '{}' prog_sata_bridge_reset_spec", trn.id);
        validate_sata_reset_spec(&owner, reset_spec, tr);
    }
    if let Some(stage2) = &trn.stage2_boot_spec {
        validate_stage2_boot_spec(&trn.id, stage2, tr);
    }
    let mut seen_ids = HashSet::new();
    let mut seen_sizes = HashSet::new();
    for build in &trn.main_builds {
        let fold = build.build_id.trim().to_ascii_uppercase();
        tr.check(seen_ids.insert(fold.clone()), || {
            format!(
                "train '{}': duplicate main_build id '{}'",
                trn.id, build.build_id
            )
        });
        tr.check(seen_sizes.insert(build.unpacked_size), || {
            format!(
                "train '{}': duplicate main_build unpacked_size {} B (ambiguous dispatch)",
                trn.id, build.unpacked_size
            )
        });
        validate_main_build_spec(&trn.id, build, tr);
    }
}
