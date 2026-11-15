//! Imperative shell for MEIHDFS allocation descriptor file resolution, logging, and JSON export.

use std::path::Path;

use anyhow::Result;

use crate::nvram::resolve_input_path;

use super::super::types::MeihdfsAllocationReport;
use super::super::utils::{export_report_to_json, open_image_buffer};
use super::decode::decode_meihdfs_slice;

/// Imperative Shell: Resolves input file, coordinates pure decoding, and writes console logs & JSON export.
pub fn decode_meihdfs_descriptors(
    input_path: &Path,
    include_journal: bool,
    json_export_path: Option<&Path>,
) -> Result<MeihdfsAllocationReport> {
    let resolved = resolve_input_path(input_path)?;
    let mmap = open_image_buffer(&resolved)?;

    let report = decode_meihdfs_slice(&mmap, include_journal, &resolved.to_string_lossy())?;

    log_println!(
        "================================================================================"
    );
    log_println!(
        " PANASONIC MEIHDFS ALLOCATION DESCRIPTORS & EXTENT FORENSIC DECODER              "
    );
    log_println!(
        "================================================================================"
    );
    log_println!(" Arquivo Analisado : {}", resolved.display());
    log_println!(
        " Modo de Origem    : {}",
        if report.is_full_hdd_image {
            "Imagem Completa de HDD (100 MB)"
        } else {
            "Tabela Extraida de Descritores (4,096 bytes)"
        }
    );
    if report.is_full_hdd_image {
        log_println!(
            " Validacao Espelho : LBA 24..31 vs LBA 57368..57375 -> {}",
            if report.is_mirror_verified {
                "100% IDENTICO BIT-A-BIT (Integridade Confirmada)"
            } else {
                "DIVERGENCIA DETECTADA"
            }
        );
    }
    log_println!(" SHA-256 Descritor : {}", report.descriptors_sha256);
    log_println!(
        " Total de Registros: {} registros (16 bytes cada, decodificados via NOT/XOR 0xFF)",
        report.records.len()
    );
    log_println!(
        "--------------------------------------------------------------------------------"
    );
    log_println!(" DISTRIBUICAO DE TAGS DECRIPTO-FORENSES (ONE'S COMPLEMENT):");
    for (tag_name, count) in &report.tag_distribution {
        let pct = (*count as f64 / report.records.len() as f64) * 100.0;
        log_println!(
            "   * {:<24} : {:>3} registros ({:5.2}%)",
            tag_name,
            count,
            pct
        );
    }
    log_println!(
        "--------------------------------------------------------------------------------"
    );
    log_println!(" MAPEAMENTO CONSOLIDADO DE EXTENTS & ARMAZENAMENTO MEIHDFS:");
    log_println!("   ID Extent | Segmentos | Total Setores | Tamanho Util | Status / Descricao");
    log_println!(
        "   ----------+-----------+---------------+--------------+-------------------------------"
    );
    for ext in report
        .extents
        .iter()
        .filter(|e| e.segments_count > 0 || e.header_record_index.is_some())
    {
        log_println!(
            "   {:<9} | {:>9} | {:>13} | {:>10} B | {}",
            ext.extent_id_hex,
            ext.segments_count,
            ext.total_sectors,
            ext.total_bytes,
            if let Some(h) = ext.header_record_index {
                format!("Header em Rec [{:3}] | {} frag(s)", h, ext.segments_count)
            } else {
                format!("Fragmentos orfaos: {} frag(s)", ext.segments_count)
            }
        );
    }

    if !report.journal_blocks.is_empty() {
        log_println!(
            "--------------------------------------------------------------------------------"
        );
        log_println!(" AUDITORIA DO POOL DE JOURNAL TRANSACIONAL DE 10 BLOCOS (LBA 16640..19199):");
        log_println!(
            "   Bloco | LBA Base | Offset     | Header Ativo | Seq ID | Pool | Checksum CRC | Payload"
        );
        log_println!(
            "   ------+----------+------------+--------------+--------+------+--------------+---------"
        );
        for jb in &report.journal_blocks {
            log_println!(
                "   {:>5} | {:>8} | {} | {:>12} | {:>6} | {:>4} | {:>12} | ~{} set",
                jb.block_index,
                jb.start_lba,
                jb.start_offset_hex,
                if jb.is_active_header {
                    "5D FF FF..."
                } else {
                    "Inativo"
                },
                jb.transaction_seq,
                jb.pool_size,
                jb.checksum_crc,
                jb.active_payload_sectors
            );
        }
    }

    if let Some(json_path) = json_export_path {
        export_report_to_json(&report, json_path, "MEIHDFS allocation report")?;
    }

    log_println!(
        "================================================================================"
    );
    log_println!(
        " DECODIFICACAO DE METADADOS MEIHDFS CONCLUIDA COM TOTAL FIDELIDADE              "
    );
    log_println!(
        "================================================================================"
    );

    Ok(report)
}

/// Backward compatibility wrapper for `decode_meihdfs_descriptors`.
pub fn decode_descriptors(image_path: &Path) -> Result<()> {
    decode_meihdfs_descriptors(image_path, true, None)?;
    Ok(())
}
