//! Panasonic DMR-EH55 Ghidra Integration Generator
//!
//! Generates official Ghidra Jython/Python-3-compatible memory mapping, symbol
//! annotation, and automated unbrick audit scripts directly from Rust constants
//! (`constants.rs` + `re::*`), guaranteeing that Rust is the Single Source of
//! Truth (SSOT) across all reverse engineering environments.

use crate::constants::*;
use crate::re::bgo_unbrick::{
    OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, ORIG_A2CD_HOOK,
    ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PATCH_A2CD_AUTOFORMAT, PATCH_DEVINFO_BRA, PATCH_SUM_BRA,
    PROG061_BASE_VA, PROG061_EXPECTED_SIZE, PROG061_NFROM_FORMAT_VA,
};
use crate::re::main_passthrough::{
    DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_ISR_CTX_PTR_VA, DEV_SERIAL_STATE_BASE_VA,
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS,
    OFF_ODD_STATE_BRANCH, ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH,
    PATCH_FPC_STATUS_CTRL, PATCH_ODD_INIT_STATUS, PATCH_ODD_STATE_BRANCH,
};
use crate::re::prog061_uart_monitor::{PATCH_A2CD_UART_MONITOR, PROG061_UART_MONITOR_VA};
use anyhow::{Context, Result};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

const BOOT_UART0_GETC_VA: u32 = 0x4400_479F;
const BOOT_CONSOLE_GETLINE_VA: u32 = 0x4400_488A;
const BOOT_UNCOMPRESS_VA: u32 = 0x4400_BCC2;
const BOOT_GET_SD_STATUS_VA: u32 = 0x4402_7826;
const BOOT_ACMD6_SWITCH_VA: u32 = 0x4402_7F79;
const BOOT_SECOND_SD_STATUS_VA: u32 = 0x4402_7F93;
const BOOT_FEISTEL_SBOX_VA: u32 = 0x4403_5D84;

const PROG061_CTRL_INFO_INIT_VA: u32 = 0x5040_A2E1;
const PROG061_BGO_MARK_OK_VA: u32 = 0x5040_A306;
const PROG061_BGO_ERROR_EXIT_VA: u32 = 0x5040_A311;
const PROG061_DAVIS_TOPOLOGY_VA: u32 = 0x5041_4908;
const PROG061_FEISTEL_SBOX_VA: u32 = 0x5043_5C0A;

const MAIN_DAVIS_TOPOLOGY_VA: u32 = 0x574D_1DE8;
const MAIN_LBA48_CLAMP_VA: u32 = 0x574D_D1FC;
const MAIN_APP_REGION_SIZE: u32 = 0x00E0_0000;
const MAIN3060_EXPECTED_SIZE: usize = 13_427_852;
const DRV_S211_EXPECTED_SIZE: usize = 1_441_792;

fn format_py_byte_tuple(bytes: &[u8]) -> String {
    let mut out = String::from("(");
    for (idx, b) in bytes.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        let _ = write!(&mut out, "0x{:02X}", b);
    }
    if bytes.len() == 1 {
        out.push(',');
    }
    out.push(')');
    out
}

/// Generates the Ghidra Memory Map setup Python script (Jython 2.7 + Python 3 compatible).
pub fn generate_ghidra_memory_map() -> String {
    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Memory Map Setup
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)
#
# Configures the complete memory map of Panasonic AM34 / MN103S (UniPhier)
# matching the DMR-EH55 VEP79138K digital board layout (Jython 2.7 + Py3 safe).

from __future__ import print_function

def _range_overlaps_existing(mem, start, size):
    end = start + size - 1
    for blk in mem.getBlocks():
        blk_start = blk.getStart().getOffset() & 0xFFFFFFFF
        blk_end = blk.getEnd().getOffset() & 0xFFFFFFFF
        if start <= blk_end and blk_start <= end:
            return blk.getName()
    return None

def setup_memory_map(prog=None):
    if prog is None:
        prog = currentProgram
    mem = prog.getMemory()
    addr_factory = prog.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()

    regions = [
        # (Name, StartHex, SizeBytes, Read, Write, Execute, IsVolatile)
        ("INTERNAL_SRAM",       0x{sram_base:08X}, 0x{sram_size:08X}, True,  True,  True,  False),
        ("CORE_MMIO",           0x{core_mmio_base:08X}, 0x{core_mmio_size:08X}, True,  True,  False, True),
        ("MAIN_SDRAM",          0x{sdram_base:08X}, 0x{sdram_size:08X}, True,  True,  True,  False),
        ("BOOT_BIN_RAM",        0x{boot_base:08X}, 0x{boot_size:08X}, True,  True,  True,  False),
        ("PROG061_FVU_RAM",     0x{prog_base:08X}, 0x{prog_size:08X}, True,  True,  True,  False),
        ("DEV_SERIAL_DATA",     0x{dev_ser_blk:08X}, 0x00002000, True,  True,  False, True),
        ("DEV_SERIAL_ISR_CTX",  0x{dev_isr_blk:08X}, 0x00002000, True,  True,  False, True),
        ("MAIN_APP_RAM",        0x{main_base:08X}, 0x{main_size:08X}, True,  True,  True,  False),
        ("FLASH_NOR_IPL",       0x{flash_base:08X}, 0x{flash_ipl_size:08X}, True,  False, True,  False),
        ("NVRAM_EEMUL",         0x{nvram_base:08X}, 0x{nvram_size:08X}, True,  True,  False, False),
        ("FLASH_NOR_CS1_IPL",   0x{nor_uncached:08X}, 0x00200000, True,  False, True,  False),
        ("FLASH_NOR_CS1_BGO",   0x{nor_bgo:08X}, 0x{nor_bgo_size:08X}, True,  True,  False, True),
        ("FLASH_NOR_CS1_APP",   0x{nor_app:08X}, 0x{nor_app_size:08X}, True,  False, True,  False),
        ("NVRAM_UNCACHED",      0x{nvram_uncached:08X}, 0x{nvram_size:08X}, True,  True,  False, False),
        ("DAVIS_IDE_PRI_MMIO",  0x{davis_pri:08X}, 0x{davis_size:08X}, True,  True,  False, True),
        ("DAVIS_IDE_SEC_MMIO",  0x{davis_sec:08X}, 0x{davis_size:08X}, True,  True,  False, True),
        ("SDCD_HOST_MMIO",      0x{sdcd_base:08X}, 0x{sdcd_size:08X}, True,  True,  False, True),
        ("SDRAM_UNCACHED",      0x{sdram_uncached:08X}, 0x{sdram_size:08X}, True,  True,  True,  False),
        ("ASIC_MMIO",           0x{mmio_base:08X}, 0x{mmio_size:08X}, True,  True,  False, True),
        ("ATA_IDE_MMIO",        0x{pata_base:08X}, 0x{pata_size:08X}, True,  True,  False, True),
        ("SC0_UART0_MMIO",      0x{sc0_base:08X}, 0x{sc0_size:08X}, True,  True,  False, True),
    ]

    print("[Panadiga] Configuring Panasonic DMR-EH55 Memory Map...")
    created = 0
    for name, start, size, r, w, x, vol in regions:
        addr = default_space.getAddress(start)
        overlap = _range_overlaps_existing(mem, start, size)
        if overlap is None:
            try:
                block = mem.createUninitializedBlock(name, addr, size, False)
                block.setRead(r)
                block.setWrite(w)
                block.setExecute(x)
                block.setVolatile(vol)
                block.setComment("Panasonic DMR-EH55 HW Subsystem: " + name)
                created += 1
                print("  [+] Created block: %s at 0x%08X (size: 0x%X)" % (name, start, size))
            except Exception as e:
                print("  [-] Could not create block %s: %s" % (name, str(e)))
        else:
            print("  [*] Overlap skipped for %s at 0x%08X (covered by %s)" % (name, start, overlap))

    print("[Panadiga] Memory Map setup complete! (%d new blocks)" % created)
    return created

