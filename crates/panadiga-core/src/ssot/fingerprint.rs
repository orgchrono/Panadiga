#![deny(unsafe_code)]

//! Structural, offset-aware NOR flash & firmware container fingerprinting.
//!
//! Eliminates false model claims from shared cross-model `DRV` payload strings (such as
//! `DMR-EH55` present at `0x09_0000` inside `DMR-ES35V`) and refuses silent model guessing
//! on dump byte size alone.

use serde::{Deserialize, Serialize};

use super::catalog::is_valid_nor_flash_size;
use super::registry::HardwareProfileRegistry;
use crate::constants::{SPANSION_MICROSECTOR_SIZE, SPANSION_NVRAM_SIZE_BYTES};

/// Shared ATAPI drive module window inside MN103S NOR dumps (`0x08_0000..0x13_0000`) where
/// unrelated model strings frequently appear and must never drive model identification.
const SHARED_DRV_SKIP_START: usize = 0x0008_0000;
const SHARED_DRV_SKIP_END: usize = 0x0013_0000;

/// Single structural or ASCII probe candidate discovered during fingerprinting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintCandidate {
    pub model_id: String,
    pub matched_probe: String,
    pub confidence_bps: u16,
}

/// Complete forensic fingerprint result for a NOR dump or firmware slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NorFingerprintReport {
    pub detected_model: Option<String>,
    pub display_label: String,
    pub is_size_only: bool,
    pub matched_probe: Option<String>,
    pub has_eemul_f1_marker: bool,
    pub dump_size_bytes: u64,
    pub candidates: Vec<FingerprintCandidate>,
}

pub(crate) fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

fn slice_contains_any_dmr_needle<'a>(
    chunk: &[u8],
    needles: &[(&'a str, &'a str)],
) -> Option<(&'a str, &'a str)> {
    let mut best: Option<(usize, &'a str, &'a str)> = None;
    let max_idx = chunk.len().saturating_sub(4);
    for pos in 0..max_idx {
        let w4 = &chunk[pos..pos + 4];
        if !matches!(w4[0], b'D' | b'd') || !w4.eq_ignore_ascii_case(b"DMR-") {
            continue;
        }
        let tail = &chunk[pos..];
        for &(needle, canonical) in needles {
            if tail.len() >= needle.len()
                && tail[..needle.len()].eq_ignore_ascii_case(needle.as_bytes())
            {
                best = match best {
                    Some((prev_len, _, _)) if prev_len >= needle.len() => best,
                    _ => Some((needle.len(), needle, canonical)),
                };
                break;
            }
        }
    }
    best.map(|(_, needle, canonical)| (needle, canonical))
}

/// Scans `data` in a single pass while skipping the shared DRV window (`0x08_0000..0x13_0000`).
fn find_dmr_model_outside_drv<'a>(
    data: &[u8],
    needles: &[(&'a str, &'a str)],
    is_nor: bool,
) -> Option<(&'a str, &'a str)> {
    if is_nor && data.len() > SHARED_DRV_SKIP_START {
        let head = &data[..SHARED_DRV_SKIP_START];
        if let Some(hit) = slice_contains_any_dmr_needle(head, needles) {
            return Some(hit);
        }
        if data.len() > SHARED_DRV_SKIP_END {
            return slice_contains_any_dmr_needle(&data[SHARED_DRV_SKIP_END..], needles);
        }
        None
    } else {
        slice_contains_any_dmr_needle(data, needles)
    }
}

/// Detects EEMUL `0xF1` active micro-sector tags at byte `+5` in the top 64 KiB of NOR dumps.
#[must_use]
pub fn detect_eemul_f1_marker(data: &[u8]) -> bool {
    if !is_valid_nor_flash_size(data.len() as u64) || data.len() < SPANSION_NVRAM_SIZE_BYTES {
        return false;
    }
    let tail = &data[data.len() - SPANSION_NVRAM_SIZE_BYTES..];
    tail.chunks(SPANSION_MICROSECTOR_SIZE)
        .any(|sec| sec.len() > 5 && sec[5] == 0xF1 && !sec[..5].iter().all(|&b| b == 0xF1))
}

fn push_candidate(
    candidates: &mut Vec<FingerprintCandidate>,
    model_id: &str,
    probe: &str,
    bps: u16,
) {
    if !candidates.iter().any(|c| c.model_id == model_id) {
        candidates.push(FingerprintCandidate {
            model_id: model_id.to_string(),
            matched_probe: probe.to_string(),
            confidence_bps: bps,
        });
    }
}

