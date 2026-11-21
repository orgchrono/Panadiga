//! Pure immutable Ghidra memory map, symbol, MMIO, and audit specifications.
//!
//! All data definitions in this module are side-effect free and serve as the
//! Single Source of Truth (SSOT) consumed by pure string renderers.

mod regions;
mod regions_static;
mod ssot_maps;
mod symbols;

pub use regions::memory_region_specs_for_model;
pub use regions_static::memory_region_specs;
pub use ssot_maps::{generate_ghidra_ssot_maps_json, ghidra_maps_ssot_json};
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
pub const MAIN3060_EXPECTED_SIZE: usize = EXPECTED_MAIN_3060NP_SIZE_BYTES;
pub const MAIN3410_EXPECTED_SIZE: usize = EXPECTED_MAIN_3410PQ_SIZE;
pub const MAIN3060_LBA48_CLAMP_VA: u32 = 0x574E_499C;
pub const MAIN3410_DEVINFO_BEQ_VA: u32 = 0x5749_8738;
pub const MAIN3060_DEVINFO_BEQ_VA: u32 = 0x574A_01AA;
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

/// Owned memory region projection derived dynamically from `HardwareProfileRegistry` for any DIGA model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMemoryRegionSpec {
    pub name: String,
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

/// MMIO & firmware signature addresses searched as little-endian u32 literal pool constants in audit script.
pub fn audit_mmio_constants() -> Vec<LabeledAddrSpec> {
    vec![
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
            address: AM34_EH55_FLASH_BGO_BASE,
            label: "FLASH_NOR_EH55_BGO_START",
        },
        LabeledAddrSpec {
            address: AM34_FLASH_BGO_BASE,
            label: "FLASH_NOR_VER063_BGO_START",
        },
        LabeledAddrSpec {
            address: AM34_DAVIS_PRI_MMIO_BASE,
            label: "DAVIS_IDE_PRI_ODD_CH0",
        },
        LabeledAddrSpec {
            address: AM34_DAVIS_SEC_MMIO_BASE,
            label: "DAVIS_IDE_SEC_HDD_CH1",
        },
        LabeledAddrSpec {
            address: AM34_AIOF_BASE + AIOF_VMODESEL_OFF,
            label: "AIOF_VMODESEL",
        },
        LabeledAddrSpec {
            address: AM34_AVDD_BASE + AVDD_NPALCTL0_OFF,
            label: "AVDD_NPALCTL0",
        },
        LabeledAddrSpec {
            address: AM34_IC7501_TBUS_BASE,
            label: "IC7501_TBUS_CTRL",
        },
    ]
}

