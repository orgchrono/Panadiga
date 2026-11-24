//! Pure UF2 block validation, family decoding, and RP2350-E10 unified UF2 combiner.

use anyhow::{Result, bail};

use super::{
    RP2040_FAMILY_ID, RP2040_SDK_FAMILY_ID, RP2350_ABS_SDK_FAMILY_ID, RP2350_ARM_S_FAMILY_ID,
    RP2350_ARM_S_SDK_FAMILY_ID, RP2350_RISCV_FAMILY_ID, RP2350_RISCV_SDK_FAMILY_ID, UF2_BLOCK_SIZE,
    UF2_FLAG_FAMILY_ID_PRESENT, UF2_MAGIC_END, UF2_MAGIC_START0, UF2_MAGIC_START1,
};

/// Typed errors returned by UF2 validation, slicing, and unified-image combination.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Uf2Error {
    #[error("Invalid UF2 payload length: {len} bytes (must be a positive multiple of 512 bytes)")]
    InvalidLength { len: usize },
    #[error("Invalid UF2 image '{label}' (size {len} bytes must be non-empty multiple of 512)")]
    InvalidLabeledLength { label: String, len: usize },
    #[error(
        "Corrupt UF2 block {block_index} magic (expected 0x0A324655/0x9E5D5157/0x0AB16F30, got 0x{start0:08X}/0x{start1:08X}/0x{end_magic:08X})"
    )]
    CorruptMagic {
        block_index: usize,
        start0: u32,
        start1: u32,
        end_magic: u32,
    },
    #[error("Corrupt UF2 block {block_index} in '{label}'")]
    CorruptLabeledMagic { block_index: usize, label: String },
    #[error("Unified UF2 contains zero blocks matching target {target:?}")]
    EmptyFamilySlice { target: Uf2TargetFamily },
}

#[inline]
fn read_u32_le(blk: &[u8; UF2_BLOCK_SIZE], offset: usize) -> u32 {
    u32::from_le_bytes([
        blk[offset],
        blk[offset + 1],
        blk[offset + 2],
        blk[offset + 3],
    ])
}

fn verify_block_magic(
    blk: &[u8; UF2_BLOCK_SIZE],
    blk_idx: usize,
    label: Option<&str>,
) -> Result<()> {
    let start0 = read_u32_le(blk, 0);
    let start1 = read_u32_le(blk, 4);
    let end_magic = read_u32_le(blk, 508);
    if start0 != UF2_MAGIC_START0 || start1 != UF2_MAGIC_START1 || end_magic != UF2_MAGIC_END {
        if let Some(tag) = label {
            return Err(Uf2Error::CorruptLabeledMagic {
                block_index: blk_idx,
                label: tag.to_string(),
            }
            .into());
        }
        return Err(Uf2Error::CorruptMagic {
            block_index: blk_idx,
            start0,
            start1,
            end_magic,
        }
        .into());
    }
    Ok(())
}

/// Validates that every 512-byte block in a buffer conforms to the UF2 specification.
/// Silicon target family carried inside a unified RP2040 / RP2350 UF2 container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Uf2TargetFamily {
    /// Raspberry Pi Pico / Pico W (`RP2040`).
    Rp2040,
    /// Raspberry Pi Pico 2 / Pico 2 W (`RP2350` ARM Cortex-M33 or RISC-V Hazard3).
    Rp2350,
}

impl Uf2TargetFamily {
    /// Detects the desired UF2 target family from `INFO_UF2.TXT` contents and optional override.
    ///
    /// Returns `None` when the caller explicitly requests `"unified"` / `"universal"` or when
    /// neither target string is recognized (fail-safe fallback writes the full unified UF2).
    pub fn detect(info_content: &str, target_override: Option<&str>) -> Option<Self> {
        if let Some(over) = target_override {
            let lower = over.to_lowercase();
            if lower.contains("unified") || lower.contains("universal") {
                return None;
            }
            if lower.contains("2350") || lower.contains("pico2") || lower.contains("pico 2") {
                return Some(Self::Rp2350);
            }
            if lower.contains("2040") || lower.contains("pico1") {
                return Some(Self::Rp2040);
            }
            return None;
        }
        if info_content.contains("RP2350") || info_content.contains("Pico 2") {
            Some(Self::Rp2350)
        } else if info_content.contains("RP2040") || info_content.contains("RPI-RP2") {
            Some(Self::Rp2040)
        } else {
            None
        }
    }

    fn matches_family_id(self, family_id: u32) -> bool {
        match self {
            Self::Rp2040 => matches!(family_id, RP2040_FAMILY_ID | RP2040_SDK_FAMILY_ID),
            Self::Rp2350 => matches!(
                family_id,
                RP2350_ABS_SDK_FAMILY_ID
                    | RP2350_ARM_S_FAMILY_ID
                    | RP2350_ARM_S_SDK_FAMILY_ID
                    | RP2350_RISCV_FAMILY_ID
                    | RP2350_RISCV_SDK_FAMILY_ID
            ),
        }
    }
}

/// Extracts only the blocks belonging to `target` from a unified UF2 buffer and re-indexes
/// `blockNo` (`0..N-1`) and `numBlocks` (`N`) purely in memory without touching disk.
pub fn extract_uf2_family_slice(unified_uf2: &[u8], target: Uf2TargetFamily) -> Result<Vec<u8>> {
    validate_uf2_buffer(unified_uf2)?;
    let parsed = parse_uf2_blocks(unified_uf2, "unified_uf2")?;
    let mut selected: Vec<[u8; UF2_BLOCK_SIZE]> = parsed
        .into_iter()
        .filter(|blk| target.matches_family_id(block_family_id(blk)))
        .collect();
    if selected.is_empty() {
        return Err(Uf2Error::EmptyFamilySlice { target }.into());
    }
    let total_blocks = u32::try_from(selected.len())?;
    let mut out = Vec::with_capacity(selected.len() * UF2_BLOCK_SIZE);
    for (idx, blk) in selected.iter_mut().enumerate() {
        let block_no = u32::try_from(idx)?;
        blk[20..24].copy_from_slice(&block_no.to_le_bytes());
        blk[24..28].copy_from_slice(&total_blocks.to_le_bytes());
        out.extend_from_slice(blk);
    }
    Ok(out)
}

