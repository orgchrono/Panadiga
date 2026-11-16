//! Panasonic Feistel cipher keys, ECB signatures, FRM container TOC, and SD/DIGADIAG constants.

/// Native block size of the symmetric cipher applied to Panasonic firmware (64 bits).
pub const CIPHER_BLOCK_SIZE: usize = 8;

/// ECB Ciphertext Leak Pattern: 8 zero-bytes (`00 00 00 00 00 00 00 00`).
/// Appears in padding regions of encrypted partitions and in PANA_DVD.FRM (196 occurrences).
pub const ECB_NULL_BLOCK_8B: [u8; 8] = [0xE5, 0xD1, 0xF3, 0x4A, 0x51, 0x8B, 0x26, 0x90];

/// ECB Ciphertext Leak Pattern: 8 virgin flash bytes (`FF FF FF FF FF FF FF FF`).
/// Appears 3,812 times in PANA_DVD.FRM (over 30 KB of unprogrammed flash padding).
pub const ECB_VIRGIN_FLASH_8B: [u8; 8] = [0x03, 0x50, 0x65, 0xC8, 0x7B, 0xB8, 0xE0, 0x90];

/// ECB Ciphertext Leak Pattern: Common partition header repeated structure.
/// Appears 1,280 times in PANA_DVD.FRM.
pub const ECB_COMMON_STRUCT_8B: [u8; 8] = [0x4D, 0x3C, 0x5D, 0xBB, 0x0C, 0x35, 0x77, 0x77];

/// Size of each Table of Contents (TOC) entry descriptor in PANA_DVD.FRM (48 bytes).
pub const PANA_FRM_TOC_ENTRY_SIZE: usize = 0x30; // 48 bytes

/// Repeated 8-byte sub-block at TOC offsets 0x0010, 0x0040, 0x0070.
pub const PANA_FRM_TOC_SUBBLOCK_1: [u8; 8] = [0xEF, 0xDC, 0x1C, 0xDA, 0x13, 0x88, 0x1B, 0xAB];

/// Repeated 8-byte sub-block at TOC offsets 0x0018 and 0x0078.
pub const PANA_FRM_TOC_SUBBLOCK_2: [u8; 8] = [0x74, 0xA7, 0x8A, 0xCD, 0x78, 0x9B, 0x84, 0xFF];

/// Global 16-byte container header for PANA_DVD.FRM.
pub const PANA_FRM_GLOBAL_HEADER: [u8; 16] = [
    0x49, 0x2F, 0x05, 0x60, 0xCE, 0xDE, 0x50, 0x8C, 0xE3, 0xC0, 0x8A, 0xFD, 0x49, 0xB6, 0x4A, 0x9A,
];

/// Global container header size in PANA_DVD.FRM (16 bytes = 0x10).
pub const PANA_FRM_GLOBAL_HEADER_SIZE: usize = 0x10;

/// Total number of Table of Contents (TOC) entries in PANA_DVD.FRM.
pub const PANA_FRM_TOC_ENTRY_COUNT: usize = 3;

/// Total size of the Table of Contents (3 entries * 48 bytes = 144 bytes / 0x90).
pub const PANA_FRM_TOC_TOTAL_SIZE: usize = PANA_FRM_TOC_ENTRY_COUNT * PANA_FRM_TOC_ENTRY_SIZE;

/// Target descriptor for Optical Drive Servo MN102/DSP at TOC Entry 1 (offset 0x0048).
pub const PANA_FRM_TOC_DRIVE_TARGET_8B: [u8; 8] = [0x3B, 0x7E, 0x7E, 0xDB, 0xB6, 0xC5, 0x7A, 0x53];

/// Target descriptor for AM34 / MN103x RISC at TOC Entries 0 and 2 (offsets 0x0018, 0x0078).
pub const PANA_FRM_TOC_AM34_TARGET_8B: [u8; 8] = [0x74, 0xA7, 0x8A, 0xCD, 0x78, 0x9B, 0x84, 0xFF];

/// Offset where the encrypted payload begins in PANA_DVD.FRM (0x00A0).
pub const PANA_FRM_PAYLOAD_OFFSET: usize = 0x00A0;

/// Feistel cipher key part 1 from `BOOT.BIN` `.data` (`0x50480278`, file offset `0x39AA8`).
pub const PANA_FEISTEL_KEY_PART1: [u8; 7] = [0x72, 0xEB, 0xCA, 0x91, 0x30, 0x45, 0x68];

