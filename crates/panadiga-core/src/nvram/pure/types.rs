//! Domain DTOs, derived Spansion flash sector geometry, and policy-gated authorization for NVRAM & Upper NOR.

use anyhow::{Context, Result, bail};

use crate::constants::{
    AM34_FLASH_CRITICAL_IPL_SIZE, FLASH_16MB_NVRAM_SIZE_BYTES, SPANSION_NVRAM_SIZE_BYTES,
};
use crate::ssot::{
    Capability, CapabilityGate, DerivedSector, FlashChipProfile, HardwareProfileRegistry,
    LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride, NorRegionKind, ResolvedModelProfile,
    RuntimeFingerprint, derive_sector_table, fingerprint_nor_or_firmware,
};

/// Audit metrics for an individual NOR flash sector (`8 KB` / `16 KB` / `32 KB` / `128 KB`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MicroSectorAudit {
    pub sector_name: String,
    pub offset: usize,
    pub active_bytes: usize,
    pub total_bytes: usize,
    pub header_hex: String,
}

/// Comprehensive audit summary of the flash cleaning operation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NvramCleanReport {
    pub input_file: String,
    pub output_file: String,
    pub backup_file: String,
    pub original_md5: String,
    pub original_sha256: String,
    pub clean_md5: String,
    pub clean_sha256: String,
    pub total_flash_bytes: usize,
    pub nvram_offset_start: usize,
    pub nvram_offset_end: usize,
    pub nvram_non_ff_bytes: usize,
    pub nvram_non_zero_bytes: usize,
    pub microsectors: Vec<MicroSectorAudit>,
    pub bootloader_immutable: bool,
}

/// Pure functional output containing cleaned flash bytes, extracted backup, and audit report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PureNvramCleanOutput {
    pub cleaned_bytes: Vec<u8>,
    pub backup_nvram_bytes: Vec<u8>,
    pub report: NvramCleanReport,
}

/// Comprehensive audit summary for a Copy-on-Write Upper NOR App/BGO flash patch.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpperNorPatchReport {
    pub input_file: String,
    pub output_file: String,
    pub backup_file: String,
    pub original_md5: String,
    pub original_sha256: String,
    pub patched_md5: String,
    pub patched_sha256: String,
    pub backup_sector_sha256: String,
    pub patched_range_before_sha256: String,
    pub patched_range_after_sha256: String,
    pub total_flash_bytes: usize,
    pub patch_offset_start: usize,
    pub patch_offset_end: usize,
    pub patch_length: usize,
    pub backup_sector_start: usize,
    pub backup_sector_end: usize,
    pub first_sector_name: String,
    pub last_sector_name: String,
    pub critical_ipl_override_used: bool,
    pub critical_ipl_untouched: bool,
    pub nvram_untouched: bool,
}

/// Pure functional output containing patched flash bytes, pre-patch sector backup, and audit report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PureUpperNorPatchOutput {
    pub patched_bytes: Vec<u8>,
    pub backup_sector_bytes: Vec<u8>,
    pub report: UpperNorPatchReport,
}

/// Model pinning & audited offline override policy options for CoW NVRAM/Upper NOR operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct NvramPolicyOptions<'a> {
    pub model_id: Option<&'a str>,
    pub lab_override: Option<&'a LabOverride>,
}

/// Descriptive labels recorded into pure CoW audit reports without filesystem side effects.
#[derive(Debug, Clone, Copy)]
pub struct SliceIoLabels<'a> {
    pub input: &'a str,
    pub output: &'a str,
    pub backup: &'a str,
}

impl<'a> SliceIoLabels<'a> {
    #[must_use]
    pub const fn new(input: &'a str, output: &'a str, backup: &'a str) -> Self {
        Self {
            input,
            output,
            backup,
        }
    }
}

pub(super) struct FlashLayoutGeometry {
    pub flash_len: usize,
    pub critical_ipl_end: usize,
    pub nvram_base_offset: usize,
    pub nvram_end_offset: usize,
    pub sectors: Vec<DerivedSector>,
    pub nvram_sectors: Vec<DerivedSector>,
}

fn canonical_flash_chip_id_for_capacity(flash_len: usize) -> Result<&'static str> {
    match flash_len {
        0x0020_0000 => Ok("spansion_s29al016d_2mb_top"),
        0x0040_0000 => Ok("spansion_s29jl032h_4mb_top"),
        0x0080_0000 => Ok("spansion_s29jl064h_8mb"),
        0x0100_0000 => Ok("spansion_s29gl128n_16mb"),
        _ => bail!(
            "Invalid flash dump size: {} bytes. Expected standard Panasonic DIGA NOR Flash capacity (2 MB, 4 MB, 8 MB or 16 MB).",
            flash_len
        ),
    }
}

pub(super) fn canonical_chip_for_capacity(flash_len: usize) -> Result<FlashChipProfile> {
    let chip_id = canonical_flash_chip_id_for_capacity(flash_len)?;
    let chip = HardwareProfileRegistry::builtin()
        .bundle()
        .flash_chips
        .get(chip_id)
        .with_context(|| format!("Missing canonical flash chip '{chip_id}' in SSOT registry"))?;
    Ok(chip.clone())
}