pub fn validate_uf2_buffer(data: &[u8]) -> Result<(usize, String)> {
    if data.len() < UF2_BLOCK_SIZE || !data.len().is_multiple_of(UF2_BLOCK_SIZE) {
        return Err(Uf2Error::InvalidLength { len: data.len() }.into());
    }

    let num_blocks = data.len() / UF2_BLOCK_SIZE;
    let mut has_rp2040 = false;
    let mut has_rp2350_arm = false;
    let mut has_rp2350_riscv = false;
    let mut has_rp2350_abs = false;
    let mut first_other_family = None;

    for (blk_idx, blk) in data.as_chunks::<UF2_BLOCK_SIZE>().0.iter().enumerate() {
        verify_block_magic(blk, blk_idx, None)?;

        let family_id = read_u32_le(blk, 28);
        match family_id {
            RP2040_FAMILY_ID | RP2040_SDK_FAMILY_ID => has_rp2040 = true,
            RP2350_ABS_SDK_FAMILY_ID => has_rp2350_abs = true,
            RP2350_ARM_S_FAMILY_ID | RP2350_ARM_S_SDK_FAMILY_ID => has_rp2350_arm = true,
            RP2350_RISCV_FAMILY_ID | RP2350_RISCV_SDK_FAMILY_ID => has_rp2350_riscv = true,
            other => {
                if first_other_family.is_none() {
                    first_other_family = Some(other);
                }
            }
        }
    }

    let family_name = if has_rp2040 && (has_rp2350_arm || has_rp2350_riscv || has_rp2350_abs) {
        "Universal RP2040 + RP2350 (Unified Multi-Family UF2)".to_string()
    } else if has_rp2350_riscv {
        "Raspberry Pi Pico 2 RISC-V (RP2350)".to_string()
    } else if has_rp2350_arm || has_rp2350_abs {
        "Raspberry Pi Pico 2 ARM-S (RP2350)".to_string()
    } else if has_rp2040 {
        "Raspberry Pi RP2040".to_string()
    } else {
        format!(
            "Pico-Compatible Controller (Family ID 0x{:08X})",
            first_other_family.unwrap_or(0)
        )
    };

    Ok((num_blocks, family_name))
}

fn parse_uf2_blocks(bytes: &[u8], label: &str) -> Result<Vec<[u8; UF2_BLOCK_SIZE]>> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(UF2_BLOCK_SIZE) {
        return Err(Uf2Error::InvalidLabeledLength {
            label: label.to_string(),
            len: bytes.len(),
        }
        .into());
    }
    let mut blocks = Vec::with_capacity(bytes.len() / UF2_BLOCK_SIZE);
    for (idx, chunk) in bytes.as_chunks::<UF2_BLOCK_SIZE>().0.iter().enumerate() {
        verify_block_magic(chunk, idx, Some(label))?;
        let mut arr = [0u8; UF2_BLOCK_SIZE];
        arr.copy_from_slice(chunk);
        blocks.push(arr);
    }
    Ok(blocks)
}

fn block_family_id(block: &[u8; UF2_BLOCK_SIZE]) -> u32 {
    let flags = read_u32_le(block, 8);
    if (flags & UF2_FLAG_FAMILY_ID_PRESENT) != 0 {
        read_u32_le(block, 28)
    } else {
        0
    }
}

/// Pure combiner building an RP2350-E10 compliant universal UF2 buffer from RP2040 + RP2350 inputs.
pub(super) fn combine_unified_uf2_bytes(
    rp2040_bytes: &[u8],
    rp2040_label: &str,
    rp2350_bytes: &[u8],
    rp2350_label: &str,
) -> Result<(Vec<u8>, usize, String)> {
    let rp2040_blocks = parse_uf2_blocks(rp2040_bytes, rp2040_label)?;
    let rp2350_blocks = parse_uf2_blocks(rp2350_bytes, rp2350_label)?;

    let mut rp2350_abs = Vec::new();
    let mut rp2350_exec = Vec::new();
    for blk in rp2350_blocks {
        if block_family_id(&blk) == RP2350_ABS_SDK_FAMILY_ID {
            rp2350_abs.push(blk);
        } else {
            rp2350_exec.push(blk);
        }
    }

    if rp2040_blocks.is_empty() || rp2350_exec.is_empty() {
        bail!("Input UF2 images must both contain executable firmware blocks");
    }

    let total_blocks = rp2350_abs.len() + rp2040_blocks.len() + rp2350_exec.len();
    let mut combined = Vec::with_capacity(total_blocks * UF2_BLOCK_SIZE);

    for blk in &rp2350_abs {
        combined.extend_from_slice(blk);
    }
    for blk in &rp2040_blocks[..rp2040_blocks.len() - 1] {
        combined.extend_from_slice(blk);
    }
    for blk in &rp2350_exec[..rp2350_exec.len() - 1] {
        combined.extend_from_slice(blk);
    }
    combined.extend_from_slice(&rp2040_blocks[rp2040_blocks.len() - 1]);
    combined.extend_from_slice(&rp2350_exec[rp2350_exec.len() - 1]);

    let (_, sha256_hex) = crate::nvram::calculate_hashes(&combined);
    Ok((combined, total_blocks, sha256_hex))
}
