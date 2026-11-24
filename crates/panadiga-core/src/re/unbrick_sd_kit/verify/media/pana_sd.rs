#![deny(unsafe_code)]

//! Container-level buffer validation for `PANA__SD.FRM` (`sd48` and `sd64` TOCs + embedded modules).

use crate::constants::PANA_SD_TOC_ENTRY_SIZE;
use crate::crypto::{ExtrheadCryptMode, TocLayout, parse_sd_frm_toc_with_layout, unpack_extrhead};
use crate::hdd::{
    HDD_CMD_M7920_200GB_BLOCKS, HDD_CMD_M7920_400GB_BLOCKS, inspect_main_buffer_capacity,
};

use super::super::super::super::bgo_unbrick::{
    PROG061_EXPECTED_SIZE, inspect_prog061_bgo_sites, prog_has_bgo_table_marker,
};
use super::super::super::super::main_bgo_retarget::{
    MAIN_BGO_STATE_MISMATCH, inspect_or_verify_main_bgo_geometry,
};
use super::super::super::super::main_passthrough::{MAIN_BASE_VA, inspect_main_passthrough_sites};
use super::super::super::super::sata_bridge_fix::{
    OFF_PROG061_SATA_BRIDGE_RESET_WAIT, inspect_main_sata_bridge_reset_site,
    inspect_prog061_sata_bridge_reset_site,
};
use super::super::super::super::sata_hdd_autoboot::{
    OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE, inspect_prog061_hdd_autoboot_site,
};
use super::super::super::super::ssot_patches::find_main_build_by_size;
use super::super::super::compose::SdLoaderProfile;
use super::super::{SdUnbrickCheck, check};
use super::dual_parity::{is_factory_unpatched_prog061, slice_module_and_check_fvu};

pub(crate) struct PanaSdInspection {
    pub present: bool,
    pub toc_entries: usize,
    pub list_sums_ok: bool,
    pub prog061_mode: Option<String>,
    pub prog061_bgo_geometry: Option<String>,
    pub prog061_sata_bridge_fix: Option<String>,
    pub prog061_hdd_autoboot: Option<String>,
    pub factory_unpatched_prog061: bool,
    pub main_build: Option<String>,
    pub main_passthrough_mode: Option<String>,
    pub main_sata_bridge_fix: Option<String>,
    pub main_bgo_geometry: Option<String>,
    pub main_hdd_dynamic: Option<bool>,
    pub main_hdd_capacity_label: Option<String>,
    pub main_extrhead_checksum_ok: Option<bool>,
    pub main_extrhead_src_addr: Option<u32>,
    pub main_effective_blocks_2kb: Option<u32>,
    pub detected_profile: Option<SdLoaderProfile>,
    pub check: SdUnbrickCheck,
}