if __name__ == "__main__":
    setup_memory_map()
"#,
        sram_base = AM34_INTERNAL_SRAM_BASE,
        sram_size = AM34_INTERNAL_SRAM_SIZE,
        core_mmio_base = AM34_MMIO_BASE,
        core_mmio_size = AM34_MMIO_SIZE,
        sdram_base = AM34_SDRAM_BASE,
        sdram_size = AM34_SDRAM_SIZE,
        boot_base = AM34_BOOT_BIN_BASE,
        boot_size = AM34_BOOT_BIN_SIZE,
        prog_base = PROG061_BASE_VA,
        prog_size = PROG061_EXPECTED_SIZE,
        dev_ser_blk = DEV_SERIAL_STATE_BASE_VA & 0xFFFF_F000,
        dev_isr_blk = DEV_SERIAL_ISR_CTX_PTR_VA & 0xFFFF_F000,
        main_base = MAIN_BASE_VA,
        main_size = MAIN_APP_REGION_SIZE,
        flash_base = AM34_FLASH_BASE,
        flash_ipl_size = AM34_FLASH_IPL_SIZE,
        nvram_base = AM34_NVRAM_BASE,
        nvram_size = AM34_NVRAM_SIZE,
        nor_uncached = AM34_FLASH_UNCACHED_BASE,
        nor_bgo = AM34_FLASH_BGO_BASE,
        nor_bgo_size = AM34_FLASH_BGO_SIZE,
        nor_app = AM34_FLASH_UNCACHED_APP_BASE,
        nor_app_size = AM34_FLASH_UNCACHED_APP_SIZE,
        nvram_uncached = AM34_NVRAM_UNCACHED_BASE,
        davis_pri = AM34_DAVIS_PRI_MMIO_BASE,
        davis_sec = AM34_DAVIS_SEC_MMIO_BASE,
        davis_size = AM34_DAVIS_MMIO_SIZE,
        sdcd_base = AM34_SDCD_MMIO_BASE,
        sdcd_size = AM34_SDCD_MMIO_SIZE,
        sdram_uncached = AM34_SDRAM_UNCACHED_BASE,
        mmio_base = AM34_ASIC_MMIO_BASE,
        mmio_size = AM34_ASIC_MMIO_SIZE,
        pata_base = AM34_PATA_MMIO_BASE,
        pata_size = AM34_PATA_MMIO_SIZE,
        sc0_base = AM34_SC0_BASE,
        sc0_size = AM34_SC0_SIZE,
    )
}

/// Generates the Ghidra Symbols and Peripheral importer script (Jython 2.7 + Python 3 safe).
pub fn generate_ghidra_symbols() -> String {
    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Symbol & Peripheral Importer
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)

from __future__ import print_function

try:
    from ghidra.program.model.symbol import SourceType
except ImportError:
    SourceType = None

