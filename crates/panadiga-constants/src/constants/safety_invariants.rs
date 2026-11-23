#![deny(unsafe_code)]

//! Centralized physical, storage, GPT, NOR/BGO geometry, and SD audit safety constants.
//!
//! Eliminates scattered magic numbers and hardcoded check/status strings across
//! `panadiga-core` and CLI/UI layers.

/// Maximum 512-byte sector count addressable by 28-bit LBA (`2^28 - 1` sectors = 128 GiB / 137.4 GB).
pub const LBA28_MAX_SECTORS_512B: u64 = 0x0FFF_FFFF;

/// Maximum 2 KiB block count addressable by 28-bit LBA in Panasonic `MAIN` `MaxProductLBA` (`0x03FF_FFFF`).
///
/// Capacities above this threshold (including `250 GB` = `0x0746_6528` and `400 GB` = `0x0BA3_FB74`)
/// MUST use a verified LBA48 `MAIN`/`DRV` firmware path to prevent 28-bit LBA wrap-around.
pub const LBA28_MAX_BLOCKS_2KB: u32 = 0x03FF_FFFF;

/// Maximum capacity of standard SDSC FAT16 cards required by legacy Panasonic DIGA bootloaders (`2 GiB`).
pub const SDSC_MAX_CAPACITY_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Canonical lowercase GPT Partition Type GUID for EFI System Partition (ESP).
pub const EFI_SYSTEM_PARTITION_GPT_GUID: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";

/// Canonical lowercase GPT Partition Type GUID for Microsoft Reserved Partition (MSR).
pub const MICROSOFT_RESERVED_GPT_GUID: &str = "e3c9e316-0b5c-4db8-817d-f92df00215ae";

/// Uniform main sector size in Matsushita/Spansion BGO FTL partitions (`64 KiB`).
pub const BGO_SECTOR_SIZE_64K: u32 = 0x0001_0000;

/// Address offset mask converting AM34 CS1 NOR virtual addresses (`0x84xx_xxxx`) into physical NOR offsets.
pub const AM34_CS1_NOR_OFFSET_MASK: u32 = 0x00FF_FFFF;

/// Minimum contiguous block stride (`64 KiB`) checked for floating-address-pin mirroring in NOR dumps.
pub const NOR_MIRROR_MIN_BLOCK_BYTES: usize = 0x0001_0000;

/// Minimum valid Panasonic DIGA NOR flash size (`2 MiB`) accepted by dump integrity analyzers.
pub const NOR_MIN_VALID_DUMP_BYTES: usize = 0x0020_0000;

/// 32-bit word indices inside the 64-word `PROG061` BGO descriptor table.
pub const BGO_TABLE_WORD_IDX_FLASH_BASE: usize = 12;
pub const BGO_TABLE_WORD_IDX_FLASH_END: usize = 13;
pub const BGO_TABLE_WORD_IDX_TOTAL_SECTORS: usize = 31;

/// Canonical SD unbrick verification overall status strings.
pub const SD_STATUS_READY: &str = "READY";
pub const SD_STATUS_READY_WITH_WARNINGS: &str = "READY_WITH_HW_WARNINGS";
pub const SD_STATUS_INVALID: &str = "INCOMPLETE_OR_INVALID";

/// Canonical SD unbrick verification check IDs.
pub const CHK_ID_BGO_GEOMETRY_MISMATCH: &str = "bgo_geometry_mismatch";
pub const CHK_ID_BGO_ALLOWLIST_INVARIANT: &str = "bgo_allowlist_invariant";
pub const CHK_ID_FVU_PANAMON_TRAP: &str = "fvu_panamon_trap";
pub const CHK_ID_DIGADIAG_BYPASS_TRAP: &str = "digadiag_bypass_trap";
pub const CHK_ID_RTSD_TARGET_CONSISTENCY: &str = "rtsd_target_consistency";
pub const CHK_ID_RTSD_LBA48_GATE: &str = "rtsd_lba48_gate";
pub const CHK_ID_MAIN_EXTRHEAD_CHECKSUM: &str = "main_extrhead_checksum";
pub const CHK_ID_MAIN_EXTRHEAD_SRC_ADDR: &str = "main_extrhead_src_addr";
pub const CHK_ID_SATA_BRIDGE_RESET_FIX: &str = "sata_bridge_reset_fix";
pub const CHK_ID_SATA_BRIDGE_STAGE2_BOOT: &str = "sata_bridge_stage2_boot";
pub const CHK_ID_MAIN_BGO_GEOMETRY: &str = "main_bgo_geometry";
