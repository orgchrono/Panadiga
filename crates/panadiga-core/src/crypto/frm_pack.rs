//! Panasonic `.FRM` Container Builders (`PANA_DVD.FRM`, `PANA__SD.FRM`, and `DIGADIAG.FRM`)
//!
//! Pure-Rust image packer matching `BOOT.BIN` (`FUN_440075aa`, `FUN_44007a12`,
//! `FUN_4400a733`, `FUN_4400a6d0`, and `FUN_4400a97c`).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::codec::{pad_to_block, parse_u32_radix, write_padded_ascii};
use super::extrhead::{ExtrheadCryptMode, pack_extrhead, parse_extrhead_header};
use super::feistel::{feistel_decrypt_buffer, feistel_encrypt_buffer};
use super::toc::{decrypt_and_parse_frm_toc, parse_sd_frm_toc};
use super::types::DecryptedFrmTocEntry;
use crate::constants::*;

/// Output container format for `.FRM` image generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrmContainerMode {
    Dvd,
    Sd,
    Sd48,
}

/// Input module descriptor for packing `PANA_DVD.FRM` or `PANA__SD.FRM`.
#[derive(Debug, Clone)]
pub struct FrmPackModule {
    pub name: String,
    pub payload: Vec<u8>,
    pub version_u16: u16,
    pub hardware_family: String,
    pub model_prefix: u8,
    pub target_flag: u8,
    pub model_group: String,
    pub tail_meta: [u8; 6],
}

impl FrmPackModule {
    /// Creates a module specification populated with EH55 defaults (`M6930`, `M`, `NP-309`).
    pub fn new(name: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            payload,
            version_u16: 0x0001,
            hardware_family: "M6930".to_string(),
            model_prefix: b'M',
            target_flag: 0,
            model_group: "NP-309".to_string(),
            tail_meta: [0u8; 6],
        }
    }
}

/// Summary of a module slot written into a packed `.FRM` container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackedFrmModuleSummary {
    pub index: usize,
    pub name: String,
    pub module_offset: u32,
    pub module_size: u32,
    pub fvu_checksum_u32: u32,
    pub list_sum_u16: u16,
}

/// Structured report emitted after packing a `PANA_DVD.FRM`, `PANA__SD.FRM`, or `DIGADIAG.FRM`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrmPackReport {
    pub mode: String,
    pub total_bytes: usize,
    pub toc_region_size: usize,
    pub modules: Vec<PackedFrmModuleSummary>,
}

/// Wraps an executable/diagnostic image into a plaintext `DIGADIAG.FRM` container padded to 128 KiB.
pub fn build_digadiag_frm(
    payload: &[u8],
    dst_addr: u32,
    tag: &str,
    build_name: &str,
    adler_step: u32,
    total_size: usize,
) -> Result<(Vec<u8>, FrmPackReport)> {
    let mut container = pack_extrhead(
        payload,
        tag,
        dst_addr,
        build_name,
        ExtrheadCryptMode::Plain,
        adler_step,
        1,
    )?;
    if container.len() > total_size {
        bail!(
            "DIGADIAG.FRM container ({} bytes) exceeds 0x{:X} byte limit",
            container.len(),
            total_size
        );
    }
    container.resize(total_size, 0);

    let hdr = parse_extrhead_header(&container[..0x40])?;
    let report = FrmPackReport {
        mode: "digadiag".to_string(),
        total_bytes: container.len(),
        toc_region_size: 0x40,
        modules: vec![PackedFrmModuleSummary {
            index: 0,
            name: hdr.tag,
            module_offset: 0x40,
            module_size: hdr.src_len,
            fvu_checksum_u32: hdr.header_checksum,
            list_sum_u16: 0,
        }],
    };
    Ok((container, report))
}