/// Critical function and table virtual addresses inspected for cross-references.
pub fn critical_xref_targets() -> Vec<LabeledAddrSpec> {
    vec![
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
        LabeledAddrSpec {
            address: MAIN3060_LBA48_CLAMP_VA,
            label: "main3060_hdd_cmd_lba48_clamp",
        },
        LabeledAddrSpec {
            address: MAIN3410_DEVINFO_BEQ_VA,
            label: "main3410_deviceinfo_check",
        },
        LabeledAddrSpec {
            address: MAIN3060_DEVINFO_BEQ_VA,
            label: "main3060_deviceinfo_check",
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

        // Strict zero-overlap invariant: every Ghidra memory region must be disjoint so none are skipped.
        for (i, a) in regions.iter().enumerate() {
            let a_end = u64::from(a.start) + u64::from(a.size);
            for b in &regions[i + 1..] {
                let b_end = u64::from(b.start) + u64::from(b.size);
                assert!(
                    a_end <= u64::from(b.start) || b_end <= u64::from(a.start),
                    "Overlapping Ghidra regions: {} (0x{:08X}..0x{:08X}) vs {} (0x{:08X}..0x{:08X})",
                    a.name,
                    a.start,
                    a_end,
                    b.name,
                    b.start,
                    b_end
                );
            }
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
        assert!(regions.iter().any(|r| {
            r.name == "FLASH_NOR_CS1_IPL" && r.start == 0x8400_0000 && r.size == 0x0013_0000
        }));
        assert!(regions.iter().any(|r| {
            r.name == "FLASH_NOR_CS1_EH55_BGO" && r.start == 0x8413_0000 && r.size == 0x002D_0000
        }));
        assert!(regions.iter().any(|r| {
            r.name == "PROG061_SDRAM_UNCACHED" && r.start == 0x9040_0000 && r.size == 0x0004_0000
        }));
        assert!(
            regions
                .iter()
                .any(|r| r.name == "AIOF_TBC_MMIO" && r.start == 0x8800_0000)
        );
        assert!(
            regions
                .iter()
                .any(|r| r.name == "TBUS_IC7501_MMIO" && r.start == 0xDC01_0000)
        );
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

    fn parse_hex_u32(val: &serde_json::Value) -> u32 {
        let s = val.as_str().expect("hex string");
        u32::from_str_radix(s.trim_start_matches("0x"), 16).expect("valid hex u32")
    }

    #[test]
    fn ghidra_ssot_json_matches_static_regions_and_projects_all_models() {
        let rendered = generate_ghidra_ssot_maps_json().expect("generate json");
        let root: serde_json::Value =
            serde_json::from_str(&rendered).expect("valid generated json");

        assert_eq!(root["schema_version"].as_u64(), Some(1));
        assert_eq!(root["language_id"].as_str(), Some("mn10300:LE:32:default"));
        assert_eq!(root["compiler_id"].as_str(), Some("default"));
        assert_eq!(root["default_model"].as_str(), Some("DMR-EH55"));

        let json_regions = root["am34_memory_regions"]
            .as_array()
            .expect("am34_memory_regions array");
        let rust_regions = memory_region_specs();
        assert_eq!(json_regions.len(), rust_regions.len());
        for (j, r) in json_regions.iter().zip(rust_regions.iter()) {
            assert_eq!(j["name"].as_str(), Some(r.name));
            assert_eq!(parse_hex_u32(&j["start"]), r.start);
            assert_eq!(parse_hex_u32(&j["size"]), r.size);
            assert_eq!(j["read"].as_bool(), Some(r.read));
            assert_eq!(j["write"].as_bool(), Some(r.write));
            assert_eq!(j["execute"].as_bool(), Some(r.execute));
            assert_eq!(j["is_volatile"].as_bool(), Some(r.is_volatile));
        }

        let windows = root["volatile_mmio_windows"]
            .as_array()
            .expect("volatile_mmio_windows array");
        assert_eq!(windows.len(), 5);

        let stages = root["stages"].as_array().expect("stages array");
        assert_eq!(stages.len(), 6);
        for key in [
            "BOOT.BIN",
            "eh55_loader_ver061_256k.bin",
            "eh55_frm_PROG061_BOOT.bin",
            "DRV_decomp.bin",
            "EH55_DRV_S211.bin",
            "EH55_MAIN_3410PQ.bin",
        ] {
            assert!(
                stages.iter().any(|s| s["key"].as_str() == Some(key)),
                "missing stage {key}"
            );
        }

        assert!(root["generic_stage_rules"]["filename_base_hints"].is_array());

        let model_maps = root["model_memory_maps"]
            .as_object()
            .expect("model_memory_maps object");
        for model in ["DMR-EH55", "DMR-EH50", "DMR-EH58", "DMR-ES35V"] {
            let entry = model_maps.get(model).expect("missing projected model");
            assert!(
                entry["regions"].as_array().is_some_and(|a| !a.is_empty()),
                "empty regions for {model}"
            );
        }
        let ez485v = model_maps
            .get("DMR-EZ485V")
            .expect("missing projected model DMR-EZ485V");
        assert_eq!(ez485v["soc_isa"].as_str(), Some("SPARC V8 (LSI DoMiNo)"));
        assert!(ez485v["regions"].as_array().is_some_and(|a| a.is_empty()));
    }
}
