//! Packing routines for `PANA_DVD.FRM`, `PANA__SD.FRM`, and `DIGADIAG.FRM`.

use anyhow::{Result, bail};

use super::super::codec::{pad_to_block, write_padded_ascii, write_toc_entry_list_sum};
use super::super::extrhead::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, pack_extrhead_with_options, parse_extrhead_header,
};
use super::super::feistel::feistel_encrypt_buffer;
use super::types::{FrmContainerMode, FrmPackModule, FrmPackReport, PackedFrmModuleSummary};
use crate::constants::*;

/// Wraps an executable/diagnostic image into a plaintext `DIGADIAG.FRM` with configurable compression algo.
pub fn build_digadiag_frm_with_algo(
    payload: &[u8],
    dst_addr: u32,
    tag: &str,
    build_name: &str,
    algo: ExtrheadCompressionAlgo,
    adler_step: u32,
    total_size: usize,
) -> Result<(Vec<u8>, FrmPackReport)> {
    let mut container = pack_extrhead_with_options(
        payload,
        tag,
        dst_addr,
        build_name,
        ExtrheadCryptMode::Plain,
        algo,
        &PANA_FEISTEL_ROUND_KEY_32B,
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

    let hdr = parse_extrhead_header(&container[..PANA_EXTRHEAD_HEADER_SIZE])?;
    let report = FrmPackReport {
        mode: "digadiag".to_string(),
        total_bytes: container.len(),
        toc_region_size: PANA_EXTRHEAD_HEADER_SIZE,
        modules: vec![PackedFrmModuleSummary {
            index: 0,
            name: hdr.tag,
            module_offset: PANA_EXTRHEAD_HEADER_SIZE as u32,
            module_size: hdr.src_len,
            fvu_checksum_u32: hdr.header_checksum,
            list_sum_u16: 0,
        }],
    };
    Ok((container, report))
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
    build_digadiag_frm_with_algo(
        payload,
        dst_addr,
        tag,
        build_name,
        ExtrheadCompressionAlgo::Zlib,
        adler_step,
        total_size,
    )
}

/// Packs modules into a `PANA_DVD.FRM` or `PANA__SD.FRM` image using an explicit Feistel round key.
pub fn pack_frm_with_key(
    modules: &[FrmPackModule],
    mode: FrmContainerMode,
    toc_region_size: Option<usize>,
    round_key: &[u8; 32],
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

        let list_sum = write_toc_entry_list_sum(entry);

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
        feistel_encrypt_buffer(&mut image, round_key, &PANA_FEISTEL_SBOX_256);
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

/// Packs modules into a `PANA_DVD.FRM` (Feistel + 48B TOC) or `PANA__SD.FRM` (plaintext + 64B/48B TOC) image.
pub fn pack_frm(
    modules: &[FrmPackModule],
    mode: FrmContainerMode,
    toc_region_size: Option<usize>,
) -> Result<(Vec<u8>, FrmPackReport)> {
    pack_frm_with_key(modules, mode, toc_region_size, &PANA_FEISTEL_ROUND_KEY_32B)
}
