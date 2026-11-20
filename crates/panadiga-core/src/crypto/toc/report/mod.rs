#![deny(unsafe_code)]

//! TOC report assembly, ECB sub-block validation, and module catalog construction.

mod format;

pub use format::format_toc_dump_lines;

use anyhow::{Result, bail};

use super::super::types::{FrmModuleInfo, FrmTocDumpReport, TocEntry, TocReport};
use super::{TocLayout, decrypt_frm_toc_with_detected_key, parse_sd_frm_toc_with_layout};
use crate::constants::*;

/// Dissects the first 512 bytes of `PANA_DVD.FRM` to verify the 48-byte TOC entries.
pub fn analyze_toc(data: &[u8]) -> TocReport {
    let subblock_1_hex = hex::encode_upper(PANA_FRM_TOC_SUBBLOCK_1);
    let subblock_2_hex = hex::encode_upper(PANA_FRM_TOC_SUBBLOCK_2);

    let mut match_1 = 0;
    let mut match_2 = 0;
    let max_toc_records = (data.len().min(512)) / PANA_FRM_TOC_ENTRY_SIZE;

    for r in 0..max_toc_records {
        let base = r * PANA_FRM_TOC_ENTRY_SIZE;
        if base + 0x18 <= data.len() && data[base + 0x10..base + 0x18] == PANA_FRM_TOC_SUBBLOCK_1 {
            match_1 += 1;
        }
        if base + 0x20 <= data.len() && data[base + 0x18..base + 0x20] == PANA_FRM_TOC_SUBBLOCK_2 {
            match_2 += 1;
        }
    }

    TocReport {
        entry_size_bytes: PANA_FRM_TOC_ENTRY_SIZE,
        total_examined_records: max_toc_records,
        common_subblock_1_hex: subblock_1_hex,
        common_subblock_1_matches: match_1,
        common_subblock_2_hex: subblock_2_hex,
        common_subblock_2_matches: match_2,
    }
}

fn build_raw_toc_entries(raw: &[u8]) -> (Vec<TocEntry>, bool) {
    let toc_definitions = [
        (
            0,
            0x0010,
            "Main CPU OS (DIGA Kernel)",
            "Matsushita UniPhier AM34 (32-bit RISC)",
            "EFDC1CDA13881BAB",
            "74A78ACD789B84FF",
        ),
        (
            1,
            0x0040,
            "DVD Drive Servo Controller",
            "Matsushita MN102 / Optical DSP",
            "EFDC1CDA13881BAB",
            "3B7E7EDBB6C57A53",
        ),
        (
            2,
            0x0070,
            "Front Panel / Timer (IC7001)",
            "Matsushita MN103S (AM3x Core Family)",
            "EFDC1CDA13881BAB",
            "74A78ACD789B84FF",
        ),
    ];

    let mut toc_entries = Vec::with_capacity(toc_definitions.len());
    let mut invariants_ok = true;

    for (idx, base_off, target_name, arch, exp_fam, exp_core) in toc_definitions {
        let entry_slice = &raw[base_off..base_off + PANA_FRM_TOC_ENTRY_SIZE];
        let mut subblocks = Vec::with_capacity(6);
        for b in 0..6 {
            let sb = &entry_slice[b * CIPHER_BLOCK_SIZE..(b + 1) * CIPHER_BLOCK_SIZE];
            subblocks.push(hex::encode_upper(sb));
        }

        let fam_id = subblocks[0].clone();
        let core_id = subblocks[1].clone();
        if fam_id != exp_fam || core_id != exp_core {
            invariants_ok = false;
        }

        toc_entries.push(TocEntry {
            index: idx,
            offset_hex: format!("0x{:04X}", base_off),
            target_name: target_name.to_string(),
            target_architecture: arch.to_string(),
            processor_family_id: fam_id,
            target_descriptor_id: core_id,
            raw_hex: hex::encode_upper(entry_slice),
            subblocks,
        });
    }

    (toc_entries, invariants_ok)
}

