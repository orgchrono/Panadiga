#![deny(unsafe_code)]

//! Container-level buffer validation for `DIGADIAG.FRM` (`ver0.61` 256 KiB raw & `ver0.63` 128 KiB `EXTRHEAD`).

use crate::constants::{
    PANA_DIGADIAG_DEFAULT_LOAD_ADDR, PANA_DIGADIAG_FRM_SIZE, PANA_DIGADIAG_RAW_VER061_SIZE,
};
use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};

use super::super::super::super::bgo_unbrick::{
    inspect_prog061_bgo_sites, prog_has_bgo_table_marker,
};
use super::super::super::cleanroom_digadiag::CLEANROOM_DIGADIAG_MAGIC;
use super::super::super::compose::SdLoaderProfile;
use super::super::{SdUnbrickCheck, check};
use super::dual_parity::is_factory_unpatched_prog061;

pub(crate) struct DigadiagInspection {
    pub present: bool,
    pub exact_size_ok: bool,
    pub dst_addr_hex: Option<String>,
    pub bgo_mode: Option<String>,
    pub bgo_geometry: Option<String>,
    pub factory_unpatched_prog061: bool,
    pub detected_profile: Option<SdLoaderProfile>,
    pub check: SdUnbrickCheck,
}

fn looks_like_mn103s_entry(bytes: &[u8]) -> bool {
    matches!(
        bytes.first().copied(),
        Some(0xDC | 0xCC | 0xCA | 0xCF | 0xCB)
    ) && bytes.iter().take(64).any(|&b| b != 0x00 && b != 0xFF)
}

fn is_cleanroom_digadiag_payload(bytes: &[u8]) -> bool {
    bytes.len() == PANA_DIGADIAG_RAW_VER061_SIZE
        && bytes.get(0x10..0x10 + CLEANROOM_DIGADIAG_MAGIC.len()) == Some(CLEANROOM_DIGADIAG_MAGIC)
}

fn detect_digadiag_mode_and_geometry(payload: &[u8]) -> (Option<String>, Option<String>) {
    if is_cleanroom_digadiag_payload(payload) {
        (Some("CLEANROOM_PANAMON1_BARE_METAL".to_string()), None)
    } else if let Ok(r) = inspect_prog061_bgo_sites(payload) {
        let geo = prog_has_bgo_table_marker(payload).then_some(r.bgo_target_geometry);
        (Some(r.detected_mode), geo)
    } else {
        (None, None)
    }
}

fn verify_raw_ver061_digadiag(bytes: &[u8]) -> DigadiagInspection {
    let (bgo_mode, bgo_geometry) = detect_digadiag_mode_and_geometry(bytes);
    let factory_unpatched_prog061 = is_factory_unpatched_prog061(bytes, bgo_mode.as_deref());
    let valid_exec = bgo_mode.is_some() || looks_like_mn103s_entry(bytes);
    let dst_hex = Some(format!("0x{PANA_DIGADIAG_DEFAULT_LOAD_ADDR:08X}"));
    let chk = check(
        "digadiag_container",
        if valid_exec { "pass" } else { "fail" },
        "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG — Loader ver0.61)",
        if valid_exec {
            format!(
                "size={} B (raw_ver061_256k=true), dst=0x{:08X}, format=RAW_VER061, bgo_mode={}",
                bytes.len(),
                PANA_DIGADIAG_DEFAULT_LOAD_ADDR,
                bgo_mode.as_deref().unwrap_or("CUSTOM_RAW_256K"),
            )
        } else {
            "Raw 256 KiB DIGADIAG.FRM lacks valid PROG061 sites or MN103S entry instruction"
                .to_string()
        },
    );
    DigadiagInspection {
        present: true,
        exact_size_ok: valid_exec,
        dst_addr_hex: if valid_exec { dst_hex } else { None },
        bgo_mode,
        bgo_geometry,
        factory_unpatched_prog061,
        detected_profile: valid_exec.then_some(SdLoaderProfile::Ver061RawSd48),
        check: chk,
    }
}

