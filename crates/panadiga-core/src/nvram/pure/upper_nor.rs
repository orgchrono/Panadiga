//! Pure policy-gated Copy-on-Write Upper NOR App/BGO flash patching with Critical Boot IPL lockout.

use anyhow::{Context, Result, bail};

use super::audit::calculate_hashes;
use super::types::{
    NvramPolicyOptions, PureUpperNorPatchOutput, SliceIoLabels, UpperNorPatchReport,
    authorize_flash_operation,
};
use crate::constants::AM34_FLASH_BASE;
use crate::ssot::{Capability, DerivedSector};

/// Policy-gated Copy-on-Write patch in Upper NOR App/BGO (`0x040000..nvram_start-1`), backing up touched sectors.
/// Enforces `Capability::UpperNorPatchOfflineCow` via `CapabilityGate` and blocks Critical Boot IPL
/// (`SA0..SA10`, `0x000000..0x03FFFF`) unless `allow_critical_ipl` is explicitly supplied alongside
/// a valid Upper NOR authorization or `LabOverride`.
pub fn patch_flash_upper_nor_slice_with_policy(
    flash_bytes: &[u8],
    flash_offset: usize,
    patch_bytes: &[u8],
    allow_critical_ipl: bool,
    policy: &NvramPolicyOptions<'_>,
    labels: SliceIoLabels<'_>,
) -> Result<PureUpperNorPatchOutput> {
    if patch_bytes.is_empty() {
        bail!("Upper NOR patch payload cannot be empty");
    }

    let layout = authorize_flash_operation(
        flash_bytes,
        Capability::UpperNorPatchOfflineCow,
        policy.model_id,
        policy.lab_override,
    )?;

    let Some(patch_end) = flash_offset.checked_add(patch_bytes.len()) else {
        bail!("Upper NOR patch offset overflow");
    };
    if patch_end > layout.flash_len {
        let hint = if flash_offset >= AM34_FLASH_BASE as usize {
            " (hint: pass physical flash offset, not AM34 bus VA)"
        } else {
            ""
        };
        bail!(
            "Upper NOR patch [0x{flash_offset:08X}..0x{patch_end:08X}) exceeds {}B NOR capacity{hint}",
            layout.flash_len
        );
    }

    let critical_ipl_end = layout.critical_ipl_end;
    let critical_ipl_override_used = flash_offset < critical_ipl_end;
    if critical_ipl_override_used && !allow_critical_ipl {
        bail!(
            "Critical Boot IPL Guard (SA0..SA10, 0x000000..0x{:06X}): offset [0x{flash_offset:06X}..0x{patch_end:06X}) requires explicit allow_critical_ipl (--allow-critical-ipl)",
            critical_ipl_end - 1
        );
    }
    if patch_end > layout.nvram_base_offset {
        bail!(
            "Upper NOR patch [0x{flash_offset:06X}..0x{patch_end:06X}) invades top NVRAM region (0x{:06X}..0x{:06X}); use `nvram patch` for NVRAM microsectors",
            layout.nvram_base_offset,
            layout.flash_len
        );
    }

    let touched_sectors: Vec<&DerivedSector> = layout
        .sectors
        .iter()
        .filter(|s| {
            let s_start = s.start_offset as usize;
            let s_end = s.end_offset_inclusive as usize + 1;
            flash_offset < s_end && patch_end > s_start
        })
        .collect();
    let first_sec = touched_sectors
        .first()
        .context("No physical flash sector matched start of Upper NOR patch span")?;
    let last_sec = touched_sectors
        .last()
        .context("No physical flash sector matched end of Upper NOR patch span")?;
    let backup_sector_start = first_sec.start_offset as usize;
    let backup_sector_end = last_sec.end_offset_inclusive as usize + 1;
    let backup_sector_bytes = flash_bytes[backup_sector_start..backup_sector_end].to_vec();

    let (orig_md5, orig_sha256) = calculate_hashes(flash_bytes);
    let (_, backup_sector_sha256) = calculate_hashes(&backup_sector_bytes);
    let (_, patched_range_before_sha256) = calculate_hashes(&flash_bytes[flash_offset..patch_end]);

    let mut patched_bytes = flash_bytes.to_vec();
    patched_bytes[flash_offset..patch_end].copy_from_slice(patch_bytes);

    if patched_bytes[..flash_offset] != flash_bytes[..flash_offset]
        || patched_bytes[patch_end..] != flash_bytes[patch_end..]
    {
        bail!("[CRITICAL ERROR] Bytes outside target Upper NOR patch span were altered! Aborting.");
    }

    let critical_ipl_untouched =
        patched_bytes[..critical_ipl_end] == flash_bytes[..critical_ipl_end];
    if !allow_critical_ipl && !critical_ipl_untouched {
        bail!(
            "[CRITICAL ERROR] Critical Boot IPL (SA0..SA10) mutated without allow_critical_ipl! Aborting."
        );
    }
    let nvram_untouched =
        patched_bytes[layout.nvram_base_offset..] == flash_bytes[layout.nvram_base_offset..];
    if !nvram_untouched {
        bail!("[CRITICAL ERROR] Top NVRAM microsectors mutated by Upper NOR patch! Aborting.");
    }

    let (patched_md5, patched_sha256) = calculate_hashes(&patched_bytes);
    let (_, patched_range_after_sha256) = calculate_hashes(&patched_bytes[flash_offset..patch_end]);

    let report = UpperNorPatchReport {
        input_file: labels.input.to_string(),
        output_file: labels.output.to_string(),
        backup_file: labels.backup.to_string(),
        original_md5: orig_md5,
        original_sha256: orig_sha256,
        patched_md5,
        patched_sha256,
        backup_sector_sha256,
        patched_range_before_sha256,
        patched_range_after_sha256,
        total_flash_bytes: layout.flash_len,
        patch_offset_start: flash_offset,
        patch_offset_end: patch_end,
        patch_length: patch_bytes.len(),
        backup_sector_start,
        backup_sector_end,
        first_sector_name: format!("SA{}", first_sec.index),
        last_sector_name: format!("SA{}", last_sec.index),
        critical_ipl_override_used,
        critical_ipl_untouched,
        nvram_untouched,
    };

    Ok(PureUpperNorPatchOutput {
        patched_bytes,
        backup_sector_bytes,
        report,
    })
}

/// Fail-closed default Upper NOR patch wrapper delegating strictly through `CapabilityGate` (no overrides).
pub fn patch_flash_upper_nor_slice(
    flash_bytes: &[u8],
    flash_offset: usize,
    patch_bytes: &[u8],
    allow_critical_ipl: bool,
    input_label: &str,
    output_label: &str,
    backup_label: &str,
) -> Result<PureUpperNorPatchOutput> {
    patch_flash_upper_nor_slice_with_policy(
        flash_bytes,
        flash_offset,
        patch_bytes,
        allow_critical_ipl,
        &NvramPolicyOptions::default(),
        SliceIoLabels::new(input_label, output_label, backup_label),
    )
}
