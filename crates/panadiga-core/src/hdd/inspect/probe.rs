//! Pure functional structural inspector for Panasonic DMR-EH55 HDD slices.

use anyhow::{Result, bail};
use byteorder::{ByteOrder as _, LittleEndian};

use super::super::{
    types::{
        AllocationJournalReport, ChunkDensityReport, HddAnalysisReport, MbrTableReport,
        PartitionReport,
    },
    utils::calculate_entropy,
};
use crate::{constants::*, nvram::calculate_hashes};

fn inspect_mbr_tables(data: &[u8]) -> Result<MbrTableReport> {
    if data.len() < LBA_SECTOR_SIZE_BYTES {
        bail!("Image slice is too small to contain LBA 0");
    }
    let lba0 = &data[..LBA_SECTOR_SIZE_BYTES];
    let (lba0_md5, lba0_sha256) = calculate_hashes(lba0);

    let format_version = LittleEndian::read_u16(&lba0[4..6]);
    let magic_ascii = String::from_utf8_lossy(&lba0[6..8]).to_string();
    let volume_guid = hex::encode_upper(&lba0[8..16]);
    let has_55aa = lba0[510..512] == STANDARD_MBR_BOOT_SIGNATURE;
    let zeros_clean = lba0[16..512].iter().all(|&b| b == 0);

    let mirror_offset = LBA_MIRROR_MBR_OFFSET as usize;
    let mirror_end = mirror_offset + LBA_SECTOR_SIZE_BYTES;
    if data.len() < mirror_end {
        bail!(
            "Image too small to contain Mirror MBR at offset 0x{:08X}",
            mirror_offset
        );
    }
    let lba_mirror = &data[mirror_offset..mirror_end];
    let (mirror_md5, mirror_sha256) = calculate_hashes(lba_mirror);
    let mirror_identical = lba0 == lba_mirror;

    Ok(MbrTableReport {
        lba0_offset: 0,
        lba0_hex_signature: hex::encode_upper(&lba0[..16]),
        format_version,
        magic_ascii,
        disk_volume_guid: volume_guid,
        has_standard_mbr_55aa: has_55aa,
        zeros_after_header_clean: zeros_clean,
        lba0_md5,
        lba0_sha256,
        mirror_lba: LBA_MIRROR_MBR_SECTOR,
        mirror_offset: LBA_MIRROR_MBR_OFFSET,
        mirror_is_bit_identical: mirror_identical,
        mirror_md5,
        mirror_sha256,
    })
}

fn inspect_alloc_and_journal(data: &[u8]) -> AllocationJournalReport {
    let alloc_start = ALLOCATION_DESCRIPTORS_OFFSET as usize;
    let alloc_end = alloc_start + ALLOCATION_DESCRIPTORS_SIZE;
    let (alloc_mirror_identical, alloc_sha256) = if data.len() >= alloc_end {
        let alloc_slice = &data[alloc_start..alloc_end];
        let (_alloc_md5, sha) = calculate_hashes(alloc_slice);
        let alloc_mirror_start = ALLOCATION_DESCRIPTORS_MIRROR_OFFSET as usize;
        let alloc_mirror_end = alloc_mirror_start + ALLOCATION_DESCRIPTORS_SIZE;
        let mirror_match = if data.len() >= alloc_mirror_end {
            alloc_slice == &data[alloc_mirror_start..alloc_mirror_end]
        } else {
            false
        };
        (mirror_match, sha)
    } else {
        (false, String::new())
    };

    let journal_cat_start = LBA_JOURNAL_CATALOG_OFFSET as usize;
    let (j_total_blocks, j_max_index) = if data.len() >= journal_cat_start + LBA_SECTOR_SIZE_BYTES {
        let cat = &data[journal_cat_start..journal_cat_start + LBA_SECTOR_SIZE_BYTES];
        (
            LittleEndian::read_u32(&cat[0..4]),
            LittleEndian::read_u32(&cat[4..8]),
        )
    } else {
        (0, 0)
    };

    let mut journal_trailers = Vec::new();
    for i in 0..JOURNAL_BLOCK_COUNT {
        let blk_start = JOURNAL_AREA_OFFSET as usize + i * JOURNAL_BLOCK_SIZE;
        if data.len() >= blk_start + JOURNAL_BLOCK_SIZE {
            let blk_slice = &data[blk_start..blk_start + JOURNAL_BLOCK_SIZE];
            let last_nz = blk_slice.iter().rposition(|&b| b != 0).unwrap_or(0);
            let trailer_hex = if last_nz >= 7 {
                hex::encode_upper(&blk_slice[last_nz - 7..=last_nz])
            } else {
                "0000000000000000".to_string()
            };
            journal_trailers.push(trailer_hex);
        } else {
            journal_trailers.push("0000000000000000".to_string());
        }
    }

    let state_marker_offset = LBA_STATE_TRANSITION_MARKER_OFFSET as usize;
    let state_marker_hex = if data.len() >= state_marker_offset + 24 {
        hex::encode_upper(&data[state_marker_offset..state_marker_offset + 24])
    } else {
        String::new()
    };

    AllocationJournalReport {
        descriptors_offset: ALLOCATION_DESCRIPTORS_OFFSET,
        descriptors_size: ALLOCATION_DESCRIPTORS_SIZE,
        descriptors_mirror_identical: alloc_mirror_identical,
        descriptors_sha256: alloc_sha256,
        journal_catalog_offset: LBA_JOURNAL_CATALOG_OFFSET,
        journal_total_blocks: j_total_blocks,
        journal_max_index: j_max_index,
        journal_block_trailers: journal_trailers,
        state_marker_offset: LBA_STATE_TRANSITION_MARKER_OFFSET,
        state_marker_hex,
    }
}

