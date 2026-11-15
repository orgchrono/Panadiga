//! Pure UF2 block validation, family decoding, and RP2350-E10 unified UF2 combiner.

use anyhow::{Result, bail};

use super::{
    RP2040_FAMILY_ID, RP2040_SDK_FAMILY_ID, RP2350_ABS_SDK_FAMILY_ID, RP2350_ARM_S_FAMILY_ID,
    RP2350_ARM_S_SDK_FAMILY_ID, RP2350_RISCV_FAMILY_ID, RP2350_RISCV_SDK_FAMILY_ID, UF2_BLOCK_SIZE,
    UF2_FLAG_FAMILY_ID_PRESENT, UF2_MAGIC_END, UF2_MAGIC_START0, UF2_MAGIC_START1,
};

fn verify_block_magic(blk: &[u8], blk_idx: usize, label: Option<&str>) -> Result<()> {
    let start0 = u32::from_le_bytes(blk[0..4].try_into().unwrap());
    let start1 = u32::from_le_bytes(blk[4..8].try_into().unwrap());
    let end_magic = u32::from_le_bytes(blk[508..512].try_into().unwrap());
    if start0 != UF2_MAGIC_START0 || start1 != UF2_MAGIC_START1 || end_magic != UF2_MAGIC_END {
        if let Some(tag) = label {
            bail!("Corrupt UF2 block {} in '{}'", blk_idx, tag);
        }
        bail!(
            "Corrupt UF2 block {} magic (expected 0x{:08X}/0x{:08X}/0x{:08X}, got 0x{:08X}/0x{:08X}/0x{:08X})",
            blk_idx,
            UF2_MAGIC_START0,
            UF2_MAGIC_START1,
            UF2_MAGIC_END,
            start0,
            start1,
            end_magic
        );
    }
    Ok(())
}

/// Validates that every 512-byte block in a buffer conforms to the UF2 specification.
pub fn validate_uf2_buffer(data: &[u8]) -> Result<(usize, String)> {
    if data.len() < UF2_BLOCK_SIZE || !data.len().is_multiple_of(UF2_BLOCK_SIZE) {
        bail!(
            "Invalid UF2 payload length: {} bytes (must be a positive multiple of 512 bytes)",
            data.len()
        );
    }

    let num_blocks = data.len() / UF2_BLOCK_SIZE;
    let mut has_rp2040 = false;
    let mut has_rp2350_arm = false;
    let mut has_rp2350_riscv = false;
    let mut has_rp2350_abs = false;
    let mut first_other_family = None;

    for (blk_idx, blk) in data.as_chunks::<UF2_BLOCK_SIZE>().0.iter().enumerate() {
        verify_block_magic(blk, blk_idx, None)?;

        let family_id = u32::from_le_bytes(blk[28..32].try_into().unwrap());
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
        bail!(
            "Invalid UF2 image '{}' (size {} bytes must be non-empty multiple of 512)",
            label,
            bytes.len()
        );
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
    let flags = u32::from_le_bytes(block[8..12].try_into().unwrap());
    if (flags & UF2_FLAG_FAMILY_ID_PRESENT) != 0 {
        u32::from_le_bytes(block[28..32].try_into().unwrap())
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