def get_panadiga_symbol_specs():
    return [
        # AM34 Reset and Interrupt Vectors
        (0x{flash_base:08X}, "_start_reset_vector", "AM34 Reset Vector - Cold Boot Entrypoint", True),
        (0x{flash_base_plus_4:08X}, "vector_nmi", "Non-Maskable Interrupt Handler", True),
        (0x{flash_base_plus_8:08X}, "vector_trap", "Software Trap Exception Handler", True),
        (0x{flash_base_plus_c:08X}, "vector_illegal_instr", "Illegal Instruction Exception Handler", True),

        # NVRAM 8KB Micro-Sectors (SA134..SA141) + Uncached CS1 NOR/BGO Mirrors
        (0x{nvram_base:08X}, "NVRAM_SECTOR_SA134", "NVRAM Micro-Sector 0: System Configuration & Boot Flags", True),
        (0x{nvram_sa135:08X}, "NVRAM_SECTOR_SA135", "NVRAM Micro-Sector 1: Tuner Channel Map & Presets", True),
        (0x{nvram_sa136:08X}, "NVRAM_SECTOR_SA136", "NVRAM Micro-Sector 2: Scheduled Timer Recordings & EPG", True),
        (0x{nvram_sa137:08X}, "NVRAM_SECTOR_SA137", "NVRAM Micro-Sector 3: User Preferences & AV Modes", True),
        (0x{nvram_sa138:08X}, "NVRAM_SECTOR_SA138", "NVRAM Micro-Sector 4: Optical Drive Pickup Laser Calibration", True),
        (0x{nvram_sa139:08X}, "NVRAM_SECTOR_SA139", "NVRAM Micro-Sector 5: HDD Serial Binding & Volume Identification", True),
        (0x{nvram_sa140:08X}, "NVRAM_SECTOR_SA140", "NVRAM Micro-Sector 6: Factory Diagnostics & Self-Test Flags", True),
        (0x{nvram_sa141:08X}, "NVRAM_SECTOR_SA141_U99", "NVRAM Micro-Sector 7: Locus of U99 Corrupted Hardware Error Logs", True),
        (0x{nor_uncached:08X}, "FLASH_NOR_UNCACHED_BASE", "Spansion S29GL064 CS1 Uncached Base (DUMPNOR 0x84000000)", True),
        (0x{nor_bgo:08X}, "FLASH_NOR_BGO_START", "BGO Flash Partition Start (32 x 64 KiB, NFROM)", True),
        (0x{nvram_uncached:08X}, "NVRAM_UNCACHED_BASE", "NVRAM Uncached CS1 Mirror (0x847F0000..0x84800000)", True),

        # UniPhier SC0 UART0 (Real P7501 Diagnostic Serial Controller at 0xD4002000)
        (0x{sc0_ctr:08X}, "SC0CTR_UART0_CTRL", "AM34 SC0 UART0 Control Register (P7501 115200 8N1)", True),
        (0x{sc0_icr:08X}, "SC0ICR_UART0_INT_CTRL", "AM34 SC0 UART0 Interrupt Control Register", True),
        (0x{sc0_txb:08X}, "SC0TXB_UART0_TX_BYTE", "AM34 SC0 UART0 Transmit Byte Register", True),
        (0x{sc0_rxb:08X}, "SC0RXB_UART0_RX_BYTE", "AM34 SC0 UART0 Receive Byte Register", True),
        (0x{sc0_str:08X}, "SC0STR_UART0_STATUS", "AM34 SC0 UART0 Status Register (mask 0xA0 TX ready)", True),

        # DAVIS IDE / ATAPI & SDCD SD-Card Host Controllers
        (0x{davis_pri:08X}, "DAVIS_IDE_PRI_ODD_CH0", "DAVIS Primary IDE Channel (ch=0, dev=0 DVD-RAM ODD ATAPI)", True),
        (0x{davis_sec:08X}, "DAVIS_IDE_SEC_HDD_CH1", "DAVIS Secondary IDE Channel (ch=1, dev=0 HDD MASTER)", True),
        (0x{davis_sec_dev:08X}, "DAVIS_IDE_SEC_DEV_SEL", "DAVIS Secondary IDE Device/Head Select (bit 4 DEV)", True),
        (0x{sdcd_base:08X}, "SDCD_HOST_BASE", "SDCD SD/MMC Host Controller MMIO Base (P6801 Slot)", True),
        (0x{sdcd_bus_cfg:08X}, "SDCD_BUS_WIDTH_CFG", "SDCD 1-bit (0x80B8) vs 4-bit (&= 0x7FFF) Bus Width Register", True),

        # UniPhier ASIC & Legacy PATA Placeholders + DRV S211 / SRAM Loader
        (0x{mmio_base:08X}, "MMIO_CHIP_ID", "Matsushita AM34 / MN103S Silicon Revision ID", True),
        (0x{mmio_base_plus_4:08X}, "MMIO_RESET_CTRL", "Subsystem Reset Register (Software Reset Controller)", True),
        (0x{mmio_base_plus_10:08X}, "MMIO_PLL_CLOCK_CFG", "PLL Multiplier and Clock Gating Control", True),
        (0x{pata_base:08X}, "PATA_DATA_16BIT", "PATA 16-bit PIO / DMA Data Register", True),
        (0x{pata_base_plus_1:08X}, "PATA_ERROR_FEATURES", "PATA Error Register (Read) / Features Register (Write)", True),
        (0x{pata_base_plus_2:08X}, "PATA_SECTOR_COUNT", "PATA Sector Count Register", True),
        (0x{pata_base_plus_7:08X}, "PATA_STATUS_COMMAND", "PATA Status Register (Read) / Command Register (Write)", True),
        (0x{loader_base:08X}, "RAM_LOADER_ENTRY", "Panadiga In-Circuit SRAM Loader Entrypoint", True),
        (0x{drv_base:08X}, "DRV_S211_DECOMP_BASE", "DRV S211 F/E Optical Drive Decompressed SDRAM Target", True),

        # Stage 1: /BOOT.BIN (NOR IPL at 0x44000000)
        (0x{boot_base:08X}, "boot_ipl_base", "Stage 1 /BOOT.BIN SDRAM Execution Base", True),
        (0x{boot_uart_getc:08X}, "boot_uart0_getc_dead", "Dead UART0 polling helper (0 active callers in production IPL)", True),
        (0x{boot_getline:08X}, "boot_console_getline_dead", "Dead BootROM console reader (0 active callers)", True),
        (0x{boot_uncompress:08X}, "boot_uncompress_fe", "Decompresses F/E DRV image into 0x90202000 & checks Adler/sum", True),
        (0x{boot_sd1:08X}, "boot_get_sd_status_1bit", "1st GetSD_Status call in 1-bit DAT0 mode (passes on SDSC)", True),
        (0x{boot_acmd6:08X}, "boot_acmd6_4bit_switch", "Sends ACMD6(2) and clears 0x89001850 bit 15 to enable 4-bit SD", True),
        (0x{boot_sd2:08X}, "boot_get_sd_status_4bit", "2nd GetSD_Status call in 4-bit DAT[3:0] mode (fails -91006 on SDHC/DAT1..3 fault)", True),
        (0x{boot_sbox:08X}, "boot_feistel_sbox_256b", "Verified 256-byte Feistel S-Box substitution table in BOOT.BIN", True),

        # Stage 2: PROG061 / DIGADIAG.FRM (0x50400000)
        (0x{prog_base:08X}, "prog061_fvu_entry", "Stage 2 PROG061 FVU Entrypoint (DIGADIAG.FRM dst=0x50400000)", True),
        (0x{prog_nfrom:08X}, "prog061_nfrom_format", "Orphan factory NFROM/BGO formatter (0 static callers; invokable from 0x5040A2CD)", True),
        (0x{prog_devinfo:08X}, "prog061_devinfo_check_beq", "Site 2 (0x50405097): DeviceInfo INFO guard (beq+21 -> bra+21)", True),
        (0x{prog_a2cd:08X}, "prog061_bgo_init_fail_hook", "Site 3 (0x5040A2CD): 20B BGO init failure branch -> AUTO_FORMAT_BGO / PANAMON1", True),
        (0x{prog_ctrl_init:08X}, "prog061_ctrl_info_init", "Control information init continuation point after BGO init", True),
        (0x{prog_bgo_ok:08X}, "prog061_bgo_mark_ok", "BGO bypass OK return point (d0 == 0)", True),
        (0x{prog_bgo_err:08X}, "prog061_bgo_error_exit", "BGO factory error return point (d0 == -1)", True),
        (0x{prog_davis:08X}, "prog061_davis_ide_topology", "Fixed DAVIS IDE topology table acStack_70 = {{2,0,1,0}} (Sec Master HDD)", True),
        (0x{prog_sum:08X}, "prog061_bgo_sum_beq", "Site 1 (0x504230F0): bgo_sum_23058 N(N+1)/2 checksum branch (beq+10 -> bra+10)", True),
        (0x{prog_sbox:08X}, "prog061_feistel_sbox_256b", "Verified 256-byte Feistel S-Box substitution table in PROG061", True),
        (0x{prog_mon:08X}, "prog061_panamon1_uart_monitor", "Verified 0xFF slack entrypoint for PANAMON1 UART0 rescue monitor", True),

        # Stage 3: EH55_MAIN_3410PQ.bin (0x57000000) & _dev_SERIAL RSP
        (0x{main_base:08X}, "main_app_base", "Stage 3 MAIN Application Base (EH55_MAIN_3410PQ.bin)", True),
        (0x{main_davis:08X}, "main_davis_ide_topology", "Fixed MAIN DAVIS IDE topology acStack_70 = {{2,0,1,0}}", True),
        (0x{main_lba48:08X}, "main_hdd_cmd_lba48_clamp", "src/hdd_cmd.c >200 GB LBA48 capacity clamp / patch site", True),
        (0x{main_odd_branch:08X}, "main_odd_state_branch", "TBC Site 2 (0x574F7B8C): ODD state check -> jmp16 +0x00D0 to 0x574F7C5C", True),
        (0x{main_odd_status:08X}, "main_odd_init_status", "TBC Site 3 (0x574F7C5C): ODD init status forced ready (clr d0; nop x5)", True),
        (0x{main_fpc_ctrl:08X}, "main_fpc_status_ctrl", "TBC Site 1 (0x5771ABFB): FPC status control -> mov 1, d0; ret", True),
        (0x{dev_ser_base:08X}, "_dev_SERIAL_state_base", "_dev_SERIAL UART0 GDB RSP protocol state struct (+0x60 = state 0..4)", True),
        (0x{dev_ser_cb:08X}, "_dev_SERIAL_callback_ptr", "_dev_SERIAL registered UART0 RSP callback pointer (+0x40)", True),
        (0x{dev_ser_isr:08X}, "_dev_SERIAL_isr_ctx_ptr", "Pointer to saved ISR register frame (saved PC at +0x64)", True),
    ]

