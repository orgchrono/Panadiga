# Ghidra Script: Panasonic DMR-EH55 Symbol & Peripheral Importer
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)

from __future__ import print_function

try:
    from ghidra.program.model.listing import CodeUnit
    from ghidra.program.model.symbol import SourceType
except ImportError:
    CodeUnit = None
    SourceType = None

def get_panadiga_symbol_specs():
    return [
        (0x80000000, "_start_reset_vector", "AM34 Reset Vector - Cold Boot Entrypoint", True),
        (0x80000004, "vector_nmi", "Non-Maskable Interrupt Handler", True),
        (0x80000008, "vector_trap", "Software Trap Exception Handler", True),
        (0x8000000C, "vector_illegal_instr", "Illegal Instruction Exception Handler", True),
        (0x807F0000, "NVRAM_SECTOR_SA134", "NVRAM Micro-Sector 0: System Configuration & Boot Flags", True),
        (0x807F2000, "NVRAM_SECTOR_SA135", "NVRAM Micro-Sector 1: Tuner Channel Map & Presets", True),
        (0x807F4000, "NVRAM_SECTOR_SA136", "NVRAM Micro-Sector 2: Scheduled Timer Recordings & EPG", True),
        (0x807F6000, "NVRAM_SECTOR_SA137", "NVRAM Micro-Sector 3: User Preferences & AV Modes", True),
        (0x807F8000, "NVRAM_SECTOR_SA138", "NVRAM Micro-Sector 4: Optical Drive Pickup Laser Calibration", True),
        (0x807FA000, "NVRAM_SECTOR_SA139", "NVRAM Micro-Sector 5: HDD Serial Binding & Volume Identification", True),
        (0x807FC000, "NVRAM_SECTOR_SA140", "NVRAM Micro-Sector 6: Factory Diagnostics & Self-Test Flags", True),
        (0x807FE000, "NVRAM_SECTOR_SA141_U99", "NVRAM Micro-Sector 7: Locus of U99 Corrupted Hardware Error Logs", True),
        (0x84000000, "FLASH_NOR_UNCACHED_BASE", "Spansion S29JL064H CS1 Uncached Base (DUMPNOR 0x84000000)", True),
        (0x84080000, "EH55_DRV_EXTRHEAD_BASE", "Real DMR-EH55 IC5001 EXTRHEAD DRV Container (SA15..SA25, 0x84080040 -> 0x90202000)", True),
        (0x84130000, "FLASH_NOR_CS1_EH55_BGO", "Real DMR-EH55 Loader ver0.61 BGO/[nfrom] Start (0x84130000..0x843FFFFF, 45 x 64 KiB SA26..SA70)", True),
        (0x84200000, "FLASH_NOR_BGO_START", "PROG061 / Loader ver0.63 BGO Flash Partition Start (0x84200000..0x843FFFFF, 32 x 64 KiB, NFROM)", True),
        (0x84400000, "UPPER_EEMUL_BASE", "Real DMR-EH55 Upper EEMUL NOR Flash FS Start (SA71..SA141, 0x84400000..0x847FFFFF, 1284 headers)", True),
        (0x847F0000, "NVRAM_UNCACHED_BASE", "NVRAM Uncached CS1 Mirror (0x847F0000..0x84800000)", True),
        (0x440337A4, "EH55_BGO_GEOMETRY_TABLE", "Real DMR-EH55 Loader ver0.61 BGO Geometry Table (45 x 64 KiB, 1 P1 + 40 P2 + 4 spares, max PID 0x0F24)", True),
        (0x90400000, "PROG061_SDRAM_UNCACHED", "Uncached SDRAM Alias of 0x50400000 PROG061 (used by PANADMP1 0x9043950E & RAM stubs 0x90439B40)", True),
        (0xD4002000, "SC0CTR_UART0_CTRL", "AM34 SC0 UART0 Control Register (P7501 115200 8N1)", True),
        (0xD4002004, "SC0ICR_UART0_INT_CTRL", "AM34 SC0 UART0 Interrupt Control Register", True),
        (0xD4002008, "SC0TXB_UART0_TX_BYTE", "AM34 SC0 UART0 Transmit Byte Register", True),
        (0xD4002009, "SC0RXB_UART0_RX_BYTE", "AM34 SC0 UART0 Receive Byte Register", True),
        (0xD400200C, "SC0STR_UART0_STATUS", "AM34 SC0 UART0 Status Register (mask 0xA0 TX ready)", True),
        (0x89000500, "DAVIS_IDE_PRI_ODD_CH0", "DAVIS Primary IDE Channel (ch=0, dev=0 DVD-RAM ODD ATAPI)", True),
        (0x89000D00, "DAVIS_IDE_SEC_HDD_CH1", "DAVIS Secondary IDE Channel (ch=1, dev=0 HDD MASTER)", True),
        (0x89000D38, "DAVIS_IDE_SEC_DEV_SEL", "DAVIS Secondary IDE Device/Head Select (bit 4 DEV)", True),
        (0x89001800, "SDCD_HOST_BASE", "SDCD SD/MMC Host Controller MMIO Base (P6801 Slot)", True),
        (0x89001850, "SDCD_BUS_WIDTH_CFG", "SDCD 1-bit (0x80B8) vs 4-bit (&= 0x7FFF) Bus Width Register", True),
        (0xC0000000, "MMIO_CHIP_ID", "Matsushita AM34 / MN103S Silicon Revision ID", True),
        (0xC0000004, "MMIO_RESET_CTRL", "Subsystem Reset Register (Software Reset Controller)", True),
        (0xC0000010, "MMIO_PLL_CLOCK_CFG", "PLL Multiplier and Clock Gating Control", True),
        (0xD4000000, "PATA_DATA_16BIT", "PATA 16-bit PIO / DMA Data Register", True),
        (0xD4000001, "PATA_ERROR_FEATURES", "PATA Error Register (Read) / Features Register (Write)", True),
        (0xD4000002, "PATA_SECTOR_COUNT", "PATA Sector Count Register", True),
        (0xD4000007, "PATA_STATUS_COMMAND", "PATA Status Register (Read) / Command Register (Write)", True),
        (0x00004000, "RAM_LOADER_ENTRY", "Panadiga In-Circuit SRAM Loader Entrypoint", True),
        (0x90202000, "DRV_S211_DECOMP_BASE", "DRV S211 F/E Optical Drive Decompressed SDRAM Target", True),
        (0x88000000, "AIOF_VMODESEL", "AIOF Video Output Mode Select (0x8020 NTSC, 0x8021 PAL-M, 0x8027 PAL)", True),
        (0x88000066, "AIOF_V_CLK_CTRL", "AIOF Vertical/Clock Control (0x8001 clock_start, 0x8039 ddr_stop)", True),
        (0x89000420, "DAVIS_CH0_RESET_CTRL", "DAVIS CH0 ODD Reset Polarity Register (value 2 active-LOW; EV-20261119-005)", True),
        (0x89000C20, "DAVIS_CH1_RESET_CTRL", "DAVIS CH1 HDD Reset Polarity Register (value 1 active-HIGH; EV-20261119-005)", True),
        (0x89800624, "AVDD_NPALCTL0", "AVDD Analog Decoder NTSC/PAL Encoder Control 0", True),
        (0xC0001000, "WDBC_RESET_CTRL", "AM34 Watchdog / System Reset Control (15s reset guard)", True),
        (0xC0001008, "SYS_STATUS_FE_REL", "AM34 System Status / Front-End Reset Release Register (bit 15 = 0x8000)", True),
        (0xC000100C, "DO_NOT_READ_LETHAL_0xC000100C", "CRITICAL WARNING: NEVER READ 0xC000100C (halts AM34 bus/WDT permanently)", True),
        (0xD8400000, "IIC0_MASTER_BASE", "AM34 I2C Master Controller 0 (IC3001 A/V Switch Matrix bus)", True),
        (0xD8401000, "IIC1_MASTER_BASE", "AM34 I2C Master Controller 1 Base", True),
        (0xDC010000, "IC7501_TBUS_CTRL", "IC7501 Front-Panel / Timer MCU TBUS Control Strobe (bit 4 ACK)", True),
        (0xDC010010, "IC7501_TBUS_TX_MBOX", "IC7501 TBUS TX 16-byte Mailbox (0x8A 255s keepalive, 0xA3 VFD text)", True),
        (0x44000000, "boot_ipl_base", "Stage 1 /BOOT.BIN SDRAM Execution Base", True),
        (0x4400479F, "boot_uart0_getc_dead", "Dead UART0 polling helper (0 active callers in production IPL)", True),
        (0x44004808, "boot_uart0_printf", "/BOOT.BIN — UART0 formatted printf dispatcher (used by boot_fmu_debug_printf)", True),
        (0x4400488A, "boot_console_getline_dead", "Dead BootROM console reader (0 active callers)", True),
        (0x440098FC, "boot_media_read_sectors", "/BOOT.BIN — Media sector read dispatcher (media_type=3 routes to SD block reader 0x440262cb)", True),
        (0x4400B3D9, "boot_fmu_debug_printf", "/BOOT.BIN — Unconditional varargs wrapper around boot_uart0_printf (0x44004808)", True),
        (0x4400BCC2, "boot_uncompress_fe", "Decompresses F/E DRV image into 0x90202000 & checks Adler/sum", True),
        (0x4400C0A8, "boot_fat_validate_vbr_bpb", "/BOOT.BIN — Validates VBR 0x55AA, BytesPerSector==512, SectorsPerCluster!=0, NumFATs in 1..=2, and FAT12/FAT16/FAT32 signature", True),
        (0x4400C696, "boot_fat_mount", "/BOOT.BIN — Mounts FAT12/FAT16 volume at 0x5056B640 (rejects FAT32 with -10025 / 0xFFFFD8D7)", True),
        (0x4400C879, "boot_fat_open", "/BOOT.BIN — Opens 8.3 file on mounted FAT volume (returns handle 200 or -10010/-10011/-10020)", True),
        (0x4400C922, "boot_fat_read", "/BOOT.BIN — Reads bytes from open FAT handle (must return 0x20000 in boot_sd_load_digadiag_frm)", True),
        (0x4400C94D, "boot_fat_seek", "/BOOT.BIN — Seeks open FAT handle (whence 1=SET, 2=CUR, 3=END; returns -10020 on invalid offset)", True),
        (0x4400CA59, "boot_fat_sd_bounce_read", "/BOOT.BIN — Reads up to 0x1FC SD sectors via bounce buffer 0x01000000 / uncached alias 0x91C00000", True),
        (0x4400CB21, "boot_fat_parse_mbr_vbr", "/BOOT.BIN — Reads LBA 0, scans 4 MBR entries (+0x1BE) or falls back to Superfloppy LBA 0 (+6 backup VBR)", True),
        (0x4400CED2, "boot_fat_cache_init", "/BOOT.BIN — Initializes FAT sector cache at 0x505724C0 (reads first 3 sectors of FAT1 or FAT2)", True),
        (0x4400CFDE, "boot_fat_parse_8_3", "/BOOT.BIN — Splits 8.3 filename into 8-byte stem and 3-byte ext (returns -10020 if >12 chars or stem >8)", True),
        (0x4400D16B, "boot_fat_open_entry", "/BOOT.BIN — Scans FAT12/FAT16 root directory for 8.3 entry (returns -10010 / 0xFFFFD8E6 if not found)", True),
        (0x4400DFC8, "boot_fat_read_clusters", "/BOOT.BIN — Reads file clusters (clamps to file_size; returns -10025 on I/O err, -10028 on bad cluster, -10030 on EOF)", True),
        (0x4400E737, "boot_fat_next_cluster", "/BOOT.BIN — Resolves next FAT12/FAT16 cluster from 3-sector FAT window at 0x505724C8", True),
        (0x440262CB, "boot_sd_read_blocks", "/BOOT.BIN — Low-level SD card block reader (invoked by boot_media_read_sectors for media_type=3)", True),
        (0x44027826, "boot_get_sd_status_1bit", "1st GetSD_Status call in 1-bit DAT0 mode (passes on SDSC)", True),
        (0x44027F79, "boot_acmd6_4bit_switch", "Sends ACMD6(2) and clears 0x89001850 bit 15 to enable 4-bit SD", True),
        (0x44027F93, "boot_get_sd_status_4bit", "2nd GetSD_Status call in 4-bit DAT[3:0] mode (fails -91006 on SDHC/DAT1..3 fault)", True),
        (0x44035D84, "boot_feistel_sbox_256b", "Verified 256-byte Feistel S-Box substitution table in BOOT.BIN", True),
        (0x50400000, "prog061_fvu_entry", "Stage 2 PROG061 FVU Entrypoint (DIGADIAG.FRM dst=0x50400000)", True),
        (0x50401F3C, "prog061_nfrom_format", "Orphan factory NFROM/BGO formatter (0 static callers; invokable from 0x5040A2CD)", True),
        (0x50405097, "prog061_devinfo_check_beq", "Site 2 (0x50405097): DeviceInfo INFO guard (beq+21 -> bra+21)", True),
        (0x5040A2CD, "prog061_bgo_init_fail_hook", "Site 3 (0x5040A2CD): 20B BGO init failure branch -> AUTO_FORMAT_BGO / PANAMON1", True),
        (0x5040A2E1, "prog061_ctrl_info_init", "Control information init continuation point after BGO init", True),
        (0x5040A306, "prog061_bgo_mark_ok", "BGO bypass OK return point (d0 == 0)", True),
        (0x5040A311, "prog061_bgo_error_exit", "BGO factory error return point (d0 == -1)", True),
        (0x50414908, "prog061_davis_ide_topology", "Fixed DAVIS IDE topology table acStack_70 = {2,0,1,0} (Sec Master HDD)", True),
        (0x504230F0, "prog061_bgo_sum_beq", "Site 1 (0x504230F0): bgo_sum_23058 N(N+1)/2 checksum branch (beq+10 -> bra+10)", True),
        (0x50435C0A, "prog061_feistel_sbox_256b", "Verified 256-byte Feistel S-Box substitution table in PROG061", True),
        (0x50439494, "prog061_panamon1_uart_monitor", "Verified 0xFF slack entrypoint for PANAMON1 UART0 rescue monitor", True),
        (0x57000000, "main_app_base", "Stage 3 MAIN Application Base (EH55_MAIN_3410PQ.bin)", True),
        (0x574D1DE8, "main_davis_ide_topology", "Fixed MAIN DAVIS IDE topology acStack_70 = {2,0,1,0}", True),
        (0x574DD1FC, "main_hdd_cmd_lba48_clamp", "src/hdd_cmd.c >200 GB LBA48 capacity clamp / patch site", True),
        (0x574F7B8C, "main_odd_state_branch", "TBC Site 2 (0x574F7B8C): ODD state check -> jmp16 +0x00D0 to 0x574F7C5C", True),
        (0x574F7C5C, "main_odd_init_status", "TBC Site 3 (0x574F7C5C): ODD init status forced ready (clr d0; nop x5)", True),
        (0x5771ABFB, "main_fpc_status_ctrl", "TBC Site 1 (0x5771ABFB): FPC status control -> mov 1, d0; ret", True),
        (0x515B2E00, "_dev_SERIAL_state_base", "_dev_SERIAL UART0 GDB RSP protocol state struct (+0x60 = state 0..4)", True),
        (0x515B2E40, "_dev_SERIAL_callback_ptr", "_dev_SERIAL registered UART0 RSP callback pointer (+0x40)", True),
        (0x5316AE8C, "_dev_SERIAL_isr_ctx_ptr", "Pointer to saved ISR register frame (saved PC at +0x64)", True),
    ]

