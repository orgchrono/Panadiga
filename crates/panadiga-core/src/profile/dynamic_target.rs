#![deny(unsafe_code)]

//! Pure inference logic for `DynamicTargetProfile`.

use super::{BgoBounds, DynamicTargetProfile, HddExpectation, NorFlashGeometry, NvramGeometry};
use crate::constants::{
    FLASH_16MB_MICROSECTOR_COUNT, FLASH_16MB_NVRAM_BASE_OFFSET, FLASH_16MB_NVRAM_SIZE_BYTES,
    LBA_MIRROR_MBR_OFFSET, PANASONIC_HDD_MBR_SIGNATURE, SPANSION_FLASH_SIZE_BYTES,
    SPANSION_MICROSECTOR_SIZE, SPANSION_NVRAM_BASE_OFFSET, SPANSION_NVRAM_SIZE_BYTES,
};
use crate::hdd::capacity_patch::find_hdd_cmd_clamp_site;
use crate::ingest::{ContainerFormat, fingerprint_binary};
use crate::ssot::catalog::{get_diga_model_catalog_dto, is_valid_nor_flash_size};

const BULK_SECTOR_BYTES: usize = 0x1_0000;
const BGO_ZERO_START: usize = 0x100;
const BGO_ZERO_END: usize = 0x200;

fn read_le_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

fn bulk_offset_to_sa(off: usize, flash_size: u64) -> usize {
    if flash_size == SPANSION_FLASH_SIZE_BYTES as u64 && off >= BULK_SECTOR_BYTES {
        (off / BULK_SECTOR_BYTES) + 7
    } else {
        off / BULK_SECTOR_BYTES
    }
}

fn infer_flash_size(bytes: &[u8], model: &str) -> (u64, Option<u64>) {
    let raw_len = bytes.len() as u64;
    let catalog = get_diga_model_catalog_dto();
    let cat_match = if model.trim().is_empty() || model.contains("Unknown") {
        None
    } else {
        catalog
            .iter()
            .find(|entry| model.contains(&entry.model_id) || entry.model_id.contains(model.trim()))
    };
    let cat_nvram_base = cat_match.and_then(|m| {
        let hex = m
            .nvram_base_hex
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        u64::from_str_radix(hex, 16).ok().map(|va| va & 0x00FF_FFFF)
    });
    if is_valid_nor_flash_size(raw_len) {
        return (raw_len, cat_nvram_base);
    }
    if let Some(m) = cat_match {
        return (m.flash_size_bytes, cat_nvram_base);
    }
    let upper = model.to_ascii_uppercase();
    if upper.contains("XW") || upper.contains("BW") || upper.contains("MN2WS") {
        (16 * 1024 * 1024, Some(FLASH_16MB_NVRAM_BASE_OFFSET as u64))
    } else {
        (
            SPANSION_FLASH_SIZE_BYTES as u64,
            Some(SPANSION_NVRAM_BASE_OFFSET as u64),
        )
    }
}

fn scan_bgo_bounds(bytes: &[u8], flash_size: u64) -> BgoBounds {
    if !is_valid_nor_flash_size(bytes.len() as u64) {
        return BgoBounds {
            detected: false,
            start_offset: 0,
            end_offset: 0,
            first_sa: 0,
            last_sa: 0,
            active_p1_sectors: Vec::new(),
            data_sector_count: 0,
        };
    }

    let mut min_off: Option<usize> = None;
    let mut max_off: Option<usize> = None;
    let mut active_p1 = Vec::new();
    let mut data_count = 0usize;

    let limit = bytes.len().saturating_sub(BULK_SECTOR_BYTES);
    let mut off = 0x4_0000usize;
    while off <= limit {
        let hdr = &bytes[off..off + BGO_ZERO_END];
        let w0 = read_le_u16(hdr, 4);
        let consistent = (0..4).all(|i| read_le_u16(hdr, 4 + i * 4) == w0);
        let zero_ok = hdr[BGO_ZERO_START..BGO_ZERO_END].iter().all(|&b| b == 0);
        if consistent && zero_ok {
            let sa = bulk_offset_to_sa(off, flash_size);
            if w0 == 0xF101 {
                active_p1.push(sa);
                min_off = Some(min_off.map_or(off, |m| m.min(off)));
                max_off = Some(max_off.map_or(off, |m| m.max(off)));
            } else if (w0 & 0xFF00) == 0xF200 {
                data_count += 1;
                min_off = Some(min_off.map_or(off, |m| m.min(off)));
                max_off = Some(max_off.map_or(off, |m| m.max(off)));
            }
        }
        off += BULK_SECTOR_BYTES;
    }

    match (min_off, max_off) {
        (Some(s), Some(e)) => BgoBounds {
            detected: true,
            start_offset: s as u64,
            end_offset: (e + BULK_SECTOR_BYTES) as u64,
            first_sa: bulk_offset_to_sa(s, flash_size),
            last_sa: bulk_offset_to_sa(e, flash_size),
            active_p1_sectors: active_p1,
            data_sector_count: data_count,
        },
        _ => BgoBounds {
            detected: false,
            start_offset: 0,
            end_offset: 0,
            first_sa: 0,
            last_sa: 0,
            active_p1_sectors: Vec::new(),
            data_sector_count: 0,
        },
    }
}