def import_symbols(prog=None):
    if prog is None:
        prog = currentProgram
    sym_table = prog.getSymbolTable()
    listing = prog.getListing()
    mem = prog.getMemory()
    addr_factory = prog.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()

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
            if is_primary:
                sym_table.setPrimary(sym)
            listing.setComment(addr, listing.PLATE_COMMENT, comment)
            imported_count += 1
            print("  [+] Labeled 0x%08X -> %s" % (addr_val, name))
        except Exception as e:
            listing.setComment(addr, listing.PLATE_COMMENT, comment)
            print("  [*] Updated comment at 0x%08X -> %s (%s)" % (addr_val, name, str(e)))

    print("[Panadiga] Symbol import complete! Annotated %d symbols (%d unmapped skipped)." % (imported_count, skipped_unmapped))
    return imported_count

if __name__ == "__main__":
    import_symbols()
"#,
        flash_base = AM34_FLASH_BASE,
        flash_base_plus_4 = AM34_FLASH_BASE + 0x04,
        flash_base_plus_8 = AM34_FLASH_BASE + 0x08,
        flash_base_plus_c = AM34_FLASH_BASE + 0x0C,
        nvram_base = AM34_NVRAM_BASE,
        nvram_sa135 = AM34_NVRAM_BASE + 0x2000,
        nvram_sa136 = AM34_NVRAM_BASE + 0x4000,
        nvram_sa137 = AM34_NVRAM_BASE + 0x6000,
        nvram_sa138 = AM34_NVRAM_BASE + 0x8000,
        nvram_sa139 = AM34_NVRAM_BASE + 0xA000,
        nvram_sa140 = AM34_NVRAM_BASE + 0xC000,
        nvram_sa141 = AM34_NVRAM_BASE + 0xE000,
        nor_uncached = AM34_FLASH_UNCACHED_BASE,
        nor_bgo = AM34_FLASH_BGO_BASE,
        nvram_uncached = AM34_NVRAM_UNCACHED_BASE,
        sc0_ctr = AM34_SC0_BASE,
        sc0_icr = AM34_SC0_BASE + 0x04,
        sc0_txb = AM34_SC0_BASE + 0x08,
        sc0_rxb = AM34_SC0_BASE + 0x09,
        sc0_str = AM34_SC0_BASE + 0x0C,
        davis_pri = AM34_DAVIS_PRI_MMIO_BASE,
        davis_sec = AM34_DAVIS_SEC_MMIO_BASE,
        davis_sec_dev = AM34_DAVIS_SEC_MMIO_BASE + 0x38,
        sdcd_base = AM34_SDCD_MMIO_BASE,
        sdcd_bus_cfg = AM34_SDCD_BUS_CFG_VA,
        mmio_base = AM34_ASIC_MMIO_BASE,
        mmio_base_plus_4 = AM34_ASIC_MMIO_BASE + 0x04,
        mmio_base_plus_10 = AM34_ASIC_MMIO_BASE + 0x10,
        pata_base = AM34_PATA_MMIO_BASE,
        pata_base_plus_1 = AM34_PATA_MMIO_BASE + 0x01,
        pata_base_plus_2 = AM34_PATA_MMIO_BASE + 0x02,
        pata_base_plus_7 = AM34_PATA_MMIO_BASE + 0x07,
        loader_base = AM34_SRAM_LOADER_BASE,
        drv_base = AM34_DRV_S211_BASE,
        boot_base = AM34_BOOT_BIN_BASE,
        boot_uart_getc = BOOT_UART0_GETC_VA,
        boot_getline = BOOT_CONSOLE_GETLINE_VA,
        boot_uncompress = BOOT_UNCOMPRESS_VA,
        boot_sd1 = BOOT_GET_SD_STATUS_VA,
        boot_acmd6 = BOOT_ACMD6_SWITCH_VA,
        boot_sd2 = BOOT_SECOND_SD_STATUS_VA,
        boot_sbox = BOOT_FEISTEL_SBOX_VA,
        prog_base = PROG061_BASE_VA,
        prog_nfrom = PROG061_NFROM_FORMAT_VA,
        prog_devinfo = PROG061_BASE_VA + OFF_DEVINFO_CHECK_BEQ as u32,
        prog_a2cd = PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
        prog_ctrl_init = PROG061_CTRL_INFO_INIT_VA,
        prog_bgo_ok = PROG061_BGO_MARK_OK_VA,
        prog_bgo_err = PROG061_BGO_ERROR_EXIT_VA,
        prog_davis = PROG061_DAVIS_TOPOLOGY_VA,
        prog_sum = PROG061_BASE_VA + OFF_BGO_SUM_BEQ as u32,
        prog_sbox = PROG061_FEISTEL_SBOX_VA,
        prog_mon = PROG061_UART_MONITOR_VA,
        main_base = MAIN_BASE_VA,
        main_davis = MAIN_DAVIS_TOPOLOGY_VA,
        main_lba48 = MAIN_LBA48_CLAMP_VA,
        main_odd_branch = MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32,
        main_odd_status = MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32,
        main_fpc_ctrl = MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32,
        dev_ser_base = DEV_SERIAL_STATE_BASE_VA,
        dev_ser_cb = DEV_SERIAL_CALLBACK_VA,
        dev_ser_isr = DEV_SERIAL_ISR_CTX_PTR_VA,
    )
}

