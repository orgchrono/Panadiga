#![deny(unsafe_code)]

//! Packing routines for `PANA_DVD.FRM`, `PANA__SD.FRM`, and `DIGADIAG.FRM`.

use anyhow::{Result, bail};

use super::super::codec::{
    compute_fvu_byte_sum, pad_to_block, write_padded_ascii, write_toc_entry_list_sum,
};
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

struct PayloadPlacement {
    mod_offset: u32,
    mod_size: u32,
    fvu_sum: u32,
}

fn write_toc_slot(
    entry: &mut [u8],
    use_dvd_layout: bool,
    module: &FrmPackModule,
    hardware_family: &str,
    placement: &PayloadPlacement,
) -> u16 {
    write_padded_ascii(&mut entry[0x00..0x0A], &module.name, 0);
    entry[0x0A..0x0C].copy_from_slice(&module.version_u16.to_le_bytes());
    entry[0x0C..0x10].copy_from_slice(&placement.mod_offset.to_le_bytes());
    write_padded_ascii(&mut entry[0x10..0x18], hardware_family, 0);
    entry[0x20..0x24].copy_from_slice(&placement.mod_size.to_le_bytes());
    entry[0x24..0x28].copy_from_slice(&placement.fvu_sum.to_le_bytes());

    if use_dvd_layout {
        entry[0x18] = module.model_prefix;
        entry[0x19] = module.target_flag;
        write_padded_ascii(&mut entry[0x1A..0x20], &module.model_group, 0);
        entry[0x28..0x2E].copy_from_slice(&module.tail_meta);
    } else {
        entry[0x18] = module.target_flag;
        write_padded_ascii(&mut entry[0x30..0x3C], &module.model_group, 0);
    }

    write_toc_entry_list_sum(entry)
}

fn record_toc_slot(
    image: &mut [u8],
    slot_idx: usize,
    stride: usize,
    use_dvd_layout: bool,
    module: &FrmPackModule,
    hardware_family: &str,
    placement: &PayloadPlacement,
) -> PackedFrmModuleSummary {
    let entry = &mut image[slot_idx * stride..(slot_idx + 1) * stride];
    let list_sum = write_toc_slot(entry, use_dvd_layout, module, hardware_family, placement);
    PackedFrmModuleSummary {
        index: slot_idx,
        name: module.name.clone(),
        module_offset: placement.mod_offset,
        module_size: placement.mod_size,
        fvu_checksum_u32: placement.fvu_sum,
        list_sum_u16: list_sum,
    }
}

/// Packs modules into a `PANA_DVD.FRM` (`Dvd` 48B / `Dvd64B` 64B) or `PANA__SD.FRM` (`Sd` / `Sd48`) image.
pub fn pack_frm_with_key(
    modules: &[FrmPackModule],
    mode: FrmContainerMode,
    toc_region_size: Option<usize>,
    round_key: &[u8; 32],
) -> Result<(Vec<u8>, FrmPackReport)> {
    if modules.is_empty() {
        bail!("At least one module is required to pack an .FRM container");
    }

    let is_dvd = matches!(mode, FrmContainerMode::Dvd | FrmContainerMode::Dvd64B);
    let is_64b_stride = matches!(mode, FrmContainerMode::Sd | FrmContainerMode::Dvd64B);
    let use_dvd_layout = matches!(
        mode,
        FrmContainerMode::Dvd | FrmContainerMode::Dvd64B | FrmContainerMode::Sd48
    );
    let stride = if is_64b_stride {
        PANA_SD_TOC_ENTRY_SIZE
    } else {
        PANA_FRM_TOC_ENTRY_SIZE
    };
    let region_size = toc_region_size.unwrap_or(if is_64b_stride {
        PANA_SD_TOC_REGION_SIZE
    } else {
        PANA_FRM_TOC_REGION_SIZE
    });

    let total_slots: usize = modules
        .iter()
        .map(|m| 1 + m.hardware_family_aliases.len())
        .sum();
    if region_size < total_slots * stride || (is_dvd && !region_size.is_multiple_of(8)) {
        bail!(
            "Invalid TOC region size 0x{:X} for {} TOC slots",
            region_size,
            total_slots
        );
    }
    if !is_dvd && is_64b_stride && total_slots > PANA_SD_TOC_MAX_ENTRIES {
        bail!(
            "PANA__SD.FRM supports at most {} TOC slots",
            PANA_SD_TOC_MAX_ENTRIES
        );
    }

    let mut image = vec![0u8; region_size];
    let mut summaries = Vec::with_capacity(total_slots);
    let mut placements = Vec::with_capacity(modules.len());

    // Pass 1: append each unique module payload once and emit primary TOC slots.
    for (idx, module) in modules.iter().enumerate() {
        let mut mod_bytes = module.payload.clone();
        if is_dvd || !is_64b_stride {
            pad_to_block(&mut mod_bytes);
        }
        let placement = PayloadPlacement {
            mod_offset: image.len() as u32,
            mod_size: mod_bytes.len() as u32,
            fvu_sum: compute_fvu_byte_sum(&mod_bytes),
        };
        image.extend_from_slice(&mod_bytes);
        summaries.push(record_toc_slot(
            &mut image,
            idx,
            stride,
            use_dvd_layout,
            module,
            &module.hardware_family,
            &placement,
        ));
        placements.push(placement);
    }

    // Pass 2: emit grouped alias TOC slots sharing the already-appended module payloads.
    let max_aliases = modules
        .iter()
        .map(|m| m.hardware_family_aliases.len())
        .max()
        .unwrap_or(0);
    let mut next_slot = modules.len();
    for alias_idx in 0..max_aliases {
        for (module, placement) in modules.iter().zip(placements.iter()) {
            if let Some(alias_family) = module.hardware_family_aliases.get(alias_idx) {
                summaries.push(record_toc_slot(
                    &mut image,
                    next_slot,
                    stride,
                    use_dvd_layout,
                    module,
                    alias_family,
                    placement,
                ));
                next_slot += 1;
            }
        }
    }

    if is_dvd {
        feistel_encrypt_buffer(&mut image, round_key, &PANA_FEISTEL_SBOX_256);
    }

    let mode_label = match mode {
        FrmContainerMode::Dvd => "dvd",
        FrmContainerMode::Dvd64B => "dvd64b",
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