fn infer_nvram(bytes: &[u8], flash_size: u64, cat_nvram_base: Option<u64>) -> NvramGeometry {
    let (default_base, default_size, default_sectors) = if flash_size >= 16 * 1024 * 1024 {
        (
            FLASH_16MB_NVRAM_BASE_OFFSET as u64,
            FLASH_16MB_NVRAM_SIZE_BYTES as u64,
            FLASH_16MB_MICROSECTOR_COUNT,
        )
    } else {
        let base = flash_size.saturating_sub(SPANSION_NVRAM_SIZE_BYTES as u64);
        (
            cat_nvram_base.unwrap_or(base),
            SPANSION_NVRAM_SIZE_BYTES as u64,
            SPANSION_NVRAM_SIZE_BYTES / SPANSION_MICROSECTOR_SIZE,
        )
    };

    let (base_offset, size_bytes, sector_count) =
        if is_valid_nor_flash_size(bytes.len() as u64) && flash_size >= 16 * 1024 * 1024 {
            let lower_half_has_data = bytes
                [FLASH_16MB_NVRAM_BASE_OFFSET..FLASH_16MB_NVRAM_BASE_OFFSET + BULK_SECTOR_BYTES]
                .iter()
                .any(|&b| b != 0xFF);
            if lower_half_has_data {
                (default_base, default_size, default_sectors)
            } else {
                (
                    flash_size - SPANSION_NVRAM_SIZE_BYTES as u64,
                    SPANSION_NVRAM_SIZE_BYTES as u64,
                    SPANSION_NVRAM_SIZE_BYTES / SPANSION_MICROSECTOR_SIZE,
                )
            }
        } else {
            (default_base, default_size, default_sectors)
        };

    let non_erased_bytes = if is_valid_nor_flash_size(bytes.len() as u64)
        && base_offset + size_bytes <= bytes.len() as u64
    {
        bytes[base_offset as usize..(base_offset + size_bytes) as usize]
            .iter()
            .filter(|&&b| b != 0xFF)
            .count()
    } else {
        0
    };

    NvramGeometry {
        base_offset,
        size_bytes,
        microsector_size_bytes: SPANSION_MICROSECTOR_SIZE,
        sector_count,
        non_erased_bytes,
    }
}

