//! Copy-on-Write NVRAM cleaning and micro-sector patching file I/O workflows.

use std::path::Path;

use anyhow::Result;

use super::super::pure::{
    NvramCleanReport, NvramPolicyOptions, PureNvramCleanOutput, SliceIoLabels,
    clean_flash_nvram_slice_with_policy, patch_flash_nvram_slice_with_policy,
};
use super::capture::{
    ensure_distinct_cow_path, read_flash_file, resolve_input_path, write_cow_file,
};

/// Executes policy-gated CoW NVRAM clean, backup, audit, and SHA-256 verification.
pub fn clean_flash_nvram_with_policy(
    input_path: &Path,
    output_path: Option<&Path>,
    policy: &NvramPolicyOptions<'_>,
) -> Result<NvramCleanReport> {
    persist_transformed_flash(
        input_path,
        output_path,
        "CLEAN",
        "PANASONIC DIGA NVRAM CLEANER & FLASH INTEGRITY ARCHITECT",
        |flash_bytes, in_lbl, out_lbl, bak_lbl| {
            clean_flash_nvram_slice_with_policy(
                flash_bytes,
                policy,
                SliceIoLabels::new(in_lbl, out_lbl, bak_lbl),
            )
        },
    )
}

/// Fail-closed default NVRAM cleaning workflow (no overrides).
pub fn clean_flash_nvram(
    input_path: &Path,
    output_path: Option<&Path>,
) -> Result<NvramCleanReport> {
    clean_flash_nvram_with_policy(input_path, output_path, &NvramPolicyOptions::default())
}

/// Policy-gated Copy-on-Write byte patch inside verified EEMUL NVRAM micro-sectors.
pub fn patch_flash_nvram_with_policy(
    input_path: &Path,
    output_path: Option<&Path>,
    nvram_rel_offset: usize,
    patch_bytes: &[u8],
    policy: &NvramPolicyOptions<'_>,
) -> Result<NvramCleanReport> {
    persist_transformed_flash(
        input_path,
        output_path,
        "PATCHED",
        "PANASONIC DIGA GUARDED NVRAM PATCHER (IPL PROTECTED)",
        |flash_bytes, in_lbl, out_lbl, bak_lbl| {
            patch_flash_nvram_slice_with_policy(
                flash_bytes,
                nvram_rel_offset,
                patch_bytes,
                policy,
                SliceIoLabels::new(in_lbl, out_lbl, bak_lbl),
            )
        },
    )
}

/// Fail-closed default guarded NVRAM patcher (no overrides).
pub fn patch_flash_nvram(
    input_path: &Path,
    output_path: Option<&Path>,
    nvram_rel_offset: usize,
    patch_bytes: &[u8],
) -> Result<NvramCleanReport> {
    patch_flash_nvram_with_policy(
        input_path,
        output_path,
        nvram_rel_offset,
        patch_bytes,
        &NvramPolicyOptions::default(),
    )
}

fn log_nvram_audit_report(
    report: &NvramCleanReport,
    resolved_output: &Path,
    backup_nvram_path: &Path,
    backup_len: usize,
) {
    let nvram_len = report
        .nvram_offset_end
        .saturating_sub(report.nvram_offset_start)
        .max(1);
    let pct_non_ff = (report.nvram_non_ff_bytes as f64 / nvram_len as f64) * 100.0;
    let pct_non_zero = (report.nvram_non_zero_bytes as f64 / nvram_len as f64) * 100.0;

    log_println!("\n[+] ANALISE ESTRUTURAL DA NVRAM CORROMPIDA:");
    log_println!(
        "    * Bytes nao-FF (dados gravados) : {:6} / {} ({:.2}%)",
        report.nvram_non_ff_bytes,
        nvram_len,
        pct_non_ff
    );
    log_println!(
        "    * Bytes nao-zero                : {:6} / {} ({:.2}%)",
        report.nvram_non_zero_bytes,
        nvram_len,
        pct_non_zero
    );

    let first_sec = report
        .microsectors
        .first()
        .map_or("SA?", |m| m.sector_name.as_str());
    let last_sec = report
        .microsectors
        .last()
        .map_or("SA?", |m| m.sector_name.as_str());

    log_println!(
        "\n    * Auditoria Detalhada dos {} Setores NVRAM ({} a {}):",
        report.microsectors.len(),
        first_sec,
        last_sec
    );
    log_println!("    +-------+----------+--------------------+-------------------------+");
    log_println!("    | Setor |  Offset  | Ativos (Nao-0xFF)  | Header (Primeiros 8B)   |");
    log_println!("    +-------+----------+--------------------+-------------------------+");
    for m in &report.microsectors {
        log_println!(
            "    | {:<5} | 0x{:06X} | {:4} / {} ({:5.1}%) | {:<23} |",
            m.sector_name,
            m.offset,
            m.active_bytes,
            m.total_bytes,
            (m.active_bytes as f64 / m.total_bytes as f64) * 100.0,
            m.header_hex
        );
    }
    log_println!("    +-------+----------+--------------------+-------------------------+");
    log_println!(
        "\n[OK] Verificacao de Integridade: Area do Bootloader/OS (primeiros {} bytes)",
        report.nvram_offset_start
    );
    log_println!("     Bit-a-bit 100% IDENTICA E IMUTAVEL.");
    log_println!(
        "[OK] Backup {} ({} B) -> {}",
        backup_nvram_path.display(),
        backup_len,
        resolved_output.display()
    );
}

