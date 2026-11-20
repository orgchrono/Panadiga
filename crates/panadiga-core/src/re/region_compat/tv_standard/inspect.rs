#![deny(unsafe_code)]

//! Strictly read-only / copy-on-write inspection of `DEV.BIN`/`ifodev2`, `MAIN`, `.FRM`,
//! and Spansion NVRAM (`SA134..SA141`) artifacts for TV/region calibration.

use anyhow::{Result, bail};

use crate::constants::{
    AM34_NVRAM_BASE, SPANSION_FLASH_SIZE_BYTES, SPANSION_NVRAM_BASE_OFFSET,
    SPANSION_NVRAM_END_OFFSET, SPANSION_NVRAM_SIZE_BYTES,
};
use crate::crypto::{decrypt_frm_toc_with_detected_key, parse_sd_frm_toc_with_layout};
use crate::nvram::{NvramCleanReport, calculate_hashes, inspect_partial_nvram_capture};

use super::super::dev_bin::{PANA_IFODEV2_SIZE, inspect_dev_bin_bytes};
use super::super::main_fmu::inspect_main_region_compat;
use super::mmio::build_tv_calibration_profile;
use super::types::{
    FrmTocInspectionSummary, Ifodev2TailMetadata, LbsRegionPresetId, TvCalibrationBundleReport,
    TvStandardKind,
};

const IFODEV2_TAIL_OFFSET: usize = 0x400;
const IFODEV2_TAIL_LEN: usize = 0x400;

fn inspect_frm_toc_summary(frm: &[u8]) -> Result<FrmTocInspectionSummary> {
    let (_, sha256) = calculate_hashes(frm);
    let (sd_layout, sd_entries) = parse_sd_frm_toc_with_layout(frm);
    let (layout, entries) = if let Some(l) = sd_layout {
        (l.label().to_string(), sd_entries)
    } else {
        let (_, _, dvd_l, dvd_entries) = decrypt_frm_toc_with_detected_key(frm);
        if dvd_entries.is_empty() {
            bail!(
                "Unrecognized .FRM container ({} B): neither valid plaintext SD TOC nor Feistel DVD TOC",
                frm.len()
            );
        }
        (dvd_l.label().to_string(), dvd_entries)
    };
    let all_list_sums_valid = !entries.is_empty() && entries.iter().all(|e| e.list_sum_valid);
    Ok(FrmTocInspectionSummary {
        layout,
        entry_count: entries.len(),
        all_list_sums_valid,
        entries,
        sha256,
    })
}

fn inspect_nvram_sa134_sa141(bytes: &[u8]) -> Result<NvramCleanReport> {
    let slice = if bytes.len() == SPANSION_FLASH_SIZE_BYTES {
        &bytes[SPANSION_NVRAM_BASE_OFFSET..SPANSION_NVRAM_END_OFFSET]
    } else if bytes.len() <= SPANSION_NVRAM_SIZE_BYTES {
        bytes
    } else {
        bail!(
            "Expected <= {SPANSION_NVRAM_SIZE_BYTES}-byte NVRAM slice or {SPANSION_FLASH_SIZE_BYTES}-byte 8 MiB flash dump, got {} B",
            bytes.len()
        );
    };
    inspect_partial_nvram_capture(slice, AM34_NVRAM_BASE, "SA134..SA141")
}

/// Inspects an optional bundle of `DEV.BIN`/`ifodev2`, `MAIN`, `.FRM`, and NVRAM `SA134..SA141`
/// bytes cleanly in read-only mode alongside the target [`super::TvCalibrationProfile`].
pub fn inspect_tv_calibration_bundle(
    preset: LbsRegionPresetId,
    standard_override: Option<TvStandardKind>,
    dev_bin_bytes: Option<&[u8]>,
    main_bytes: Option<&[u8]>,
    frm_bytes: Option<&[u8]>,
    nvram_or_flash_bytes: Option<&[u8]>,
) -> Result<TvCalibrationBundleReport> {
    let profile = build_tv_calibration_profile(preset, standard_override);
    let mut warnings = Vec::new();

    let (dev_bin, ifodev2_tail) = match dev_bin_bytes {
        Some(bytes) => {
            let rep = inspect_dev_bin_bytes(bytes)?;
            let tail = if bytes.len() == PANA_IFODEV2_SIZE {
                let tail_slice =
                    &bytes[IFODEV2_TAIL_OFFSET..IFODEV2_TAIL_OFFSET + IFODEV2_TAIL_LEN];
                let (_, sha256) = calculate_hashes(tail_slice);
                warnings.push(
                    "ifodev2 0x400..0x7FF payload inspected read-only (metadata_and_dev_oem_only=true; silent byte mutation blocked).".to_string(),
                );
                Some(Ifodev2TailMetadata {
                    offset_range_hex: "0x0400..0x0800".to_string(),
                    byte_len: tail_slice.len(),
                    non_ff_bytes: tail_slice.iter().filter(|&&b| b != 0xFF).count(),
                    prefix_hex: hex::encode(&tail_slice[..16]),
                    sha256,
                    metadata_and_dev_oem_only: true,
                })
            } else {
                None
            };
            if rep.model != profile.chassis_family && preset != LbsRegionPresetId::CustomInspected {
                warnings.push(format!(
                    "DEV.BIN/ifodev2 model '{}' differs from preset '{}' expected family '{}'",
                    rep.model,
                    preset.as_str(),
                    profile.chassis_family
                ));
            }
            (Some(rep), tail)
        }
        None => (None, None),
    };

    let main_region = main_bytes.map(inspect_main_region_compat).transpose()?;
    let frm_toc = frm_bytes.map(inspect_frm_toc_summary).transpose()?;
    let nvram_sa134_sa141 = nvram_or_flash_bytes
        .map(inspect_nvram_sa134_sa141)
        .transpose()?;

    Ok(TvCalibrationBundleReport {
        profile,
        dev_bin,
        ifodev2_tail,
        metadata_and_dev_oem_only: true,
        main_region,
        frm_toc,
        nvram_sa134_sa141,
        warnings,
    })
}
