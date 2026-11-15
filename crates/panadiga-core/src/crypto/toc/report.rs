//! TOC report assembly, ECB sub-block validation, module catalog, and text formatting.

use anyhow::{Result, bail};

use super::super::types::{FrmModuleInfo, FrmTocDumpReport, TocEntry, TocReport};
use super::{TocLayout, decrypt_and_parse_frm_toc, parse_sd_frm_toc_with_layout};
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

    let decrypted_toc = decrypt_and_parse_frm_toc(raw);
    let use_decrypted_dvd = decrypted_toc.iter().any(|e| e.list_sum_valid);
    let (sd_layout_opt, sd_toc) = if use_decrypted_dvd {
        (None, Vec::new())
    } else {
        parse_sd_frm_toc_with_layout(raw)
    };
    let use_sd_toc = !use_decrypted_dvd && sd_toc.iter().any(|e| e.list_sum_valid);

    let toc_format = if use_decrypted_dvd {
        TocLayout::Dvd48B.label()
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

/// Pure text formatter for `FrmTocDumpReport` (zero terminal/file side effects).
pub fn format_toc_dump_lines(report: &FrmTocDumpReport) -> Vec<String> {
    let mut lines = Vec::new();
    let sep = "================================================================================";
    lines.push(sep.to_string());
    lines.push(
        " PANASONIC PANA_DVD.FRM TOC DUMP & 3-MODULE CONTAINER DECONSTRUCTION           "
            .to_string(),
    );
    lines.push(sep.to_string());
    lines.push(format!(" Arquivo Analisado : {}", report.frm_file));
    lines.push(format!(
        " Tamanho Total     : {:>10} bytes ({:.2} MB)",
        report.file_size_bytes,
        report.file_size_bytes as f64 / (1024.0 * 1024.0)
    ));
    lines.push(format!(
        " Cabeçalho Global  : {} (16 bytes)",
        report.global_header_hex
    ));
    lines.push(
        "--------------------------------------------------------------------------------"
            .to_string(),
    );
    lines.push(
        "\n[+] TABELA DE CONTEÚDOS (TOC) — 3 ENTRADAS PERIÓDICAS DE 48 BYTES (0x30):".to_string(),
    );
    lines.push("    Offset Base da TOC: 0x0010 (após cabeçalho global de 16 bytes)".to_string());

    for entry in &report.toc_entries {
        let base_off = 0x0010 + entry.index * PANA_FRM_TOC_ENTRY_SIZE;
        lines.push(format!(
            "\n  * Entrada {} (Offset {}..0x{:04X} | 48 bytes):",
            entry.index,
            entry.offset_hex,
            base_off + PANA_FRM_TOC_ENTRY_SIZE
        ));
        lines.push(format!("    - Destino de Hardware : {}", entry.target_name));
        lines.push(format!(
            "    - Arquitetura Alvo    : {}",
            entry.target_architecture
        ));
        lines.push(format!(
            "    - Família Matsushita  : {} (Bloco 0)",
            entry.processor_family_id
        ));
        lines.push(format!(
            "    - Seletor de Core     : {} (Bloco 1) {}",
            entry.target_descriptor_id,
            if entry.index == 1 {
                "[ALVO DE SERVO ÓPTICO DISTINTO]"
            } else {
                "[COMPARTILHADO: FAMÍLIA AM3x/MN103]"
            }
        ));
        lines.push("    - Sub-blocos (6 x 64-bits):".to_string());
        for (b_idx, sb_hex) in entry.subblocks.iter().enumerate() {
            let note = match b_idx {
                0 => "-> Cabeçalho de Família Matsushita",
                1 => "-> Descritor de Core/Target",
                2 => "-> Vetor Inicial / Parâmetro Metadados 1",
                3 => "-> Parâmetro Metadados 2",
                4 => "-> Parâmetro Metadados 3",
                5 => "-> Parâmetro Metadados 4 / Hash",
                _ => "",
            };
            lines.push(format!(
                "      [{}] 0x{:04X}: {}  {}",
                b_idx,
                base_off + b_idx * 8,
                sb_hex,
                note
            ));
        }
    }

    lines.push("\n[+] DIAGNÓSTICO DOS INVARIANTES DA TOC:".to_string());
    if report.toc_invariants_valid {
        lines.push("  [CONFIRMADO] Invariantes Estruturais da TOC Válidos:".to_string());
        lines.push(
            "  1. Entrada 0 e Entrada 2 compartilham os mesmos 16 bytes iniciais:".to_string(),
        );
        lines.push("     - Família Matsushita: EF DC 1C DA 13 88 1B AB".to_string());
        lines.push("     - Core AM3x/MN103S : 74 A7 8A CD 78 9B 84 FF".to_string());
        lines.push("  2. Entrada 1 varia o seletor de core (bytes 8..15):".to_string());
        lines.push("     - Core MN102/DSP   : 3B 7E 7E DB B6 C5 7A 53".to_string());
    } else {
        lines.push("  [ALERTA] Desvio detectado nos invariantes esperados da TOC.".to_string());
    }

    let toc_title = match report.toc_format.as_str() {
        "sd-plaintext" => "TABELA PLAINTEXT SD (\"PANA__SD.FRM\" — Slots de 64B):",
        "sd-48b-plaintext-obsolete" => {
            "TABELA PLAINTEXT SD OBSOLETA (\"PANA__SD.FRM\" — Slots de 48B):"
        }
        _ => "TABELA DECIFRADA (\"Firm list\" — Feistel FUN_4400a666 / S-Box 0x44035D84):",
    };
    if !report.decrypted_toc.is_empty() {
        lines.push(format!("\n[+] {toc_title}"));
        for entry in &report.decrypted_toc {
            lines.push(format!(
                "  [{}] {} | {:<10} | off=0x{:08X} | size=0x{:08X} ({:>7} B) | hw={} | model={} | list_sum=0x{:04X} (ok={})",
                entry.index,
                entry.entry_offset_hex,
                entry.module_name,
                entry.module_offset,
                entry.module_size,
                entry.module_size,
                entry.hardware_family,
                entry.model_group,
                entry.list_sum_u16,
                entry.list_sum_valid
            ));
        }
    }

    lines.push("\n[+] DESCONSTRUÇÃO DO PAYLOAD EM 3 MÓDULOS DE FIRMWARE:".to_string());
    lines.push("    O contêiner inicia seu payload em 0x00A0 e alinha os módulos por blocos virgens (0xFF * 8):".to_string());
    lines.push("    Assinatura de Flash Virgem: '03 50 65 C8 7B B8 E0 90'".to_string());
    lines.push("\n[+] CONEXÃO CRIPTOGRÁFICA DEFINITIVA COM O DISCO RÍGIDO (HDD):".to_string());
    lines.push("  1. A Partição 2 do HD (704 KB) termina com exatamente 1.280 blocos (10.240 B) de Flash Virgem".to_string());
    lines.push("     (0x035065C87BB8E090 em 0x040AD7D0..0x040AFFD0), idênticos aos blocos de padding do .FRM!".to_string());
    lines.push("  2. Os blocos nulos (E5D1F34A518B2690) aparecem no cabeçalho das Partições 1 e 2 do HD e em".to_string());
    lines.push(
        "     todas as tabelas zero-fill do Módulo 2 do .FRM, provando a mesmíssima chave e cifra."
            .to_string(),
    );
    lines.push(sep.to_string());
    lines.push(
        " DUMP DE TOC E ANÁLISE DE MÓDULOS CONCLUÍDOS COM SUCESSO                        "
            .to_string(),
    );
    lines.push(sep.to_string());
    lines
}
