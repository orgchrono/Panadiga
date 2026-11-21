//! Copy-on-Write Upper NOR App/BGO flash patching file I/O workflow.

use std::path::Path;

use anyhow::Result;

use super::super::pure::{
    NvramPolicyOptions, SliceIoLabels, UpperNorPatchReport, patch_flash_upper_nor_slice_with_policy,
};
use super::capture::{
    ensure_distinct_cow_path, read_flash_file, resolve_input_path, write_cow_file,
};

/// Policy-gated Copy-on-Write patch in Upper NOR App/BGO (`0x040000..nvram_start-1`) with mandatory sector backup and SHA-256 audit.
pub fn patch_flash_upper_nor_file_with_policy(
    input_path: &Path,
    output_path: &Path,
    backup_path: Option<&Path>,
    flash_offset: usize,
    patch_bytes: &[u8],
    allow_critical_ipl: bool,
    policy: &NvramPolicyOptions<'_>,
) -> Result<UpperNorPatchReport> {
    let resolved_input = resolve_input_path(input_path)?;
    ensure_distinct_cow_path(&resolved_input, output_path, "Output", "input flash dump")?;

    let resolved_backup = match backup_path {
        Some(p) => p.to_path_buf(),
        None => {
            let dir = output_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let stem = output_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("flash");
            dir.join(format!("{stem}_upper_nor_backup_0x{flash_offset:06X}.bin"))
        }
    };
    ensure_distinct_cow_path(
        &resolved_input,
        &resolved_backup,
        "Backup",
        "input flash dump",
    )?;
    ensure_distinct_cow_path(
        output_path,
        &resolved_backup,
        "Backup",
        "output flash image",
    )?;

    let flash_bytes = read_flash_file(&resolved_input)?;
    let pure_output = patch_flash_upper_nor_slice_with_policy(
        &flash_bytes,
        flash_offset,
        patch_bytes,
        allow_critical_ipl,
        policy,
        SliceIoLabels::new(
            &resolved_input.to_string_lossy(),
            &output_path.to_string_lossy(),
            &resolved_backup.to_string_lossy(),
        ),
    )?;

    write_cow_file(
        &resolved_backup,
        &pure_output.backup_sector_bytes,
        "Upper NOR sector backup",
    )?;
    write_cow_file(
        output_path,
        &pure_output.patched_bytes,
        "patched Upper NOR flash image",
    )?;

    let rep = &pure_output.report;
    log_println!(
        "[UPPER-NOR CoW] {} -> {} (backup: {} [{}-{}, {} B])",
        rep.input_file,
        rep.output_file,
        rep.backup_file,
        rep.first_sector_name,
        rep.last_sector_name,
        pure_output.backup_sector_bytes.len()
    );
    log_println!(
        "  Range SHA-256 : {} -> {} | Full SHA-256: {} -> {}",
        rep.patched_range_before_sha256,
        rep.patched_range_after_sha256,
        rep.original_sha256,
        rep.patched_sha256
    );
    Ok(pure_output.report)
}

/// Fail-closed default Upper NOR CoW file patcher delegating strictly through `CapabilityGate` (no overrides).
pub fn patch_flash_upper_nor_file(
    input_path: &Path,
    output_path: &Path,
    backup_path: Option<&Path>,
    flash_offset: usize,
    patch_bytes: &[u8],
    allow_critical_ipl: bool,
) -> Result<UpperNorPatchReport> {
    patch_flash_upper_nor_file_with_policy(
        input_path,
        output_path,
        backup_path,
        flash_offset,
        patch_bytes,
        allow_critical_ipl,
        &NvramPolicyOptions::default(),
    )
}
