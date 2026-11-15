//! Filesystem I/O shell for Panasonic Flash NVRAM inspection and surgical cleaning.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::pure::{
    NvramCleanReport, PureNvramCleanOutput, clean_flash_nvram_slice, patch_flash_nvram_slice,
};
use crate::constants::*;

/// Intelligently resolves an input file path across working directory and known folders,
/// transparently handling both uncompressed (.bin) and compressed (.bin.gz) files.
pub fn resolve_input_path(input: &Path) -> Result<PathBuf> {
    if input.exists() {
        return Ok(input.to_path_buf());
    }

    let gz_input = PathBuf::from(format!("{}.gz", input.display()));
    if gz_input.exists() {
        return Ok(gz_input);
    }

    let filename = input.file_name().unwrap_or(input.as_os_str());

    let mut candidate_dirs = vec![
        PathBuf::from("firmware_and_images"),
        PathBuf::from("../firmware_and_images"),
        PathBuf::from("../../firmware_and_images"),
        PathBuf::from(".."),
        PathBuf::from("../.."),
        PathBuf::from("."),
    ];

    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        candidate_dirs.push(exe_dir.join("firmware_and_images"));
        candidate_dirs.push(exe_dir.to_path_buf());
        if let Some(parent) = exe_dir.parent() {
            candidate_dirs.push(parent.join("firmware_and_images"));
            candidate_dirs.push(parent.to_path_buf());
        }
    }

    for dir in candidate_dirs {
        let direct = dir.join(input);
        if direct.exists() {
            return Ok(direct);
        }
        let direct_gz = dir.join(&gz_input);
        if direct_gz.exists() {
            return Ok(direct_gz);
        }
        let by_name = dir.join(filename);
        if by_name.exists() {
            return Ok(by_name);
        }
        let by_name_gz = dir.join(format!("{}.gz", filename.to_string_lossy()));
        if by_name_gz.exists() {
            return Ok(by_name_gz);
        }
    }

    bail!("Flash binary input file not found: {}", input.display())
}

/// Reads a resolved flash binary file into a byte vector.
/// Shared by both `inspect_flash_nvram` (read-only) and `clean_flash_nvram` (read-write).
fn read_flash_file(resolved_input: &Path) -> Result<Vec<u8>> {
    let mut flash_bytes = Vec::with_capacity(SPANSION_FLASH_SIZE_BYTES);
    let mut file = File::open(resolved_input)
        .with_context(|| format!("Failed to open flash binary: {}", resolved_input.display()))?;
    file.read_to_end(&mut flash_bytes)
        .with_context(|| "Failed reading flash binary content")?;
    Ok(flash_bytes)
}

/// Inspects a flash dump in a 100% read-only manner without any disk-write side effects.
pub fn inspect_flash_nvram(input_path: &Path) -> Result<NvramCleanReport> {
    let resolved_input = resolve_input_path(input_path)?;
    let flash_bytes = read_flash_file(&resolved_input)?;

    let dummy_output = resolved_input.with_file_name("flash_clean_preview.bin");
    let dummy_backup = resolved_input.with_file_name("nvram_corrupt_backup.bin");

    let pure_output = clean_flash_nvram_slice(
        &flash_bytes,
        &resolved_input.to_string_lossy(),
        &dummy_output.to_string_lossy(),
        &dummy_backup.to_string_lossy(),
    )?;

    Ok(pure_output.report)
}

/// Executes the complete NVRAM clean, backup, audit, and verification workflow.
pub fn clean_flash_nvram(
    input_path: &Path,
    output_path: Option<&Path>,
) -> Result<NvramCleanReport> {
    persist_transformed_flash(
        input_path,
        output_path,
        "CLEAN",
        "PANASONIC DMR-EH55 NVRAM CLEANER & FLASH INTEGRITY ARCHITECT",
        |flash_bytes, in_lbl, out_lbl, bak_lbl| {
            clean_flash_nvram_slice(flash_bytes, in_lbl, out_lbl, bak_lbl)
        },
    )
}

