//! Verification and Feistel DVD-to-SD conversion helpers for `.FRM` containers.

use anyhow::{Context, Result, bail};

use super::super::codec::parse_u32_radix;
use super::super::feistel::feistel_decrypt_buffer;
use super::super::toc::{decrypt_frm_toc_with_detected_key, parse_sd_frm_toc};
use super::super::types::DecryptedFrmTocEntry;
use super::pack::pack_frm;
use super::types::{FrmContainerMode, FrmPackModule, FrmPackReport};
use crate::constants::*;

/// Re-parses a packed `dvd`, `sd`, or `sd48` `.FRM` image, checking TOC checksums and payload FVU sums.
pub fn verify_packed_frm_roundtrip(
    bytes: &[u8],
    mode: FrmContainerMode,
    expected_modules: usize,
) -> Result<Vec<DecryptedFrmTocEntry>> {
    let (detected_key, entries) = match mode {
        FrmContainerMode::Dvd => {
            let (_, rk, _, parsed) = decrypt_frm_toc_with_detected_key(bytes);
            (Some(rk), parsed)
        }
        FrmContainerMode::Sd | FrmContainerMode::Sd48 => (None, parse_sd_frm_toc(bytes)),
    };
    if entries.len() < expected_modules {
        bail!(
            "Packed .FRM TOC yielded {} slot(s), expected at least {}",
            entries.len(),
            expected_modules
        );
    }

    let is_dvd = mode == FrmContainerMode::Dvd;
    let dvd_key = detected_key.unwrap_or(PANA_FEISTEL_ROUND_KEY_32B);
    for entry in &entries[..expected_modules] {
        if !entry.list_sum_valid {
            bail!(
                "Invalid TOC list_sum on slot {} ('{}'): expected 0x{:04X}, got 0x{:04X}",
                entry.index,
                entry.module_name,
                entry.calculated_list_sum_u16,
                entry.list_sum_u16
            );
        }
        if entry.module_name.trim().is_empty() {
            bail!("Empty module name on active TOC slot {}", entry.index);
        }
        let start = entry.module_offset as usize;
        let len = entry.module_size as usize;
        let end = start
            .checked_add(len)
            .context("TOC module range overflow")?;
        if end > bytes.len() {
            bail!(
                "Module '{}' ({:#X}..{:#X}) exceeds container length {:#X}",
                entry.module_name,
                start,
                end,
                bytes.len()
            );
        }
        let mut payload_buf;
        let payload_slice: &[u8] = if is_dvd {
            payload_buf = bytes[start..end].to_vec();
            feistel_decrypt_buffer(&mut payload_buf, &dvd_key, &PANA_FEISTEL_SBOX_256);
            &payload_buf
        } else {
            &bytes[start..end]
        };
        let actual_sum = payload_slice
            .iter()
            .fold(0u32, |acc, &b| acc.wrapping_add(u32::from(b)));
        if actual_sum != entry.fvu_checksum_u32 {
            bail!(
                "FVU byte-sum mismatch for '{}': TOC 0x{:08X} vs computed 0x{:08X}",
                entry.module_name,
                entry.fvu_checksum_u32,
                actual_sum
            );
        }
    }

    Ok(entries.into_iter().take(expected_modules).collect())
}

/// Unpacks a Feistel-encrypted `PANA_DVD.FRM` image into `FrmPackModule` descriptors preserving factory TOC fields.
pub fn unpack_dvd_frm_to_modules(dvd_bytes: &[u8]) -> Result<Vec<FrmPackModule>> {
    let (_key_name, round_key, _layout, entries) = decrypt_frm_toc_with_detected_key(dvd_bytes);
    if entries.is_empty()
        || entries
            .iter()
            .any(|e| !e.list_sum_valid || e.module_name.trim().is_empty())
    {
        bail!("Input does not contain a valid Feistel PANA_DVD.FRM TOC");
    }

    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let start = entry.module_offset as usize;
        let end = start
            .checked_add(entry.module_size as usize)
            .context("PANA_DVD.FRM module offset overflow")?;
        if end > dvd_bytes.len() || entry.module_size == 0 {
            bail!(
                "PANA_DVD.FRM module '{}' range 0x{:X}..0x{:X} exceeds container ({} B)",
                entry.module_name,
                start,
                end,
                dvd_bytes.len()
            );
        }
        let mut payload = dvd_bytes[start..end].to_vec();
        feistel_decrypt_buffer(&mut payload, &round_key, &PANA_FEISTEL_SBOX_256);
        let ver_u32 = parse_u32_radix(&entry.version_u16_hex)?;
        let mut module = FrmPackModule::new(entry.module_name, payload);
        module.version_u16 = u16::try_from(ver_u32).context("TOC version_u16 out of range")?;
        module.hardware_family = entry.hardware_family;
        module.target_flag = entry.target_flag;
        module.model_group = entry.model_group;
        out.push(module);
    }
    Ok(out)
}

/// Converts a Feistel-encrypted `PANA_DVD.FRM` container into a factory plaintext 64B-TOC `PANA__SD.FRM`.
pub fn convert_pana_dvd_to_factory_sd(dvd_bytes: &[u8]) -> Result<(Vec<u8>, FrmPackReport)> {
    let modules = unpack_dvd_frm_to_modules(dvd_bytes)?;
    pack_frm(&modules, FrmContainerMode::Sd, None)
}
