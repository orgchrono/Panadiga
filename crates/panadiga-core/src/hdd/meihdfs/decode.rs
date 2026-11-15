//! Pure functional decoders for MEIHDFS allocation descriptors, extents, and journal blocks.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use crate::constants::*;
use crate::nvram::calculate_hashes;

use super::super::types::{
    JournalBlockDecoded, MeihdfsAllocationRecord, MeihdfsAllocationReport, MeihdfsExtentSegment,
    MeihdfsExtentSummary,
};

/// Pure Functional Core: Decodes a single 16-byte bit-inverted MEIHDFS allocation record.
pub fn decode_single_meihdfs_record(raw_chunk: &[u8], index: usize) -> MeihdfsAllocationRecord {
    let offset = index * MEIHDFS_RECORD_SIZE;
    let inv_chunk: [u8; MEIHDFS_RECORD_SIZE] = std::array::from_fn(|j| raw_chunk[j] ^ 0xFF);

    let tag = inv_chunk[0];
    let raw_tag = raw_chunk[0];
    let subtag = inv_chunk[1];

    let (tag_name, extent_id, sector_count, lba_offset, flags, description) = match tag {
        MEIHDFS_TAG_EXTENT_HEADER => {
            let id = LittleEndian::read_u16(&inv_chunk[2..4]);
            let fl = LittleEndian::read_u16(&inv_chunk[4..6]);
            let p1 = LittleEndian::read_u16(&inv_chunk[6..8]);
            let p2 = LittleEndian::read_u32(&inv_chunk[8..12]);
            (
                "0x5D (ExtentHeader)".to_string(),
                Some(id),
                None,
                None,
                Some(fl),
                format!(
                    "Extent ID: 0x{:04X} | Subtag: 0x{:02X} | Flags: 0x{:04X} | Segments: {} | Param: 0x{:08X}",
                    id, subtag, fl, p1, p2
                ),
            )
        }
        MEIHDFS_TAG_EXTENT_POINTER => {
            let id = LittleEndian::read_u16(&inv_chunk[2..4]);
            let sectors = LittleEndian::read_u16(&inv_chunk[4..6]) as u32;
            let lba = LittleEndian::read_u16(&inv_chunk[6..8]) as u32;
            let fl = subtag as u16;
            (
                "0x55 (ExtentPointer)".to_string(),
                Some(id),
                Some(sectors),
                Some(lba),
                Some(fl),
                format!(
                    "Extent ID: 0x{:04X} | Sectors: {} ({} bytes) | LBA Off: {} | Flags: 0x{:02X}",
                    id,
                    sectors,
                    sectors * 512,
                    lba,
                    subtag
                ),
            )
        }
        MEIHDFS_TAG_EXTENT_CONTINUATION => {
            let id = LittleEndian::read_u16(&inv_chunk[2..4]);
            (
                "0x57 (ExtentContinuation)".to_string(),
                Some(id),
                None,
                None,
                Some(subtag as u16),
                format!(
                    "Continuation ID: 0x{:04X} | Subtag: 0x{:02X} | Payload: {}",
                    id,
                    subtag,
                    hex::encode_upper(&inv_chunk[4..12])
                ),
            )
        }
        MEIHDFS_TAG_EXTENT_BOUNDARY => {
            let id = LittleEndian::read_u16(&inv_chunk[2..4]);
            (
                "0x5F (ExtentBoundary)".to_string(),
                Some(id),
                None,
                None,
                Some(subtag as u16),
                format!(
                    "Boundary ID: 0x{:04X} | Subtag: 0x{:02X} | Limiter: {}",
                    id,
                    subtag,
                    hex::encode_upper(&inv_chunk[4..12])
                ),
            )
        }
        MEIHDFS_TAG_VNODE_ENTRY => {
            let id = LittleEndian::read_u16(&inv_chunk[2..4]);
            (
                "0xE0 (VNodeEntry)".to_string(),
                Some(id),
                None,
                None,
                Some(subtag as u16),
                format!(
                    "VNode Type: 0x{:02X} | Node Index: 0x{:04X} | Metadata: {}",
                    subtag,
                    id,
                    hex::encode_upper(&inv_chunk[4..12])
                ),
            )
        }
        MEIHDFS_TAG_VINODE_LINK => {
            let parent = LittleEndian::read_u16(&inv_chunk[2..4]);
            let child = LittleEndian::read_u16(&inv_chunk[4..6]);
            (
                "0xE1 (VInodeLink)".to_string(),
                Some(child),
                None,
                None,
                Some(subtag as u16),
                format!(
                    "Directory Link | Flags: 0x{:02X} | Parent: 0x{:04X} -> Child: 0x{:04X}",
                    subtag, parent, child
                ),
            )
        }
        _ => (
            format!("0x{:02X} (Unknown)", tag),
            None,
            None,
            None,
            None,
            format!("Unrecognized Record Tag: 0x{:02X}", tag),
        ),
    };

    MeihdfsAllocationRecord {
        index,
        offset_in_table: offset,
        raw_hex: hex::encode_upper(raw_chunk),
        inverted_hex: hex::encode_upper(inv_chunk),
        raw_tag,
        decoded_tag: tag,
        tag_name,
        extent_id,
        sector_count,
        lba_offset,
        flags,
        tail_hex: hex::encode_upper(&inv_chunk[12..16]),
        description,
    }
}