/// Feistel cipher key part 2 from `BOOT.BIN` `.data` (`0x5048027F`, file offset `0x39AAF`).
pub const PANA_FEISTEL_KEY_PART2: [u8; 8] = [0xAD, 0x99, 0xDF, 0x56, 0x72, 0x3A, 0xB8, 0x10];

/// Reconstructed 7-byte master key (`ab[i] = p2[i] ^ p1[i]`, with `p2[7]` XORed into indices 1 and 4).
pub const PANA_FEISTEL_MASTER_KEY_7B: [u8; 7] = [0xDF, 0x62, 0x15, 0xC7, 0x52, 0x7F, 0xD0];

/// 4-byte IV used by `FUN_44011d42` to expand the 7-byte master key into the 32-byte round key.
pub const PANA_FEISTEL_KEYSCHED_IV: [u8; 4] = [0x35, 0x7D, 0xFB, 0x9A];

/// Pre-expanded 32-byte Feistel round key (`FUN_44011d42`) for EH55/EH58 `.FRM` and HDD partitions.
pub const PANA_FEISTEL_ROUND_KEY_32B: [u8; 32] = [
    0xA3, 0xE7, 0x9C, 0x01, 0xB2, 0x5A, 0x54, 0x74, 0x59, 0x1D, 0xB2, 0xEF, 0x67, 0x26, 0x6D, 0xFF,
    0x63, 0x03, 0x87, 0x07, 0xB6, 0x56, 0x94, 0x0F, 0x02, 0x6D, 0xA5, 0x61, 0x25, 0x51, 0xF3, 0x15,
];

/// Official 256-byte substitution table from `BOOT.BIN` (`0x44035D84`) / `PROG061_BOOT` (`0x50435C0A`).
pub const PANA_FEISTEL_SBOX_256: [u8; 256] = [
    0x00, 0x01, 0xBB, 0x9C, 0x3F, 0x5E, 0xC7, 0x3E, 0x36, 0xA4, 0x92, 0x93, 0x38, 0x9B, 0x8D, 0x1A,
    0x3C, 0x84, 0xF7, 0x2A, 0xCB, 0x3D, 0x70, 0xAD, 0x30, 0xA2, 0xC1, 0x95, 0x03, 0x39, 0xA5, 0x0E,
    0xE6, 0xFB, 0x09, 0x1E, 0xB9, 0xC4, 0x24, 0x57, 0x5F, 0xC3, 0x5D, 0xFA, 0x60, 0x56, 0xFD, 0xED,
    0x81, 0xF3, 0x4A, 0xFF, 0x85, 0xDB, 0xCD, 0xA0, 0xD0, 0x9D, 0x8B, 0x26, 0x4C, 0x50, 0x0C, 0x1D,
    0x7D, 0x77, 0xDE, 0x7A, 0x0A, 0x0B, 0x73, 0xE1, 0x54, 0x76, 0xE8, 0xC2, 0x28, 0x05, 0x98, 0x99,
    0x29, 0x4B, 0xEE, 0x87, 0x42, 0x59, 0x65, 0xB8, 0x1F, 0xDF, 0x23, 0x58, 0x63, 0x69, 0x1C, 0x79,
    0x64, 0xC5, 0x6F, 0x13, 0x3B, 0xCE, 0x08, 0xA7, 0xB2, 0x12, 0x20, 0x7F, 0xE2, 0x89, 0x21, 0x2B,
    0x41, 0x49, 0x7C, 0x82, 0xBE, 0xF2, 0x43, 0x8E, 0x86, 0x02, 0x9E, 0xD8, 0x67, 0x8F, 0xA3, 0xE7,
    0xBC, 0xF8, 0x66, 0x22, 0x4D, 0xE5, 0xBA, 0xD2, 0xDA, 0xEA, 0x61, 0x25, 0xB0, 0xC0, 0x7B, 0xA6,
    0x48, 0x96, 0xDD, 0xDC, 0x71, 0x90, 0x55, 0x5B, 0x4F, 0x4E, 0x6D, 0xD1, 0x11, 0x34, 0xAA, 0x1B,
    0xF4, 0x17, 0x80, 0xB7, 0xCC, 0x8C, 0xD9, 0x37, 0x8A, 0x2C, 0x94, 0x16, 0x72, 0x40, 0xEF, 0x9A,
    0xC8, 0x47, 0xF6, 0xFC, 0x2E, 0xB3, 0x2F, 0xBD, 0xCF, 0xA9, 0x15, 0x10, 0x18, 0xEC, 0x6A, 0x6E,
    0xC9, 0x91, 0x53, 0x78, 0xA8, 0x3A, 0xAF, 0x0D, 0xE0, 0x97, 0x32, 0x51, 0xB1, 0xF5, 0x27, 0x2D,
    0x35, 0x88, 0x14, 0xE3, 0xFE, 0x06, 0xD7, 0xD6, 0xAB, 0xA1, 0xD5, 0xE4, 0x45, 0x44, 0x9F, 0x0F,
    0x5A, 0x83, 0xEB, 0xCA, 0x07, 0x04, 0xB4, 0xB6, 0x52, 0xAE, 0xD4, 0x33, 0x31, 0x75, 0xD3, 0x6C,
    0x62, 0xBF, 0x6B, 0xF9, 0xAC, 0x46, 0xF0, 0x5C, 0x19, 0x7E, 0x68, 0xB5, 0xF1, 0x74, 0xC6, 0xE9,
];

