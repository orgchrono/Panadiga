#![deny(unsafe_code)]

//! Universal SSOT lookup and fail-closed patch dispatch over `profiles/firmware_trains/*.json`.
//!
//! Eliminates per-build size/VA switches in Rust by resolving `MainBuildSpec`,
//! `SataBridgeResetSpec`, `HddAutobootSpec`, and native BGO checks directly from the
//! compile-time validated [`HardwareProfileRegistry`].

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use crate::ssot::registry::{
    BinaryPatchSiteSpec, FirmwareTrainProfile, HardwareProfileRegistry, MainBuildSpec,
    SataBridgeResetSpec, parse_hex_bytes,
};

use super::main_bgo_retarget::MainBgoGeometryStatus;
use super::patch_spec::{PatchApplyAction, PatchSpec, apply_patch_specs_in_place};
use super::sata_bridge_fix::{SataBridgeResetFixMode, SataBridgeResetSiteStatus, inspect_site};
use super::sata_bridge_stage2::HddAutobootTrainSpec;

/// Resolves the firmware train + `MainBuildSpec` matching an unpacked `MAIN` byte length.
#[must_use]
pub fn find_main_build_by_size(
    size: usize,
) -> Option<(&'static FirmwareTrainProfile, &'static MainBuildSpec)> {
    HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .find_map(|train| {
            train
                .main_builds
                .iter()
                .find(|build| build.unpacked_size as usize == size)
                .map(|build| (train, build))
        })
}

/// Resolves the firmware train + `MainBuildSpec` matching a `MAIN` build ID (case-insensitive).
#[must_use]
pub fn find_main_build_by_id(
    build_id: &str,
) -> Option<(&'static FirmwareTrainProfile, &'static MainBuildSpec)> {
    let key = build_id.trim();
    HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .find_map(|train| {
            train
                .main_builds
                .iter()
                .find(|build| build.build_id.eq_ignore_ascii_case(key))
                .map(|build| (train, build))
        })
}

/// Resolves the SSOT `PROG` SATA-bridge reset spec by bootloader profile key (`ver061`, etc.).
#[must_use]
pub fn find_prog_sata_reset_spec(
    loader_profile_key: &str,
) -> Option<(&'static FirmwareTrainProfile, &'static SataBridgeResetSpec)> {
    let key = loader_profile_key.trim();
    HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .find(|train| train.loader_profile_key == key)
        .and_then(|train| {
            train
                .prog_sata_bridge_reset_spec
                .as_ref()
                .map(|spec| (train, spec))
        })
}

/// Converts a declarative SSOT `HddAutobootSpec` into a runtime [`HddAutobootTrainSpec`].
#[must_use]
pub fn hdd_autoboot_spec_for_loader(loader_profile_key: &str) -> Option<HddAutobootTrainSpec> {
    let key = loader_profile_key.trim();
    let train = HardwareProfileRegistry::builtin()
        .bundle()
        .firmware_trains
        .values()
        .find(|t| t.loader_profile_key == key)?;
    let s = train.hdd_autoboot_spec.as_ref()?;
    let hook_vec = parse_hex_bytes(&s.hook_expect_hex).ok()?;
    let hook_expect: [u8; 7] = hook_vec.try_into().ok()?;
    let sym = &s.symbols;

    Some(HddAutobootTrainSpec {
        train_id: train.id.as_str(),
        base_va: s.base_va,
        expected_size: s.expected_size as usize,
        hook_offset: s.hook_offset as usize,
        hook_expect,
        payload_offset: s.payload_offset as usize,
        require_reset_fix_1000ms: s.require_reset_fix_1000ms,
        uart_abort_polls: s.uart_abort_polls,
        uart_poll_ticks_10ms: s.uart_poll_ticks_10ms,
        hdd_primary_lba: s.hdd_primary_lba,
        hdd_mirror_lba_delta: s.hdd_mirror_lba_delta,
        hdd_probe_sectors: s.hdd_probe_sectors,
        feistel_crypt_span: s.feistel_crypt_span,
        main_va_min: s.main_va_min,
        main_va_max: s.main_va_max,
        dcache_purge_base: s.dcache_purge_base,
        dcache_purge_len: s.dcache_purge_len,
        sd_info_bss_va: s.sd_info_bss_va,
        hdd_dma_buf_va: s.hdd_dma_buf_va,
        fvu_load_uncached_va: s.fvu_load_uncached_va,
        norfrom_fvu_base_va: s.norfrom_fvu_base_va,
        dcache_purge_va: sym.dcache_purge_va,
        uart0_getc_nb_va: sym.uart0_getc_nb_va,
        uart0_printf_va: sym.uart0_printf_va,
        check_update_flag_va: sym.check_update_flag_va,
        hdd_max_lba_va: sym.hdd_max_lba_va,
        hdd_read_va: sym.hdd_read_va,
        sd_init_va: sym.sd_init_va,
        stop_timer_irq_va: sym.stop_timer_irq_va,
        feistel_decrypt_va: sym.feistel_decrypt_va,
        extrhead_get_size_va: sym.extrhead_get_size_va,
        extrhead_check_hdr_va: sym.extrhead_check_hdr_va,
        extrhead_is_comp_va: sym.extrhead_is_comp_va,
        extrhead_get_src_va: sym.extrhead_get_src_va,
        extrhead_get_entry_va: sym.extrhead_get_entry_va,
        extrhead_uncomp_va: sym.extrhead_uncomp_va,
        fat_mount_va: sym.fat_mount_va,
        fat_open_va: sym.fat_open_va,
        fat_close_va: sym.fat_close_va,
        disable_irq_va: sym.disable_irq_va,
        memcpy_va: sym.memcpy_va,
        sleep_10ms_va: sym.sleep_10ms_va,
    })
}

