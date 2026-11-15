//! Pure immutable inspection & algebraic validation of SD unbrick media (`DIGADIAG.FRM` + `PANA__SD.FRM`).

use serde::{Deserialize, Serialize};

use crate::constants::{
    PANA_DIGADIAG_DEFAULT_LOAD_ADDR, PANA_DIGADIAG_FRM_SIZE, PANA_SD_TOC_ENTRY_SIZE,
};
use crate::crypto::{ExtrheadCryptMode, TocLayout, parse_sd_frm_toc_with_layout, unpack_extrhead};
use crate::hdd::inspect_main_buffer_capacity;

use super::super::bgo_unbrick::inspect_prog061_bgo_sites;
use super::super::main_passthrough::{MAIN_BASE_VA, inspect_main_passthrough_sites};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdUnbrickCheck {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdMediaVerificationReport {
    pub sd_root_dir: String,
    pub overall_status: String,
    pub digadiag_present: bool,
    pub digadiag_exact_size_ok: bool,
    pub digadiag_dst_addr_hex: Option<String>,
    pub digadiag_bgo_mode: Option<String>,
    pub pana_sd_present: bool,
    pub pana_sd_toc_entries: usize,
    pub pana_sd_list_sums_ok: bool,
    pub pana_sd_prog061_mode: Option<String>,
    pub pana_sd_main_build: Option<String>,
    pub pana_sd_main_passthrough_mode: Option<String>,
    pub pana_sd_main_hdd_dynamic: Option<bool>,
    pub checks: Vec<SdUnbrickCheck>,
}

fn check(
    id: &str,
    severity: &str,
    title: impl Into<String>,
    detail: impl Into<String>,
) -> SdUnbrickCheck {
    SdUnbrickCheck {
        id: id.to_string(),
        severity: severity.to_string(),
        title: title.into(),
        detail: detail.into(),
    }
}

pub(crate) fn verify_digadiag_buffer(
    bytes_opt: Option<Result<&[u8], &str>>,
) -> (bool, bool, Option<String>, Option<String>, SdUnbrickCheck) {
    let bytes = match bytes_opt {
        None => {
            return (
                false,
                false,
                None,
                None,
                check(
                    "digadiag_container",
                    "fail",
                    "DIGADIAG.FRM missing",
                    "Required at SD root for BOOT.BIN Service/Diag SDDIAG boot",
                ),
            );
        }
        Some(Err(err)) => {
            return (
                true,
                false,
                None,
                None,
                check("digadiag_container", "fail", "DIGADIAG.FRM read error", err),
            );
        }
        Some(Ok(b)) => b,
    };

    let exact_sz = bytes.len() == PANA_DIGADIAG_FRM_SIZE;
    if !bytes.starts_with(b"EXTRHEAD") {
        return (
            true,
            exact_sz,
            None,
            None,
            check(
                "digadiag_container",
                "fail",
                "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)",
                "DIGADIAG.FRM must start with plaintext 'EXTRHEAD' (BOOT.BIN SDDIAG requirement)",
            ),
        );
    }

    match unpack_extrhead(bytes, ExtrheadCryptMode::Plain) {
        Ok(u) => {
            let dst_hex = Some(format!("0x{:08X}", u.header.dst_addr));
            let bgo_mode = inspect_prog061_bgo_sites(&u.decompressed)
                .ok()
                .map(|r| r.detected_mode);
            let ok = exact_sz
                && u.checksum_valid
                && u.footer_present
                && u.header.dst_addr == PANA_DIGADIAG_DEFAULT_LOAD_ADDR;
            let chk = check(
                "digadiag_container",
                if ok { "pass" } else { "fail" },
                "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)",
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
            (true, exact_sz, dst_hex, bgo_mode, chk)
        }
        Err(e) => (
            true,
            exact_sz,
            None,
            None,
            check(
                "digadiag_container",
                "fail",
                "DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)",
                format!("Failed to unpack plaintext EXTRHEAD: {e}"),
            ),
        ),
    }
}

pub(crate) struct PanaSdInspection {
    pub present: bool,
    pub toc_entries: usize,
    pub list_sums_ok: bool,
    pub prog061_mode: Option<String>,
    pub main_build: Option<String>,
    pub main_passthrough_mode: Option<String>,
    pub main_hdd_dynamic: Option<bool>,
    pub check: SdUnbrickCheck,
}

pub(crate) fn verify_pana_sd_buffer(bytes_opt: Option<&[u8]>) -> PanaSdInspection {
    let Some(sd_bytes) = bytes_opt else {
        return PanaSdInspection {
            present: false,
            toc_entries: 0,
            list_sums_ok: false,
            prog061_mode: None,
            main_build: None,
            main_passthrough_mode: None,
            main_hdd_dynamic: None,
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

    let mut prog061_mode = None;
    let mut main_build = None;
    let mut main_passthrough_mode = None;
    let mut main_hdd_dynamic = None;

    for e in &toc {
        let start = e.module_offset as usize;
        let Some(end) = start.checked_add(e.module_size as usize) else {
            continue;
        };
        let Some(mod_slice) = sd_bytes.get(start..end) else {
            continue;
        };
        let name_up = e.module_name.trim().to_ascii_uppercase();
        if name_up.starts_with("PROG") {
            prog061_mode = inspect_prog061_bgo_sites(mod_slice)
                .ok()
                .map(|b| b.detected_mode);
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

    let is_64b_layout = matches!(toc_layout, Some(TocLayout::Sd64B));
    let is_48b_obsolete = matches!(toc_layout, Some(TocLayout::Sd48B));
    let stride = toc_layout.map_or(PANA_SD_TOC_ENTRY_SIZE, TocLayout::stride_bytes);
    let chk_severity = if list_sums_ok && is_64b_layout {
        "pass"
    } else {
        "fail"
    };
    let chk_title = match toc_layout {
        Some(layout) => layout.title(),
        None => "PANA__SD.FRM (Invalid/Corrupt SD TOC)",
    };
    let chk_suffix = if is_48b_obsolete {
        ", rejected_by_boot_bin=48B_OPCAO_3_OBSOLETE"
    } else {
        ""
    };
    let chk = check(
        "pana_sd_container",
        chk_severity,
        chk_title,
        format!(
            "entries={}, list_sums_ok={}, entry_stride={}B{}, prog061={}, main_build={}, tbc_mode={}",
            toc_entries,
            list_sums_ok,
            stride,
            chk_suffix,
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
        main_build,
        main_passthrough_mode,
        main_hdd_dynamic,
        check: chk,
    }
}

/// Pure function verifying in-memory `DIGADIAG.FRM` and optional `PANA__SD.FRM` buffers.
pub fn verify_sd_buffers(
    sd_root_label: &str,
    digadiag_opt: Option<Result<&[u8], &str>>,
    pana_sd_opt: Option<&[u8]>,
) -> SdMediaVerificationReport {
    let (
        digadiag_present,
        digadiag_exact_size_ok,
        digadiag_dst_addr_hex,
        digadiag_bgo_mode,
        diag_chk,
    ) = verify_digadiag_buffer(digadiag_opt);

    let sd = verify_pana_sd_buffer(pana_sd_opt);
    let hw_warn = check(
        "sd_hw_91006",
        "warn",
        "BOOT.BIN SD-91006 Hardware Gate (1-bit -> 4-bit ACMD6)",
        "Requires legacy SDSC card (<= 2 GiB, FAT16/FAT12) AND intact SD DAT1..DAT3 traces (P6801/P6802 <-> P53501). BOOT.BIN has no UART RX console so Pico cannot force 1-bit mode over P7501.",
    );
    let checks = vec![diag_chk, sd.check, hw_warn];

    let overall_status = if checks.iter().any(|c| c.severity == "fail") {
        "INCOMPLETE_OR_INVALID"
    } else if checks.iter().any(|c| c.severity == "warn") {
        "READY_WITH_HW_WARNINGS"
    } else {
        "READY"
    }
    .to_string();

    SdMediaVerificationReport {
        sd_root_dir: sd_root_label.to_string(),
        overall_status,
        digadiag_present,
        digadiag_exact_size_ok,
        digadiag_dst_addr_hex,
        digadiag_bgo_mode,
        pana_sd_present: sd.present,
        pana_sd_toc_entries: sd.toc_entries,
        pana_sd_list_sums_ok: sd.list_sums_ok,
        pana_sd_prog061_mode: sd.prog061_mode,
        pana_sd_main_build: sd.main_build,
        pana_sd_main_passthrough_mode: sd.main_passthrough_mode,
        pana_sd_main_hdd_dynamic: sd.main_hdd_dynamic,
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_non_extrhead_buffers_fail_cleanly() {
        let rep_missing = verify_sd_buffers("/mem/sd", None, None);
        assert_eq!(rep_missing.overall_status, "INCOMPLETE_OR_INVALID");
        assert!(!rep_missing.digadiag_present);

        let bad = vec![0xAAu8; 64];
        let rep_bad = verify_sd_buffers("/mem/sd", Some(Ok(&bad)), None);
        assert_eq!(rep_bad.overall_status, "INCOMPLETE_OR_INVALID");
        assert!(rep_bad.digadiag_present);
        assert!(!rep_bad.digadiag_exact_size_ok);
    }

    #[test]
    fn obsolete_48b_sd_toc_is_marked_fail_with_48b_stride() {
        let mut sd48 = vec![0u8; 0x1880];
        sd48[0..8].copy_from_slice(b"DRV S211");
        sd48[0x0C..0x10].copy_from_slice(&0x1800u32.to_le_bytes());
        sd48[0x20..0x24].copy_from_slice(&64u32.to_le_bytes());
        let sum: u16 = sd48[..0x2E]
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
        sd48[0x2E..0x30].copy_from_slice(&sum.to_le_bytes());

        let rep = verify_sd_buffers("/mem/sd48", None, Some(&sd48));
        assert!(rep.pana_sd_present);
        assert!(rep.pana_sd_list_sums_ok);
        assert_eq!(rep.overall_status, "INCOMPLETE_OR_INVALID");
        let sd_chk = rep
            .checks
            .iter()
            .find(|c| c.id == "pana_sd_container")
            .unwrap();
        assert_eq!(sd_chk.severity, "fail");
        assert!(sd_chk.title.contains("48B Obsolete"));
        assert!(sd_chk.detail.contains("entry_stride=48B"));
    }
}