pub(crate) fn verify_digadiag_buffer(bytes_opt: Option<Result<&[u8], &str>>) -> DigadiagInspection {
    let bytes = match bytes_opt {
        None => {
            return DigadiagInspection {
                present: false,
                exact_size_ok: false,
                dst_addr_hex: None,
                bgo_mode: None,
                bgo_geometry: None,
                factory_unpatched_prog061: false,
                detected_profile: None,
                check: check(
                    "digadiag_container",
                    "fail",
                    "DIGADIAG.FRM missing",
                    "Required at SD root for Loader ver0.61/ver0.63 SDDIAG boot",
                ),
            };
        }
        Some(Err(err)) => {
            return DigadiagInspection {
                present: true,
                exact_size_ok: false,
                dst_addr_hex: None,
                bgo_mode: None,
                bgo_geometry: None,
                factory_unpatched_prog061: false,
                detected_profile: None,
                check: check("digadiag_container", "fail", "DIGADIAG.FRM read error", err),
            };
        }
        Some(Ok(b)) => b,
    };

    if !bytes.starts_with(b"EXTRHEAD") {
        if bytes.len() == PANA_DIGADIAG_RAW_VER061_SIZE {
            return verify_raw_ver061_digadiag(bytes);
        }
        return DigadiagInspection {
            present: true,
            exact_size_ok: false,
            dst_addr_hex: None,
            bgo_mode: None,
            bgo_geometry: None,
            factory_unpatched_prog061: false,
            detected_profile: None,
            check: check(
                "digadiag_container",
                "fail",
                "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)",
                format!(
                    "Invalid DIGADIAG.FRM ({} B): expected 262,144 B raw (Loader ver0.61) or 131,072 B plaintext 'EXTRHEAD' (Loader ver0.63)",
                    bytes.len(),
                ),
            ),
        };
    }

    let exact_sz = bytes.len() == PANA_DIGADIAG_FRM_SIZE;
    match unpack_extrhead(bytes, ExtrheadCryptMode::Plain) {
        Ok(u) => {
            let dst_hex = Some(format!("0x{:08X}", u.header.dst_addr));
            let (bgo_mode, bgo_geometry) = detect_digadiag_mode_and_geometry(&u.decompressed);
            let factory_unpatched_prog061 =
                is_factory_unpatched_prog061(&u.decompressed, bgo_mode.as_deref());
            let ok = exact_sz
                && u.checksum_valid
                && u.footer_present
                && u.header.dst_addr == PANA_DIGADIAG_DEFAULT_LOAD_ADDR;
            let chk = check(
                "digadiag_container",
                if ok { "pass" } else { "fail" },
                "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG — Loader ver0.63)",
                format!(
                    "size={} B (exact_128k={}), tag='{}', dst=0x{:08X}, adler_ok={}, footer={}, bgo_mode={}",
                    bytes.len(),
                    exact_sz,
                    u.header.tag,
                    u.header.dst_addr,
                    u.checksum_valid,
                    u.footer_present,
                    bgo_mode.as_deref().unwrap_or("CUSTOM_PAYLOAD"),
                ),
            );
            DigadiagInspection {
                present: true,
                exact_size_ok: exact_sz,
                dst_addr_hex: dst_hex,
                bgo_mode,
                bgo_geometry,
                factory_unpatched_prog061,
                detected_profile: ok.then_some(SdLoaderProfile::Ver063ExtrheadSd64),
                check: chk,
            }
        }
        Err(e) => DigadiagInspection {
            present: true,
            exact_size_ok: exact_sz,
            dst_addr_hex: None,
            bgo_mode: None,
            bgo_geometry: None,
            factory_unpatched_prog061: false,
            detected_profile: None,
            check: check(
                "digadiag_container",
                "fail",
                "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)",
                format!("Failed to unpack plaintext EXTRHEAD: {e}"),
            ),
        },
    }
}
