//! Stage 1 `/BOOT.BIN` (`0x44000000`) FAT12/FAT16, SDCD, and Feistel S-Box symbols.

use super::super::{
    BOOT_ACMD6_SWITCH_VA, BOOT_CONSOLE_GETLINE_VA, BOOT_FEISTEL_SBOX_VA, BOOT_GET_SD_STATUS_VA,
    BOOT_SECOND_SD_STATUS_VA, BOOT_UART0_GETC_VA, BOOT_UNCOMPRESS_VA, SymbolSpec,
};
use super::sym;
use crate::constants::AM34_BOOT_BIN_BASE;

pub(super) const SPECS: &[SymbolSpec] = &[
    sym(
        AM34_BOOT_BIN_BASE,
        "boot_ipl_base",
        "Stage 1 /BOOT.BIN SDRAM Execution Base",
    ),
    sym(
        BOOT_UART0_GETC_VA,
        "boot_uart0_getc_dead",
        "Dead UART0 polling helper (0 active callers in production IPL)",
    ),
    sym(
        0x4400_4808,
        "boot_uart0_printf",
        "/BOOT.BIN — UART0 formatted printf dispatcher (used by boot_fmu_debug_printf)",
    ),
    sym(
        BOOT_CONSOLE_GETLINE_VA,
        "boot_console_getline_dead",
        "Dead BootROM console reader (0 active callers)",
    ),
    sym(
        0x4400_98FC,
        "boot_media_read_sectors",
        "/BOOT.BIN — Media sector read dispatcher (media_type=3 routes to SD block reader 0x440262cb)",
    ),
    sym(
        0x4400_B3D9,
        "boot_fmu_debug_printf",
        "/BOOT.BIN — Unconditional varargs wrapper around boot_uart0_printf (0x44004808)",
    ),
    sym(
        BOOT_UNCOMPRESS_VA,
        "boot_uncompress_fe",
        "Decompresses F/E DRV image into 0x90202000 & checks Adler/sum",
    ),
    sym(
        0x4400_C0A8,
        "boot_fat_validate_vbr_bpb",
        "/BOOT.BIN — Validates VBR 0x55AA, BytesPerSector==512, SectorsPerCluster!=0, NumFATs in 1..=2, and FAT12/FAT16/FAT32 signature",
    ),
    sym(
        0x4400_C696,
        "boot_fat_mount",
        "/BOOT.BIN — Mounts FAT12/FAT16 volume at 0x5056B640 (rejects FAT32 with -10025 / 0xFFFFD8D7)",
    ),
    sym(
        0x4400_C879,
        "boot_fat_open",
        "/BOOT.BIN — Opens 8.3 file on mounted FAT volume (returns handle 200 or -10010/-10011/-10020)",
    ),
    sym(
        0x4400_C922,
        "boot_fat_read",
        "/BOOT.BIN — Reads bytes from open FAT handle (must return 0x20000 in boot_sd_load_digadiag_frm)",
    ),
    sym(
        0x4400_C94D,
        "boot_fat_seek",
        "/BOOT.BIN — Seeks open FAT handle (whence 1=SET, 2=CUR, 3=END; returns -10020 on invalid offset)",
    ),
    sym(
        0x4400_CA59,
        "boot_fat_sd_bounce_read",
        "/BOOT.BIN — Reads up to 0x1FC SD sectors via bounce buffer 0x01000000 / uncached alias 0x91C00000",
    ),
    sym(
        0x4400_CB21,
        "boot_fat_parse_mbr_vbr",
        "/BOOT.BIN — Reads LBA 0, scans 4 MBR entries (+0x1BE) or falls back to Superfloppy LBA 0 (+6 backup VBR)",
    ),
    sym(
        0x4400_CED2,
        "boot_fat_cache_init",
        "/BOOT.BIN — Initializes FAT sector cache at 0x505724C0 (reads first 3 sectors of FAT1 or FAT2)",
    ),
    sym(
        0x4400_CFDE,
        "boot_fat_parse_8_3",
        "/BOOT.BIN — Splits 8.3 filename into 8-byte stem and 3-byte ext (returns -10020 if >12 chars or stem >8)",
    ),
    sym(
        0x4400_D16B,
        "boot_fat_open_entry",
        "/BOOT.BIN — Scans FAT12/FAT16 root directory for 8.3 entry (returns -10010 / 0xFFFFD8E6 if not found)",
    ),
    sym(
        0x4400_DFC8,
        "boot_fat_read_clusters",
        "/BOOT.BIN — Reads file clusters (clamps to file_size; returns -10025 on I/O err, -10028 on bad cluster, -10030 on EOF)",
    ),
    sym(
        0x4400_E737,
        "boot_fat_next_cluster",
        "/BOOT.BIN — Resolves next FAT12/FAT16 cluster from 3-sector FAT window at 0x505724C8",
    ),
    sym(
        0x4402_62CB,
        "boot_sd_read_blocks",
        "/BOOT.BIN — Low-level SD card block reader (invoked by boot_media_read_sectors for media_type=3)",
    ),
    sym(
        BOOT_GET_SD_STATUS_VA,
        "boot_get_sd_status_1bit",
        "1st GetSD_Status call in 1-bit DAT0 mode (passes on SDSC)",
    ),
    sym(
        BOOT_ACMD6_SWITCH_VA,
        "boot_acmd6_4bit_switch",
        "Sends ACMD6(2) and clears 0x89001850 bit 15 to enable 4-bit SD",
    ),
    sym(
        BOOT_SECOND_SD_STATUS_VA,
        "boot_get_sd_status_4bit",
        "2nd GetSD_Status call in 4-bit DAT[3:0] mode (fails -91006 on SDHC/DAT1..3 fault)",
    ),
    sym(
        BOOT_FEISTEL_SBOX_VA,
        "boot_feistel_sbox_256b",
        "Verified 256-byte Feistel S-Box substitution table in BOOT.BIN",
    ),
];