/// Pure Functional Core: Groups decoded records into consolidated extent summaries.
pub fn aggregate_meihdfs_extents(records: &[MeihdfsAllocationRecord]) -> Vec<MeihdfsExtentSummary> {
    let mut extents_map: BTreeMap<u16, (Option<usize>, Vec<MeihdfsExtentSegment>)> =
        BTreeMap::new();
    for rec in records {
        if let Some(id) = rec.extent_id {
            let entry = extents_map.entry(id).or_insert((None, Vec::new()));
            if rec.decoded_tag == MEIHDFS_TAG_EXTENT_HEADER && entry.0.is_none() {
                entry.0 = Some(rec.index);
            }
            if rec.decoded_tag == MEIHDFS_TAG_EXTENT_POINTER {
                entry.1.push(MeihdfsExtentSegment {
                    record_index: rec.index,
                    subtag: rec.flags.unwrap_or(0) as u8,
                    sector_count: rec.sector_count.unwrap_or(0) as u16,
                    lba_offset_hint: rec.lba_offset.unwrap_or(0) as u16,
                    flags: rec.flags.unwrap_or(0),
                    tail_hex: rec.tail_hex.clone(),
                });
            }
        }
    }

    extents_map
        .into_iter()
        .map(|(id, (hdr_idx, segments))| {
            let total_sectors: u32 = segments.iter().map(|s| s.sector_count as u32).sum();
            let total_bytes = total_sectors as u64 * 512;
            MeihdfsExtentSummary {
                extent_id: id,
                extent_id_hex: format!("0x{:04X}", id),
                header_record_index: hdr_idx,
                total_sectors,
                total_bytes,
                segments_count: segments.len(),
                segments,
            }
        })
        .collect()
}