/// Size of the `"Firm list"` TOC header region at the beginning of `PANA_DVD.FRM` (`0x1800` / 6,144 B).
pub const PANA_FRM_TOC_REGION_SIZE: usize = 0x1800;

/// Exact offset and size of the `DRV S211` module inside decrypted `PANA_DVD.FRM`.
pub const PANA_FRM_DRV_S211_OFFSET: usize = 0x0000_1800;
pub const PANA_FRM_DRV_S211_SIZE: usize = 0x000B_0000;

/// Exact offset and size of the `PROG061 ` bootloader module inside decrypted `PANA_DVD.FRM`.
pub const PANA_FRM_PROG061_OFFSET: usize = 0x000B_1800;
pub const PANA_FRM_PROG061_SIZE: usize = 0x0004_0000;

/// Exact offset and size of the `MAIN3060NP` system firmware module inside decrypted `PANA_DVD.FRM`.
pub const PANA_FRM_MAIN3060_OFFSET: usize = 0x000F_1800;
pub const PANA_FRM_MAIN3060_SIZE: usize = 0x0063_D4E0;

/// Encrypted prefix/suffix span applied to `MAIN` `EXTRHEAD`/`EXTRFOOT` containers (`0x19000` / 102,400 B).
pub const PANA_EXTRHEAD_MAIN_CRYPT_SPAN: usize = 0x0001_9000;

/// Encrypted prefix span applied to `DRV` `EXTRHEAD` containers (`0x2800` / 10,240 B).
pub const PANA_EXTRHEAD_DRV_CRYPT_SPAN: usize = 0x0000_2800;

/// Default sampling stride for Panasonic's strided Adler-32 routine (`FUN_4401e5a3`).
pub const PANA_EXTRHEAD_DEFAULT_ADLER_STEP: usize = 16;

/// Size of each 64-byte plaintext TOC entry inside `PANA__SD.FRM` (`FUN_4400a733` / `FUN_4400a6d0`).
pub const PANA_SD_TOC_ENTRY_SIZE: usize = 0x40;

/// Maximum number of module slots in `PANA__SD.FRM` (`128 x 0x40 = 0x2000`).
pub const PANA_SD_TOC_MAX_ENTRIES: usize = 128;

/// Total size of the plaintext `PANA__SD.FRM` TOC table in bytes (`0x2000` / 8,192 B).
pub const PANA_SD_TOC_REGION_SIZE: usize = PANA_SD_TOC_ENTRY_SIZE * PANA_SD_TOC_MAX_ENTRIES;

/// Fixed total size of a padded `DIGADIAG.FRM` diagnostic container (`0x20000` / 131,072 B).
pub const PANA_DIGADIAG_FRM_SIZE: usize = 0x0002_0000;

/// Default MN103S RAM jump target executed by `BOOT.BIN` `FUN_44006cd9` after unpacking `DIGADIAG.FRM`.
pub const PANA_DIGADIAG_DEFAULT_LOAD_ADDR: u32 = 0x5040_0000;

/// Module 1 (Sub-system / OSD) start offset in PANA_DVD.FRM (0x00A0).
pub const PANA_FRM_MOD1_START_OFFSET: usize = 0x00A0;
/// Module 1 active payload end offset (0x0AE480).
pub const PANA_FRM_MOD1_PAYLOAD_END: usize = 0x0AE480;
/// Module 1 virgin flash padding end offset (0x0AEFD0).
pub const PANA_FRM_MOD1_VIRGIN_PAD_END: usize = 0x0AEFD0;
/// Module 1 fixed partition structure end offset (0x0B17D0).
pub const PANA_FRM_MOD1_STRUCT_END: usize = 0x0B17D0;
/// Module 1 boundary offset (0x0B1800).
pub const PANA_FRM_MOD1_BOUNDARY: usize = 0x0B1800;