fn discover_firmware_partitions(data: &[u8]) -> Vec<PartitionReport> {
    let mut partitions = Vec::new();

    let p1_offset = LBA_PARTITION_1_OFFSET as usize;
    if data.len() >= p1_offset + LBA_PARTITION_1_ALIGNED_SIZE
        && data[p1_offset..p1_offset + 8] == PANASONIC_HDD_PARTITION_MAGIC
    {
        let payload = &data[p1_offset..p1_offset + LBA_PARTITION_1_PAYLOAD_SIZE];
        let aligned = &data[p1_offset..p1_offset + LBA_PARTITION_1_ALIGNED_SIZE];
        let (exact_md5, exact_sha256) = calculate_hashes(payload);
        let (aligned_md5, aligned_sha256) = calculate_hashes(aligned);
        let ent = calculate_entropy(payload);
        let hdr_64 = hex::encode_upper(&payload[..64]);

        partitions.push(PartitionReport {
            partition_id: 1,
            name: "Firmware Principal & Codecs (AM34 / DIGA OS)".to_string(),
            offset: LBA_PARTITION_1_OFFSET,
            offset_hex: format!("0x{:08X}", LBA_PARTITION_1_OFFSET),
            lba_start: LBA_PARTITION_1_SECTOR,
            exact_payload_size: LBA_PARTITION_1_PAYLOAD_SIZE,
            sector_aligned_size: LBA_PARTITION_1_ALIGNED_SIZE,
            sectors_count: LBA_PARTITION_1_ALIGNED_SIZE / LBA_SECTOR_SIZE_BYTES,
            signature_hex: hex::encode_upper(PANASONIC_HDD_PARTITION_MAGIC),
            entropy: (ent * 10000.0).round() / 10000.0,
            exact_md5,
            exact_sha256,
            aligned_md5: Some(aligned_md5),
            aligned_sha256: Some(aligned_sha256),
            header_hex_64: hdr_64,
        });
    }

    let p2_offset = LBA_PARTITION_2_OFFSET as usize;
    if data.len() >= p2_offset + LBA_PARTITION_2_PAYLOAD_SIZE
        && data[p2_offset..p2_offset + 8] == PANASONIC_HDD_PARTITION_MAGIC
    {
        let payload = &data[p2_offset..p2_offset + LBA_PARTITION_2_PAYLOAD_SIZE];
        let (exact_md5, exact_sha256) = calculate_hashes(payload);
        let ent = calculate_entropy(payload);
        let hdr_64 = hex::encode_upper(&payload[..64]);

        partitions.push(PartitionReport {
            partition_id: 2,
            name: "Sub-sistema / Microcodigo & Fontes OSD".to_string(),
            offset: LBA_PARTITION_2_OFFSET,
            offset_hex: format!("0x{:08X}", LBA_PARTITION_2_OFFSET),
            lba_start: LBA_PARTITION_2_SECTOR,
            exact_payload_size: LBA_PARTITION_2_PAYLOAD_SIZE,
            sector_aligned_size: LBA_PARTITION_2_PAYLOAD_SIZE,
            sectors_count: LBA_PARTITION_2_PAYLOAD_SIZE / LBA_SECTOR_SIZE_BYTES,
            signature_hex: hex::encode_upper(PANASONIC_HDD_PARTITION_MAGIC),
            entropy: (ent * 10000.0).round() / 10000.0,
            exact_md5,
            exact_sha256,
            aligned_md5: None,
            aligned_sha256: None,
            header_hex_64: hdr_64,
        });
    }

    partitions
}

/// 100% pure functional inspection of a Panasonic HDD image memory slice.
/// Performs complete structural forensic analysis, MBR validation, journal inspection,
/// partition discovery, and density computation with zero disk I/O side-effects.
pub fn inspect_hdd_slice(data: &[u8], image_label: &str) -> Result<HddAnalysisReport> {
    let file_size = data.len() as u64;
    let total_sectors = file_size / LBA_SECTOR_SIZE_BYTES as u64;
    let (global_md5, global_sha256) = calculate_hashes(data);

    let mbr_report = inspect_mbr_tables(data)?;
    let alloc_journal_report = inspect_alloc_and_journal(data);
    let partitions = discover_firmware_partitions(data);

    let mut density_map = Vec::new();
    let chunk_size = 1024 * 1024;
    let total_chunks = (file_size as usize) / chunk_size;

    for mb in 0..total_chunks {
        let start = mb * chunk_size;
        let end = start + chunk_size;
        let chunk = &data[start..end];
        let non_zero = chunk.iter().filter(|&&b| b != 0).count();
        let pct = (non_zero as f64 / chunk_size as f64) * 100.0;

        density_map.push(ChunkDensityReport {
            mb_index: mb,
            offset: start as u64,
            non_zero_bytes: non_zero,
            percentage: (pct * 100.0).round() / 100.0,
        });
    }

    Ok(HddAnalysisReport {
        image_path: image_label.to_string(),
        file_size_bytes: file_size,
        file_size_mb: ((file_size as f64 / (1024.0 * 1024.0)) * 100.0).round() / 100.0,
        total_sectors,
        global_md5,
        global_sha256,
        mbr_tables: mbr_report,
        allocation_and_journal: alloc_journal_report,
        partitions,
        density_map,
    })
}
