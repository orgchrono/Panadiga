//! Low-level L2P, sector offset, migration bitmap, and 1->0 compatibility helpers for EH55 BGO FTL.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::constants::{AM34_EH55_FLASH_BGO_BASE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};

use super::{EH55_BGO_FIRST_SA, EH55_BGO_LAST_SA, EH55_BGO_SECTOR_STRIDE};

pub(super) fn read_le_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

/// Translates a physical Spansion sector index (`26..=70`) into its byte offset inside an 8 MiB NOR dump.
pub fn eh55_bgo_sector_offset(sa: usize) -> Result<usize> {
    if !(EH55_BGO_FIRST_SA..=EH55_BGO_LAST_SA).contains(&sa) {
        bail!(
            "EH55 BGO sector SA{sa} outside valid range SA{EH55_BGO_FIRST_SA}..=SA{EH55_BGO_LAST_SA}"
        );
    }
    let rel_base = (AM34_EH55_FLASH_BGO_BASE - AM34_FLASH_UNCACHED_BASE) as usize;
    Ok(rel_base + (sa - EH55_BGO_FIRST_SA) * EH55_BGO_SECTOR_STRIDE)
}

/// Evaluates the highest uncommitted migration bitmap slot (`fun_50423540`) at `buf[off..off+38]`.
#[must_use]
pub fn eval_bgo_migration_index(buf: &[u8], off: usize) -> Option<usize> {
    if off + 38 > buf.len() {
        return None;
    }
    let w0 = read_le_u16(buf, off);
    let w1 = read_le_u16(buf, off + 2);
    let w2 = read_le_u16(buf, off + 4);
    let w3 = read_le_u16(buf, off + 6);
    let mask = (!(w0 | w1 | w2)) & w3;
    if mask == 0 {
        return None;
    }
    let bit = 15 - mask.leading_zeros() as usize;
    for u in (bit + 1)..15 {
        if read_le_u16(buf, off + 8 + u * 2) != 0xFFFF {
            return None;
        }
    }
    Some(bit)
}

/// Resolves the migration target physical sector word (`fun_5042283d`) or `0xFFFF` if none.
#[must_use]
pub fn eval_bgo_migration_target(buf: &[u8], off: usize) -> u16 {
    match eval_bgo_migration_index(buf, off) {
        Some(idx) => read_le_u16(buf, off + 8 + idx * 2),
        None => 0xFFFF,
    }
}

/// Parses active L2P page-to-slot mappings across `SA41` (`0x220000`) and `SA29` (`0x160000`).
pub fn parse_eh55_bgo_l2p(nor_8mb: &[u8]) -> Result<BTreeMap<u16, u16>> {
    if nor_8mb.len() != AM34_FLASH_SIZE as usize {
        bail!(
            "Expected {AM34_FLASH_SIZE}-byte 8 MiB NOR dump, got {}",
            nor_8mb.len()
        );
    }
    let mut l2p = BTreeMap::new();
    for sa in [41usize, 29usize] {
        let base = eh55_bgo_sector_offset(sa)?;
        for sp in 2..256usize {
            let off = base + sp * 0x100;
            let b = &nor_8mb[off..off + 4];
            if b == [0xFF, 0xFF, 0xFF, 0xFF] {
                continue;
            }
            for s in 0..7usize {
                let soff = off + 4 + s * 0x24;
                let tid = read_le_u16(nor_8mb, soff);
                let t2 = read_le_u16(nor_8mb, soff + 2);
                let st = if t2 == 0 {
                    0u8
                } else {
                    (((b[3] >> s) & 1) << 3)
                        | (((b[2] >> s) & 1) << 2)
                        | (((b[1] >> s) & 1) << 1)
                        | ((b[0] >> s) & 1)
                };
                if st == 8 {
                    for i in 0..16u16 {
                        let w = read_le_u16(nor_8mb, soff + 4 + usize::from(i) * 2);
                        let pid = tid.wrapping_mul(16).wrapping_add(i);
                        if l2p.insert(pid, w).is_some() {
                            bail!("Duplicate active BGO L2P mapping for pid 0x{pid:04X}");
                        }
                    }
                }
            }
        }
    }
    Ok(l2p)
}

/// Verifies that every byte transition `orig -> target` satisfies pure NOR `1 -> 0` programming (`(o & n) == n`).
pub fn verify_nor_one_to_zero_compatible(orig: &[u8], target: &[u8]) -> Result<usize> {
    if orig.len() != target.len() {
        bail!(
            "NOR compatibility buffer length mismatch: orig={} vs target={}",
            orig.len(),
            target.len()
        );
    }
    let mut modified = 0usize;
    for (idx, (&o, &n)) in orig.iter().zip(target.iter()).enumerate() {
        if (o & n) != n {
            bail!("0->1 NOR bit violation at offset 0x{idx:06X}: old=0x{o:02X}, new=0x{n:02X}");
        }
        if o != n {
            modified += 1;
        }
    }
    Ok(modified)
}

pub(super) fn read_bgo_page<'a>(
    nor_8mb: &'a [u8],
    l2p: &BTreeMap<u16, u16>,
    f2_bases: &BTreeMap<u16, usize>,
    pid: u16,
) -> Option<&'a [u8]> {
    let &w = l2p.get(&pid)?;
    if w == 0xFFFF {
        return None;
    }
    let fid = w >> 8;
    let slot = usize::from(w & 0xFF) >> 1;
    if slot >= 126 {
        return None;
    }
    let &base = f2_bases.get(&fid)?;
    let tag_pid = read_le_u16(nor_8mb, base + 0x200 + slot * 4);
    if tag_pid != pid {
        return None;
    }
    let poff = base + 0x400 + slot * 0x200;
    nor_8mb.get(poff..poff + 0x200)
}
