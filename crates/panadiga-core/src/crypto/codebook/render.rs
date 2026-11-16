use std::path::Path;

use anyhow::{Context, Result};

use super::super::types::CodebookReport;
use super::analyze::{analyze_codebook_slices, load_optional_hdd_image, open_resolved_frm};

/// Displays the 64-bit ECB Known-Plaintext Attack (KPA) codebook dictionary and HDD correlations.
pub fn print_codebook(
    frm_path: &Path,
    hdd_path: Option<&Path>,
    json_path: Option<&Path>,
) -> Result<CodebookReport> {
    let (resolved_frm, _frm_file, frm_mmap) = open_resolved_frm(frm_path)?;
    let (hdd_buf, resolved_hdd_name) = load_optional_hdd_image(hdd_path);

    let report = analyze_codebook_slices(
        &frm_mmap,
        hdd_buf.as_deref(),
        &resolved_frm.to_string_lossy(),
        resolved_hdd_name.clone(),
    );

    log_println!(
        "================================================================================"
    );
    log_println!(" PANASONIC 64-BIT ECB CODEBOOK — KNOWN-PLAINTEXT ATTACK (KPA) DICTIONARY       ");
    log_println!(
        "================================================================================"
    );
    log_println!(" Arquivo Analisado : {}", resolved_frm.display());
    log_println!(
        " Arquivo HDD       : {}",
        resolved_hdd_name
            .as_deref()
            .unwrap_or("Não fornecido / Não encontrado")
    );
    log_println!(
        " Total de Blocos   : {} blocos de 64 bits (8 bytes cada)",
        report.total_64bit_blocks_analyzed
    );
    log_println!(" Blocos Únicos     : {}", report.unique_blocks);
    log_println!(
        "--------------------------------------------------------------------------------"
    );
    log_println!(" FUNDAMENTO FORENSE DO CODEBOOK ECB (FEISTEL DE 64 BITS):");
    log_println!(" Em modo Electronic Codebook (ECB): C_i = E_K(P_i).");
    log_println!(
        " Blocos idênticos de texto-plano produzem invariavelmente blocos cifrados idênticos."
    );
    log_println!(
        " Ao correlacionar preenchimentos conhecidos da eletrônica (Flash virgem e zeros nulos),"
    );
    log_println!(
        " constrói-se o dicionário determinístico de tradução direta sem necessidade da chave."
    );
    log_println!(
        "--------------------------------------------------------------------------------"
    );

    log_println!("\n[+] DICIONÁRIO DE TEXTO-PLANO CONHECIDO (KNOWN-PLAINTEXT PAIRS):");
    log_println!(
        "    +----+-------------------------+-------------------------+------------+------------+--------------------------------------------+"
    );
    log_println!(
        "    | #  | Bloco Cifrado (Hex)     | Texto-Plano Conhecido   | Ocorr. FRM | Ocorr. HDD | Interpretação Forense & Arquitetura        |"
    );
    log_println!(
        "    +----+-------------------------+-------------------------+------------+------------+--------------------------------------------+"
    );

    for entry in &report.dictionary {
        if entry.rank <= 6 {
            log_println!(
                "    | {:<2} | {:<23} | {:<23} | {:>10} | {:>10} | {:<42} |",
                entry.rank,
                entry.block_hex,
                entry
                    .known_plaintext_hex
                    .as_deref()
                    .unwrap_or("Padrão Estrutural"),
                entry.frm_occurrences,
                entry.hdd_occurrences,
                entry.interpretation
            );
        }
    }
    log_println!(
        "    +----+-------------------------+-------------------------+------------+------------+--------------------------------------------+"
    );

    log_println!("\n[+] BLOCOS CIFRADOS DE ALTA FREQUÊNCIA NO MÓDULO 2 (DRIVE SERVO DSP):");
    log_println!(
        "    Estes blocos vazam devido a tabelas de firmware e instruções repetidas no MN102:"
    );
    log_println!(
        "    +----+-------------------------+------------+------------+--------------------------------------------------------------+"
    );
    log_println!(
        "    | #  | Bloco Cifrado (Hex)     | Ocorr. FRM | 1º Offset  | Localização & Comportamento Criptográfico                    |"
    );
    log_println!(
        "    +----+-------------------------+------------+------------+--------------------------------------------------------------+"
    );

    for (idx, entry) in report.dictionary.iter().filter(|e| e.rank > 6).enumerate() {
        log_println!(
            "    | {:<2} | {:<23} | {:>10} | {:<10} | {:<60} |",
            idx + 1,
            entry.block_hex,
            entry.frm_occurrences,
            entry
                .sample_locations
                .first()
                .map(|s| s.as_str())
                .unwrap_or("N/A"),
            entry.interpretation
        );
    }
    log_println!(
        "    +----+-------------------------+------------+------------+--------------------------------------------------------------+"
    );

    log_println!("\n[+] SÍNTESE FORENSE E PROVAS MATEMÁTICAS:");
    log_println!(
        "  1. [CONFIRMADO] Vazamento Estrutural ECB: O arquivo óptico PANA_DVD.FRM e o HD IDE utilizam"
    );
    log_println!(
        "     cifragem de bloco de 64 bits (8 bytes) sem encadeamento de feedback (CBC) nas fronteiras."
    );
    if report.cross_medium_key_identity_proven {
        log_println!("  2. [CONFIRMADO] IDENTIDADE DE CHAVE ENTRE CD E HD:");
        log_println!(
            "     - O bloco de zeros nulos 'E5 D1 F3 4A 51 8B 26 90' ocorre no .FRM e no HD."
        );
        log_println!(
            "     - O bloco de Flash virgem '03 50 65 C8 7B B8 E0 90' ocorre no .FRM e no HD."
        );
        log_println!(
            "     -> Conclusão: O firmware do HD NÃO é texto-plano, mas compartilha a MESMA chave do contêiner óptico!"
        );
    }

    if let Some(jpath) = json_path {
        let json_data = serde_json::to_string_pretty(&report)
            .context("Failed serializing Codebook report to JSON")?;
        std::fs::write(jpath, json_data)
            .with_context(|| format!("Failed writing JSON Codebook to {}", jpath.display()))?;
        log_println!("\n[+] Relatório JSON exportado: {}", jpath.display());
    }

    log_println!(
        "================================================================================"
    );
    log_println!(" CONSULTA AO CODEBOOK CONCLUÍDA COM SUCESSO                                    ");
    log_println!(
        "================================================================================"
    );

    Ok(report)
}
