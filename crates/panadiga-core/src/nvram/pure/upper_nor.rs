//! Pure Copy-on-Write Upper NOR App/BGO flash patching with Critical Boot IPL lockout.

use anyhow::{Result, bail};

use super::audit::calculate_hashes;
use super::types::{PureUpperNorPatchOutput, UpperNorPatchReport, resolve_flash_layout};
use crate::constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, SPANSION_MAIN_SECTOR_SIZE_BYTES,
};

/// Copy-on-write patch in Upper NOR App/BGO (`0x040000..nvram_start-1`), backing up touched 64 KiB sectors.
/// Refuses Critical Boot IPL (`SA0..SA3`: `0x000000..0x03FFFF`) unless `allow_critical_ipl` is explicitly `true`,
/// and refuses invading top NVRAM microsectors (`nvram_start..flash_len`).
pub fn patch_flash_upper_nor_slice(
    flash_bytes: &[u8],
    flash_offset: usize,
    patch_bytes: &[u8],
    allow_critical_ipl: bool,
    input_label: &str,
    output_label: &str,
    backup_label: &str,
) -> Result<PureUpperNorPatchOutput> {
    if patch_bytes.is_empty() {
        bail!("Upper NOR patch payload cannot be empty");
    }
    let layout = resolve_flash_layout(flash_bytes.len())?;
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

    let critical_ipl_end = AM34_FLASH_CRITICAL_IPL_SIZE as usize;
    let critical_ipl_override_used = flash_offset < critical_ipl_end;
    if critical_ipl_override_used && !allow_critical_ipl {
        bail!(
            "Critical Boot IPL Guard (SA0..SA3, 0x000000..0x{:06X}): offset [0x{flash_offset:06X}..0x{patch_end:06X}) requires explicit allow_critical_ipl (--allow-critical-ipl)",
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

    let sector_size = SPANSION_MAIN_SECTOR_SIZE_BYTES;
    let backup_sector_start = (flash_offset / sector_size) * sector_size;
    let backup_sector_end = patch_end.div_ceil(sector_size) * sector_size;
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
            "[CRITICAL ERROR] Critical Boot IPL (SA0..SA3) mutated without allow_critical_ipl! Aborting."
        );
    }
    let nvram_untouched =
        patched_bytes[layout.nvram_base_offset..] == flash_bytes[layout.nvram_base_offset..];
    if !nvram_untouched {
        bail!("[CRITICAL ERROR] Top NVRAM microsectors mutated by Upper NOR patch! Aborting.");
    }

    let (patched_md5, patched_sha256) = calculate_hashes(&patched_bytes);
    let (_, patched_range_after_sha256) = calculate_hashes(&patched_bytes[flash_offset..patch_end]);
    let first_idx = backup_sector_start / sector_size;
    let last_idx = (backup_sector_end / sector_size) - 1;

    let report = UpperNorPatchReport {
        input_file: input_label.to_string(),
        output_file: output_label.to_string(),
        backup_file: backup_label.to_string(),
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
        first_sector_name: format!("SA{first_idx}"),
        last_sector_name: format!("SA{last_idx}"),
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