/// Packs modules into a `PANA_DVD.FRM` (Feistel + 48B TOC) or `PANA__SD.FRM` (plaintext + 64B/48B TOC) image.
pub fn pack_frm(
    modules: &[FrmPackModule],
    mode: FrmContainerMode,
    toc_region_size: Option<usize>,
) -> Result<(Vec<u8>, FrmPackReport)> {
    if modules.is_empty() {
        bail!("At least one module is required to pack an .FRM container");
    }

    let is_dvd = mode == FrmContainerMode::Dvd;
    let is_48b = matches!(mode, FrmContainerMode::Dvd | FrmContainerMode::Sd48);
    let stride = if is_48b {
        PANA_FRM_TOC_ENTRY_SIZE
    } else {
        PANA_SD_TOC_ENTRY_SIZE
    };
    let region_size = toc_region_size.unwrap_or(if is_48b {
        PANA_FRM_TOC_REGION_SIZE
    } else {
        PANA_SD_TOC_REGION_SIZE
    });

    if region_size < modules.len() * stride || (is_48b && !region_size.is_multiple_of(8)) {
        bail!(
            "Invalid TOC region size 0x{:X} for {} modules",
            region_size,
            modules.len()
        );
    }
    if !is_48b && modules.len() > PANA_SD_TOC_MAX_ENTRIES {
        bail!(
            "PANA__SD.FRM supports at most {} modules",
            PANA_SD_TOC_MAX_ENTRIES
        );
    }

    let mut image = vec![0u8; region_size];
    let mut summaries = Vec::with_capacity(modules.len());

    for (idx, module) in modules.iter().enumerate() {
        let mut mod_bytes = module.payload.clone();
        if is_48b {
            pad_to_block(&mut mod_bytes);
        }
        let mod_offset = image.len() as u32;
        let mod_size = mod_bytes.len() as u32;
        let fvu_sum = mod_bytes
            .iter()
            .fold(0u32, |acc, &b| acc.wrapping_add(u32::from(b)));

        let entry = &mut image[idx * stride..(idx + 1) * stride];
        write_padded_ascii(&mut entry[0x00..0x0A], &module.name, 0);
        entry[0x0A..0x0C].copy_from_slice(&module.version_u16.to_le_bytes());
        entry[0x0C..0x10].copy_from_slice(&mod_offset.to_le_bytes());
        write_padded_ascii(&mut entry[0x10..0x18], &module.hardware_family, 0);
        entry[0x20..0x24].copy_from_slice(&mod_size.to_le_bytes());
        entry[0x24..0x28].copy_from_slice(&fvu_sum.to_le_bytes());

        if is_48b {
            entry[0x18] = module.model_prefix;
            entry[0x19] = module.target_flag;
            write_padded_ascii(&mut entry[0x1A..0x20], &module.model_group, 0);
            entry[0x28..0x2E].copy_from_slice(&module.tail_meta);
        } else {
            entry[0x18] = module.target_flag;
            write_padded_ascii(&mut entry[0x30..0x3C], &module.model_group, 0);
        }

        let sum_off = stride - 2;
        let list_sum = entry[..sum_off]
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
        entry[sum_off..stride].copy_from_slice(&list_sum.to_le_bytes());

        image.extend_from_slice(&mod_bytes);
        summaries.push(PackedFrmModuleSummary {
            index: idx,
            name: module.name.clone(),
            module_offset: mod_offset,
            module_size: mod_size,
            fvu_checksum_u32: fvu_sum,
            list_sum_u16: list_sum,
        });
    }

    if is_dvd {
        feistel_encrypt_buffer(
            &mut image,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );
    }

    let mode_label = match mode {
        FrmContainerMode::Dvd => "dvd",
        FrmContainerMode::Sd => "sd",
        FrmContainerMode::Sd48 => "sd48",
    };
    let report = FrmPackReport {
        mode: mode_label.to_string(),
        total_bytes: image.len(),
        toc_region_size: region_size,
        modules: summaries,
    };
    Ok((image, report))
}

