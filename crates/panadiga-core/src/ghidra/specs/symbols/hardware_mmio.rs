//! AM34 reset/exception vectors, Spansion NOR/NVRAM sectors, SC0 UART, DAVIS/SDCD, and ASIC/PATA MMIO symbols.

use super::super::SymbolSpec;
use super::sym;
use crate::constants::{
    AM34_ASIC_MMIO_BASE, AM34_DAVIS_PRI_MMIO_BASE, AM34_DAVIS_SEC_MMIO_BASE, AM34_DRV_S211_BASE,
    AM34_EH55_BGO_TABLE_VA, AM34_EH55_DRV_EXTRHEAD_BASE, AM34_EH55_FLASH_BGO_BASE,
    AM34_EH55_UPPER_EEMUL_BASE, AM34_FLASH_BASE, AM34_FLASH_BGO_BASE, AM34_FLASH_UNCACHED_BASE,
    AM34_NVRAM_BASE, AM34_NVRAM_UNCACHED_BASE, AM34_PATA_MMIO_BASE,
    AM34_PROG061_SDRAM_UNCACHED_BASE, AM34_SC0_BASE, AM34_SDCD_BUS_CFG_VA, AM34_SDCD_MMIO_BASE,
    AM34_SRAM_LOADER_BASE,
};

pub(super) const SPECS: &[SymbolSpec] = &[
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
        "Spansion S29JL064H CS1 Uncached Base (DUMPNOR 0x84000000)",
    ),
    sym(
        AM34_EH55_DRV_EXTRHEAD_BASE,
        "EH55_DRV_EXTRHEAD_BASE",
        "Real DMR-EH55 IC5001 EXTRHEAD DRV Container (SA15..SA25, 0x84080040 -> 0x90202000)",
    ),
    sym(
        AM34_EH55_FLASH_BGO_BASE,
        "FLASH_NOR_CS1_EH55_BGO",
        "Real DMR-EH55 Loader ver0.61 BGO/[nfrom] Start (0x84130000..0x843FFFFF, 45 x 64 KiB SA26..SA70)",
    ),
    sym(
        AM34_FLASH_BGO_BASE,
        "FLASH_NOR_BGO_START",
        "PROG061 / Loader ver0.63 BGO Flash Partition Start (0x84200000..0x843FFFFF, 32 x 64 KiB, NFROM)",
    ),
    sym(
        AM34_EH55_UPPER_EEMUL_BASE,
        "UPPER_EEMUL_BASE",
        "Real DMR-EH55 Upper EEMUL NOR Flash FS Start (SA71..SA141, 0x84400000..0x847FFFFF, 1284 headers)",
    ),
    sym(
        AM34_NVRAM_UNCACHED_BASE,
        "NVRAM_UNCACHED_BASE",
        "NVRAM Uncached CS1 Mirror (0x847F0000..0x84800000)",
    ),
    sym(
        AM34_EH55_BGO_TABLE_VA,
        "EH55_BGO_GEOMETRY_TABLE",
        "Real DMR-EH55 Loader ver0.61 BGO Geometry Table (45 x 64 KiB, 1 P1 + 40 P2 + 4 spares, max PID 0x0F24)",
    ),
    sym(
        AM34_PROG061_SDRAM_UNCACHED_BASE,
        "PROG061_SDRAM_UNCACHED",
        "Uncached SDRAM Alias of 0x50400000 PROG061 (used by PANADMP1 0x9043950E & RAM stubs 0x90439B40)",
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
];