/// Copy-on-write guarded write/patch of bytes strictly within writable NVRAM microsectors.
pub fn patch_flash_nvram(
    input_path: &Path,
    output_path: Option<&Path>,
    nvram_rel_offset: usize,
    patch_bytes: &[u8],
) -> Result<NvramCleanReport> {
    persist_transformed_flash(
        input_path,
        output_path,
        "PATCHED",
        "PANASONIC DMR-EH55 GUARDED NVRAM PATCHER (IPL PROTECTED)",
        |flash_bytes, in_lbl, out_lbl, bak_lbl| {
            patch_flash_nvram_slice(
                flash_bytes,
                nvram_rel_offset,
                patch_bytes,
                in_lbl,
                out_lbl,
                bak_lbl,
            )
        },
    )
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
    let same_path = resolved_input == resolved_output
        || (resolved_input.canonicalize().ok().is_some()
            && resolved_input.canonicalize().ok() == resolved_output.canonicalize().ok());
    if same_path {
        bail!(
            "[FORENSIC INVARIANT VIOLATION] Output path ({}) cannot overwrite input flash dump ({}) in-place.",
            resolved_output.display(),
            resolved_input.display()
        );
    }

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
    let size_mb = flash_len as f64 / (1024.0 * 1024.0);
    log_println!(
        "  Dump Size    : {} bytes ({:.2} MB / {} Mbit)",
        flash_len,
        size_mb,
        (flash_len * 8) / (1024 * 1024)
    );
    log_println!("  Original MD5 : {}", pure_output.report.original_md5);
    log_println!("  Orig SHA-256 : {}", pure_output.report.original_sha256);
    log_println!("----------------------------------------------------------------------");

    if let Some(parent) = backup_nvram_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = resolved_output.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(&backup_nvram_path, &pure_output.backup_nvram_bytes).with_context(|| {
        format!(
            "Failed writing corrupt NVRAM backup to {}",
            backup_nvram_path.display()
        )
    })?;

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

    let pct_non_ff =
        (pure_output.report.nvram_non_ff_bytes as f64 / SPANSION_NVRAM_SIZE_BYTES as f64) * 100.0;
    let pct_non_zero =
        (pure_output.report.nvram_non_zero_bytes as f64 / SPANSION_NVRAM_SIZE_BYTES as f64) * 100.0;

    log_println!("\n[+] ANALISE ESTRUTURAL DA NVRAM CORROMPIDA:");
    log_println!(
        "    * Bytes nao-FF (dados gravados) : {:6} / {} ({:.2}%)",
        pure_output.report.nvram_non_ff_bytes,
        SPANSION_NVRAM_SIZE_BYTES,
        pct_non_ff
    );
    log_println!(
        "    * Bytes nao-zero                : {:6} / {} ({:.2}%)",
        pure_output.report.nvram_non_zero_bytes,
        SPANSION_NVRAM_SIZE_BYTES,
        pct_non_zero
    );

    let base_sector_idx = pure_output
        .report
        .microsectors
        .first()
        .map(|m| {
            m.sector_name
                .trim_start_matches("SA")
                .parse::<usize>()
                .unwrap_or(134)
        })
        .unwrap_or(134);

    log_println!(
        "\n    * Auditoria Detalhada dos 8 Micro-Setores (SA{} a SA{} - 8 KB cada):",
        base_sector_idx,
        base_sector_idx + 7
    );
    log_println!("    +-------+----------+--------------------+-------------------------+");
    log_println!("    | Setor |  Offset  | Ativos (Nao-0xFF)  | Header (Primeiros 8B)   |");
    log_println!("    +-------+----------+--------------------+-------------------------+");

    for m in &pure_output.report.microsectors {
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

    let bootloader_size = pure_output.report.nvram_offset_start;
    log_println!(
        "\n[OK] Verificacao de Integridade: Area do Bootloader/OS (primeiros {} bytes)",
        bootloader_size
    );
    log_println!("     Bit-a-bit 100% IDENTICA E IMUTAVEL.");

    fs::write(&resolved_output, &pure_output.cleaned_bytes).with_context(|| {
        format!(
            "Failed writing clean binary to {}",
            resolved_output.display()
        )
    })?;

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