fn build_frm_module_catalog(file_size: usize) -> Vec<FrmModuleInfo> {
    vec![
        FrmModuleInfo {
            module_index: 1,
            name: "Módulo 1: Sub-sistema / OSD & Fontes".to_string(),
            target: "Front Panel / OSD / Sub-system Controller".to_string(),
            start_offset_hex: format!("0x{:08X}", PANA_FRM_MOD1_START_OFFSET),
            end_offset_hex: format!("0x{:08X}", PANA_FRM_MOD1_BOUNDARY),
            payload_size_bytes: PANA_FRM_MOD1_PAYLOAD_END - PANA_FRM_MOD1_START_OFFSET,
            padding_size_bytes: PANA_FRM_MOD1_BOUNDARY - PANA_FRM_MOD1_PAYLOAD_END,
            hdd_correlation: "Partição 2 do HD (LBA 131072 / 64 MB, 704 KB)".to_string(),
        },
        FrmModuleInfo {
            module_index: 2,
            name: "Módulo 2: Firmware do Drive Óptico (DSP/Servo)".to_string(),
            target: "Panasonic Optical Drive Servo Controller (MN102)".to_string(),
            start_offset_hex: format!("0x{:08X}", PANA_FRM_MOD2_START_OFFSET),
            end_offset_hex: format!("0x{:08X}", PANA_FRM_MOD2_BOUNDARY),
            payload_size_bytes: PANA_FRM_MOD2_PAYLOAD_END - PANA_FRM_MOD2_START_OFFSET,
            padding_size_bytes: PANA_FRM_MOD2_BOUNDARY - PANA_FRM_MOD2_PAYLOAD_END,
            hdd_correlation: "Microcódigo interno transferido ao drive via ATAPI".to_string(),
        },
        FrmModuleInfo {
            module_index: 3,
            name: "Módulo 3: Main CPU Operating System & Codecs".to_string(),
            target: "Panasonic AM34 / MN103S (UniPhier Core)".to_string(),
            start_offset_hex: format!("0x{:08X}", PANA_FRM_MOD3_START_OFFSET),
            end_offset_hex: format!("0x{:08X}", file_size),
            payload_size_bytes: file_size.saturating_sub(PANA_FRM_MOD3_START_OFFSET),
            padding_size_bytes: 0,
            hdd_correlation: "Partição 1 do HD (LBA 65536 / 32 MB, 5.89 MB)".to_string(),
        },
    ]
}

/// Pure function that dissects an in-memory `.FRM` container into a structured `FrmTocDumpReport`.
pub fn analyze_frm_toc_report(raw: &[u8], source_label: &str) -> Result<FrmTocDumpReport> {
    let file_size = raw.len();
    if file_size < PANA_FRM_PAYLOAD_OFFSET {
        bail!("FRM file too small ({file_size} bytes) to contain valid header and TOC");
    }

    let global_header_hex = hex::encode_upper(&raw[0..PANA_FRM_GLOBAL_HEADER_SIZE]);
    let (toc_entries, invariants_ok) = build_raw_toc_entries(raw);

    let (_kname, _rk, dvd_layout, decrypted_toc) = decrypt_frm_toc_with_detected_key(raw);
    let use_decrypted_dvd = decrypted_toc.iter().any(|e| e.list_sum_valid);
    let (sd_layout_opt, sd_toc) = if use_decrypted_dvd {
        (None, Vec::new())
    } else {
        parse_sd_frm_toc_with_layout(raw)
    };
    let use_sd_toc = !use_decrypted_dvd && sd_toc.iter().any(|e| e.list_sum_valid);

    let toc_format = if use_decrypted_dvd {
        dvd_layout.label()
    } else if use_sd_toc {
        sd_layout_opt.map_or("sd-plaintext", TocLayout::label)
    } else {
        "unknown"
    };

    let selected_toc = if use_sd_toc { sd_toc } else { decrypted_toc };

    Ok(FrmTocDumpReport {
        frm_file: source_label.to_string(),
        file_size_bytes: file_size,
        toc_format: toc_format.to_string(),
        global_header_hex,
        toc_entries,
        decrypted_toc: selected_toc,
        modules: build_frm_module_catalog(file_size),
        toc_invariants_valid: invariants_ok,
    })
}