/// Builds a dynamic target hardware profile from raw NOR, `.FRM`, or MEIHFS HDD bytes.
pub fn build_dynamic_target_profile(bytes: &[u8], label: &str) -> DynamicTargetProfile {
    let fp = fingerprint_binary(bytes, label);
    let (flash_size_bytes, cat_nvram_base) = infer_flash_size(bytes, &fp.detected_model);
    let nor_flash = NorFlashGeometry {
        flash_size_bytes,
        sector_stride_bytes: BULK_SECTOR_BYTES,
        bulk_sector_count: (flash_size_bytes as usize) / BULK_SECTOR_BYTES,
        ipl_readonly_bytes: flash_size_bytes.min(0x4_0000),
    };
    let bgo_bounds = scan_bgo_bounds(bytes, flash_size_bytes);
    let nvram = infer_nvram(bytes, flash_size_bytes, cat_nvram_base);

    let meihfs_mbr_present = (bytes.starts_with(b"SH")
        || bytes.starts_with(&PANASONIC_HDD_MBR_SIGNATURE))
        && bytes.len() >= 512;
    let mirror_off = LBA_MIRROR_MBR_OFFSET as usize;
    let mirror_mbr_present = bytes.len() >= mirror_off + PANASONIC_HDD_MBR_SIGNATURE.len()
        && (&bytes[mirror_off..mirror_off + 2] == b"SH"
            || bytes[mirror_off..].starts_with(&PANASONIC_HDD_MBR_SIGNATURE));
    let clamp_site_offset = find_hdd_cmd_clamp_site(bytes);
    let max_expected_gb = if clamp_site_offset.is_some() || flash_size_bytes >= 16 * 1024 * 1024 {
        2000
    } else {
        400
    };

    let mut notes = Vec::new();
    if matches!(fp.container, ContainerFormat::RawNor) && !bgo_bounds.detected {
        notes.push(
            "NOR dump carries no active 0xF101/0xF2xx BGO sectors (erased or different FTL)."
                .into(),
        );
    }
    if clamp_site_offset.is_some() {
        notes.push(
            "Located hdd_cmd.c RTSD capacity clamp pattern supporting dynamic unlock.".into(),
        );
    }

    DynamicTargetProfile {
        label: label.to_string(),
        model: fp.detected_model,
        container: fp.container,
        cipher_family: fp.cipher_family,
        nor_flash,
        bgo_bounds,
        nvram,
        hdd: HddExpectation {
            meihfs_mbr_present,
            mirror_mbr_present,
            clamp_site_offset,
            min_expected_gb: 80,
            max_expected_gb,
            supports_dynamic_unlock: clamp_site_offset.is_some(),
        },
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{PANA_FEISTEL_ROUND_KEY_32B, SPANSION_NVRAM_BASE_OFFSET};
    use crate::crypto::{FrmContainerMode, FrmPackModule, pack_frm_with_key};

    fn stamp_bgo(nor: &mut [u8], off: usize, word: u16) {
        for i in 0..4 {
            nor[off + 4 + i * 4..off + 6 + i * 4].copy_from_slice(&word.to_le_bytes());
        }
        nor[off + 0x100..off + 0x200].fill(0x00);
    }

    #[test]
    fn test_dynamic_profile_4_8_16mb_nor_frm_and_meihfs() {
        let nor4 = vec![0xFFu8; 4 * 1024 * 1024];
        let p4 = build_dynamic_target_profile(&nor4, "nor4m.bin");
        assert_eq!(p4.nor_flash.flash_size_bytes, 4 * 1024 * 1024);
        assert_eq!(p4.nvram.base_offset, 0x3F_0000);

        let mut nor8 = vec![0xFFu8; SPANSION_FLASH_SIZE_BYTES];
        stamp_bgo(&mut nor8, 0x15_0000, 0xF201);
        stamp_bgo(&mut nor8, 0x20_0000, 0xF101);
        nor8[SPANSION_NVRAM_BASE_OFFSET..SPANSION_NVRAM_BASE_OFFSET + 16].fill(0x5A);
        let p8 = build_dynamic_target_profile(&nor8, "nor8m.bin");
        assert!(p8.bgo_bounds.detected);
        assert_eq!(p8.bgo_bounds.active_p1_sectors, vec![39]);
        assert_eq!(p8.nvram.non_erased_bytes, 16);

        let mut nor16 = vec![0xFFu8; 16 * 1024 * 1024];
        nor16[FLASH_16MB_NVRAM_BASE_OFFSET] = 0xAA;
        let p16 = build_dynamic_target_profile(&nor16, "nor16m.bin");
        assert_eq!(p16.nvram.base_offset, FLASH_16MB_NVRAM_BASE_OFFSET as u64);
        assert_eq!(p16.nvram.size_bytes, FLASH_16MB_NVRAM_SIZE_BYTES as u64);

        let frm = pack_frm_with_key(
            &[FrmPackModule::new("MAIN", b"DMR-EH65".to_vec())],
            FrmContainerMode::Dvd64B,
            Some(0x2000),
            &PANA_FEISTEL_ROUND_KEY_32B,
        )
        .unwrap();
        let pf = build_dynamic_target_profile(&frm.0, "PANA_DVD.FRM");
        assert_eq!(pf.container, ContainerFormat::Dvd64B);

        let mut hdd = vec![0u8; 1024];
        hdd[..2].copy_from_slice(b"SH");
        let ph = build_dynamic_target_profile(&hdd, "hdd.bin");
        assert!(ph.hdd.meihfs_mbr_present);
    }
}