/// Evaluates loader banners, EEMUL signatures, and safe outside-DRV identifiers on a binary slice.
///
/// Never invents a model ID strictly from byte length (`is_size_only = true`, `detected_model = None`).
#[must_use]
pub fn fingerprint_nor_or_firmware(data: &[u8]) -> NorFingerprintReport {
    let is_nor = is_valid_nor_flash_size(data.len() as u64);
    let has_eemul_f1_marker = detect_eemul_f1_marker(data);
    let models = HardwareProfileRegistry::builtin().list_models();
    let mut candidates = Vec::new();

    // 1. Offset-bounded loader fingerprints declared inside `profiles/firmware_trains/*.json`.
    for resolved in &models {
        let Some(train) = &resolved.firmware_train else {
            continue;
        };
        let Some(fp) = &train.loader_fingerprint else {
            continue;
        };
        let start = fp.offset_start as usize;
        let end = (fp.offset_end as usize).min(data.len());
        if start < end && contains_bytes(&data[start..end], fp.ascii_marker.as_bytes()) {
            push_candidate(
                &mut candidates,
                &resolved.manifest.model_id,
                &format!("loader_banner:{}@0x{:X}", fp.ascii_marker, start),
                9800,
            );
        }
    }

    // 2. Secondary structural LSI DoMiNo / VxWorks signatures on non-MN103S 4 MiB dumps.
    if candidates.is_empty() && is_nor {
        let has_wind_river = contains_bytes(data, b"Wind River");
        let has_lsi_clsi = contains_bytes(data, b"CLsiPMABS");
        if has_wind_river || has_lsi_clsi {
            let probe = if has_wind_river && has_lsi_clsi {
                "vxworks_wind_river+CLsiPMABS"
            } else if has_wind_river {
                "vxworks_wind_river"
            } else {
                "lsi_CLsiPMABS"
            };
            push_candidate(&mut candidates, "DMR-EZ485V", probe, 9200);
        }
    }

    // 3. Model ID / exact alias ASCII match outside shared DRV (`0x08_0000..0x13_0000`).
    if candidates.is_empty() {
        let mut needles: Vec<(&str, &str)> = Vec::new();
        for resolved in &models {
            let primary = resolved.manifest.model_id.as_str();
            needles.push((primary, primary));
            for alias in &resolved.manifest.exact_aliases {
                needles.push((alias.as_str(), primary));
            }
        }
        needles.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(b.0)));

        if let Some((needle, canonical)) = find_dmr_model_outside_drv(data, &needles, is_nor) {
            push_candidate(
                &mut candidates,
                canonical,
                &format!("ascii_non_drv:{needle}"),
                8200,
            );
        }
    }

    if let Some(best) = candidates.first() {
        return NorFingerprintReport {
            detected_model: Some(best.model_id.clone()),
            display_label: best.model_id.clone(),
            is_size_only: false,
            matched_probe: Some(best.matched_probe.clone()),
            has_eemul_f1_marker,
            dump_size_bytes: data.len() as u64,
            candidates,
        };
    }

    let display_label = if is_nor {
        format!(
            "Unidentified NOR image ({} MiB — size-only)",
            data.len() / (1024 * 1024)
        )
    } else {
        "Unidentified Firmware Image".to_string()
    };

    NorFingerprintReport {
        detected_model: None,
        display_label,
        is_size_only: true,
        matched_probe: None,
        has_eemul_f1_marker,
        dump_size_bytes: data.len() as u64,
        candidates,
    }
}

/// Safe replacement returning either the confirmed canonical model ID or an honest unidentified label.
#[must_use]
pub fn detect_diga_model_from_slice_safe(data: &[u8]) -> String {
    let report = fingerprint_nor_or_firmware(data);
    report.detected_model.unwrap_or(report.display_label)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plant(buf: &mut [u8], offset: usize, bytes: &[u8]) {
        buf[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    #[test]
    fn skips_shared_drv_so_es35v_is_not_misidentified_as_eh55() {
        let mut dump = vec![0xFFu8; 8 * 1024 * 1024];
        plant(&mut dump, 0x0003_F000, b"Loader ver0.18 [06/03/13]");
        plant(&mut dump, 0x0009_0000, b"DMR-EH55S159");
        plant(
            &mut dump,
            0x007F_2000,
            &[0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0xF1, 0xFF, 0xFF],
        );

        let fp = fingerprint_nor_or_firmware(&dump);
        assert_eq!(fp.detected_model.as_deref(), Some("DMR-ES35V"));
        assert!(!fp.is_size_only);
        assert!(fp.has_eemul_f1_marker);
    }

    #[test]
    fn identifies_eh55_loader_ver061_with_eemul_marker() {
        let mut dump = vec![0xFFu8; 8 * 1024 * 1024];
        plant(&mut dump, 0x0003_F000, b"Loader ver0.61 [06/03/06]");
        plant(
            &mut dump,
            0x007F_0000,
            &[0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0xF1, 0xFF, 0xFF],
        );

        let fp = fingerprint_nor_or_firmware(&dump);
        assert_eq!(fp.detected_model.as_deref(), Some("DMR-EH55"));
        assert!(!fp.is_size_only);
        assert!(fp.has_eemul_f1_marker);
    }

    #[test]
    fn identifies_ez485v_lsi_vxworks_without_eemul() {
        let mut dump = vec![0xFFu8; 4 * 1024 * 1024];
        plant(&mut dump, 0x0000_40D6, b"ZEBRA 64");
        plant(&mut dump, 0x0036_0010, b"CLsiPMABS");

        let fp = fingerprint_nor_or_firmware(&dump);
        assert_eq!(fp.detected_model.as_deref(), Some("DMR-EZ485V"));
        assert!(!fp.is_size_only);
        assert!(!fp.has_eemul_f1_marker);
    }

    #[test]
    fn refuses_size_only_guess_on_blank_or_unlabeled_nor_dumps() {
        for size in [4 * 1024 * 1024, 8 * 1024 * 1024] {
            let blank = vec![0x55u8; size];
            let fp = fingerprint_nor_or_firmware(&blank);
            assert_eq!(fp.detected_model, None);
            assert!(fp.is_size_only);
            assert!(!fp.has_eemul_f1_marker);
            assert!(fp.display_label.contains("size-only"));
        }

        // Planted DMR-EH55 strictly inside DRV window alone must NOT trigger DMR-EH55 guess.
        let mut drv_only = vec![0xFFu8; 8 * 1024 * 1024];
        plant(&mut drv_only, 0x0009_0000, b"DMR-EH55");
        let fp = fingerprint_nor_or_firmware(&drv_only);
        assert_eq!(fp.detected_model, None);
        assert!(fp.is_size_only);
    }
}
