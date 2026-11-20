#![deny(unsafe_code)]

//! Container-level buffer validation for `PANA__SD.FRM` (`sd48` and `sd64` TOCs + embedded modules).

use crate::constants::PANA_SD_TOC_ENTRY_SIZE;
use crate::crypto::{ExtrheadCryptMode, TocLayout, parse_sd_frm_toc_with_layout, unpack_extrhead};
use crate::hdd::inspect_main_buffer_capacity;

use super::super::super::super::bgo_unbrick::inspect_prog061_bgo_sites;
use super::super::super::super::main_passthrough::{MAIN_BASE_VA, inspect_main_passthrough_sites};
use super::super::super::compose::SdLoaderProfile;
use super::super::{SdUnbrickCheck, check};
use super::dual_parity::{is_factory_unpatched_prog061, slice_module_and_check_fvu};

pub(crate) struct PanaSdInspection {
    pub present: bool,
    pub toc_entries: usize,
    pub list_sums_ok: bool,
    pub prog061_mode: Option<String>,
    pub factory_unpatched_prog061: bool,
    pub main_build: Option<String>,
    pub main_passthrough_mode: Option<String>,
    pub main_hdd_dynamic: Option<bool>,
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
            factory_unpatched_prog061: false,
            main_build: None,
            main_passthrough_mode: None,
            main_hdd_dynamic: None,
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
    let mut factory_unpatched_prog061 = false;
    let mut main_build = None;
    let mut main_passthrough_mode = None;
    let mut main_hdd_dynamic = None;

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
                None
            };
            let prog_buf = unpacked_prog.as_deref().unwrap_or(mod_slice);
            prog061_mode = inspect_prog061_bgo_sites(prog_buf)
                .ok()
                .map(|b| b.detected_mode);
            if is_factory_unpatched_prog061(prog_buf, prog061_mode.as_deref()) {
                factory_unpatched_prog061 = true;
            }
        } else if name_up.starts_with("MAIN")
            && let Ok(u) = unpack_extrhead(mod_slice, ExtrheadCryptMode::Main)
        {
            main_build = Some(if u.header.build_name.is_empty() {
                e.module_name.trim().to_string()
            } else {
                u.header.build_name.clone()
            });
            main_passthrough_mode = inspect_main_passthrough_sites(&u.decompressed)
                .ok()
                .map(|p| p.detected_mode);
            main_hdd_dynamic = inspect_main_buffer_capacity(
                &u.decompressed,
                MAIN_BASE_VA,
                "EXTRHEAD_CONTAINER",
                Some(u.header.build_name.clone()),
                "PANA__SD.FRM:MAIN",
            )
            .ok()
            .map(|cap| cap.is_dynamic_unclamped);
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
        factory_unpatched_prog061,
        main_build,
        main_passthrough_mode,
        main_hdd_dynamic,
        detected_profile: (list_sums_ok && fvu_sums_ok)
            .then_some(detected_profile)
            .flatten(),
        check: chk,
    }
}