/// Pure Functional Core: Decodes the 10 transactional journal blocks from a byte slice.
pub fn decode_journal_blocks_slice(data: &[u8]) -> Vec<JournalBlockDecoded> {
    let mut journal_blocks = Vec::with_capacity(JOURNAL_BLOCK_COUNT);
    for b_idx in 0..JOURNAL_BLOCK_COUNT {
        let b_start = JOURNAL_AREA_OFFSET as usize + b_idx * JOURNAL_BLOCK_SIZE;
        if data.len() >= b_start + JOURNAL_BLOCK_SIZE {
            let blk = &data[b_start..b_start + JOURNAL_BLOCK_SIZE];
            let is_active = blk[0] == MEIHDFS_TAG_EXTENT_HEADER;
            let hdr_hex = hex::encode_upper(&blk[..16]);
            let crc_hex = hex::encode_upper(
                &blk[JOURNAL_TRANSACTION_TRAILER_OFFSET..JOURNAL_TRANSACTION_TRAILER_OFFSET + 4],
            );
            let pool_size = LittleEndian::read_u32(
                &blk[JOURNAL_TRANSACTION_TRAILER_OFFSET + 4
                    ..JOURNAL_TRANSACTION_TRAILER_OFFSET + 8],
            );
            let seq_id = LittleEndian::read_u32(
                &blk[JOURNAL_TRANSACTION_SEQ_OFFSET..JOURNAL_TRANSACTION_SEQ_OFFSET + 4],
            );
            let lba = (b_start / LBA_SECTOR_SIZE_BYTES) as u64;

            journal_blocks.push(JournalBlockDecoded {
                block_index: b_idx,
                start_lba: lba,
                start_offset_hex: format!("0x{:08X}", b_start),
                is_active_header: is_active,
                header_hex_16: hdr_hex,
                transaction_seq: seq_id,
                pool_size,
                checksum_crc: crc_hex,
                trailer_offset_hex: format!("0x{:05X}", JOURNAL_TRANSACTION_TRAILER_OFFSET),
                active_payload_sectors: (JOURNAL_TRANSACTION_SEQ_OFFSET + 4)
                    / LBA_SECTOR_SIZE_BYTES
                    + 1,
            });
        }
    }
    journal_blocks
}

/// Pure Functional Core: Decodes MEIHDFS descriptors and extent structures in-memory with zero I/O.
pub fn decode_meihdfs_slice(
    data: &[u8],
    include_journal: bool,
    source_label: &str,
) -> Result<MeihdfsAllocationReport> {
    let file_len = data.len();
    let is_full_image = file_len >= (LBA_MIRROR_MBR_OFFSET as usize + ALLOCATION_DESCRIPTORS_SIZE);

    let (alloc_slice, mirror_verified) = if is_full_image {
        let p_start = ALLOCATION_DESCRIPTORS_OFFSET as usize;
        let p_end = p_start + ALLOCATION_DESCRIPTORS_SIZE;
        if file_len < p_end {
            bail!(
                "Image slice too small to contain allocation descriptors table at 0x{:08X}",
                p_start
            );
        }
        let desc = &data[p_start..p_end];

        let m_start = ALLOCATION_DESCRIPTORS_MIRROR_OFFSET as usize;
        let m_end = m_start + ALLOCATION_DESCRIPTORS_SIZE;
        let mirror_ok = if file_len >= m_end {
            desc == &data[m_start..m_end]
        } else {
            false
        };
        (desc, mirror_ok)
    } else {
        if file_len < ALLOCATION_DESCRIPTORS_SIZE {
            bail!(
                "Input descriptors buffer too small (expected at least {} bytes, found {})",
                ALLOCATION_DESCRIPTORS_SIZE,
                file_len
            );
        }
        (&data[..ALLOCATION_DESCRIPTORS_SIZE], false)
    };

    let (_, descriptors_sha256) = calculate_hashes(alloc_slice);

    let records: Vec<MeihdfsAllocationRecord> = (0..MEIHDFS_DESCRIPTOR_RECORDS_COUNT)
        .map(|i| {
            let offset = i * MEIHDFS_RECORD_SIZE;
            decode_single_meihdfs_record(&alloc_slice[offset..offset + MEIHDFS_RECORD_SIZE], i)
        })
        .collect();

    let tag_distribution: BTreeMap<String, usize> =
        records.iter().fold(BTreeMap::new(), |mut acc, rec| {
            *acc.entry(rec.tag_name.clone()).or_insert(0) += 1;
            acc
        });

    let extents = aggregate_meihdfs_extents(&records);

    let journal_blocks = if include_journal && is_full_image {
        decode_journal_blocks_slice(data)
    } else {
        Vec::new()
    };

    Ok(MeihdfsAllocationReport {
        source_path: source_label.to_string(),
        is_full_hdd_image: is_full_image,
        is_mirror_verified: mirror_verified,
        descriptors_sha256,
        total_records: records.len(),
        tag_distribution,
        records,
        extents,
        journal_blocks,
    })
}
