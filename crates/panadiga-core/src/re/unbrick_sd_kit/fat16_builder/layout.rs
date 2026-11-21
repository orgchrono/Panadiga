#![deny(unsafe_code)]

//! Exact FAT16B (`0x06`, `LBA 63`, `RsvdSecCnt=1`) partition and BPB geometry calculations.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub const FAT16_SECTOR_BYTES: usize = 512;
pub const FAT16_PARTITION_TYPE_LBA: u8 = 0x06;
pub const FAT16_VBR_LBA_START: u32 = 63;
pub const FAT16_RESERVED_SECTORS: u16 = 1;
pub const FAT16_NUM_FATS: u8 = 2;
pub const FAT16_ROOT_ENTRY_COUNT: u16 = 512;
pub const FAT16_ROOT_SECTORS: u32 = 32;
pub const FAT16_MIN_VOLUME_MB: u32 = 32;
pub const FAT16_DEFAULT_VOLUME_MB: u32 = 32;
pub const FAT16_MAX_VOLUME_MB: u32 = 256;
pub const FAT16_VOL_ID_DETERMINISTIC: u32 = 0x2006_0306;
pub const FAT16_DOS_DATE_2006_03_06: u16 = 0x3466;
pub const FAT16_DOS_TIME_MIDNIGHT: u16 = 0x0000;

/// Validated geometry parameters for a deterministic FAT16B (`0x06`) SD card image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fat16SdGeometry {
    pub volume_mb: u32,
    pub lba_start: u32,
    pub volume_sectors: u32,
    pub total_image_sectors: u32,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entry_count: u16,
    pub sectors_per_fat: u16,
    pub data_clusters: u32,
}

impl Fat16SdGeometry {
    #[must_use]
    pub const fn fat1_lba(self) -> u32 {
        self.lba_start + self.reserved_sectors as u32
    }

    #[must_use]
    pub const fn fat2_lba(self) -> u32 {
        self.fat1_lba() + self.sectors_per_fat as u32
    }

    #[must_use]
    pub const fn root_dir_lba(self) -> u32 {
        self.fat2_lba() + self.sectors_per_fat as u32
    }

    #[must_use]
    pub const fn data_start_lba(self) -> u32 {
        self.root_dir_lba() + FAT16_ROOT_SECTORS
    }

    #[must_use]
    pub const fn total_image_bytes(self) -> usize {
        self.total_image_sectors as usize * FAT16_SECTOR_BYTES
    }

    #[must_use]
    pub const fn cluster_size_bytes(self) -> usize {
        self.sectors_per_cluster as usize * FAT16_SECTOR_BYTES
    }
}

/// Computes deterministic FAT16B geometry (`32..=256 MiB`, default `32 MiB` -> `65536` sectors, `FATSz16=32`).
pub fn compute_fat16_sd_geometry(volume_mb: u32) -> Result<Fat16SdGeometry> {
    if !(FAT16_MIN_VOLUME_MB..=FAT16_MAX_VOLUME_MB).contains(&volume_mb) {
        bail!(
            "FAT16B SD volume_mb={volume_mb} out of allowed range {FAT16_MIN_VOLUME_MB}..={FAT16_MAX_VOLUME_MB} MiB"
        );
    }
    let volume_sectors = volume_mb * 2048;
    let spc: u8 = if volume_mb <= 128 { 8 } else { 16 };
    let fixed_overhead = u32::from(FAT16_RESERVED_SECTORS) + FAT16_ROOT_SECTORS;

    let mut fat_sz = 1u32;
    loop {
        let data_secs = volume_sectors - fixed_overhead - u32::from(FAT16_NUM_FATS) * fat_sz;
        let clusters = data_secs / u32::from(spc);
        let needed_fat = ((clusters + 2) * 2).div_ceil(FAT16_SECTOR_BYTES as u32);
        if needed_fat <= fat_sz {
            if !(4085..=65_524).contains(&clusters) {
                bail!(
                    "Computed cluster count {clusters} is not valid FAT16 (must be 4085..=65524)"
                );
            }
            return Ok(Fat16SdGeometry {
                volume_mb,
                lba_start: FAT16_VBR_LBA_START,
                volume_sectors,
                total_image_sectors: FAT16_VBR_LBA_START + volume_sectors,
                sectors_per_cluster: spc,
                reserved_sectors: FAT16_RESERVED_SECTORS,
                num_fats: FAT16_NUM_FATS,
                root_entry_count: FAT16_ROOT_ENTRY_COUNT,
                sectors_per_fat: fat_sz as u16,
                data_clusters: clusters,
            });
        }
        fat_sz = needed_fat;
    }
}

fn is_valid_8_3_byte(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-')
}

/// Strictly validates and encodes a pure uppercase 8.3 FAT16 filename without LFN or OS junk.
pub fn encode_dos_8_3_name(name: &str) -> Result<[u8; 11]> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed != trimmed.to_ascii_uppercase() {
        bail!("FAT16 root filename '{name}' must be pure non-empty uppercase ASCII 8.3");
    }
    let (stem, ext) = match trimmed.split_once('.') {
        Some((s, e)) => (s, e),
        None => (trimmed, ""),
    };
    if ext.contains('.') || stem.is_empty() || stem.len() > 8 || ext.len() > 3 {
        bail!("FAT16 filename '{name}' violates strict 8.3 length limits");
    }
    if !stem.bytes().all(is_valid_8_3_byte) || !ext.bytes().all(is_valid_8_3_byte) {
        bail!("FAT16 filename '{name}' contains unsupported characters (no spaces, LFN, or dots)");
    }
    let mut raw = [b' '; 11];
    raw[..stem.len()].copy_from_slice(stem.as_bytes());
    raw[8..8 + ext.len()].copy_from_slice(ext.as_bytes());
    Ok(raw)
}

/// Converts a 32-bit LBA to legacy MBR CHS tuple `(head, sector_cyl_lo, cyl_hi)` for `255` heads / `63` spt.
#[must_use]
pub fn lba_to_mbr_chs(lba: u32) -> [u8; 3] {
    let spt = 63u32;
    let heads = 255u32;
    let cyl = (lba / (heads * spt)).min(1023);
    let rem = lba % (heads * spt);
    let head = rem / spt;
    let sec = (rem % spt) + 1;
    let sec_byte = ((sec as u8) & 0x3F) | (((cyl >> 8) as u8 & 0x03) << 6);
    [head as u8, sec_byte, (cyl & 0xFF) as u8]
}