fn persist_transformed_flash(
    input_path: &Path,
    output_path: Option<&Path>,
    default_suffix: &str,
    banner_title: &str,
    transform: impl FnOnce(&[u8], &str, &str, &str) -> Result<PureNvramCleanOutput>,
) -> Result<NvramCleanReport> {
    let resolved_input = resolve_input_path(input_path)?;
    let resolved_output = match output_path {
        Some(p) => p.to_path_buf(),
        None => {
            let parent = resolved_input.parent().unwrap_or_else(|| Path::new("."));
            let stem = resolved_input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("flash_dump");
            let ext = resolved_input
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin");
            parent.join(format!("{stem}_{default_suffix}.{ext}"))
        }
    };

    // Forensic Invariant #3: Reject in-place overwrite of original flash dump
    ensure_distinct_cow_path(
        &resolved_input,
        &resolved_output,
        "Output",
        "input flash dump",
    )?;

    let backup_dir = resolved_output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let backup_nvram_path = backup_dir.join("nvram_corrupt_backup.bin");

    log_println!("======================================================================");
    log_println!("   {banner_title}");
    log_println!("            Spansion S29JL064H NOR Flash (8 MB / 64 Mbit)             ");
    log_println!("======================================================================");
    log_println!("  Input File   : {}", resolved_input.display());
    log_println!("  Output File  : {}", resolved_output.display());
    log_println!("  Backup File  : {}", backup_nvram_path.display());
    log_println!("----------------------------------------------------------------------");

    let flash_bytes = read_flash_file(&resolved_input)?;
    let pure_output = transform(
        &flash_bytes,
        &resolved_input.to_string_lossy(),
        &resolved_output.to_string_lossy(),
        &backup_nvram_path.to_string_lossy(),
    )?;

    let flash_len = pure_output.report.total_flash_bytes;
    log_println!(
        "  Dump Size    : {} bytes ({:.2} MB / {} Mbit)",
        flash_len,
        flash_len as f64 / (1024.0 * 1024.0),
        (flash_len * 8) / (1024 * 1024)
    );
    log_println!("  Original MD5 : {}", pure_output.report.original_md5);
    log_println!("  Orig SHA-256 : {}", pure_output.report.original_sha256);
    log_println!("----------------------------------------------------------------------");

    write_cow_file(
        &backup_nvram_path,
        &pure_output.backup_nvram_bytes,
        "corrupt NVRAM backup",
    )?;
    log_println!(
        "[OK] Corrupted NVRAM sector (0x{:06X} - 0x{:06X}) extracted to:",
        pure_output.report.nvram_offset_start,
        pure_output.report.nvram_offset_end - 1
    );
    log_println!(
        "     -> {} ({} bytes)",
        backup_nvram_path.display(),
        pure_output.backup_nvram_bytes.len()
    );

    log_nvram_audit_report(
        &pure_output.report,
        &resolved_output,
        &backup_nvram_path,
        pure_output.backup_nvram_bytes.len(),
    );
    write_cow_file(&resolved_output, &pure_output.cleaned_bytes, "clean binary")?;

    log_println!("----------------------------------------------------------------------");
    log_println!("[SUCESSO] Imagem limpa gerada com perfeicao:");
    log_println!("  Arquivo Limpo: {}", resolved_output.display());
    log_println!("  Clean MD5    : {}", pure_output.report.clean_md5);
    log_println!("  Clean SHA-256: {}", pure_output.report.clean_sha256);
    log_println!("======================================================================");
    log_println!("PRONTO PARA REGRAVACAO NA MEMORIA TSOP-48 VIA XGECU T48 / TL866II / SVOD 4!");
    log_println!("Ao inicializar, a CPU Panasonic AM34 recriara a NVRAM limpa de fabrica.");
    log_println!("======================================================================");

    Ok(pure_output.report)
}
