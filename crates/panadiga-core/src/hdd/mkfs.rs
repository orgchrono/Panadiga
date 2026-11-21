//! Procedural MEIHDFS (Matsushita High-Density File System) Synthesizer / mkfs
//!
//! Generates valid, mathematically exact Panasonic DIGA disk layouts from first principles
//! WITHOUT requiring copyrighted Panasonic firmware binary dumps:
//! - Master MBR Superblock (LBA 0) with signature "SH"
//! - Inverted Allocation Descriptors table (LBA 24..31)
//! - Transactional Journal Catalog (LBA 16384)
//! - Redundant Mirror MBR (LBA 57344) and Mirror Descriptors (LBA 57368)
//! - Dynamically sized Partition 3 (User Video Area) calculated for ANY drive size (80GB to 2TB+)

use crate::constants::*;
use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};

/// Holds the procedurally generated MEIHDFS structures in memory.
#[derive(Debug, Clone)]
pub struct MeihdfsImage {
    pub total_sectors: u64,
    pub volume_guid: [u8; 16],
    pub mbr_lba0: [u8; LBA_SECTOR_SIZE_BYTES],
    pub descriptors_lba24: [u8; ALLOCATION_DESCRIPTORS_SIZE],
    pub journal_header_lba16384: [u8; LBA_SECTOR_SIZE_BYTES],
    pub mirror_mbr_lba57344: [u8; LBA_SECTOR_SIZE_BYTES],
    pub mirror_descriptors_lba57368: [u8; ALLOCATION_DESCRIPTORS_SIZE],
    pub p1_lba_start: u64,
    pub p1_sectors: u64,
    pub p2_lba_start: u64,
    pub p2_sectors: u64,
    pub p3_lba_start: u64,
    pub p3_sectors: u64,
}