/// Re-parses a packed `dvd`, `sd`, or `sd48` `.FRM` image, checking TOC checksums and payload FVU sums.
pub fn verify_packed_frm_roundtrip(
    bytes: &[u8],
    mode: FrmContainerMode,
    expected_modules: usize,
) -> Result<Vec<DecryptedFrmTocEntry>> {
    let entries = match mode {
        FrmContainerMode::Dvd => decrypt_and_parse_frm_toc(bytes),
        FrmContainerMode::Sd | FrmContainerMode::Sd48 => parse_sd_frm_toc(bytes),
    };
    if entries.len() < expected_modules {
        bail!(
            "Packed .FRM TOC yielded {} slot(s), expected at least {}",
            entries.len(),
            expected_modules
        );
    }

    let is_dvd = mode == FrmContainerMode::Dvd;
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
            feistel_decrypt_buffer(
                &mut payload_buf,
                &PANA_FEISTEL_ROUND_KEY_32B,
                &PANA_FEISTEL_SBOX_256,
            );
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
    let entries = decrypt_and_parse_frm_toc(dvd_bytes);
    if entries.is_empty()
        || entries
            .iter()
            .any(|e| !e.list_sum_valid || e.module_name.trim().is_empty())
    {
        bail!("Input does not contain a valid Feistel 48B PANA_DVD.FRM TOC");
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
        feistel_decrypt_buffer(
            &mut payload,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nvram::calculate_hashes;

    #[test]
    fn test_verify_packed_frm_roundtrip_for_sd_sd48_and_dvd() {
        let mut main_mod = FrmPackModule::new("MAIN3410PQ", vec![0xAB; 64]);
        main_mod.target_flag = 1;
        let mut prog_mod = FrmPackModule::new("PROG061 ", vec![0x11; 32]);
        prog_mod.target_flag = 0;

        for mode in [
            FrmContainerMode::Sd,
            FrmContainerMode::Sd48,
            FrmContainerMode::Dvd,
        ] {
            let (mut image, _report) = pack_frm(&[main_mod.clone(), prog_mod.clone()], mode, None)
                .expect("pack container");
            let verified = verify_packed_frm_roundtrip(&image, mode, 2).expect("verify roundtrip");
            assert_eq!(verified.len(), 2);
            assert_eq!(verified[0].target_flag, 1);
            assert!(verified[0].list_sum_valid);

            // Corrupt last payload byte -> checksum mismatch must fail verification.
            let last = image.len() - 1;
            image[last] ^= 0x55;
            assert!(verify_packed_frm_roundtrip(&image, mode, 2).is_err());
        }
    }

    #[test]
    fn test_convert_pana_dvd_to_factory_sd_synthetic_and_workspace_roundtrip() {
        let mut m_drv = FrmPackModule::new("DRV S211", vec![0x12u8; 64]);
        m_drv.version_u16 = 0x975E;
        let mut m_prog = FrmPackModule::new("PROG061 ", vec![0x34u8; 32]);
        m_prog.version_u16 = 0x1202;
        m_prog.target_flag = 1;
        let mut m_main = FrmPackModule::new("MAIN3060NP", vec![0x56u8; 96]);
        m_main.version_u16 = 0x96AA;
        let mods = vec![m_drv, m_prog, m_main];

        let (dvd_frm, _) = pack_frm(&mods, FrmContainerMode::Dvd, None).unwrap();
        let (expected_sd, _) = pack_frm(&mods, FrmContainerMode::Sd, None).unwrap();
        let (converted_sd, _) = convert_pana_dvd_to_factory_sd(&dvd_frm).unwrap();
        assert_eq!(converted_sd, expected_sd);

        let ws_dvd = resolve_workspace_root().join("firmware_and_images/PANA_DVD.FRM");
        if ws_dvd.is_file() {
            let dvd_real = std::fs::read(&ws_dvd).unwrap();
            let (sd_real, _) = convert_pana_dvd_to_factory_sd(&dvd_real).unwrap();
            assert_eq!(sd_real.len(), 7_533_792);
            let (_, sha) = calculate_hashes(&sd_real);
            assert_eq!(
                sha,
                "edbe9ea8ec8a4905c5c891d37484205e2897c12a50f73d272a268ebe91bb7a1a"
            );
        }
    }
}
