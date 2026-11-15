//! Imperative shell for Panasonic HDD structural inspection, logging, and JSON export.

use std::path::Path;

use anyhow::Result;

use crate::{constants::*, nvram::resolve_input_path};

use super::super::{
    types::HddAnalysisReport,
    utils::{export_report_to_json, open_image_buffer},
};
use super::probe::inspect_hdd_slice;

/// Performs complete structural inspection and outputs rich visual console telemetry.
pub fn inspect_hdd_image(
    image_path: &Path,
    show_density: bool,
    json_export_path: Option<&Path>,
) -> Result<HddAnalysisReport> {
    let resolved_path = resolve_input_path(image_path)?;
    let buffer = open_image_buffer(&resolved_path)?;
    let file_size = buffer.len() as u64;
    let total_sectors = file_size / LBA_SECTOR_SIZE_BYTES as u64;

    let report = inspect_hdd_slice(&buffer, &resolved_path.to_string_lossy())?;

    log_println!(
        "================================================================================"
    );
    log_println!(
        " PANASONIC DMR-EH55 HDD FORENSIC PARSER & STORAGE ARCHITECTURE AUDITOR          "
    );
    log_println!(
        "================================================================================"
    );
    log_println!(" Arquivo da Imagem : {}", resolved_path.display());
    log_println!(
        " Tamanho Total     : {:>11} bytes ({:.2} MB)",
        file_size,
        file_size as f64 / (1024.0 * 1024.0)
    );
    log_println!(
        " Total de Setores  : {:>11} LBAs ({} bytes/setor)",
        total_sectors,
        LBA_SECTOR_SIZE_BYTES
    );
    log_println!(" MD5 Global        : {}", report.global_md5);
    log_println!(" SHA-256 Global    : {}", report.global_sha256);
    log_println!(
        "--------------------------------------------------------------------------------"
    );

    log_println!("\n[+] ESTRUTURA DO MASTER BOOT RECORD & TABELA ESPELHO (LBA 0 & LBA 57344)");
    log_println!("  * LBA 0 (Offset 0x00000000):");
    log_println!(
        "    - Assinatura Bruta  : {}",
        report.mbr_tables.lba0_hex_signature
    );
    log_println!(
        "    - Versao de Formato : Rev {}.0 (0x{:04X})",
        report.mbr_tables.format_version,
        report.mbr_tables.format_version
    );
    log_println!(
        "    - Magic Tag ASCII   : '{}' (System Storage Header)",
        report.mbr_tables.magic_ascii
    );
    log_println!(
        "    - Disk Volume GUID  : {} (Identificador de Formatacao)",
        report.mbr_tables.disk_volume_guid
    );
    log_println!(
        "    - MBR Padrao 0x55AA : {}",
        if report.mbr_tables.has_standard_mbr_55aa {
            "SIM"
        } else {
            "NAO (Assinatura Proprietaria Panasonic MEIHDFS RAW)"
        }
    );
    log_println!(
        "    - Zeros em 16..511  : {}",
        if report.mbr_tables.zeros_after_header_clean {
            "100% Zeros limpos"
        } else {
            "Contem dados adicionais"
        }
    );
    log_println!(
        "    - SHA-256 LBA 0     : {}",
        report.mbr_tables.lba0_sha256
    );

    log_println!(
        "  * LBA {} (Offset 0x{:08X} = 28.00 MB):",
        LBA_MIRROR_MBR_SECTOR,
        LBA_MIRROR_MBR_OFFSET
    );
    log_println!(
        "    - Status do Espelho : {}",
        if report.mbr_tables.mirror_is_bit_identical {
            "IDENTICO BIT-A-BIT AO LBA 0 (Integridade Confirmada)"
        } else {
            "DIVERGENCIA DETECTADA"
        }
    );
    log_println!(
        "    - SHA-256 LBA Mirror: {}",
        report.mbr_tables.mirror_sha256
    );

    log_println!("\n[+] METADADOS DE ALOCACAO & JOURNAL TRANSACIONAL");
    log_println!(
        "  * Descritores Setores 24..31 (0x{:08X}, {} bytes):",
        report.allocation_and_journal.descriptors_offset,
        report.allocation_and_journal.descriptors_size
    );
    log_println!(
        "    - Espelho em LBA 57368 (0x{:08X}): {}",
        ALLOCATION_DESCRIPTORS_MIRROR_OFFSET,
        if report.allocation_and_journal.descriptors_mirror_identical {
            "100% IDENTICO (Integridade Confirmada)"
        } else {
            "Divergente"
        }
    );
    log_println!(
        "    - SHA-256 Descritores  : {}",
        report.allocation_and_journal.descriptors_sha256
    );

    log_println!(
        "  * LBA 16384 (Offset 0x{:08X} = 8.00 MB):",
        report.allocation_and_journal.journal_catalog_offset
    );
    log_println!(
        "    - Catalogo Journal     : {} blocos (Max Index: {})",
        report.allocation_and_journal.journal_total_blocks,
        report.allocation_and_journal.journal_max_index
    );

    log_println!(
        "  * Regiao de Journal (0x{:08X} a 0x{:08X} = {} bytes):",
        JOURNAL_AREA_OFFSET,
        JOURNAL_AREA_END_OFFSET,
        JOURNAL_AREA_SIZE
    );
    for (i, trailer) in report
        .allocation_and_journal
        .journal_block_trailers
        .iter()
        .enumerate()
    {
        let blk_start = JOURNAL_AREA_OFFSET as usize + i * JOURNAL_BLOCK_SIZE;
        let start_lba = blk_start / LBA_SECTOR_SIZE_BYTES;
        log_println!(
            "    - Bloco {:2} | LBA {:5} (0x{:08X}) | Trailer: {}",
            i,
            start_lba,
            blk_start,
            trailer
        );
    }

    log_println!(
        "  * LBA 49152 (Offset 0x{:08X} = 24.00 MB):",
        report.allocation_and_journal.state_marker_offset
    );
    log_println!(
        "    - Marcador de Transicao: {}",
        report.allocation_and_journal.state_marker_hex
    );

    log_println!("\n[+] PARTICÕES DE FIRMWARE PROPRIETÁRIAS (Magic: 51 87 13 8D 5B 0A FB 22)");
    for p in &report.partitions {
        log_println!("  ------------------------------------------------------------------------");
        log_println!("  PARTICAO {}: {}", p.partition_id, p.name);
        log_println!("  ------------------------------------------------------------------------");
        log_println!(
            "  Offset Inicial    : {} (LBA {})",
            p.offset_hex,
            p.lba_start
        );
        log_println!(
            "  Tamanho Util      : {:>10} bytes ({:.3} MB / 0x{:X})",
            p.exact_payload_size,
            p.exact_payload_size as f64 / (1024.0 * 1024.0),
            p.exact_payload_size
        );
        log_println!(
            "  Setores Ocupados  : {:>10} LBAs ({} bytes)",
            p.sectors_count,
            p.sectors_count * LBA_SECTOR_SIZE_BYTES
        );
        log_println!("  Assinatura Magic  : {}", p.signature_hex);
        log_println!(
            "  Entropia Shannon  : {:.4} bits/byte (Alta Densidade / Criptografia Feistel)",
            p.entropy
        );
        log_println!("  MD5 Payload Util  : {}", p.exact_md5);
        log_println!("  SHA-256 Util      : {}", p.exact_sha256);
        if let Some(sha_al) = &p.aligned_sha256 {
            log_println!("  SHA-256 Alinhado  : {}", sha_al);
        }
    }

    if show_density {
        log_println!("\n[+] MAPEAMENTO VISUAL DE DENSIDADE (Blocos de 1 MB):");
        for chunk in &report.density_map {
            if chunk.non_zero_bytes > 0 {
                let bar_len = (chunk.percentage / 5.0).round() as usize;
                let bar: String = "#".repeat(bar_len);
                log_println!(
                    "  MB {:02} (0x{:08X}): {:>9} bytes [{:<20}] ({:5.1}%)",
                    chunk.mb_index,
                    chunk.offset,
                    chunk.non_zero_bytes,
                    bar,
                    chunk.percentage
                );
            }
        }
    }

    if let Some(json_path) = json_export_path {
        export_report_to_json(&report, json_path, "HDD analysis report")?;
    }

    log_println!(
        "================================================================================"
    );
    log_println!(
        " ANALISE ESTRUTURAL DE HDD CONCLUIDA COM SUCESSO                                "
    );
    log_println!(
        "================================================================================"
    );

    Ok(report)
}