/// Generates a clean procedural MEIHDFS filesystem image for a target sector count.
pub fn generate_meihdfs_layout(
    total_sectors: u64,
    custom_guid: Option<[u8; 16]>,
) -> Result<MeihdfsImage> {
    const MIN_SECTORS: u64 = 200_000; // ~102 MB minimum required for system partitions
    if total_sectors < MIN_SECTORS {
        bail!(
            "Target drive size too small for MEIHDFS: {} sectors (minimum {} sectors / ~102 MB required)",
            total_sectors,
            MIN_SECTORS
        );
    }

    let volume_guid = custom_guid.unwrap_or([
        0x62, 0x8C, 0x6E, 0xF5, 0x1B, 0x2C, 0x36, 0x66, 0xAA, 0x55, 0x00, 0x16, 0x4D, 0x45, 0x49,
        0x48,
    ]);

    let p1_start = LBA_PARTITION_1_SECTOR; // 65536 (32 MB)
    let p1_sectors = 65536u64; // 32 MB
    let p2_start = LBA_PARTITION_2_SECTOR; // 131072 (64 MB)
    let p2_sectors = 65536u64; // 32 MB
    let p3_start = 196608u64; // 96 MB
    let p3_sectors = total_sectors.saturating_sub(p3_start);

    // 1. Generate LBA 0 (Master MBR Superblock)
    // Real Panasonic DIGA drives keep LBA 0[16..512] zeroed (`zeros_after_header_clean == true`);
    // partition extents reside in the Allocation Descriptors (LBA 24..31 / LBA 57368).
    let mut mbr = [0u8; LBA_SECTOR_SIZE_BYTES];
    // Signature prefix: 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x53 ('S'), 0x48 ('H')
    mbr[0..8].copy_from_slice(&PANASONIC_HDD_MBR_SIGNATURE);
    // 8-byte Volume GUID prefix (LBA 0[8..16]); bytes 16..512 stay 0x00 per `hdd/inspect/probe.rs`.
    mbr[8..16].copy_from_slice(&volume_guid[..8]);

    // 2. Generate Allocation Descriptors (256 records of 16 bytes, bitwise inverted ^ 0xFF)
    let mut descriptors = [0u8; ALLOCATION_DESCRIPTORS_SIZE];
    // Helper closure to set an inverted record
    let mut write_record = |idx: usize, rec: &[u8; 16]| {
        let base = idx * 16;
        for j in 0..16 {
            descriptors[base + j] = rec[j] ^ 0xFF;
        }
    };
    let write_partition_extent =
        |hdr_idx: usize,
         id: u16,
         start_lba: u64,
         sector_count: u64,
         write_rec: &mut dyn FnMut(usize, &[u8; 16])| {
            let mut hdr = [0u8; 16];
            hdr[0] = MEIHDFS_TAG_EXTENT_HEADER;
            hdr[1] = id as u8;
            LittleEndian::write_u16(&mut hdr[2..4], id);
            LittleEndian::write_u16(&mut hdr[4..6], 0x0001);
            write_rec(hdr_idx, &hdr);

            let mut ptr = [0u8; 16];
            ptr[0] = MEIHDFS_TAG_EXTENT_POINTER;
            ptr[1] = 0x00;
            LittleEndian::write_u16(&mut ptr[2..4], id);
            // Full 32-bit LBA & sector counts (up to 2 TiB = u32::MAX sectors) with zero 16-bit truncation.
            let sectors_u32 = sector_count.min(u32::MAX as u64) as u32;
            let start_u32 = start_lba.min(u32::MAX as u64) as u32;
            LittleEndian::write_u32(&mut ptr[4..8], sectors_u32);
            LittleEndian::write_u32(&mut ptr[8..12], start_u32);
            write_rec(hdr_idx + 1, &ptr);
        };

    write_partition_extent(0, 1, p1_start, p1_sectors, &mut write_record);
    write_partition_extent(2, 2, p2_start, p2_sectors, &mut write_record);
    write_partition_extent(4, 3, p3_start, p3_sectors, &mut write_record);

    // Records 6..255 are free unallocated slots (Tag 0xFF in clear -> 0x00 on disk)
    for i in 6..MEIHDFS_DESCRIPTOR_RECORDS_COUNT {
        let mut free_rec = [0xFFu8; 16];
        free_rec[0] = MEIHDFS_TAG_FREE;
        write_record(i, &free_rec);
    }

    // 3. Generate Audited 10-Block Transactional Journal Catalog Header (LBA 16384 = 0x00800000)
    // Matches `hdd/inspect/probe.rs`: [0..4] = total blocks (0x0A), [4..8] = max index (0x09).
    let mut journal_hdr = [0u8; LBA_SECTOR_SIZE_BYTES];
    LittleEndian::write_u32(&mut journal_hdr[0..4], JOURNAL_BLOCK_COUNT as u32);
    LittleEndian::write_u32(
        &mut journal_hdr[4..8],
        (JOURNAL_BLOCK_COUNT.saturating_sub(1)) as u32,
    );

    // 4. Mirror MBR (LBA 57344) and Mirror Descriptors (LBA 57368) - Bit-for-bit replicas
    let mirror_mbr = mbr;
    let mirror_desc = descriptors;

    Ok(MeihdfsImage {
        total_sectors,
        volume_guid,
        mbr_lba0: mbr,
        descriptors_lba24: descriptors,
        journal_header_lba16384: journal_hdr,
        mirror_mbr_lba57344: mirror_mbr,
        mirror_descriptors_lba57368: mirror_desc,
        p1_lba_start: p1_start,
        p1_sectors,
        p2_lba_start: p2_start,
        p2_sectors,
        p3_lba_start: p3_start,
        p3_sectors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedural_meihdfs_synthesis_minimum_drive() {
        let sectors = 312_581_808u64; // ~160 GB drive
        let img = generate_meihdfs_layout(sectors, None).expect("Should generate layout");

        // Verify MBR Signature and strict trailing zero cleanliness (matching real Panasonic drives)
        assert_eq!(&img.mbr_lba0[0..8], &PANASONIC_HDD_MBR_SIGNATURE);
        assert!(img.mbr_lba0[16..512].iter().all(|&b| b == 0));
        // Verify Mirror MBR is 100% bit-identical
        assert_eq!(&img.mbr_lba0[..], &img.mirror_mbr_lba57344[..]);
        // Verify Mirror Descriptors are 100% bit-identical
        assert_eq!(
            &img.descriptors_lba24[..],
            &img.mirror_descriptors_lba57368[..]
        );

        // Verify Partition 3 expands dynamically
        assert_eq!(img.p3_lba_start, 196608);
        assert_eq!(img.p3_sectors, sectors - 196608);

        // Verify first descriptor record is bitwise inverted extent header
        let raw_tag = img.descriptors_lba24[0];
        let clear_tag = raw_tag ^ 0xFF;
        assert_eq!(clear_tag, MEIHDFS_TAG_EXTENT_HEADER);
    }

    #[test]
    fn test_procedural_meihdfs_rejects_undersized_drive() {
        let sectors = 50_000u64; // Too small
        let res = generate_meihdfs_layout(sectors, None);
        assert!(res.is_err());
    }

    #[test]
    fn test_procedural_meihdfs_passes_strict_forensic_probe() {
        let img = generate_meihdfs_layout(312_581_808, None).unwrap();
        let mirror_desc_off = ALLOCATION_DESCRIPTORS_MIRROR_OFFSET as usize;
        let buf_len = mirror_desc_off + ALLOCATION_DESCRIPTORS_SIZE;
        let mut buf = vec![0u8; buf_len];
        buf[..LBA_SECTOR_SIZE_BYTES].copy_from_slice(&img.mbr_lba0);
        let desc_off = ALLOCATION_DESCRIPTORS_OFFSET as usize;
        buf[desc_off..desc_off + ALLOCATION_DESCRIPTORS_SIZE]
            .copy_from_slice(&img.descriptors_lba24);
        let jrnl_off = LBA_JOURNAL_CATALOG_OFFSET as usize;
        buf[jrnl_off..jrnl_off + LBA_SECTOR_SIZE_BYTES]
            .copy_from_slice(&img.journal_header_lba16384);
        let mirror_mbr_off = LBA_MIRROR_MBR_OFFSET as usize;
        buf[mirror_mbr_off..mirror_mbr_off + LBA_SECTOR_SIZE_BYTES]
            .copy_from_slice(&img.mirror_mbr_lba57344);
        buf[mirror_desc_off..mirror_desc_off + ALLOCATION_DESCRIPTORS_SIZE]
            .copy_from_slice(&img.mirror_descriptors_lba57368);

        let rep = crate::hdd::inspect::inspect_hdd_slice(&buf, "mem://mkfs_roundtrip").unwrap();
        assert_eq!(rep.mbr_tables.magic_ascii, "SH");
        assert!(rep.mbr_tables.zeros_after_header_clean);
        assert!(rep.mbr_tables.mirror_is_bit_identical);
        assert!(rep.allocation_and_journal.descriptors_mirror_identical);
        assert_eq!(rep.allocation_and_journal.journal_total_blocks, 10);
        assert_eq!(rep.allocation_and_journal.journal_max_index, 9);

        let dec =
            crate::hdd::meihdfs::decode_meihdfs_slice(&buf, false, "mem://mkfs_roundtrip").unwrap();
        assert_eq!(dec.extents.len(), 3);
        assert_eq!(dec.records[1].sector_count, Some(65536));
        assert_eq!(dec.records[1].lba_offset, Some(65536));
        assert_eq!(dec.records[3].sector_count, Some(65536));
        assert_eq!(dec.records[3].lba_offset, Some(131072));
        assert_eq!(dec.records[5].sector_count, Some(312_385_200));
        assert_eq!(dec.records[5].lba_offset, Some(196608));
        assert_eq!(dec.tag_distribution.get("0xFF (Free)").copied(), Some(250));
    }
}