/// Generates an automated Ghidra Jython 2.7 / Python-3 stage detection, patch-verification,
/// MMIO-constant & XREF audit script (`ghidra_auto_unbrick_audit.py`).
pub fn generate_ghidra_auto_unbrick_audit() -> String {
    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Automated Stage & Unbrick Forensics Audit
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)
#
# Auto-detects open firmware (BOOT.BIN, PROG061, MAIN3410PQ, MAIN3060NP, DRV_S211, NOR),
# maps missing AM34 MMIO/SDRAM blocks, imports SSOT symbols, verifies live BGO & AV TBC
# opcodes, hunts unindexed MMIO literals & diagnostic strings, adds bookmarks, and writes JSON.

from __future__ import print_function

import json
import os
import struct
import sys
import tempfile

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if _SCRIPT_DIR not in sys.path:
    sys.path.insert(0, _SCRIPT_DIR)

from ghidra_am34_memory_map import setup_memory_map
from ghidra_import_symbols_panadiga import import_symbols

ORIG_SUM_BEQ = {orig_sum_beq}
PATCH_SUM_BRA = {patch_sum_bra}
ORIG_DEVINFO_BEQ = {orig_devinfo_beq}
PATCH_DEVINFO_BRA = {patch_devinfo_bra}
ORIG_A2CD_HOOK = {orig_a2cd}
PATCH_A2CD_AUTOFORMAT = {patch_a2cd_auto}
PATCH_A2CD_UART_MONITOR = {patch_a2cd_mon}

ORIG_FPC_STATUS_CTRL = {orig_fpc}
PATCH_FPC_STATUS_CTRL = {patch_fpc}
ORIG_ODD_STATE_BRANCH = {orig_odd_br}
PATCH_ODD_STATE_BRANCH = {patch_odd_br}
ORIG_ODD_INIT_STATUS = {orig_odd_st}
PATCH_ODD_INIT_STATUS = {patch_odd_st}

AUDIT_KEYWORDS = (
    "SetBusWidth",
    "-91006",
    "-90002",
    "BGO driver init",
    "Control information",
    "nfrom",
    "startup drive",
    "80040904",
    "_dev_SERIAL",
    "EXTRHEAD",
    "Firm list",
    "PANAMON1",
    "DDR CHECK",
)

AUDIT_MMIO_CONSTANTS = (
    (0x{sdcd_bus_cfg:08X}, "SDCD_BUS_WIDTH_CFG"),
    (0x{sc0_txb:08X}, "SC0TXB_UART0_TX_BYTE"),
    (0x{sc0_str:08X}, "SC0STR_UART0_STATUS"),
    (0x{nor_bgo:08X}, "FLASH_NOR_BGO_START"),
    (0x{davis_pri:08X}, "DAVIS_IDE_PRI_ODD_CH0"),
    (0x{davis_sec:08X}, "DAVIS_IDE_SEC_HDD_CH1"),
)

CRITICAL_XREF_TARGETS = (
    (0x{boot_uart_getc:08X}, "boot_uart0_getc_dead"),
    (0x{boot_getline:08X}, "boot_console_getline_dead"),
    (0x{boot_sd1:08X}, "boot_get_sd_status_1bit"),
    (0x{boot_sd2:08X}, "boot_get_sd_status_4bit"),
    (0x{prog_nfrom:08X}, "prog061_nfrom_format"),
    (0x{main_lba48:08X}, "main_hdd_cmd_lba48_clamp"),
)

