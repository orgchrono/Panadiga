#![deny(unsafe_code)]

//! Low-level L2P, sector offset, migration bitmap, and 1->0 compatibility helpers for BGO FTL.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::constants::{AM34_EH55_FLASH_BGO_BASE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};

use super::{
    EH55_BGO_EXPECTED_P2_COUNT, EH55_BGO_FIRST_SA, EH55_BGO_LAST_SA, EH55_BGO_MAX_PID,
    EH55_BGO_SECTOR_STRIDE,
};

/// Parameterized NOR BGO FTL geometry (defaults to DMR-EH55 `SA26..=SA70` in 8 MiB flash).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BgoSectorGeometry {
    pub first_sa: usize,
    pub last_sa: usize,
    pub base_offset: usize,
    pub sector_stride: usize,
    pub expected_flash_bytes: usize,
    pub expected_p2_count: usize,
    pub max_pid: u16,
}

impl BgoSectorGeometry {
    /// Returns the verified Panasonic DMR-EH55 (`Loader ver0.61`) 45-sector BGO geometry.
    #[must_use]
    pub const fn eh55_default() -> Self {
        Self {
            first_sa: EH55_BGO_FIRST_SA,
            last_sa: EH55_BGO_LAST_SA,
            base_offset: (AM34_EH55_FLASH_BGO_BASE - AM34_FLASH_UNCACHED_BASE) as usize,
            sector_stride: EH55_BGO_SECTOR_STRIDE,
            expected_flash_bytes: AM34_FLASH_SIZE as usize,
            expected_p2_count: EH55_BGO_EXPECTED_P2_COUNT,
            max_pid: EH55_BGO_MAX_PID,
        }
    }

    /// Derives [`BgoSectorGeometry`] dynamically from a resolved SSOT hardware profile.
    pub fn from_resolved_profile(resolved: &crate::ssot::ResolvedModelProfile) -> Result<Self> {
        let chip = resolved.flash_chip.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Profile '{}' does not define flash_chip metadata",
                resolved.manifest.model_id
            )
        })?;
        let layout = resolved.nor_layout.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Profile '{}' does not define nor_layout regions",
                resolved.manifest.model_id
            )
        })?;
        let bgo = layout
            .find_region(crate::ssot::NorRegionKind::Bgo)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Profile '{}' does not contain a BGO NOR region",
                    resolved.manifest.model_id
                )
            })?;
        let (first_u32, last_u32) =
            crate::ssot::sector_range_for_aligned_span(chip, bgo.start_offset, bgo.end_offset)?;
        let sectors = crate::ssot::derive_sector_table(chip)?;
        let first_sa = first_u32 as usize;
        let last_sa = last_u32 as usize;
        let sector_stride = sectors
            .get(first_sa)
            .map(|s| s.size_bytes as usize)
            .ok_or_else(|| anyhow::anyhow!("Derived BGO SA{first_sa} missing from sector table"))?;
        if sectors[first_sa..=last_sa]
            .iter()
            .any(|s| s.size_bytes as usize != sector_stride)
        {
            anyhow::bail!(
                "Profile '{}' BGO span SA{first_sa}..=SA{last_sa} has non-uniform sector stride",
                resolved.manifest.model_id
            );
        }
        let total_bgo_sectors = last_sa - first_sa + 1;
        let expected_p2_count = if total_bgo_sectors >= 5 {
            total_bgo_sectors - 5
        } else {
            total_bgo_sectors.saturating_sub(1)
        };
        let max_pid = if resolved.manifest.model_id.eq_ignore_ascii_case("DMR-EH55")
            || expected_p2_count == EH55_BGO_EXPECTED_P2_COUNT
        {
            EH55_BGO_MAX_PID
        } else {
            (expected_p2_count.saturating_mul(126)).min(usize::from(u16::MAX - 1)) as u16
        };
        Ok(Self {
            first_sa,
            last_sa,
            base_offset: usize::try_from(bgo.start_offset)?,
            sector_stride,
            expected_flash_bytes: usize::try_from(chip.total_bytes)?,
            expected_p2_count,
            max_pid,
        })
    }

    /// Resolves a model profile by ID (e.g. `"DMR-EH55"`) from [`HardwareProfileRegistry`]
    /// and computes its runtime [`BgoSectorGeometry`].
    pub fn from_model_profile(model_id: &str) -> Result<Self> {
        let resolved = crate::ssot::HardwareProfileRegistry::builtin().resolve_model(model_id)?;
        Self::from_resolved_profile(&resolved)
    }

    /// Resolves a runtime [`BgoSectorGeometry`] matching an arbitrary flash dump byte length.
    pub fn for_flash_size(flash_len: usize) -> Result<Self> {
        let reg = crate::ssot::HardwareProfileRegistry::builtin();
        let models = reg.list_models();
        let has_bgo_len = |m: &&crate::ssot::ResolvedModelProfile| {
            m.flash_chip
                .as_ref()
                .is_some_and(|c| c.total_bytes as usize == flash_len)
                && m.nor_layout
                    .as_ref()
                    .and_then(|l| l.find_region(crate::ssot::NorRegionKind::Bgo))
                    .is_some()
        };
        let resolved = models
            .iter()
            .find(|m| m.manifest.model_id.eq_ignore_ascii_case("DMR-EH55") && has_bgo_len(m))
            .or_else(|| models.iter().find(has_bgo_len))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No SSOT model profile with BGO region matches {flash_len}-byte NOR"
                )
            })?;
        Self::from_resolved_profile(resolved)
    }

    /// Translates a physical sector index inside `[first_sa..=last_sa]` to its NOR byte offset.
    pub fn sector_offset(&self, sa: usize) -> Result<usize> {
        if !(self.first_sa..=self.last_sa).contains(&sa) {
            bail!(
                "BGO sector SA{sa} outside valid range SA{}..=SA{}",
                self.first_sa,
                self.last_sa
            );
        }
        Ok(self.base_offset + (sa - self.first_sa) * self.sector_stride)
    }
}