def import_symbols(prog=None):
    if prog is None:
        prog = currentProgram
    sym_table = prog.getSymbolTable()
    listing = prog.getListing()
    mem = prog.getMemory()
    addr_factory = prog.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()
    plate_comment = CodeUnit.PLATE_COMMENT if CodeUnit is not None else 3

    symbols = get_panadiga_symbol_specs()
    print("[Panadiga] Annotating Panasonic DMR-EH55 Symbols and Peripherals...")
    imported_count = 0
    skipped_unmapped = 0

    for addr_val, name, comment, is_primary in symbols:
        addr = default_space.getAddress(addr_val)
        if mem.getBlock(addr) is None:
            skipped_unmapped += 1
            continue
        try:
            sym = sym_table.createLabel(addr, name, SourceType.USER_DEFINED)
            if is_primary and sym is not None:
                sym.setPrimary()
            listing.setComment(addr, plate_comment, comment)
            imported_count += 1
            print("  [+] Labeled 0x%08X -> %s" % (addr_val, name))
        except Exception as e:
            listing.setComment(addr, plate_comment, comment)
            print("  [*] Updated comment at 0x%08X -> %s (%s)" % (addr_val, name, str(e)))

    print("[Panadiga] Symbol import complete! Annotated %d symbols (%d unmapped skipped)." % (imported_count, skipped_unmapped))
    return imported_count

if __name__ == "__main__":
    import_symbols()