def detect_stage_from_metadata(image_base, prog_name, strings_found, exec_size):
    name_up = (prog_name or "").upper()
    hits = set(strings_found or ())
    if image_base == 0x{boot_base:08X} or "BOOT" in name_up or "SetBusWidth" in hits or "-91006" in hits:
        return "BOOT.BIN"
    if image_base == 0x{prog_base:08X} or exec_size == 0x{prog_size:X} or "PROG061" in name_up or "BGO driver init" in hits:
        return "PROG061"
    if exec_size == {main3060_size} or "3060NP" in name_up:
        return "MAIN3060NP"
    if image_base == 0x{main_base:08X} or exec_size == {main3410_size} or "3410PQ" in name_up or "MAIN" in name_up:
        return "MAIN3410PQ"
    if image_base == 0x{drv_base:08X} or exec_size == {drv_size} or "DRV" in name_up:
        return "DRV_S211"
    if image_base in (0x{flash_base:08X}, 0x{nor_uncached:08X}) or exec_size == 0x{flash_size:X}:
        return "NOR_8MB"
    return "UNKNOWN"

def _label_pair(actual, orig, patch, orig_name, patch_name):
    if actual is None:
        return "UNMAPPED"
    t = tuple(b & 0xFF for b in actual)
    if t == orig:
        return orig_name
    if t == patch:
        return patch_name
    return "CUSTOM(" + " ".join(["%02X" % b for b in t[:8]]) + ")"

def classify_prog061_sites(sum_bytes, devinfo_bytes, a2cd_bytes):
    sum_state = _label_pair(sum_bytes, ORIG_SUM_BEQ, PATCH_SUM_BRA, "FACTORY", "BYPASSED")
    dev_state = _label_pair(devinfo_bytes, ORIG_DEVINFO_BEQ, PATCH_DEVINFO_BRA, "FACTORY", "BYPASSED")
    a2cd_t = tuple(b & 0xFF for b in a2cd_bytes) if a2cd_bytes is not None else None
    if a2cd_t == ORIG_A2CD_HOOK:
        a2cd_state = "FACTORY"
    elif a2cd_t == PATCH_A2CD_AUTOFORMAT:
        a2cd_state = "AUTO_FORMAT_BGO"
    elif a2cd_t == PATCH_A2CD_UART_MONITOR:
        a2cd_state = "UART_MONITOR_PANAMON1"
    elif a2cd_t is None:
        a2cd_state = "UNMAPPED"
    else:
        a2cd_state = "CUSTOM"

    if sum_state == "FACTORY" and dev_state == "FACTORY" and a2cd_state == "FACTORY":
        mode = "FACTORY_ORIGINAL"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state == "FACTORY":
        mode = "BYPASS_CHECKS_ONLY"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state in ("AUTO_FORMAT_BGO", "UART_MONITOR_PANAMON1"):
        mode = a2cd_state
    else:
        mode = "CUSTOM_OR_PARTIAL"

    return {{
        "mode": mode,
        "sites": {{
            "0x{prog_sum:08X}_bgo_sum_beq": sum_state,
            "0x{prog_devinfo:08X}_devinfo_check_beq": dev_state,
            "0x{prog_a2cd:08X}_bgo_init_fail_hook": a2cd_state,
        }},
    }}

def classify_main3410_sites(fpc_bytes, branch_bytes, status_bytes):
    fpc_state = _label_pair(fpc_bytes, ORIG_FPC_STATUS_CTRL, PATCH_FPC_STATUS_CTRL, "FACTORY", "PATCHED")
    br_state = _label_pair(branch_bytes, ORIG_ODD_STATE_BRANCH, PATCH_ODD_STATE_BRANCH, "FACTORY", "PATCHED")
    st_state = _label_pair(status_bytes, ORIG_ODD_INIT_STATUS, PATCH_ODD_INIT_STATUS, "FACTORY", "PATCHED")

    states = (fpc_state, br_state, st_state)
    if all(s == "FACTORY" for s in states):
        mode = "FACTORY_ORIGINAL"
    elif all(s == "PATCHED" for s in states):
        mode = "AV_PASSTHROUGH"
    else:
        mode = "CUSTOM_OR_PARTIAL"

    return {{
        "mode": mode,
        "sites": {{
            "0x{main_fpc_ctrl:08X}_fpc_status_ctrl": fpc_state,
            "0x{main_odd_branch:08X}_odd_state_branch": br_state,
            "0x{main_odd_status:08X}_odd_init_status": st_state,
        }},
    }}

def scan_ascii_keywords(raw_bytes, base_addr, keywords=AUDIT_KEYWORDS, max_hits_per_kw=8):
    haystack = bytearray(raw_bytes)
    hits = {{}}
    for kw in keywords:
        needle = bytearray(kw.encode("ascii"))
        pos = 0
        addrs = []
        while len(addrs) < max_hits_per_kw:
            idx = haystack.find(needle, pos)
            if idx < 0:
                break
            addrs.append("0x%08X" % (base_addr + idx))
            pos = idx + len(needle)
        if addrs:
            hits[kw] = addrs
    return hits

def scan_le32_constants(raw_bytes, base_addr, target_consts=AUDIT_MMIO_CONSTANTS, max_hits=16):
    haystack = bytearray(raw_bytes)
    results = {{}}
    for val, label in target_consts:
        needle = bytearray(struct.pack("<I", val))
        pos = 0
        addrs = []
        while len(addrs) < max_hits:
            idx = haystack.find(needle, pos)
            if idx < 0:
                break
            addrs.append("0x%08X" % (base_addr + idx))
            pos = idx + 1
        if addrs:
            results[label] = addrs
    return results

def _read_prog_bytes(prog, addr_val, count):
    mem = prog.getMemory()
    addr = prog.getAddressFactory().getDefaultAddressSpace().getAddress(addr_val)
    blk = mem.getBlock(addr)
    if blk is None or not blk.isInitialized():
        return None
    try:
        import jarray
        buf = jarray.zeros(count, "b")
        n = mem.getBytes(addr, buf)
        if n != count:
            return None
        return [b & 0xFF for b in buf]
    except Exception:
        return None

