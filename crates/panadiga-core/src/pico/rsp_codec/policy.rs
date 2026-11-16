//! Layered Spansion NOR Flash write-protection policies (`NvramOnly` vs `UpperNorAppUnlocked`).

use serde::{Deserialize, Serialize};

use crate::constants::{AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_UNCACHED_BASE};

/// Strictly read-only IPL NOR windows (`0x80000000..0x807EFFFF` cached & `0x84000000..0x847EFFFF` uncached).
pub const IPL_READONLY_RANGES: [(u64, u64); 2] =
    [(0x8000_0000, 0x807F_0000), (0x8400_0000, 0x847F_0000)];

/// Permanently read-only Critical Boot IPL windows (`SA0..SA3`: `0x80000000..0x8003FFFF` & `0x84000000..0x8403FFFF`).
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
    /// Default `U99`-safe mode: blocks all IPL/App NOR (`0x80000000..0x807EFFFF`), permitting NVRAM & RAM/MMIO only.
    #[default]
    NvramOnly,
    /// Advanced engineering mode: unlocks Upper NOR App/BGO (`0x80040000..nvram_start-1`), permanently blocking `SA0..SA3`.
    UpperNorAppUnlocked,
}

fn address_overlaps_ranges(addr: u32, length: usize, ranges: &[(u64, u64)]) -> bool {
    let start = addr as u64;
    let span = length.max(1) as u64;
    let raw_end = (start + span) & 0xFFFF_FFFF;
    let end = if raw_end < start {
        0x1_0000_0000
    } else {
        raw_end
    };
    ranges.iter().any(|&(lo, hi)| start < hi && end > lo)
}

/// Returns `true` when `[addr, addr + length)` touches read-only Spansion IPL NOR flash (`NvramOnly` policy).
pub fn is_ipl_readonly_address(addr: u32, length: usize) -> bool {
    address_overlaps_ranges(addr, length, &IPL_READONLY_RANGES)
}

/// Returns `true` when `[addr, addr + length)` touches Critical Boot IPL (`SA0..SA3`: first 256 KiB).
pub fn is_critical_ipl_boot_address(addr: u32, length: usize) -> bool {
    address_overlaps_ranges(addr, length, &CRITICAL_IPL_BOOT_RANGES)
}

/// Evaluates whether a GDB RSP memory write is blocked (`E05`) under the given [`NorWritePolicy`].
pub fn is_nor_write_blocked_with_policy(addr: u32, length: usize, policy: NorWritePolicy) -> bool {
    match policy {
        NorWritePolicy::NvramOnly => is_ipl_readonly_address(addr, length),
        NorWritePolicy::UpperNorAppUnlocked => is_critical_ipl_boot_address(addr, length),
    }
}
