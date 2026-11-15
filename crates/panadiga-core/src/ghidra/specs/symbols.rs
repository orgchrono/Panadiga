//! Verified SSOT symbol catalog across NOR IPL, BOOT, PROG061, MAIN, and MMIO.

use super::{
    BOOT_ACMD6_SWITCH_VA, BOOT_CONSOLE_GETLINE_VA, BOOT_FEISTEL_SBOX_VA, BOOT_GET_SD_STATUS_VA,
    BOOT_SECOND_SD_STATUS_VA, BOOT_UART0_GETC_VA, BOOT_UNCOMPRESS_VA, MAIN_DAVIS_TOPOLOGY_VA,
    MAIN_LBA48_CLAMP_VA, PROG061_BGO_ERROR_EXIT_VA, PROG061_BGO_MARK_OK_VA,
    PROG061_CTRL_INFO_INIT_VA, PROG061_DAVIS_TOPOLOGY_VA, PROG061_FEISTEL_SBOX_VA, SymbolSpec,
};
use crate::constants::*;
use crate::re::bgo_unbrick::{
    OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, PROG061_BASE_VA,
    PROG061_NFROM_FORMAT_VA,
};
use crate::re::main_passthrough::{
    DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_ISR_CTX_PTR_VA, DEV_SERIAL_STATE_BASE_VA, MAIN_BASE_VA,
    OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
};
use crate::re::prog061_uart_monitor::PROG061_UART_MONITOR_VA;

const fn sym(address: u32, name: &'static str, comment: &'static str) -> SymbolSpec {
    SymbolSpec {
        address,
        name,
        comment,
        is_primary: true,
    }
}

