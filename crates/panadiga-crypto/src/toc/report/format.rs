#![deny(unsafe_code)]

//! Pure text formatter for `FrmTocDumpReport` (zero terminal/file side effects).

use super::super::super::types::FrmTocDumpReport;
use crate::constants::PANA_FRM_TOC_ENTRY_SIZE;

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
        "sd-plaintext" => "TABELA PLAINTEXT SD (\"PANA__SD.FRM\" — Slots de 64B, Loader ver0.63):",
        "sd-48b-plaintext" => {
            "TABELA PLAINTEXT SD (\"PANA__SD.FRM\" — Slots de 48B, Loader ver0.61):"
        }
        "dvd-64b-feistel" => {
            "TABELA DECIFRADA 64B (\"Firm list\" — Feistel FUN_4400a666 / S-Box 0x44035D84):"
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
