#![deny(unsafe_code)]

//! Low-level L2P, sector offset, migration bitmap, and 1->0 compatibility helpers for BGO FTL.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{AM34_EH55_FLASH_BGO_BASE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};
use crate::crypto::codec::extract_c_ascii;

use super::{
    EH55_BGO_EXPECTED_P2_COUNT, EH55_BGO_FIRST_SA, EH55_BGO_LAST_SA, EH55_BGO_MAX_PID,
    EH55_BGO_SECTOR_STRIDE,
};

/// Typed errors returned by BGO FTL L2P geometry, sector indexing, and 1->0 verification helpers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BgoParseError {
    #[error("Expected {expected}-byte NOR dump, got {actual}")]
    InvalidImageLength { expected: usize, actual: usize },
    #[error("BGO sector SA{sa} outside valid range SA{first_sa}..=SA{last_sa}")]
    SectorOutOfRange {
        sa: usize,
        first_sa: usize,
        last_sa: usize,
    },
    #[error("Duplicate active BGO L2P mapping for pid 0x{pid:04X}")]
    DuplicateActiveL2pMapping { pid: u16 },
    #[error("NOR compatibility buffer length mismatch: orig={orig} vs target={target}")]
    BufferLengthMismatch { orig: usize, target: usize },
    #[error("0->1 NOR bit violation at offset 0x{offset:06X}: old=0x{old:02X}, new=0x{new:02X}")]
    NorZeroToOneViolation { offset: usize, old: u8, new: u8 },
}

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
            return Err(BgoParseError::SectorOutOfRange {
                sa,
                first_sa: self.first_sa,
                last_sa: self.last_sa,
            }
            .into());
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
    if base.checked_add(0x200).is_none_or(|end| end > nor.len()) {
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
    if off.checked_add(38).is_none_or(|end| end > buf.len()) {
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

/// Decodes the 4-bit status nibble of a BGO Partition-1 (`0xF101`) L2P slot (`8 == valid`).
pub(crate) fn p1_slot_state(b: &[u8], s: usize, t2: u16) -> u8 {
    if t2 == 0 {
        return 0;
    }
    (((b[3] >> s) & 1) << 3)
        | (((b[2] >> s) & 1) << 2)
        | (((b[1] >> s) & 1) << 1)
        | ((b[0] >> s) & 1)
}

/// Parses active L2P page-to-slot mappings across all `0xF101` BGO Partition-1 sectors in `geom`.
pub fn parse_bgo_l2p_with_geometry(
    nor: &[u8],
    geom: &BgoSectorGeometry,
) -> Result<BTreeMap<u16, u16>> {
    if nor.len() != geom.expected_flash_bytes {
        return Err(BgoParseError::InvalidImageLength {
            expected: geom.expected_flash_bytes,
            actual: nor.len(),
        }
        .into());
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
                if p1_slot_state(b, s, t2) == 8 {
                    for i in 0..16u16 {
                        let w = read_le_u16(nor, soff + 4 + usize::from(i) * 2);
                        let pid = tid.wrapping_mul(16).wrapping_add(i);
                        if l2p.insert(pid, w).is_some() {
                            return Err(BgoParseError::DuplicateActiveL2pMapping { pid }.into());
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
        return Err(BgoParseError::BufferLengthMismatch {
            orig: orig.len(),
            target: target.len(),
        }
        .into());
    }
    let mut modified = 0usize;
    for (idx, (&o, &n)) in orig.iter().zip(target.iter()).enumerate() {
        if (o & n) != n {
            return Err(BgoParseError::NorZeroToOneViolation {
                offset: idx,
                old: o,
                new: n,
            }
            .into());
        }
        if o != n {
            modified += 1;
        }
    }
    Ok(modified)
}

/// Reads the live 512-byte BGO payload page for logical `pid` if both L2P and F2 tag match.
pub(crate) fn read_bgo_page<'a>(
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

/// Decoded 16-byte NFD directory entry inside `PID 0x0F02..=0x0F04`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NfdDirEntry {
    pub dir_pid: u16,
    pub slot_idx: usize,
    pub name: String,
    pub byte_size: u32,
    pub start_pid: u16,
}

/// Collects valid `0xF2xx` Partition-2 sector bases (`fid -> flash_offset`) in `geom`.
pub(crate) fn collect_f2_bases(
    nor: &[u8],
    geom: &BgoSectorGeometry,
) -> Result<BTreeMap<u16, usize>> {
    if nor.len() != geom.expected_flash_bytes {
        return Err(BgoParseError::InvalidImageLength {
            expected: geom.expected_flash_bytes,
            actual: nor.len(),
        }
        .into());
    }
    let mut f2_bases = BTreeMap::new();
    let max_f2_word = 0xF200u16.saturating_add(geom.expected_p2_count as u16);
    for sa in geom.first_sa..=geom.last_sa {
        let base = geom.sector_offset(sa)?;
        let w0 = read_le_u16(nor, base + 4);
        let consistent = (0..4).all(|i| read_le_u16(nor, base + 4 + i * 4) == w0);
        let zero_marker_ok = nor[base + 0x100..base + 0x200].iter().all(|&b| b == 0);
        let mig_target = eval_bgo_migration_target(nor, base + 0x14);
        if consistent
            && zero_marker_ok
            && (0xF201..=max_f2_word).contains(&w0)
            && mig_target == 0xFFFF
        {
            let fid = w0 & 0xFF;
            if f2_bases.insert(fid, base).is_some() {
                bail!("Duplicate active Partition-2 sector ID 0xF2{fid:02X} at SA{sa}");
            }
        }
    }
    Ok(f2_bases)
}

/// Concatenates the 15 Primary NFD FAT pages (`0x0F07..=0x0F15`), failing closed if any is absent.
pub(crate) fn read_primary_fat(
    nor: &[u8],
    l2p: &BTreeMap<u16, u16>,
    f2_bases: &BTreeMap<u16, usize>,
) -> Result<Vec<u8>> {
    let mut fat = Vec::with_capacity(15 * 0x200);
    for idx in 0..15u16 {
        let fat_pid = 0x0F07 + idx;
        let pg = read_bgo_page(nor, l2p, f2_bases, fat_pid)
            .ok_or_else(|| anyhow::anyhow!("Missing BGO Primary FAT page 0x{fat_pid:04X}"))?;
        fat.extend_from_slice(pg);
    }
    Ok(fat)
}

/// Scans `PID 0x0F02`, `0x0F03`, and `0x0F04` (`16` entries x `16` bytes) for `target_name`.
#[must_use]
pub fn find_nfd_dir_entry(
    nor: &[u8],
    l2p: &BTreeMap<u16, u16>,
    f2_bases: &BTreeMap<u16, usize>,
    target_name: &str,
) -> Option<NfdDirEntry> {
    for dir_pid in [0x0F02u16, 0x0F03, 0x0F04] {
        let Some(dp) = read_bgo_page(nor, l2p, f2_bases, dir_pid) else {
            continue;
        };
        for slot_idx in 0..16usize {
            let ent = &dp[slot_idx * 16..(slot_idx + 1) * 16];
            if ent[0] == 0 || ent[0] == 0xFF {
                continue;
            }
            let name = extract_c_ascii(&ent[..8]);
            if name == target_name {
                return Some(NfdDirEntry {
                    dir_pid,
                    slot_idx,
                    name,
                    byte_size: u32::from_le_bytes([ent[8], ent[9], ent[10], ent[11]]),
                    start_pid: read_le_u16(ent, 12),
                });
            }
        }
    }
    None
}

/// Resolves the logical BGO page ID for `filename` at `page_index` by walking the Primary FAT.
pub fn resolve_nfd_file_page_pid(
    nor: &[u8],
    geom: &BgoSectorGeometry,
    filename: &str,
    page_index: usize,
) -> Result<u16> {
    let l2p = parse_bgo_l2p_with_geometry(nor, geom)?;
    let f2_bases = collect_f2_bases(nor, geom)?;
    let entry = find_nfd_dir_entry(nor, &l2p, &f2_bases, filename)
        .ok_or_else(|| anyhow::anyhow!("BGO NFD file '{filename}' not found in directory"))?;
    let total_pages = (entry.byte_size as usize).div_ceil(0x200);
    if page_index >= total_pages {
        bail!(
            "BGO NFD file '{filename}' page index {page_index} out of bounds (pages={total_pages}, size={})",
            entry.byte_size
        );
    }
    let fat = read_primary_fat(nor, &l2p, &f2_bases)?;
    let mut cur = entry.start_pid;
    for step in 0..page_index {
        let off = usize::from(cur.saturating_sub(1)) * 2;
        if cur == 0 || cur == 0xFFFF || cur == 0xFFED || off + 2 > fat.len() {
            bail!("Broken BGO FAT chain for '{filename}' at hop {step}: cur=0x{cur:04X}");
        }
        cur = read_le_u16(&fat, off);
    }
    if cur == 0 || cur == 0xFFFF || cur == 0xFFED {
        bail!("Invalid resolved PID 0x{cur:04X} for '{filename}' page {page_index}");
    }
    Ok(cur)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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

    #[test]
    fn resolve_nfd_file_page_pid_walks_repaired_nor_fat_chains() {
        let nor_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
        if !nor_path.is_file() {
            return;
        }
        let nor = std::fs::read(&nor_path).unwrap();
        let geom = BgoSectorGeometry::eh55_default();
        assert_eq!(
            resolve_nfd_file_page_pid(&nor, &geom, "config1", 0).unwrap(),
            0x0018
        );
        assert_eq!(
            resolve_nfd_file_page_pid(&nor, &geom, "config1", 9).unwrap(),
            0x000C
        );
        assert_eq!(
            resolve_nfd_file_page_pid(&nor, &geom, "update", 0).unwrap(),
            0x000D
        );
    }

    #[test]
    fn bgo_parse_error_and_overflow_safe_bounds_fail_closed() {
        assert!(!is_active_bgo_p1_sector(&[0u8; 64], usize::MAX - 8));
        assert_eq!(eval_bgo_migration_index(&[0u8; 64], usize::MAX - 4), None);

        let err_sa = eh55_bgo_sector_offset(999).unwrap_err();
        assert!(matches!(
            err_sa.downcast_ref::<BgoParseError>(),
            Some(BgoParseError::SectorOutOfRange { sa: 999, .. })
        ));

        let err_nor = verify_nor_one_to_zero_compatible(&[0x0F], &[0xF0]).unwrap_err();
        assert_eq!(
            err_nor.downcast_ref::<BgoParseError>(),
            Some(&BgoParseError::NorZeroToOneViolation {
                offset: 0,
                old: 0x0F,
                new: 0xF0
            })
        );
    }
}