/// Complete verified SSOT symbol catalog across NOR IPL, BOOT, PROG061, MAIN, and MMIO.
pub fn symbol_specs() -> Vec<SymbolSpec> {
    vec![
        // AM34 Reset and Interrupt Vectors
        sym(
            AM34_FLASH_BASE,
            "_start_reset_vector",
            "AM34 Reset Vector - Cold Boot Entrypoint",
        ),
        sym(
            AM34_FLASH_BASE + 0x04,
            "vector_nmi",
            "Non-Maskable Interrupt Handler",
        ),
        sym(
            AM34_FLASH_BASE + 0x08,
            "vector_trap",
            "Software Trap Exception Handler",
        ),
        sym(
            AM34_FLASH_BASE + 0x0C,
            "vector_illegal_instr",
            "Illegal Instruction Exception Handler",
        ),
        // NVRAM 8KB Micro-Sectors (SA134..SA141) + Uncached CS1 NOR/BGO Mirrors
        sym(
            AM34_NVRAM_BASE,
            "NVRAM_SECTOR_SA134",
            "NVRAM Micro-Sector 0: System Configuration & Boot Flags",
        ),
        sym(
            AM34_NVRAM_BASE + 0x2000,
            "NVRAM_SECTOR_SA135",
            "NVRAM Micro-Sector 1: Tuner Channel Map & Presets",
        ),
        sym(
            AM34_NVRAM_BASE + 0x4000,
            "NVRAM_SECTOR_SA136",
            "NVRAM Micro-Sector 2: Scheduled Timer Recordings & EPG",
        ),
        sym(
            AM34_NVRAM_BASE + 0x6000,
            "NVRAM_SECTOR_SA137",
            "NVRAM Micro-Sector 3: User Preferences & AV Modes",
        ),
        sym(
            AM34_NVRAM_BASE + 0x8000,
            "NVRAM_SECTOR_SA138",
            "NVRAM Micro-Sector 4: Optical Drive Pickup Laser Calibration",
        ),
        sym(
            AM34_NVRAM_BASE + 0xA000,
            "NVRAM_SECTOR_SA139",
            "NVRAM Micro-Sector 5: HDD Serial Binding & Volume Identification",
        ),
        sym(
            AM34_NVRAM_BASE + 0xC000,
            "NVRAM_SECTOR_SA140",
            "NVRAM Micro-Sector 6: Factory Diagnostics & Self-Test Flags",
        ),
        sym(
            AM34_NVRAM_BASE + 0xE000,
            "NVRAM_SECTOR_SA141_U99",
            "NVRAM Micro-Sector 7: Locus of U99 Corrupted Hardware Error Logs",
        ),
        sym(
            AM34_FLASH_UNCACHED_BASE,
            "FLASH_NOR_UNCACHED_BASE",
            "Spansion S29GL064 CS1 Uncached Base (DUMPNOR 0x84000000)",
        ),
        sym(
            AM34_FLASH_BGO_BASE,
            "FLASH_NOR_BGO_START",
            "BGO Flash Partition Start (32 x 64 KiB, NFROM)",
        ),
        sym(
            AM34_NVRAM_UNCACHED_BASE,
            "NVRAM_UNCACHED_BASE",
            "NVRAM Uncached CS1 Mirror (0x847F0000..0x84800000)",
        ),
        // UniPhier SC0 UART0 (Real P7501 Diagnostic Serial Controller at 0xD4002000)
        sym(
            AM34_SC0_BASE,
            "SC0CTR_UART0_CTRL",
            "AM34 SC0 UART0 Control Register (P7501 115200 8N1)",
        ),
        sym(
            AM34_SC0_BASE + 0x04,
            "SC0ICR_UART0_INT_CTRL",
            "AM34 SC0 UART0 Interrupt Control Register",
        ),
        sym(
            AM34_SC0_BASE + 0x08,
            "SC0TXB_UART0_TX_BYTE",
            "AM34 SC0 UART0 Transmit Byte Register",
        ),
        sym(
            AM34_SC0_BASE + 0x09,
            "SC0RXB_UART0_RX_BYTE",
            "AM34 SC0 UART0 Receive Byte Register",
        ),
        sym(
            AM34_SC0_BASE + 0x0C,
            "SC0STR_UART0_STATUS",
            "AM34 SC0 UART0 Status Register (mask 0xA0 TX ready)",
        ),
        // DAVIS IDE / ATAPI & SDCD SD-Card Host Controllers
        sym(
            AM34_DAVIS_PRI_MMIO_BASE,
            "DAVIS_IDE_PRI_ODD_CH0",
            "DAVIS Primary IDE Channel (ch=0, dev=0 DVD-RAM ODD ATAPI)",
        ),
        sym(
            AM34_DAVIS_SEC_MMIO_BASE,
            "DAVIS_IDE_SEC_HDD_CH1",
            "DAVIS Secondary IDE Channel (ch=1, dev=0 HDD MASTER)",
        ),
        sym(
            AM34_DAVIS_SEC_MMIO_BASE + 0x38,
            "DAVIS_IDE_SEC_DEV_SEL",
            "DAVIS Secondary IDE Device/Head Select (bit 4 DEV)",
        ),
        sym(
            AM34_SDCD_MMIO_BASE,
            "SDCD_HOST_BASE",
            "SDCD SD/MMC Host Controller MMIO Base (P6801 Slot)",
        ),
        sym(
            AM34_SDCD_BUS_CFG_VA,
            "SDCD_BUS_WIDTH_CFG",
            "SDCD 1-bit (0x80B8) vs 4-bit (&= 0x7FFF) Bus Width Register",
        ),
        // UniPhier ASIC & Legacy PATA Placeholders + DRV S211 / SRAM Loader
        sym(
            AM34_ASIC_MMIO_BASE,
            "MMIO_CHIP_ID",
            "Matsushita AM34 / MN103S Silicon Revision ID",
        ),
        sym(
            AM34_ASIC_MMIO_BASE + 0x04,
            "MMIO_RESET_CTRL",
            "Subsystem Reset Register (Software Reset Controller)",
        ),
        sym(
            AM34_ASIC_MMIO_BASE + 0x10,
            "MMIO_PLL_CLOCK_CFG",
            "PLL Multiplier and Clock Gating Control",
        ),
        sym(
            AM34_PATA_MMIO_BASE,
            "PATA_DATA_16BIT",
            "PATA 16-bit PIO / DMA Data Register",
        ),
        sym(
            AM34_PATA_MMIO_BASE + 0x01,
            "PATA_ERROR_FEATURES",
            "PATA Error Register (Read) / Features Register (Write)",
        ),
        sym(
            AM34_PATA_MMIO_BASE + 0x02,
            "PATA_SECTOR_COUNT",
            "PATA Sector Count Register",
        ),
        sym(
            AM34_PATA_MMIO_BASE + 0x07,
            "PATA_STATUS_COMMAND",
            "PATA Status Register (Read) / Command Register (Write)",
        ),
        sym(
            AM34_SRAM_LOADER_BASE,
            "RAM_LOADER_ENTRY",
            "Panadiga In-Circuit SRAM Loader Entrypoint",
        ),
        sym(
            AM34_DRV_S211_BASE,
            "DRV_S211_DECOMP_BASE",
            "DRV S211 F/E Optical Drive Decompressed SDRAM Target",
        ),
        // Stage 1: /BOOT.BIN (NOR IPL at 0x44000000)
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
            BOOT_CONSOLE_GETLINE_VA,
            "boot_console_getline_dead",
            "Dead BootROM console reader (0 active callers)",
        ),
        sym(
            BOOT_UNCOMPRESS_VA,
            "boot_uncompress_fe",
            "Decompresses F/E DRV image into 0x90202000 & checks Adler/sum",
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
        // Stage 2: PROG061 / DIGADIAG.FRM (0x50400000)
        sym(
            PROG061_BASE_VA,
            "prog061_fvu_entry",
            "Stage 2 PROG061 FVU Entrypoint (DIGADIAG.FRM dst=0x50400000)",
        ),
        sym(
            PROG061_NFROM_FORMAT_VA,
            "prog061_nfrom_format",
            "Orphan factory NFROM/BGO formatter (0 static callers; invokable from 0x5040A2CD)",
        ),
        sym(
            PROG061_BASE_VA + OFF_DEVINFO_CHECK_BEQ as u32,
            "prog061_devinfo_check_beq",
            "Site 2 (0x50405097): DeviceInfo INFO guard (beq+21 -> bra+21)",
        ),
        sym(
            PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
            "prog061_bgo_init_fail_hook",
            "Site 3 (0x5040A2CD): 20B BGO init failure branch -> AUTO_FORMAT_BGO / PANAMON1",
        ),
        sym(
            PROG061_CTRL_INFO_INIT_VA,
            "prog061_ctrl_info_init",
            "Control information init continuation point after BGO init",
        ),
        sym(
            PROG061_BGO_MARK_OK_VA,
            "prog061_bgo_mark_ok",
            "BGO bypass OK return point (d0 == 0)",
        ),
        sym(
            PROG061_BGO_ERROR_EXIT_VA,
            "prog061_bgo_error_exit",
            "BGO factory error return point (d0 == -1)",
        ),
        sym(
            PROG061_DAVIS_TOPOLOGY_VA,
            "prog061_davis_ide_topology",
            "Fixed DAVIS IDE topology table acStack_70 = {2,0,1,0} (Sec Master HDD)",
        ),
        sym(
            PROG061_BASE_VA + OFF_BGO_SUM_BEQ as u32,
            "prog061_bgo_sum_beq",
            "Site 1 (0x504230F0): bgo_sum_23058 N(N+1)/2 checksum branch (beq+10 -> bra+10)",
        ),
        sym(
            PROG061_FEISTEL_SBOX_VA,
            "prog061_feistel_sbox_256b",
            "Verified 256-byte Feistel S-Box substitution table in PROG061",
        ),
        sym(
            PROG061_UART_MONITOR_VA,
            "prog061_panamon1_uart_monitor",
            "Verified 0xFF slack entrypoint for PANAMON1 UART0 rescue monitor",
        ),
        // Stage 3: EH55_MAIN_3410PQ.bin (0x57000000) & _dev_SERIAL RSP
        sym(
            MAIN_BASE_VA,
            "main_app_base",
            "Stage 3 MAIN Application Base (EH55_MAIN_3410PQ.bin)",
        ),
        sym(
            MAIN_DAVIS_TOPOLOGY_VA,
            "main_davis_ide_topology",
            "Fixed MAIN DAVIS IDE topology acStack_70 = {2,0,1,0}",
        ),
        sym(
            MAIN_LBA48_CLAMP_VA,
            "main_hdd_cmd_lba48_clamp",
            "src/hdd_cmd.c >200 GB LBA48 capacity clamp / patch site",
        ),
        sym(
            MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32,
            "main_odd_state_branch",
            "TBC Site 2 (0x574F7B8C): ODD state check -> jmp16 +0x00D0 to 0x574F7C5C",
        ),
        sym(
            MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32,
            "main_odd_init_status",
            "TBC Site 3 (0x574F7C5C): ODD init status forced ready (clr d0; nop x5)",
        ),
        sym(
            MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32,
            "main_fpc_status_ctrl",
            "TBC Site 1 (0x5771ABFB): FPC status control -> mov 1, d0; ret",
        ),
        sym(
            DEV_SERIAL_STATE_BASE_VA,
            "_dev_SERIAL_state_base",
            "_dev_SERIAL UART0 GDB RSP protocol state struct (+0x60 = state 0..4)",
        ),
        sym(
            DEV_SERIAL_CALLBACK_VA,
            "_dev_SERIAL_callback_ptr",
            "_dev_SERIAL registered UART0 RSP callback pointer (+0x40)",
        ),
        sym(
            DEV_SERIAL_ISR_CTX_PTR_VA,
            "_dev_SERIAL_isr_ctx_ptr",
            "Pointer to saved ISR register frame (saved PC at +0x64)",
        ),
    ]
}