def _read_initialized_block_u8(blk):
    import jarray
    size = int(blk.getSize())
    data = bytearray(size)
    chunk_size = 0x100000
    offset = 0
    while offset < size:
        n = min(chunk_size, size - offset)
        jbuf = jarray.zeros(n, "b")
        addr = blk.getStart().add(offset)
        blk.getBytes(addr, jbuf)
        data[offset:offset + n] = bytearray(b & 0xFF for b in jbuf)
        offset += n
    return data

def _add_bookmark(bm_mgr, space, addr_hex, category, comment):
    if bm_mgr is None:
        return
    try:
        addr = space.getAddress(int(addr_hex, 16))
        bm_mgr.setBookmark(addr, "Analysis", category, comment)
    except Exception:
        pass

def _collect_xref_counts(prog):
    ref_mgr = prog.getReferenceManager()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    mem = prog.getMemory()
    counts = {{}}
    for addr_val, name in CRITICAL_XREF_TARGETS + AUDIT_MMIO_CONSTANTS:
        addr = space.getAddress(addr_val)
        if mem.getBlock(addr) is None:
            continue
        refs = []
        it = ref_mgr.getReferencesTo(addr)
        while it.hasNext() and len(refs) < 32:
            refs.append("0x%08X" % (it.next().getFromAddress().getOffset() & 0xFFFFFFFF))
        counts[name] = {{"target": "0x%08X" % addr_val, "xref_count": len(refs), "callers": refs}}
    return counts

def _default_report_path(prog):
    exe_path = prog.getExecutablePath()
    if exe_path:
        parent = os.path.dirname(os.path.abspath(exe_path))
        base = os.path.basename(exe_path) + ".panadiga_audit.json"
        if os.path.isdir(parent):
            return os.path.join(parent, base)
    return os.path.join(tempfile.gettempdir(), "panadiga_ghidra_audit.json")

def run_unbrick_audit(prog=None, output_json=None):
    if prog is None:
        prog = currentProgram
    setup_memory_map(prog)
    imported = import_symbols(prog)

    mem = prog.getMemory()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    bm_mgr = prog.getBookmarkManager()
    image_base = prog.getImageBase().getOffset() & 0xFFFFFFFF

    exec_size = 0
    string_hits = {{}}
    mmio_lit_hits = {{}}
    for blk in mem.getBlocks():
        if not blk.isInitialized():
            continue
        blk_size = int(blk.getSize())
        exec_size += blk_size
        blk_base = blk.getStart().getOffset() & 0xFFFFFFFF
        raw = _read_initialized_block_u8(blk)
        for kw, addrs in scan_ascii_keywords(raw, blk_base).items():
            string_hits.setdefault(kw, []).extend(addrs)
            _add_bookmark(bm_mgr, space, addrs[0], "Panadiga-Str", kw)
        for label, addrs in scan_le32_constants(raw, blk_base).items():
            mmio_lit_hits.setdefault(label, []).extend(addrs)
            _add_bookmark(bm_mgr, space, addrs[0], "Panadiga-MMIO", label)

    stage = detect_stage_from_metadata(image_base, prog.getName(), string_hits.keys(), exec_size)
    prog061_status = classify_prog061_sites(
        _read_prog_bytes(prog, 0x{prog_sum:08X}, {orig_sum_len}),
        _read_prog_bytes(prog, 0x{prog_devinfo:08X}, {orig_devinfo_len}),
        _read_prog_bytes(prog, 0x{prog_a2cd:08X}, {orig_a2cd_len}),
    )
    main3410_status = classify_main3410_sites(
        _read_prog_bytes(prog, 0x{main_fpc_ctrl:08X}, {orig_fpc_len}),
        _read_prog_bytes(prog, 0x{main_odd_branch:08X}, {orig_odd_br_len}),
        _read_prog_bytes(prog, 0x{main_odd_status:08X}, {orig_odd_st_len}),
    )

    for site_addr in ("0x{prog_sum:08X}", "0x{prog_devinfo:08X}", "0x{prog_a2cd:08X}", "0x{main_fpc_ctrl:08X}", "0x{main_odd_branch:08X}", "0x{main_odd_status:08X}"):
        _add_bookmark(bm_mgr, space, site_addr, "Panadiga-Unbrick", "Verified Panadiga patch/audit site")

    report = {{
        "program": prog.getName(),
        "image_base": "0x%08X" % image_base,
        "initialized_bytes": exec_size,
        "detected_stage": stage,
        "symbols_annotated": imported,
        "prog061_bgo_status": prog061_status,
        "main3410_passthrough_status": main3410_status,
        "diagnostic_strings": string_hits,
        "mmio_literal_pools": mmio_lit_hits,
        "xrefs": _collect_xref_counts(prog),
    }}

    out_path = output_json or _default_report_path(prog)
    with open(out_path, "w") as fh:
        json.dump(report, fh, indent=2, sort_keys=True)

    print("=" * 72)
    print("[Panadiga Audit] Stage=%s | Base=0x%08X | PROG061=%s | MAIN3410=%s" % (
        stage, image_base, prog061_status["mode"], main3410_status["mode"]
    ))
    print("[Panadiga Audit] Strings matched: %d | MMIO literals: %d | Report: %s" % (
        len(string_hits), len(mmio_lit_hits), out_path
    ))
    print("=" * 72)
    return report

if __name__ == "__main__":
    run_unbrick_audit()
