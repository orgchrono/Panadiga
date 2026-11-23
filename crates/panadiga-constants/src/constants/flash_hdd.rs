//! Spansion NOR Flash, HDD MEIHFS geometry, and transactional journal constants.

// ============================================================================
// 1. Spansion S29JL064H NOR Flash (8 MB / 64 Mbit, TSOP-48) Memory Layout
// ============================================================================

/// Total raw size of the Spansion S29JL064H NOR Flash (8 Megabytes / 64 Mbits).
pub const SPANSION_FLASH_SIZE_BYTES: usize = 8 * 1024 * 1024; // 8,388,608 bytes

/// Boundary separating the OS / Bootloader payload from the top NVRAM sector.
/// The Bootloader and system software reside in 0x0000_0000 .. 0x007E_FFFF (8,323,072 bytes).
pub const SPANSION_BOOTLOADER_SIZE_BYTES: usize = 0x007F_0000; // 8,323,072 bytes

/// Base memory offset where the top NVRAM/EEPROM emulation sector begins in physical flash.
pub const SPANSION_NVRAM_BASE_OFFSET: usize = 0x007F_0000;

/// Total size of the NVRAM parameter storage area (top 64 Kilobytes).
pub const SPANSION_NVRAM_SIZE_BYTES: usize = 64 * 1024; // 65,536 bytes

/// Total size of the NVRAM parameter storage area for 16 MB NOR Flash models (top 128 Kilobytes).
pub const FLASH_16MB_NVRAM_SIZE_BYTES: usize = 128 * 1024; // 131,072 bytes

/// Total count of 8 KB micro-sectors in the 16 MB NOR Flash top NVRAM region (16 microsectors).
pub const FLASH_16MB_MICROSECTOR_COUNT: usize = 16;

/// Base memory offset where the 128 KB NVRAM begins in a 16 MB NOR Flash image.
pub const FLASH_16MB_NVRAM_BASE_OFFSET: usize = 0x00FE_0000;

/// End memory offset of the NVRAM area (must equal `SPANSION_FLASH_SIZE_BYTES`).
pub const SPANSION_NVRAM_END_OFFSET: usize = SPANSION_NVRAM_BASE_OFFSET + SPANSION_NVRAM_SIZE_BYTES;

/// Size of each individual hardware micro-sector within the top 64 KB bank.
/// In the Spansion top-boot architecture (Model S29JL064H), the top 64 KB sector
/// is subdivided into 8 protected microsectors of 8 KB each.
pub const SPANSION_MICROSECTOR_SIZE: usize = 8 * 1024; // 8,192 bytes

/// Total count of 8 KB micro-sectors in the top 64 KB sector.
pub const SPANSION_MICROSECTOR_COUNT: usize = 8;

/// Base sector index according to Spansion S29JL064H datasheet nomenclature.
/// Sectors SA134 through SA141 correspond to offsets 0x007F_0000 through 0x007F_FFFF.
pub const SPANSION_MICROSECTOR_BASE_INDEX: usize = 134;

/// Erased state byte for NOR flash memory cells (all bits high).
pub const FLASH_ERASED_BYTE: u8 = 0xFF;

// ============================================================================
// 2. Hard Disk Drive (HDD) Geometry, MEIHDFS Filesystem & Partition Signatures
// ============================================================================

/// Standard ATA/IDE sector size in bytes (LBA addressing unit).
pub const LBA_SECTOR_SIZE_BYTES: usize = 512;

/// Panasonic proprietary MEIHDFS MBR signature prefix (8 bytes).
/// Bytes 0x00-0x03: 0x00000000 (Reserved)
/// Bytes 0x04-0x05: 0x0100 (Format Revision 1.0, little-endian)
/// Bytes 0x06-0x07: 0x53, 0x48 ("SH" in ASCII -> System Storage Header)
pub const PANASONIC_HDD_MBR_SIGNATURE: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x53, 0x48];

/// Full 16-byte header seen on DMR-EH55 LBA 0 (including Disk Volume GUID).
pub const PANASONIC_HDD_MBR_FULL_SIGNATURE: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x53, 0x48, 0x62, 0x8C, 0x6E, 0xF5, 0x1B, 0x2C, 0x36, 0x66,
];