/// Module 2 (Drive Servo DSP) start offset in PANA_DVD.FRM (0x0B1800).
pub const PANA_FRM_MOD2_START_OFFSET: usize = 0x0B1800;
/// Module 2 active payload end offset (0x0EAC98).
pub const PANA_FRM_MOD2_PAYLOAD_END: usize = 0x0EAC98;
/// Module 2 virgin flash padding end offset (0x0F17F8).
pub const PANA_FRM_MOD2_VIRGIN_PAD_END: usize = 0x0F17F8;
/// Module 2 boundary offset (0x0F1818).
pub const PANA_FRM_MOD2_BOUNDARY: usize = 0x0F1818;

/// Module 3 (Main CPU OS AM34 / DIGA OS) start offset in PANA_DVD.FRM (0x0F1818).
pub const PANA_FRM_MOD3_START_OFFSET: usize = 0x0F1818;

/// Standard `EXTRHEAD` header (`0x40` = 64 B) and `EXTRFOOT` footer (`8` B) sizes.
pub const PANA_EXTRHEAD_HEADER_SIZE: usize = 0x40;
pub const PANA_EXTRHEAD_FOOTER_SIZE: usize = 8;

/// Panasonic Okumura LZSS (`algo == 2`) sliding dictionary & hash chain constants.
pub const PANA_LZSS_WINDOW_SIZE: usize = 0x1000;
pub const PANA_LZSS_WINDOW_INIT: usize = 0x0FEE;
pub const PANA_LZSS_MAX_UNCODED: usize = 3;
pub const PANA_LZSS_MAX_CODED: usize = PANA_LZSS_MAX_UNCODED + 16;
pub const PANA_LZSS_HASH_BITS: usize = 13;
pub const PANA_LZSS_HASH_SIZE: usize = 1 << PANA_LZSS_HASH_BITS;
pub const PANA_LZSS_KNUTH_HASH_MULT: u32 = 2_654_435_761;

/// `MainList` multi-segment header, entry stride, target chunk sizes, and overlap crypt spans.
pub const PANA_MAIN_LIST_HEADER_SIZE: usize = 20;
pub const PANA_MAIN_LIST_ENTRY_SIZE: usize = 8;
pub const PANA_MAIN_LIST_LEGACY_SEG_TARGET: u32 = 32 * 1024 * 1024;
pub const PANA_MAIN_LIST_MODERN_SEG_TARGET: u32 = 16 * 1024 * 1024;
pub const PANA_MAIN_LIST_MODERN_CRYPT_SPAN: usize = 0x1400;

/// Optical drive (`DRV`) `0x20`-byte header size and unencrypted 48-byte trailer length.
pub const PANA_DRV_HEADER_SIZE: usize = 0x20;
pub const PANA_DRV_TAIL_CLEAR_BYTES: usize = 48;

/// Panasonic 512-byte `DEV.BIN` service container descriptor geometry and signatures.
pub const PANA_DEV_BIN_SIZE: usize = 512;
pub const PANA_DEV_BIN_MAGIC: [u8; 4] = [0xAA, 0x55, 0xFF, 0xFF];
pub const PANA_DEV_BIN_SIG: &[u8; 12] = b"PANA_DVD_REC";

/// `libdownloadlib.so` network stream decipher IV & feedback step (`pana-fw/crypto/decipher.go`).
pub const PANA_LIBDOWNLOADLIB_CIPHER_IV: u32 = 904;
pub const PANA_LIBDOWNLOADLIB_CIPHER_STEP: u32 = 38_400 + 163;

/// Embedded filesystem container signatures extracted from demuxed `MAIN` partitions.
pub const PANA_CRAMFS_MAGIC: u32 = 0x28CD_3D45;
pub const PANA_CRAMFS_SIGNATURE: &[u8; 16] = b"Compressed ROMFS";
pub const PANA_CRAMFS_FLAG_FSID_VERSION_2: u32 = 0x0000_0001;
pub const PANA_ROMFS_MAGIC: &[u8; 8] = b"-rom1fs-";

/// Default `src/hdd_cmd.c` M7920 fallback limit in 2 KB blocks (~199.91 GB).
pub const PANA_HDD_CMD_M7920_FALLBACK_BLOCKS: u32 = 0x05D0_DDBA;