pub(crate) fn verify_pana_sd_buffer(bytes_opt: Option<&[u8]>) -> PanaSdInspection {
    let Some(sd_bytes) = bytes_opt else {
        return PanaSdInspection {
            present: false,
            toc_entries: 0,
            list_sums_ok: false,
            prog061_mode: None,
            prog061_bgo_geometry: None,
            prog061_sata_bridge_fix: None,
            prog061_hdd_autoboot: None,
            factory_unpatched_prog061: false,
            main_build: None,
            main_passthrough_mode: None,
            main_sata_bridge_fix: None,
            main_bgo_geometry: None,
            main_hdd_dynamic: None,
            main_hdd_capacity_label: None,
            main_extrhead_checksum_ok: None,
            main_extrhead_src_addr: None,
            main_effective_blocks_2kb: None,
            detected_profile: None,
            check: check(
                "pana_sd_container",
                "warn",
                "PANA__SD.FRM absent (DIGADIAG-only SD)",
                "Valid if you only want DIGADIAG.FRM (PROG061 BGO/UART monitor) to run from SD without reflashing MAIN/DRV",
            ),
        };
    };

    let (toc_layout, toc) = parse_sd_frm_toc_with_layout(sd_bytes);
    let toc_entries = toc.len();
    let list_sums_ok = !toc.is_empty()
        && toc.iter().all(|e| {
            e.list_sum_valid
                && (e.module_offset as usize)
                    .checked_add(e.module_size as usize)
                    .is_some_and(|end| end <= sd_bytes.len())
        });

    let mut fvu_sums_ok = !toc.is_empty();
    let mut prog061_mode = None;
    let mut prog061_bgo_geometry = None;
    let mut prog061_sata_bridge_fix = None;
    let mut prog061_hdd_autoboot = None;
    let mut factory_unpatched_prog061 = false;
    let mut main_build = None;
    let mut main_passthrough_mode = None;
    let mut main_sata_bridge_fix = None;
    let mut main_bgo_geometry = None;
    let mut main_hdd_dynamic = None;
    let mut main_hdd_capacity_label = None;
    let mut main_extrhead_checksum_ok = None;
    let mut main_extrhead_src_addr = None;
    let mut main_effective_blocks_2kb = None;

    for e in &toc {
        let Some((mod_slice, slot_fvu_ok)) = slice_module_and_check_fvu(sd_bytes, e) else {
            fvu_sums_ok = false;
            continue;
        };
        if !slot_fvu_ok {
            fvu_sums_ok = false;
        }
        let name_up = e.module_name.trim().to_ascii_uppercase();
        if name_up.starts_with("PROG") {
            let unpacked_prog = if mod_slice.starts_with(b"EXTRHEAD") {
                unpack_extrhead(mod_slice, ExtrheadCryptMode::Plain)
                    .ok()
                    .map(|u| u.decompressed)
            } else {
                super::super::super::super::eh55_bgo::unpack_nor_autoboot_capsule(mod_slice)
            };
            let prog_buf = unpacked_prog.as_deref().unwrap_or(mod_slice);
            if let Ok(b) = inspect_prog061_bgo_sites(prog_buf) {
                if prog_has_bgo_table_marker(prog_buf) {
                    prog061_bgo_geometry = Some(b.bgo_target_geometry);
                }
                prog061_mode = Some(b.detected_mode);
            }
            if prog_buf.len() == PROG061_EXPECTED_SIZE {
                let is_populated_slice = |slice_opt: Option<&[u8]>| {
                    slice_opt.is_some_and(|s| {
                        !s.iter().all(|&b| b == 0xFF) && !s.iter().all(|&b| b == 0x00)
                    })
                };
                let site_slice = prog_buf.get(
                    OFF_PROG061_SATA_BRIDGE_RESET_WAIT..OFF_PROG061_SATA_BRIDGE_RESET_WAIT + 11,
                );
                let should_inspect =
                    prog_has_bgo_table_marker(prog_buf) || is_populated_slice(site_slice);
                if should_inspect && let Ok(st) = inspect_prog061_sata_bridge_reset_site(prog_buf) {
                    prog061_sata_bridge_fix = Some(st.state);
                }
                let hook_slice = prog_buf.get(
                    OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE..OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE + 7,
                );
                if is_populated_slice(hook_slice)
                    && let Ok(s2) = inspect_prog061_hdd_autoboot_site(prog_buf)
                {
                    prog061_hdd_autoboot = Some(s2.state);
                }
            }
            if is_factory_unpatched_prog061(prog_buf, prog061_mode.as_deref()) {
                factory_unpatched_prog061 = true;
            }
        } else if name_up.starts_with("MAIN") {
            let looks_like_extrhead = mod_slice.starts_with(b"EXTRHEAD")
                || (mod_slice.len() >= 0x40
                    && crate::crypto::extrhead::detect_extrhead_key_and_double(mod_slice, None)
                        .is_some());
            match unpack_extrhead(mod_slice, ExtrheadCryptMode::Main) {
                Ok(u) => {
                    main_extrhead_checksum_ok = Some(u.checksum_valid && u.footer_present);
                    main_extrhead_src_addr = Some(u.header.src_addr);
                    main_build = Some(if u.header.build_name.is_empty() {
                        e.module_name.trim().to_string()
                    } else {
                        u.header.build_name.clone()
                    });
                    main_passthrough_mode = inspect_main_passthrough_sites(&u.decompressed)
                        .ok()
                        .map(|p| p.detected_mode);
                    if find_main_build_by_size(u.decompressed.len()).is_some() {
                        if let Ok(st) = inspect_main_sata_bridge_reset_site(&u.decompressed) {
                            main_sata_bridge_fix = Some(st.state);
                        }
                        main_bgo_geometry = Some(
                            inspect_or_verify_main_bgo_geometry(&u.decompressed).map_or_else(
                                |_| MAIN_BGO_STATE_MISMATCH.to_string(),
                                |st| st.state,
                            ),
                        );
                    }
                    if let Ok(cap) = inspect_main_buffer_capacity(
                        &u.decompressed,
                        MAIN_BASE_VA,
                        "EXTRHEAD_CONTAINER",
                        Some(u.header.build_name.clone()),
                        "PANA__SD.FRM:MAIN",
                    ) {
                        main_hdd_dynamic = Some(cap.is_dynamic_unclamped);
                        main_effective_blocks_2kb = Some(
                            cap.slot_400gb_blocks_2kb
                                .max(cap.slot_200gb_blocks_2kb)
                                .max(cap.slot_fallback_blocks_2kb),
                        );
                        main_hdd_capacity_label = Some(if cap.is_dynamic_unclamped {
                            "dynamic".to_string()
                        } else if (cap.slot_400gb_blocks_2kb == HDD_CMD_M7920_400GB_BLOCKS
                            && cap.slot_200gb_blocks_2kb == HDD_CMD_M7920_200GB_BLOCKS)
                            || cap.slot_fallback_blocks_2kb == HDD_CMD_M7920_200GB_BLOCKS
                        {
                            "factory".to_string()
                        } else {
                            format!("{}GB", cap.effective_default_limit_gb.round() as u32)
                        });
                    }
                }
                Err(_) if looks_like_extrhead => {
                    main_extrhead_checksum_ok = Some(false);
                }
                Err(_) => {}
            }
        }
    }

    let detected_profile = match toc_layout {
        Some(TocLayout::Sd48B) => Some(SdLoaderProfile::Ver061RawSd48),
        Some(TocLayout::Sd64B) => Some(SdLoaderProfile::Ver063ExtrheadSd64),
        _ => None,
    };
    let stride = toc_layout.map_or(PANA_SD_TOC_ENTRY_SIZE, TocLayout::stride_bytes);
    let integrity_ok = list_sums_ok && fvu_sums_ok && detected_profile.is_some();
    let chk_severity = if integrity_ok { "pass" } else { "fail" };
    let chk_title = match toc_layout {
        Some(layout) => layout.title(),
        None => "PANA__SD.FRM (Invalid/Corrupt SD TOC)",
    };
    let profile_label = detected_profile.map_or("unknown", SdLoaderProfile::as_str);
    let chk = check(
        "pana_sd_container",
        chk_severity,
        chk_title,
        format!(
            "entries={}, list_sums_ok={}, fvu_sums_ok={}, entry_stride={}B, loader_profile={}, prog061={}, main_build={}, tbc_mode={}",
            toc_entries,
            list_sums_ok,
            fvu_sums_ok,
            stride,
            profile_label,
            prog061_mode.as_deref().unwrap_or("n/a"),
            main_build.as_deref().unwrap_or("n/a"),
            main_passthrough_mode
                .as_deref()
                .unwrap_or("factory_or_non_3410pq"),
        ),
    );

    PanaSdInspection {
        present: true,
        toc_entries,
        list_sums_ok,
        prog061_mode,
        prog061_bgo_geometry,
        prog061_sata_bridge_fix,
        prog061_hdd_autoboot,
        factory_unpatched_prog061,
        main_build,
        main_passthrough_mode,
        main_sata_bridge_fix,
        main_bgo_geometry,
        main_hdd_dynamic,
        main_hdd_capacity_label,
        main_extrhead_checksum_ok,
        main_extrhead_src_addr,
        main_effective_blocks_2kb,
        detected_profile: (list_sums_ok && fvu_sums_ok)
            .then_some(detected_profile)
            .flatten(),
        check: chk,
    }
}