#[deprecated(note = "Use hdd_autoboot_spec_for_loader")]
#[must_use]
pub fn stage2_boot_spec_for_loader(loader_profile_key: &str) -> Option<HddAutobootTrainSpec> {
    hdd_autoboot_spec_for_loader(loader_profile_key)
}

fn parse_site_pair(site: &BinaryPatchSiteSpec) -> Result<(Vec<u8>, Vec<u8>)> {
    let expect = parse_hex_bytes(&site.expect_hex)
        .map_err(|e| anyhow::anyhow!("Invalid expect_hex on '{}': {e}", site.name))?;
    let replace = parse_hex_bytes(&site.replace_hex)
        .map_err(|e| anyhow::anyhow!("Invalid replace_hex on '{}': {e}", site.name))?;
    Ok((expect, replace))
}

/// Executes a closure against an ephemeral [`PatchSpec`] built from an SSOT `BinaryPatchSiteSpec`.
fn with_patch_spec<T>(
    base_va: u32,
    expected_size: Option<usize>,
    site: &BinaryPatchSiteSpec,
    f: impl FnOnce(&PatchSpec<'_>) -> Result<T>,
) -> Result<T> {
    let (expect, replace) = parse_site_pair(site)?;
    let spec = PatchSpec {
        name: &site.name,
        description: &site.description,
        va: site.va,
        base_va,
        expect: &expect,
        replace: &replace,
        required_len: expected_size,
        input_sha256: None,
    };
    f(&spec)
}

/// Inspects a SATA-bridge hard-reset timing site defined declaratively in SSOT.
pub(crate) fn inspect_ssot_sata_reset_site(
    buf: &[u8],
    reset_spec: &SataBridgeResetSpec,
) -> Result<SataBridgeResetSiteStatus> {
    with_patch_spec(
        reset_spec.base_va,
        Some(reset_spec.expected_size as usize),
        &reset_spec.site,
        |patch| inspect_site(&reset_spec.target_name, buf, patch),
    )
}

/// Applies or restores a SATA-bridge hard-reset timing site defined declaratively in SSOT.
pub(crate) fn apply_ssot_sata_reset_fix_in_place(
    buf: &mut [u8],
    reset_spec: &SataBridgeResetSpec,
    mode: SataBridgeResetFixMode,
) -> Result<()> {
    with_patch_spec(
        reset_spec.base_va,
        Some(reset_spec.expected_size as usize),
        &reset_spec.site,
        |patch| {
            let action = match mode {
                SataBridgeResetFixMode::Wait1000Ms => PatchApplyAction::ApplyReplace,
                SataBridgeResetFixMode::FactoryOriginal => PatchApplyAction::RestoreExpect,
            };
            apply_patch_specs_in_place(buf, &[(patch, action)])
        },
    )
}

/// Verifies native BGO flash base alignment of an unpacked `MAIN` build from its SSOT [`MainBuildSpec`].
pub fn verify_main_native_bgo_from_spec(
    buf: &[u8],
    build: &MainBuildSpec,
) -> Result<MainBgoGeometryStatus> {
    if buf.len() != build.unpacked_size as usize {
        bail!(
            "Invalid MAIN{} size {} B (expected {} B)",
            build.build_id,
            buf.len(),
            build.unpacked_size
        );
    }
    let detect_va = build.native_bgo_detect_va.with_context(|| {
        format!(
            "MAIN{} SSOT entry lacks native_bgo_detect_va",
            build.build_id
        )
    })?;
    let expected_flash = build.native_bgo_flash_base_va.with_context(|| {
        format!(
            "MAIN{} SSOT entry lacks native_bgo_flash_base_va",
            build.build_id
        )
    })?;
    let off = detect_va
        .checked_sub(build.base_va)
        .map(|v| v as usize)
        .filter(|&o| o + 4 <= buf.len())
        .with_context(|| {
            format!(
                "MAIN{} native_bgo_detect_va 0x{detect_va:08X} out of bounds",
                build.build_id
            )
        })?;
    let lit = LittleEndian::read_u32(&buf[off..off + 4]);
    if lit != expected_flash {
        bail!(
            "MAIN{} BGO base literal mismatch at 0x{detect_va:08X}: got 0x{lit:08X}, expected 0x{expected_flash:08X}",
            build.build_id
        );
    }
    Ok(MainBgoGeometryStatus {
        state: build.native_bgo_geometry_id.clone(),
        geometry_id: Some(build.native_bgo_geometry_id.clone()),
        table_va_hex: format!("0x{detect_va:08X}"),
        bgo_flash_base_hex: format!("0x{lit:08X}"),
        nfd_bss_end_hex: "0x00000000".to_string(),
    })
}