impl Default for BgoSectorGeometry {
    fn default() -> Self {
        Self::eh55_default()
    }
}

pub(super) fn read_le_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

/// Translates a physical Spansion sector index (`26..=70`) into its byte offset inside an 8 MiB NOR dump.
pub fn eh55_bgo_sector_offset(sa: usize) -> Result<usize> {
    BgoSectorGeometry::eh55_default().sector_offset(sa)
}

/// Returns `true` when the sector at `base` carries a consistent `0xF101` P1 signature and 0x100-byte zero marker.
#[must_use]
pub fn is_active_bgo_p1_sector(nor: &[u8], base: usize) -> bool {
    if base + 0x200 > nor.len() {
        return false;
    }
    let w0 = read_le_u16(nor, base + 4);
    w0 == 0xF101
        && (0..4).all(|i| read_le_u16(nor, base + 4 + i * 4) == 0xF101)
        && nor[base + 0x100..base + 0x200].iter().all(|&b| b == 0)
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

/// Parses active L2P page-to-slot mappings across all `0xF101` BGO Partition-1 sectors in `geom`.
pub fn parse_bgo_l2p_with_geometry(
    nor: &[u8],
    geom: &BgoSectorGeometry,
) -> Result<BTreeMap<u16, u16>> {
    if nor.len() != geom.expected_flash_bytes {
        bail!(
            "Expected {}-byte NOR dump, got {}",
            geom.expected_flash_bytes,
            nor.len()
        );
    }
    let mut l2p = BTreeMap::new();
    for sa in geom.first_sa..=geom.last_sa {
        let base = geom.sector_offset(sa)?;
        if !is_active_bgo_p1_sector(nor, base) {
            continue;
        }
        for sp in 2..256usize {
            let off = base + sp * 0x100;
            let b = &nor[off..off + 4];
            if b == [0xFF, 0xFF, 0xFF, 0xFF] {
                continue;
            }
            for s in 0..7usize {
                let soff = off + 4 + s * 0x24;
                let tid = read_le_u16(nor, soff);
                let t2 = read_le_u16(nor, soff + 2);
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
                        let w = read_le_u16(nor, soff + 4 + usize::from(i) * 2);
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

/// Parses active L2P page-to-slot mappings by scanning all active `0xF101` Partition-1 sectors in EH55 `SA26..=SA70`.
pub fn parse_eh55_bgo_l2p(nor_8mb: &[u8]) -> Result<BTreeMap<u16, u16>> {
    parse_bgo_l2p_with_geometry(nor_8mb, &BgoSectorGeometry::eh55_default())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_p1_0xf101_discovery_on_shifted_sectors() {
        let mut nor = vec![0xFFu8; AM34_FLASH_SIZE as usize];
        for (sa, tid, slot_val) in [(35usize, 0u16, 0x0204u16), (50usize, 1u16, 0x0306u16)] {
            let base = eh55_bgo_sector_offset(sa).unwrap();
            for i in 0..4 {
                nor[base + 4 + i * 4..base + 6 + i * 4].copy_from_slice(&0xF101u16.to_le_bytes());
            }
            nor[base + 0x100..base + 0x200].fill(0x00);
            let sp = base + 2 * 0x100;
            nor[sp..sp + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            let soff = sp + 4;
            nor[soff..soff + 2].copy_from_slice(&tid.to_le_bytes());
            nor[soff + 2..soff + 4].copy_from_slice(&0xFFFFu16.to_le_bytes());
            nor[soff + 4..soff + 6].copy_from_slice(&slot_val.to_le_bytes());
        }
        let l2p = parse_eh55_bgo_l2p(&nor).unwrap();
        assert_eq!(l2p.get(&0x0000), Some(&0x0204));
        assert_eq!(l2p.get(&0x0010), Some(&0x0306));
    }
}