/// Standard IBM PC MBR boot signature (0x55AA at offsets 510..512).
/// Panasonic MEIHDFS raw drives deliberately lack this signature to prevent PC OS corruption.
pub const STANDARD_MBR_BOOT_SIGNATURE: [u8; 2] = [0x55, 0xAA];

/// Proprietary Panasonic Partition Header Magic (8 bytes).
/// Marks the starting boundary of embedded firmware images and system containers.
pub const PANASONIC_HDD_PARTITION_MAGIC: [u8; 8] = [0x51, 0x87, 0x13, 0x8D, 0x5B, 0x0A, 0xFB, 0x22];

// ============================================================================
// 3. Disk Layout Offsets, Mirror Tables & Partition Dimensions
// ============================================================================

/// Offset of Allocation Descriptors table (LBA 24 to 31, 8 sectors = 4096 bytes).
pub const ALLOCATION_DESCRIPTORS_OFFSET: u64 = 0x0000_3000;
pub const ALLOCATION_DESCRIPTORS_SIZE: usize = 4096;

/// Mirror offset of Allocation Descriptors (LBA 57368 to 57375).
pub const ALLOCATION_DESCRIPTORS_MIRROR_OFFSET: u64 = 0x01C0_3000;

/// Offset of the Transactional Journal Catalog (LBA 16384 / 8.00 MB).
pub const LBA_JOURNAL_CATALOG_OFFSET: u64 = 0x0080_0000;

/// Start offset of the Journal region (0x0082_0000 to 0x0096_0000 = 1,310,720 bytes).
pub const JOURNAL_AREA_OFFSET: u64 = 0x0082_0000;
pub const JOURNAL_AREA_END_OFFSET: u64 = 0x0096_0000;
pub const JOURNAL_AREA_SIZE: usize = (JOURNAL_AREA_END_OFFSET - JOURNAL_AREA_OFFSET) as usize; // 1,310,720 bytes
pub const JOURNAL_BLOCK_SIZE: usize = 128 * 1024; // 131,072 bytes (256 sectors)
pub const JOURNAL_BLOCK_COUNT: usize = 10;

/// State transition marker / boundary sector at 24.00 MB (LBA 49152).
pub const LBA_STATE_TRANSITION_MARKER_OFFSET: u64 = 0x0180_0000;

/// Mirror MBR backup table offset (28.00 MB, LBA 57344).
/// Sector 57344 contains a bit-for-bit identical replica of Sector 0.
pub const LBA_MIRROR_MBR_OFFSET: u64 = 0x01C0_0000;
pub const LBA_MIRROR_MBR_SECTOR: u64 = 57344;

/// Partition 1: Main System Firmware, Codecs & AM34 OS Container (32.00 MB).
pub const LBA_PARTITION_1_OFFSET: u64 = 0x0200_0000;
pub const LBA_PARTITION_1_SECTOR: u64 = 65536;

/// Exact payload size of Partition 1 (6,173,264 bytes / ~5.887 MB).
pub const LBA_PARTITION_1_PAYLOAD_SIZE: usize = 0x005E_3250;

/// Sector-aligned size of Partition 1 (6,173,696 bytes / 12,058 sectors of 512 bytes).
pub const LBA_PARTITION_1_ALIGNED_SIZE: usize = 0x005E_3400;

/// Partition 2: Secondary Firmware, Microcode, Sub-system & OSD Fonts (64.00 MB).
pub const LBA_PARTITION_2_OFFSET: u64 = 0x0400_0000;
pub const LBA_PARTITION_2_SECTOR: u64 = 131072;

/// Exact payload size of Partition 2 (720,896 bytes / 704 KB / 1,408 sectors).
pub const LBA_PARTITION_2_PAYLOAD_SIZE: usize = 0x000B_0000;

/// Pre-padding size of Partition 2 before trailing 48 padding bytes (720,848 bytes).
pub const LBA_PARTITION_2_PRE_PADDING_SIZE: usize = 0x000A_FFD0;

