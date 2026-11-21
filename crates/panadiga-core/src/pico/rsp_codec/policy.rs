//! Layered Spansion NOR Flash write-protection policies (`NvramOnly` vs `UpperNorAppUnlocked`).

use serde::{Deserialize, Serialize};

use crate::constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE,
    FLASH_16MB_NVRAM_SIZE_BYTES, SPANSION_NVRAM_SIZE_BYTES,
};

/// Strictly read-only IPL NOR windows (`0x80000000..0x807EFFFF` cached & `0x84000000..0x847EFFFF` uncached).
pub const IPL_READONLY_RANGES: [(u64, u64); 2] =
    [(0x8000_0000, 0x807F_0000), (0x8400_0000, 0x847F_0000)];

/// Permanently read-only Critical Boot IPL windows (`SA0..SA10` = 256 KiB: `0x80000000..0x8003FFFF` & `0x84000000..0x8403FFFF`).
pub const CRITICAL_IPL_BOOT_RANGES: [(u64, u64); 2] = [
    (
        AM34_FLASH_BASE as u64,
        AM34_FLASH_BASE as u64 + AM34_FLASH_CRITICAL_IPL_SIZE as u64,
    ),
    (
        AM34_FLASH_UNCACHED_BASE as u64,
        AM34_FLASH_UNCACHED_BASE as u64 + AM34_FLASH_CRITICAL_IPL_SIZE as u64,
    ),
];

/// Layered GDB RSP NOR flash write protection policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NorWritePolicy {
    /// Default `U99`-safe mode: blocks all IPL/App NOR (`0x80000000..nvram_start-1`), permitting NVRAM & RAM/MMIO only.
    #[default]
    NvramOnly,
    /// Advanced engineering mode: unlocks Upper NOR App/BGO (`0x80040000..nvram_start-1`), permanently blocking `SA0..SA10`.
    UpperNorAppUnlocked,
}

fn address_overlaps_ranges(addr: u32, length: usize, ranges: &[(u64, u64)]) -> bool {
    let start = addr as u64;
    let span = length.max(1) as u64;
    let end = start.saturating_add(span).min(0x1_0000_0000);
    ranges.iter().any(|&(lo, hi)| start < hi && end > lo)
}

fn normalize_flash_bytes(flash_bytes: u64) -> u64 {
    match flash_bytes {
        0x0020_0000 | 0x0040_0000 | 0x0080_0000 | 0x0100_0000 => flash_bytes,
        _ => AM34_FLASH_SIZE as u64,
    }
}

fn nor_nvram_offset_for_flash(flash_bytes: u64) -> u64 {
    let total = normalize_flash_bytes(flash_bytes);
    let nvram_size = if total >= 16 * 1024 * 1024 {
        FLASH_16MB_NVRAM_SIZE_BYTES as u64
    } else {
        SPANSION_NVRAM_SIZE_BYTES as u64
    };
    total.saturating_sub(nvram_size)
}

/// Returns `true` when `[addr, addr + length)` touches cached or uncached NOR flash for a target `flash_bytes` capacity.
pub fn is_flash_address_in_range(addr: u32, length: usize, flash_bytes: u64) -> bool {
    let span = normalize_flash_bytes(flash_bytes);
    let cached_lo = AM34_FLASH_BASE as u64;
    let uncached_lo = AM34_FLASH_UNCACHED_BASE as u64;
    let ranges = [
        (cached_lo, cached_lo + span),
        (uncached_lo, uncached_lo + span),
    ];
    address_overlaps_ranges(addr, length, &ranges)
}

/// Returns `true` when `[addr, addr + length)` touches NVRAM micro-sectors for a target `flash_bytes` capacity.
pub fn is_nvram_address_for_flash(addr: u32, length: usize, flash_bytes: u64) -> bool {
    let total = normalize_flash_bytes(flash_bytes);
    let nvram_off = nor_nvram_offset_for_flash(total);
    let cached_lo = AM34_FLASH_BASE as u64;
    let uncached_lo = AM34_FLASH_UNCACHED_BASE as u64;
    let ranges = [
        (cached_lo + nvram_off, cached_lo + total),
        (uncached_lo + nvram_off, uncached_lo + total),
    ];
    address_overlaps_ranges(addr, length, &ranges)
}

