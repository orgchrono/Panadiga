//! Pure immutable Ghidra memory map, symbol, MMIO, and audit specifications.
//!
//! All data definitions in this module are side-effect free and serve as the
//! Single Source of Truth (SSOT) consumed by pure string renderers.

mod regions;
mod symbols;

pub use regions::memory_region_specs;
pub use symbols::symbol_specs;

use crate::constants::*;
use crate::re::bgo_unbrick::PROG061_NFROM_FORMAT_VA;
use crate::re::main_passthrough::EXPECTED_MAIN_3410PQ_SIZE;

pub const BOOT_UART0_GETC_VA: u32 = 0x4400_479F;
pub const BOOT_CONSOLE_GETLINE_VA: u32 = 0x4400_488A;
pub const BOOT_UNCOMPRESS_VA: u32 = 0x4400_BCC2;
pub const BOOT_GET_SD_STATUS_VA: u32 = 0x4402_7826;
pub const BOOT_ACMD6_SWITCH_VA: u32 = 0x4402_7F79;
pub const BOOT_SECOND_SD_STATUS_VA: u32 = 0x4402_7F93;
pub const BOOT_FEISTEL_SBOX_VA: u32 = 0x4403_5D84;

pub const PROG061_CTRL_INFO_INIT_VA: u32 = 0x5040_A2E1;
pub const PROG061_BGO_MARK_OK_VA: u32 = 0x5040_A306;
pub const PROG061_BGO_ERROR_EXIT_VA: u32 = 0x5040_A311;
pub const PROG061_DAVIS_TOPOLOGY_VA: u32 = 0x5041_4908;
pub const PROG061_FEISTEL_SBOX_VA: u32 = 0x5043_5C0A;

pub const MAIN_DAVIS_TOPOLOGY_VA: u32 = 0x574D_1DE8;
pub const MAIN_LBA48_CLAMP_VA: u32 = 0x574D_D1FC;
pub const MAIN_APP_REGION_SIZE: u32 = 0x00E0_0000;
pub const MAIN3060_EXPECTED_SIZE: usize = 13_427_852;
pub const MAIN3410_EXPECTED_SIZE: usize = EXPECTED_MAIN_3410PQ_SIZE;
pub const DRV_S211_EXPECTED_SIZE: usize = 1_441_792;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegionSpec {
    pub name: &'static str,
    pub start: u32,
    pub size: u32,
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub is_volatile: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolSpec {
    pub address: u32,
    pub name: &'static str,
    pub comment: &'static str,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabeledAddrSpec {
    pub address: u32,
    pub label: &'static str,
}

pub const AUDIT_KEYWORDS: &[&str] = &[
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
];

/// MMIO addresses searched as little-endian u32 literal pool constants in audit script.
pub fn audit_mmio_constants() -> [LabeledAddrSpec; 6] {
    [
        LabeledAddrSpec {
            address: AM34_SDCD_BUS_CFG_VA,
            label: "SDCD_BUS_WIDTH_CFG",
        },
        LabeledAddrSpec {
            address: AM34_SC0_BASE + 0x08,
            label: "SC0TXB_UART0_TX_BYTE",
        },
        LabeledAddrSpec {
            address: AM34_SC0_BASE + 0x0C,
            label: "SC0STR_UART0_STATUS",
        },
        LabeledAddrSpec {
            address: AM34_FLASH_BGO_BASE,
            label: "FLASH_NOR_BGO_START",
        },
        LabeledAddrSpec {
            address: AM34_DAVIS_PRI_MMIO_BASE,
            label: "DAVIS_IDE_PRI_ODD_CH0",
        },
        LabeledAddrSpec {
            address: AM34_DAVIS_SEC_MMIO_BASE,
            label: "DAVIS_IDE_SEC_HDD_CH1",
        },
    ]
}

/// Critical function and table virtual addresses inspected for cross-references.
pub fn critical_xref_targets() -> [LabeledAddrSpec; 6] {
    [
        LabeledAddrSpec {
            address: BOOT_UART0_GETC_VA,
            label: "boot_uart0_getc_dead",
        },
        LabeledAddrSpec {
            address: BOOT_CONSOLE_GETLINE_VA,
            label: "boot_console_getline_dead",
        },
        LabeledAddrSpec {
            address: BOOT_GET_SD_STATUS_VA,
            label: "boot_get_sd_status_1bit",
        },
        LabeledAddrSpec {
            address: BOOT_SECOND_SD_STATUS_VA,
            label: "boot_get_sd_status_4bit",
        },
        LabeledAddrSpec {
            address: PROG061_NFROM_FORMAT_VA,
            label: "prog061_nfrom_format",
        },
        LabeledAddrSpec {
            address: MAIN_LBA48_CLAMP_VA,
            label: "main_hdd_cmd_lba48_clamp",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn specs_satisfy_mathematical_uniqueness_and_non_empty_invariants() {
        let regions = memory_region_specs();
        let mut region_names = HashSet::new();
        for reg in &regions {
            assert!(!reg.name.is_empty());
            assert!(reg.size > 0);
            assert!(
                reg.start.checked_add(reg.size - 1).is_some(),
                "region {} wraps u32",
                reg.name
            );
            assert!(
                region_names.insert(reg.name),
                "duplicate region {}",
                reg.name
            );
        }

        let symbols = symbol_specs();
        let mut sym_names = HashSet::new();
        for s in &symbols {
            assert!(!s.name.is_empty());
            assert!(!s.comment.is_empty());
            assert!(sym_names.insert(s.name), "duplicate symbol {}", s.name);
        }

        assert!(
            regions
                .iter()
                .any(|r| r.name == "SC0_UART0_MMIO" && r.start == 0xD400_2000)
        );
        assert!(
            regions
                .iter()
                .any(|r| r.name == "SDCD_HOST_MMIO" && r.start == 0x8900_1800)
        );
        assert!(
            regions
                .iter()
                .any(|r| r.name == "FLASH_NOR_CS1_BGO" && r.start == 0x8420_0000)
        );
        assert!(regions.iter().any(|r| {
            r.name == "FLASH_NOR_CS1_EH55_BGO" && r.start == 0x8413_0000 && r.size == 0x002D_0000
        }));
        assert!(regions.iter().any(|r| {
            r.name == "PROG061_SDRAM_UNCACHED" && r.start == 0x9040_0000 && r.size == 0x0004_0000
        }));
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "EH55_BGO_GEOMETRY_TABLE" && s.address == 0x4403_37A4)
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "UPPER_EEMUL_BASE" && s.address == 0x8440_0000)
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "SC0TXB_UART0_TX_BYTE" && s.address == 0xD400_2008)
        );
        assert!(!symbols.iter().any(|s| s.address == 0xC000_0100));
    }
}
