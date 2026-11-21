#![deny(unsafe_code)]

//! Pure datasheet-backed sector index and geometry label derivation (eliminates manual SA drift).

use anyhow::{Result, bail};

use crate::types::FlashChipProfile;

/// Physical NOR sector computed purely from [`FlashChipProfile::sector_banks`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedSector {
    pub index: u32,
    pub start_offset: u32,
    pub end_offset_inclusive: u32,
    pub size_bytes: u32,
    pub bank_name: String,
}

/// Expands all physical sectors `SA0..SA(n-1)` from bank descriptors in ascending address order.
pub fn derive_sector_table(chip: &FlashChipProfile) -> Result<Vec<DerivedSector>> {
    if !chip.writable {
        return Ok(Vec::new());
    }
    if chip.sector_banks.is_empty() {
        bail!(
            "Writable flash '{}' must declare at least one sector bank",
            chip.id
        );
    }
    let mut table = Vec::new();
    let mut cursor: u64 = 0;
    let mut sa_idx: u32 = 0;
    for bank in &chip.sector_banks {
        if bank.count == 0 || bank.sector_size == 0 {
            bail!(
                "Flash '{}' bank '{}' has zero count or sector_size",
                chip.id,
                bank.name
            );
        }
        for _ in 0..bank.count {
            let sz = u64::from(bank.sector_size);
            let next_cursor = cursor
                .checked_add(sz)
                .ok_or_else(|| anyhow::anyhow!("Flash '{}' bank size overflow", chip.id))?;
            let start = u32::try_from(cursor)
                .map_err(|_| anyhow::anyhow!("Flash '{}' offset >4GiB", chip.id))?;
            let end_inc = u32::try_from(next_cursor - 1)
                .map_err(|_| anyhow::anyhow!("Flash '{}' end offset >4GiB", chip.id))?;
            table.push(DerivedSector {
                index: sa_idx,
                start_offset: start,
                end_offset_inclusive: end_inc,
                size_bytes: bank.sector_size,
                bank_name: bank.name.clone(),
            });
            cursor = next_cursor;
            sa_idx = sa_idx
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Flash '{}' sector count overflow", chip.id))?;
        }
    }
    Ok(table)
}

/// Maps a physical flash byte offset inside `0..total_bytes` to its enclosing [`DerivedSector`].
pub fn sector_of_offset(chip: &FlashChipProfile, offset: u32) -> Result<DerivedSector> {
    let table = derive_sector_table(chip)?;
    table
        .into_iter()
        .find(|s| offset >= s.start_offset && offset <= s.end_offset_inclusive)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Offset 0x{offset:08X} outside writable sector table of flash '{}'",
                chip.id
            )
        })
}

impl FlashChipProfile {
    /// Returns the datasheet-derived physical sector (`SA` index, start/end offset, size) at `offset`.
    #[must_use]
    pub fn sector_at_offset(&self, offset: u32) -> Option<DerivedSector> {
        sector_of_offset(self, offset).ok()
    }
}

/// Validates exact sector-boundary alignment of an inclusive span `[start_offset..=end_offset]`
/// and returns `(first_sa, last_sa)`.
pub fn sector_range_for_aligned_span(
    chip: &FlashChipProfile,
    start_offset: u32,
    end_offset_inclusive: u32,
) -> Result<(u32, u32)> {
    if start_offset > end_offset_inclusive {
        bail!(
            "Invalid flash span [0x{start_offset:08X}..=0x{end_offset_inclusive:08X}] on '{}'",
            chip.id
        );
    }
    let table = derive_sector_table(chip)?;
    let first = table
        .iter()
        .find(|s| s.start_offset == start_offset)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Region start 0x{start_offset:08X} is not aligned to any sector start in '{}'",
                chip.id
            )
        })?;
    let last = table
        .iter()
        .find(|s| s.end_offset_inclusive == end_offset_inclusive)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Region end 0x{end_offset_inclusive:08X} is not aligned to any sector end in '{}'",
                chip.id
            )
        })?;
    Ok((first.index, last.index))
}

/// Formats an inclusive SA range concisely (`"SA127"` or `"SA134..SA141"`).
#[must_use]
pub fn format_sa_range(first_sa: u32, last_sa: u32) -> String {
    if first_sa == last_sa {
        format!("SA{first_sa}")
    } else {
        format!("SA{first_sa}..SA{last_sa}")
    }
}

/// Derives canonical human-readable geometry summary directly from `sector_banks`.
#[must_use]
pub fn derived_geometry_label(chip: &FlashChipProfile) -> String {
    if !chip.writable {
        return format!("OTP PROM {} KiB (read-only)", chip.total_bytes / 1024);
    }
    let total_sectors: u32 = chip.sector_banks.iter().map(|b| b.count).sum();
    let banks_desc: Vec<String> = chip
        .sector_banks
        .iter()
        .map(|b| format!("{}x{}KB", b.count, b.sector_size / 1024))
        .collect();
    if total_sectors == 0 {
        return "No writable sectors".to_string();
    }
    format!(
        "{} ({}, SA0..SA{})",
        banks_desc.join(" + "),
        chip.part_number,
        total_sectors - 1
    )
}