/// Returns `true` when `[addr, addr + length)` touches read-only Spansion IPL/Upper NOR for `flash_bytes` under `NvramOnly`.
pub fn is_ipl_readonly_address_for_flash(addr: u32, length: usize, flash_bytes: u64) -> bool {
    let nvram_off = nor_nvram_offset_for_flash(flash_bytes);
    let cached_lo = AM34_FLASH_BASE as u64;
    let uncached_lo = AM34_FLASH_UNCACHED_BASE as u64;
    let ranges = [
        (cached_lo, cached_lo + nvram_off),
        (uncached_lo, uncached_lo + nvram_off),
    ];
    address_overlaps_ranges(addr, length, &ranges)
}

/// Permanently blocked AM34 lethal MMIO register (`0xC000100C..0xC000100F`, WDT/bus hang on read or write).
pub const AM34_LETHAL_MMIO_ADDR: u32 = 0xC000_100C;
pub const AM34_LETHAL_MMIO_LEN: usize = 4;

/// Full AM34 CS0 NOR bus aperture (`0x80000000..0x88000000`, cached + uncached + address-alias mirrors).
pub const AM34_CS0_APERTURE_START: u64 = AM34_FLASH_BASE as u64;
pub const AM34_CS0_APERTURE_END: u64 = AM34_FLASH_UNCACHED_BASE as u64 + 0x0400_0000;

/// Returns `true` when `[addr, addr + length)` touches the lethal AM34 MMIO register `0xC000100C..0xC000100F`.
pub fn is_am34_lethal_mmio_address(addr: u32, length: usize) -> bool {
    let lo = AM34_LETHAL_MMIO_ADDR as u64;
    let hi = lo + AM34_LETHAL_MMIO_LEN as u64;
    address_overlaps_ranges(addr, length, &[(lo, hi)])
}

fn is_cs0_alias_mirror_for_flash(addr: u32, length: usize, flash_bytes: u64) -> bool {
    let start = addr as u64;
    let span = length.max(1) as u64;
    let end = start.saturating_add(span).min(0x1_0000_0000);
    if !(start < AM34_CS0_APERTURE_END && end > AM34_CS0_APERTURE_START) {
        return false;
    }
    let total = normalize_flash_bytes(flash_bytes);
    let cached_lo = AM34_FLASH_BASE as u64;
    let uncached_lo = AM34_FLASH_UNCACHED_BASE as u64;
    let in_cached_primary = start >= cached_lo && end <= cached_lo + total;
    let in_uncached_primary = start >= uncached_lo && end <= uncached_lo + total;
    !(in_cached_primary || in_uncached_primary)
}

/// Evaluates whether a GDB RSP memory write is blocked (`E05`) for arbitrary DIGA flash capacities (`2`/`4`/`8`/`16` MiB).
pub fn is_nor_write_blocked_for_flash(
    addr: u32,
    length: usize,
    flash_bytes: u64,
    policy: NorWritePolicy,
) -> bool {
    if is_am34_lethal_mmio_address(addr, length) {
        return true;
    }
    if is_cs0_alias_mirror_for_flash(addr, length, flash_bytes) {
        return true;
    }
    match policy {
        NorWritePolicy::NvramOnly => is_ipl_readonly_address_for_flash(addr, length, flash_bytes),
        NorWritePolicy::UpperNorAppUnlocked => is_critical_ipl_boot_address(addr, length),
    }
}

/// Returns `true` when `[addr, addr + length)` touches read-only Spansion IPL NOR flash (`NvramOnly` policy).
pub fn is_ipl_readonly_address(addr: u32, length: usize) -> bool {
    address_overlaps_ranges(addr, length, &IPL_READONLY_RANGES)
}