// ============================================================================
// 4. MEIHDFS Low-Level Allocation Descriptors & Record Tags (XOR 0xFF Decoded)
// ============================================================================

/// Size of each structured allocation record in bytes (16 bytes).
pub const MEIHDFS_RECORD_SIZE: usize = 16;

/// Total number of 16-byte records in the 4096-byte Allocation Descriptors table (256 records).
pub const MEIHDFS_DESCRIPTOR_RECORDS_COUNT: usize = 256;

/// MEIHDFS Allocation Record Tag 0x5D: Extent Descriptor Header / Node Header.
/// Raw byte on disk: 0xA2 (0xA2 ^ 0xFF = 0x5D).
pub const MEIHDFS_TAG_EXTENT_HEADER: u8 = 0x5D;

/// MEIHDFS Allocation Record Tag 0x55: Extent Segment Pointer / Allocation Map Entry.
/// Raw byte on disk: 0xAA (0xAA ^ 0xFF = 0x55).
pub const MEIHDFS_TAG_EXTENT_POINTER: u8 = 0x55;

/// MEIHDFS Allocation Record Tag 0x57: Extent Continuation Pointer / Indirect Block.
/// Raw byte on disk: 0xA8 (0xA8 ^ 0xFF = 0x57).
pub const MEIHDFS_TAG_EXTENT_CONTINUATION: u8 = 0x57;

/// MEIHDFS Allocation Record Tag 0x5F: Extent Pool Boundary / Volume Limiter.
/// Raw byte on disk: 0xA0 (0xA0 ^ 0xFF = 0x5F).
pub const MEIHDFS_TAG_EXTENT_BOUNDARY: u8 = 0x5F;

/// MEIHDFS Allocation Record Tag 0xE0: Virtual Node Table Entry / Inode Attributes.
/// Raw byte on disk: 0x1F (0x1F ^ 0xFF = 0xE0).
pub const MEIHDFS_TAG_VNODE_ENTRY: u8 = 0xE0;

/// MEIHDFS Allocation Record Tag 0xE1: Virtual Inode Directory Link / Hierarchical Index.
/// Raw byte on disk: 0x1E (0x1E ^ 0xFF = 0xE1).
pub const MEIHDFS_TAG_VINODE_LINK: u8 = 0xE1;

/// MEIHDFS Allocation Record Tag 0xFF: Unallocated / Free record slot.
/// Raw byte on disk: 0x00 (0x00 ^ 0xFF = 0xFF).
pub const MEIHDFS_TAG_FREE: u8 = 0xFF;

/// Relative offset within each 128 KB Journal block where the transaction commit trailer begins (0x12C0C).
pub const JOURNAL_TRANSACTION_TRAILER_OFFSET: usize = 0x12C0C;

/// Relative offset of the 32-bit transaction sequence ID in each 128 KB Journal block (0x12C14).
pub const JOURNAL_TRANSACTION_SEQ_OFFSET: usize = 0x12C14;

/// Total count of 16-byte records in the 4096-byte allocation table (LBAs 24..31).
pub const MEIHDFS_RECORD_COUNT: usize = ALLOCATION_DESCRIPTORS_SIZE / MEIHDFS_RECORD_SIZE; // 256 records

pub const MEIHDFS_TAG_EXTENT_5D: u8 = MEIHDFS_TAG_EXTENT_HEADER;
pub const MEIHDFS_TAG_SYNC_55: u8 = MEIHDFS_TAG_EXTENT_POINTER;
pub const MEIHDFS_TAG_POINTER_57: u8 = MEIHDFS_TAG_EXTENT_CONTINUATION;
pub const MEIHDFS_TAG_DESCRIPTOR_5F: u8 = MEIHDFS_TAG_EXTENT_BOUNDARY;
pub const MEIHDFS_TAG_DIR_ENTRY_E0: u8 = MEIHDFS_TAG_VNODE_ENTRY;
pub const MEIHDFS_TAG_VIRTUAL_NODE_E1: u8 = MEIHDFS_TAG_VINODE_LINK;
