//! Pure policy-gated CoW NVRAM erasure (`0xFF`) and micro-sector patching transformations.

use anyhow::{Result, bail};

use super::audit::{audit_nvram_sectors, calculate_hashes};
use super::types::{
    NvramCleanReport, NvramPolicyOptions, PureNvramCleanOutput, SliceIoLabels,
    authorize_flash_operation,
};
use crate::constants::FLASH_ERASED_BYTE;
use crate::ssot::Capability;

fn transform_flash_nvram_slice(
    flash_bytes: &[u8],
    policy: &NvramPolicyOptions<'_>,
    labels: SliceIoLabels<'_>,
    mutator: impl FnOnce(&mut [u8]) -> Result<()>,
) -> Result<PureNvramCleanOutput> {
    let layout = authorize_flash_operation(
        flash_bytes,
        Capability::NvramSanitizeOfflineCow,
        policy.model_id,
        policy.lab_override,
    )?;
    let nvram_base_offset = layout.nvram_base_offset;
    let nvram_end_offset = layout.nvram_end_offset;
    let bootloader_size = nvram_base_offset;

    let (orig_md5, orig_sha256) = calculate_hashes(flash_bytes);
    let backup_nvram_bytes = flash_bytes[nvram_base_offset..nvram_end_offset].to_vec();

    let mut cleaned_bytes = flash_bytes.to_vec();
    mutator(&mut cleaned_bytes[nvram_base_offset..nvram_end_offset])?;

    if cleaned_bytes[..bootloader_size] != flash_bytes[..bootloader_size] {
        bail!(
            "[CRITICAL ERROR] Bootloader/OS memory region (0x00000000 - 0x{:08X}) was modified! Aborting.",
            bootloader_size
        );
    }

    let (non_ff, non_zero, microsectors) = audit_nvram_sectors(
        &cleaned_bytes[nvram_base_offset..nvram_end_offset],
        &layout.nvram_sectors,
    );
    let (clean_md5, clean_sha256) = calculate_hashes(&cleaned_bytes);

    let report = NvramCleanReport {
        input_file: labels.input.to_string(),
        output_file: labels.output.to_string(),
        backup_file: labels.backup.to_string(),
        original_md5: orig_md5,
        original_sha256: orig_sha256,
        clean_md5,
        clean_sha256,
        total_flash_bytes: layout.flash_len,
        nvram_offset_start: nvram_base_offset,
        nvram_offset_end: nvram_end_offset,
        nvram_non_ff_bytes: non_ff,
        nvram_non_zero_bytes: non_zero,
        microsectors,
        bootloader_immutable: true,
    };

    Ok(PureNvramCleanOutput {
        cleaned_bytes,
        backup_nvram_bytes,
        report,
    })
}

/// Policy-gated Copy-on-Write NVRAM `0xFF` erasure over an immutable flash slice.
pub fn clean_flash_nvram_slice_with_policy(
    flash_bytes: &[u8],
    policy: &NvramPolicyOptions<'_>,
    labels: SliceIoLabels<'_>,
) -> Result<PureNvramCleanOutput> {
    transform_flash_nvram_slice(flash_bytes, policy, labels, |nv| {
        nv.fill(FLASH_ERASED_BYTE);
        Ok(())
    })
}

/// Policy-gated Copy-on-Write NVRAM byte patch inside verified EEMUL top micro-sectors.
pub fn patch_flash_nvram_slice_with_policy(
    flash_bytes: &[u8],
    nvram_rel_offset: usize,
    patch_bytes: &[u8],
    policy: &NvramPolicyOptions<'_>,
    labels: SliceIoLabels<'_>,
) -> Result<PureNvramCleanOutput> {
    if patch_bytes.is_empty() {
        bail!("NVRAM patch payload cannot be empty");
    }
    transform_flash_nvram_slice(flash_bytes, policy, labels, |nv| {
        let Some(end) = nvram_rel_offset.checked_add(patch_bytes.len()) else {
            bail!("NVRAM patch offset overflow");
        };
        if end > nv.len() {
            bail!(
                "Guarded NVRAM write rejected [0x{nvram_rel_offset:X}..0x{end:X}): exceeds {}B writable NVRAM sector (IPL protected)",
                nv.len()
            );
        }
        nv[nvram_rel_offset..end].copy_from_slice(patch_bytes);
        Ok(())
    })
}

/// Fail-closed default clean wrapper delegating strictly through `CapabilityGate` (no overrides).
pub fn clean_flash_nvram_slice(
    flash_bytes: &[u8],
    input_label: &str,
    output_label: &str,
    backup_label: &str,
) -> Result<PureNvramCleanOutput> {
    clean_flash_nvram_slice_with_policy(
        flash_bytes,
        &NvramPolicyOptions::default(),
        SliceIoLabels::new(input_label, output_label, backup_label),
    )
}

/// Fail-closed default patch wrapper delegating strictly through `CapabilityGate` (no overrides).
pub fn patch_flash_nvram_slice(
    flash_bytes: &[u8],
    nvram_rel_offset: usize,
    patch_bytes: &[u8],
    input_label: &str,
    output_label: &str,
    backup_label: &str,
) -> Result<PureNvramCleanOutput> {
    patch_flash_nvram_slice_with_policy(
        flash_bytes,
        nvram_rel_offset,
        patch_bytes,
        &NvramPolicyOptions::default(),
        SliceIoLabels::new(input_label, output_label, backup_label),
    )
}