/// Returns `true` when `[addr, addr + length)` touches Critical Boot IPL (`SA0..SA10`: first 256 KiB).
pub fn is_critical_ipl_boot_address(addr: u32, length: usize) -> bool {
    address_overlaps_ranges(addr, length, &CRITICAL_IPL_BOOT_RANGES)
}

/// Evaluates whether a GDB RSP memory write is blocked (`E05`) under the given [`NorWritePolicy`].
pub fn is_nor_write_blocked_with_policy(addr: u32, length: usize, policy: NorWritePolicy) -> bool {
    is_nor_write_blocked_for_flash(addr, length, AM34_FLASH_SIZE as u64, policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_lethal_mmio_and_cs0_alias_mirrors_across_capacities() {
        assert!(is_am34_lethal_mmio_address(0xC000_100C, 4));
        assert!(is_am34_lethal_mmio_address(0xC000_100A, 4));
        assert!(is_nor_write_blocked_with_policy(
            0xC000_100C,
            4,
            NorWritePolicy::UpperNorAppUnlocked
        ));
        // CS0 alias mirrors on 8 MiB flash must be blocked even when Upper NOR is unlocked
        assert!(is_nor_write_blocked_with_policy(
            0x8080_0000,
            4,
            NorWritePolicy::UpperNorAppUnlocked
        ));
        assert!(is_nor_write_blocked_with_policy(
            0x8480_0000,
            4,
            NorWritePolicy::UpperNorAppUnlocked
        ));
        // Straddle past top of 8 MiB NVRAM must be blocked
        assert!(is_nor_write_blocked_with_policy(
            0x807F_FFFC,
            8,
            NorWritePolicy::NvramOnly
        ));
        // 4 MiB flash CS0 mirror at 0x80400000 must be blocked
        assert!(is_nor_write_blocked_for_flash(
            0x8040_0000,
            4,
            4 * 1024 * 1024,
            NorWritePolicy::UpperNorAppUnlocked
        ));
    }

    #[test]
    fn nor_policy_guards_32bit_wrap_and_16mb_upper_nor() {
        assert!(is_ipl_readonly_address(0x8400_0000, 1));
        assert!(is_critical_ipl_boot_address(0x8400_0000, 1));
        assert!(!address_overlaps_ranges(
            0xFFFF_FFFF,
            16,
            &[(0x8000_0000, 0x8004_0000)]
        ));
        assert!(is_nor_write_blocked_for_flash(
            0x8080_0000,
            4,
            16 * 1024 * 1024,
            NorWritePolicy::NvramOnly
        ));
        assert!(!is_nor_write_blocked_for_flash(
            0x80FE_0000,
            4,
            16 * 1024 * 1024,
            NorWritePolicy::NvramOnly
        ));
        // F2: 2 MiB (0x801F0000) & 4 MiB (0x803F0000) NVRAM must NOT be clamped up to 8 MiB IPL.
        assert!(is_nvram_address_for_flash(0x801F_0000, 4, 2 * 1024 * 1024));
        assert!(!is_flash_address_in_range(0x8040_0000, 4, 2 * 1024 * 1024));
        assert!(!is_nor_write_blocked_for_flash(
            0x801F_0000,
            4,
            2 * 1024 * 1024,
            NorWritePolicy::NvramOnly
        ));
        assert!(is_nor_write_blocked_for_flash(
            0x801E_FFFF,
            1,
            2 * 1024 * 1024,
            NorWritePolicy::NvramOnly
        ));
        assert!(is_nvram_address_for_flash(0x843F_0000, 4, 4 * 1024 * 1024));
        assert!(!is_nor_write_blocked_for_flash(
            0x843F_0000,
            4,
            4 * 1024 * 1024,
            NorWritePolicy::NvramOnly
        ));
        assert!(is_nor_write_blocked_for_flash(
            0x803E_FFFF,
            1,
            4 * 1024 * 1024,
            NorWritePolicy::NvramOnly
        ));
    }
}
