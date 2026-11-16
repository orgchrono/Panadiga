//! Pure immutable inspection & algebraic validation of SD unbrick media (`DIGADIAG.FRM` + `PANA__SD.FRM`).

mod media;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

pub(crate) use self::media::{verify_digadiag_buffer, verify_pana_sd_buffer};

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

pub(crate) fn check(
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