"#,
        orig_sum_beq = format_py_byte_tuple(&ORIG_SUM_BEQ),
        patch_sum_bra = format_py_byte_tuple(&PATCH_SUM_BRA),
        orig_devinfo_beq = format_py_byte_tuple(&ORIG_DEVINFO_BEQ),
        patch_devinfo_bra = format_py_byte_tuple(&PATCH_DEVINFO_BRA),
        orig_a2cd = format_py_byte_tuple(&ORIG_A2CD_HOOK),
        patch_a2cd_auto = format_py_byte_tuple(&PATCH_A2CD_AUTOFORMAT),
        patch_a2cd_mon = format_py_byte_tuple(&PATCH_A2CD_UART_MONITOR),
        orig_fpc = format_py_byte_tuple(&ORIG_FPC_STATUS_CTRL),
        patch_fpc = format_py_byte_tuple(&PATCH_FPC_STATUS_CTRL),
        orig_odd_br = format_py_byte_tuple(&ORIG_ODD_STATE_BRANCH),
        patch_odd_br = format_py_byte_tuple(&PATCH_ODD_STATE_BRANCH),
        orig_odd_st = format_py_byte_tuple(&ORIG_ODD_INIT_STATUS),
        patch_odd_st = format_py_byte_tuple(&PATCH_ODD_INIT_STATUS),
        sdcd_bus_cfg = AM34_SDCD_BUS_CFG_VA,
        sc0_txb = AM34_SC0_BASE + 0x08,
        sc0_str = AM34_SC0_BASE + 0x0C,
        nor_bgo = AM34_FLASH_BGO_BASE,
        davis_pri = AM34_DAVIS_PRI_MMIO_BASE,
        davis_sec = AM34_DAVIS_SEC_MMIO_BASE,
        boot_uart_getc = BOOT_UART0_GETC_VA,
        boot_getline = BOOT_CONSOLE_GETLINE_VA,
        boot_sd1 = BOOT_GET_SD_STATUS_VA,
        boot_sd2 = BOOT_SECOND_SD_STATUS_VA,
        prog_nfrom = PROG061_NFROM_FORMAT_VA,
        main_lba48 = MAIN_LBA48_CLAMP_VA,
        boot_base = AM34_BOOT_BIN_BASE,
        prog_base = PROG061_BASE_VA,
        prog_size = PROG061_EXPECTED_SIZE,
        main3060_size = MAIN3060_EXPECTED_SIZE,
        main3410_size = EXPECTED_MAIN_3410PQ_SIZE,
        main_base = MAIN_BASE_VA,
        drv_base = AM34_DRV_S211_BASE,
        drv_size = DRV_S211_EXPECTED_SIZE,
        flash_base = AM34_FLASH_BASE,
        nor_uncached = AM34_FLASH_UNCACHED_BASE,
        flash_size = AM34_FLASH_SIZE,
        prog_sum = PROG061_BASE_VA + OFF_BGO_SUM_BEQ as u32,
        prog_devinfo = PROG061_BASE_VA + OFF_DEVINFO_CHECK_BEQ as u32,
        prog_a2cd = PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
        main_fpc_ctrl = MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32,
        main_odd_branch = MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32,
        main_odd_status = MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32,
        orig_sum_len = ORIG_SUM_BEQ.len(),
        orig_devinfo_len = ORIG_DEVINFO_BEQ.len(),
        orig_a2cd_len = ORIG_A2CD_HOOK.len(),
        orig_fpc_len = ORIG_FPC_STATUS_CTRL.len(),
        orig_odd_br_len = ORIG_ODD_STATE_BRANCH.len(),
        orig_odd_st_len = ORIG_ODD_INIT_STATUS.len(),
    )
}

/// Exports all Ghidra scripts to the target directory.
pub fn export_all_ghidra_scripts(target_dir: &Path) -> Result<()> {
    if !target_dir.exists() {
        fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed creating directory {:?}", target_dir))?;
    }

    let map_path = target_dir.join("ghidra_am34_memory_map.py");
    let sym_path_en = target_dir.join("ghidra_import_symbols_panadiga.py");
    let sym_path_pt = target_dir.join("ghidra_importar_simbolos_panadiga.py");
    let audit_path = target_dir.join("ghidra_auto_unbrick_audit.py");

    fs::write(&map_path, generate_ghidra_memory_map())
        .with_context(|| format!("Failed writing {:?}", map_path))?;
    fs::write(&sym_path_en, generate_ghidra_symbols())
        .with_context(|| format!("Failed writing {:?}", sym_path_en))?;
    fs::write(&audit_path, generate_ghidra_auto_unbrick_audit())
        .with_context(|| format!("Failed writing {:?}", audit_path))?;

    let wrapper_pt = r#"# Ghidra Script: Panasonic DMR-EH55 Symbol Importer (Backward-Compatibility Wrapper)
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Jython 2.7 + Python 3 compatible)
from __future__ import print_function

import os
import runpy

_target = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ghidra_import_symbols_panadiga.py")
runpy.run_path(_target, init_globals={"currentProgram": globals().get("currentProgram")}, run_name="__main__")
"#;
    fs::write(&sym_path_pt, wrapper_pt)
        .with_context(|| format!("Failed writing {:?}", sym_path_pt))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ghidra_scripts_use_verified_sc0_and_jython_safe_syntax() {
        let map_py = generate_ghidra_memory_map();
        let sym_py = generate_ghidra_symbols();
        let audit_py = generate_ghidra_auto_unbrick_audit();

        assert!(map_py.contains("SC0_UART0_MMIO") && map_py.contains("0xD4002000"));
        assert!(map_py.contains("SDCD_HOST_MMIO") && map_py.contains("0x89001800"));
        assert!(map_py.contains("FLASH_NOR_CS1_BGO") && map_py.contains("0x84200000"));
        assert!(sym_py.contains("SC0TXB_UART0_TX_BYTE") && sym_py.contains("0xD4002008"));
        assert!(sym_py.contains("SDCD_BUS_WIDTH_CFG") && sym_py.contains("0x89001850"));
        assert!(!sym_py.contains("0xC0000100"));

        for script in [&map_py, &sym_py, &audit_py] {
            assert!(!script.contains("from pathlib import"));
            assert!(!script.contains("0xFFFFFFFFL"));
        }
    }

    #[test]
    fn export_all_writes_valid_scripts_including_audit() {
        let dir = std::env::temp_dir().join(format!("panadiga_ghidra_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        export_all_ghidra_scripts(&dir).expect("export all");
        for file in [
            "ghidra_am34_memory_map.py",
            "ghidra_import_symbols_panadiga.py",
            "ghidra_importar_simbolos_panadiga.py",
            "ghidra_auto_unbrick_audit.py",
        ] {
            assert!(dir.join(file).is_file(), "missing {}", file);
        }
        let wrapper = fs::read_to_string(dir.join("ghidra_importar_simbolos_panadiga.py")).unwrap();
        assert!(!wrapper.contains("pathlib"));
        let _ = fs::remove_dir_all(&dir);
    }
}