fn default_nvram_tail_bytes(flash_len: usize) -> usize {
    if flash_len >= 16 * 1024 * 1024 {
        FLASH_16MB_NVRAM_SIZE_BYTES
    } else {
        SPANSION_NVRAM_SIZE_BYTES
    }
}

pub(super) fn resolve_flash_layout(
    flash_len: usize,
    resolved_model: Option<&ResolvedModelProfile>,
) -> Result<FlashLayoutGeometry> {
    let chip = match resolved_model.and_then(|r| r.flash_chip.as_ref()) {
        Some(c) => {
            if !c.writable {
                bail!("Flash chip '{}' is read-only OTP PROM", c.part_number);
            }
            if c.total_bytes as usize != flash_len {
                bail!(
                    "Dump size ({flash_len} B) mismatches declared flash '{}' ({} B)",
                    c.part_number,
                    c.total_bytes
                );
            }
            c.clone()
        }
        None => canonical_chip_for_capacity(flash_len)?,
    };

    let sectors = derive_sector_table(&chip)?;
    let critical_ipl_end = resolved_model
        .and_then(|r| r.nor_layout.as_ref())
        .and_then(|l| l.find_region(NorRegionKind::Ipl))
        .map_or(AM34_FLASH_CRITICAL_IPL_SIZE as usize, |r| {
            r.end_offset as usize + 1
        });

    let (nvram_base_offset, nvram_end_offset) = resolved_model
        .and_then(|r| r.nor_layout.as_ref())
        .and_then(|l| l.find_region(NorRegionKind::Eemul))
        .map_or_else(
            || {
                (
                    flash_len.saturating_sub(default_nvram_tail_bytes(flash_len)),
                    flash_len,
                )
            },
            |r| (r.start_offset as usize, r.end_offset as usize + 1),
        );

    let nvram_sectors: Vec<DerivedSector> = sectors
        .iter()
        .filter(|s| {
            let s_start = s.start_offset as usize;
            let s_end = s.end_offset_inclusive as usize + 1;
            nvram_base_offset < s_end && nvram_end_offset > s_start
        })
        .cloned()
        .collect();

    if nvram_sectors.is_empty() {
        bail!(
            "Resolved flash '{}' yielded zero NVRAM sectors for span 0x{nvram_base_offset:06X}..0x{nvram_end_offset:06X}",
            chip.part_number
        );
    }

    Ok(FlashLayoutGeometry {
        flash_len,
        critical_ipl_end,
        nvram_base_offset,
        nvram_end_offset,
        sectors,
        nvram_sectors,
    })
}

/// Resolves exact NVRAM tail sectors for a capacity without authorizing writes (used for C0 audit).
pub(super) fn nvram_sectors_for_capacity(flash_len: usize) -> Result<Vec<DerivedSector>> {
    Ok(resolve_flash_layout(flash_len, None)?.nvram_sectors)
}

/// Shared fail-closed policy enforcement before any offline CoW NVRAM or Upper NOR mutation.
pub(super) fn authorize_flash_operation(
    flash_bytes: &[u8],
    capability: Capability,
    model_id: Option<&str>,
    lab_override: Option<&LabOverride>,
) -> Result<FlashLayoutGeometry> {
    let fp = fingerprint_nor_or_firmware(flash_bytes);
    let explicit_model = model_id
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("AUTO"));
    let target_model = explicit_model.or(fp.detected_model.as_deref());

    if let Some(model_name) = target_model {
        let resolved = HardwareProfileRegistry::builtin().resolve_model(model_name)?;
        let runtime_fp = RuntimeFingerprint {
            dump_size_bytes: Some(flash_bytes.len() as u64),
            has_eemul_f1_marker: fp.has_eemul_f1_marker,
            ..Default::default()
        };
        match CapabilityGate::authorize(&resolved, capability, &runtime_fp, lab_override) {
            Ok(_) => resolve_flash_layout(flash_bytes.len(), Some(&resolved)),
            Err(denial) => bail!(
                "Policy denied {:?} for model '{}': {} (lab_override_eligible={})",
                denial.capability,
                denial.model_id,
                denial.reason,
                denial.lab_override_eligible
            ),
        }
    } else {
        let valid_override = lab_override.is_some_and(|ov| {
            ov.disclaimer_ack.trim() == LAB_OVERRIDE_DISCLAIMER_ACK
                && ov.operator_reason.trim().len() >= 5
        });
        if !valid_override {
            bail!(
                "Policy denied {:?} for unidentified flash dump ({}): size-only heuristics are locked. Pass a verified model or LabOverride (ack='{}', reason >= 5 chars).",
                capability,
                fp.display_label,
                LAB_OVERRIDE_DISCLAIMER_ACK
            );
        }
        resolve_flash_layout(flash_bytes.len(), None)
    }
}
