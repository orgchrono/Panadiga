//! AM34 / UniPhier hardware bus regions, MMIO bases, and structured SSOT memory map.

/// AM34 Internal Work SRAM Base Physical Address (0x0000_0000).
pub const AM34_INTERNAL_SRAM_BASE: u32 = 0x0000_0000;
/// AM34 Internal Work SRAM Size (256 Kilobytes).
pub const AM34_INTERNAL_SRAM_SIZE: u32 = 0x0004_0000;
pub const AM34_SRAM_BASE: u32 = AM34_INTERNAL_SRAM_BASE;
pub const AM34_SRAM_SIZE: u32 = AM34_INTERNAL_SRAM_SIZE;

/// RAM Loader Injection Physical Address (0x0000_4000).
pub const AM34_SRAM_LOADER_BASE: u32 = 0x0000_4000;
/// RAM Loader Memory Size (4 Kilobytes).
pub const AM34_SRAM_LOADER_SIZE: u32 = 0x0000_1000;

/// Core MMIO Peripheral Bus Space (0x1000_0000).
pub const AM34_MMIO_BASE: u32 = 0x1000_0000;
/// Core MMIO Space Size (64 Kilobytes).
pub const AM34_MMIO_SIZE: u32 = 0x0001_0000;

/// External System SDRAM Base Address (0x4000_0000).
pub const AM34_SDRAM_BASE: u32 = 0x4000_0000;
/// External System SDRAM Size (64 Megabytes).
pub const AM34_SDRAM_SIZE: u32 = 0x0400_0000;

/// Spansion NOR Flash Hardware Mirror Base Address (0x8000_0000).
pub const AM34_FLASH_BASE: u32 = 0x8000_0000;
pub const AM34_FLASH_BUS_BASE: u32 = AM34_FLASH_BASE;
/// Spansion NOR Flash Total Memory Aperture (8 Megabytes).
pub const AM34_FLASH_SIZE: u32 = 0x0080_0000;
/// Spansion NOR Flash Protected IPL / Kernel Size (SA0..SA133 = 8,128 Kilobytes).
pub const AM34_FLASH_IPL_SIZE: u32 = 0x007F_0000;

/// NVRAM Top-Boot Partition Base Physical Address (0x807F_0000).
pub const AM34_NVRAM_BASE: u32 = 0x807F_0000;
pub const AM34_NVRAM_BUS_BASE: u32 = AM34_NVRAM_BASE;
/// NVRAM Parameter Storage Area Size (64 Kilobytes).
pub const AM34_NVRAM_SIZE: u32 = 0x0001_0000;

/// Panasonic UniPhier ASIC MMIO Base Physical Address (0xC000_0000).
pub const AM34_ASIC_MMIO_BASE: u32 = 0xC000_0000;
/// UniPhier System MMIO Space Size (64 Kilobytes).
pub const AM34_ASIC_MMIO_SIZE: u32 = 0x0001_0000;

/// PATA / IDE Controller Registers Base Physical Address (0xD400_0000).
pub const AM34_PATA_MMIO_BASE: u32 = 0xD400_0000;
/// PATA / IDE Controller Space Size (4 Kilobytes).
pub const AM34_PATA_MMIO_SIZE: u32 = 0x0000_1000;

/// Stage-1 `/BOOT.BIN` SDRAM Execution Window (0x4400_0000, 256 KiB).
pub const AM34_BOOT_BIN_BASE: u32 = 0x4400_0000;
pub const AM34_BOOT_BIN_SIZE: u32 = 0x0004_0000;

/// Spansion NOR Flash Uncached Hardware Aperture (CS1, 0x8400_0000..0x8480_0000).
pub const AM34_FLASH_UNCACHED_BASE: u32 = 0x8400_0000;
/// Spansion NOR BGO Partition Base inside Uncached Aperture (`0x8420_0000..0x8440_0000`, 32x64 KiB).
pub const AM34_FLASH_BGO_BASE: u32 = 0x8420_0000;
pub const AM34_FLASH_BGO_SIZE: u32 = 0x0020_0000;
/// Spansion NOR Main Application Region inside Uncached Aperture (`0x8440_0000..0x847F_0000`).
pub const AM34_FLASH_UNCACHED_APP_BASE: u32 = 0x8440_0000;
pub const AM34_FLASH_UNCACHED_APP_SIZE: u32 = 0x003F_0000;
/// NVRAM Top-Boot Partition Uncached Aperture (`0x847F_0000..0x8480_0000`).
pub const AM34_NVRAM_UNCACHED_BASE: u32 = 0x847F_0000;

/// `DAVIS` Primary IDE / ATAPI Channel (`ch=0, dev=0` ODD) MMIO Base (`0x8900_0500`).
pub const AM34_DAVIS_PRI_MMIO_BASE: u32 = 0x8900_0500;
/// `DAVIS` Secondary IDE Channel (`ch=1, dev=0` HDD MASTER) MMIO Base (`0x8900_0D00`).
pub const AM34_DAVIS_SEC_MMIO_BASE: u32 = 0x8900_0D00;
pub const AM34_DAVIS_MMIO_SIZE: u32 = 0x0000_0100;

/// `SDCD` SD/MMC Host Controller MMIO Base (`0x8900_1800`) and 1-bit/4-bit Bus Width Register (`0x8900_1850`).
pub const AM34_SDCD_MMIO_BASE: u32 = 0x8900_1800;
pub const AM34_SDCD_MMIO_SIZE: u32 = 0x0000_0100;
pub const AM34_SDCD_BUS_CFG_VA: u32 = 0x8900_1850;

/// Uncached System SDRAM Mirror Base (`0x9000_0000`) and `DRV S211` Decompression Target (`0x9020_2000`).
pub const AM34_SDRAM_UNCACHED_BASE: u32 = 0x9000_0000;
pub const AM34_DRV_S211_BASE: u32 = 0x9020_2000;

/// UniPhier `SC0` UART0 Peripheral Base (`0xD400_2000`: `SC0CTR/+00`, `SC0ICR/+04`, `SC0TXB/+08`, `SC0RXB/+09`, `SC0STR/+0C`).
pub const AM34_SC0_BASE: u32 = 0xD400_2000;
pub const AM34_SC0_SIZE: u32 = 0x0000_0020;
pub const AM34_SC0_TXB_VA: u32 = AM34_SC0_BASE + 0x08;
pub const AM34_SC0_RXB_VA: u32 = AM34_SC0_BASE + 0x09;
pub const AM34_SC0_STR_VA: u32 = AM34_SC0_BASE + 0x0C;

/// Standard virtual load bases for unpacked Panasonic `PROG061` and `MAIN` modules in SDRAM.
pub const AM34_PROG061_BASE_VA: u32 = 0x5040_0000;
pub const AM34_MAIN_BASE_VA: u32 = 0x5700_0000;

/// Expected unpacked binary sizes for audited `MAIN` firmware builds (`3410PQ` & `3060NP`).
pub const EXPECTED_MAIN_3410PQ_SIZE_BYTES: usize = 13_003_052;
pub const EXPECTED_MAIN_3060NP_SIZE_BYTES: usize = 13_427_852;

/// Strongly-typed hardware memory region descriptor for Panasonic Matsushita AM34 / MN103S.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryRegionDefinition {
    pub name: &'static str,
    pub start: u32,
    pub end: u32,
    pub size_kb: usize,
    pub perms: &'static str,
}

impl MemoryRegionDefinition {
    /// Formats the region descriptor into standard JSON format for CLI and API consumers.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "start": format!("0x{:08X}", self.start),
            "end": format!("0x{:08X}", self.end),
            "size_kb": self.size_kb,
            "perms": self.perms
        })
    }
}

/// Official Matsushita AM34 / MN103S memory map regions.
pub const AM34_SYSTEM_MEMORY_REGIONS: &[MemoryRegionDefinition] = &[
    MemoryRegionDefinition {
        name: "INTERNAL_SRAM",
        start: 0x0000_0000,
        end: 0x0003_FFFF,
        size_kb: 256,
        perms: "rwx",
    },
    MemoryRegionDefinition {
        name: "CORE_MMIO",
        start: 0x1000_0000,
        end: 0x1000_FFFF,
        size_kb: 64,
        perms: "rw-",
    },
    MemoryRegionDefinition {
        name: "MAIN_SDRAM",
        start: 0x4000_0000,
        end: 0x43FF_FFFF,
        size_kb: 65536,
        perms: "rwx",
    },
    MemoryRegionDefinition {
        name: "FLASH_NOR_IPL",
        start: 0x8000_0000,
        end: 0x807E_FFFF,
        size_kb: 8128,
        perms: "r-x",
    },
    MemoryRegionDefinition {
        name: "NVRAM_U99",
        start: 0x807F_0000,
        end: 0x807F_FFFF,
        size_kb: 64,
        perms: "rw-",
    },
    MemoryRegionDefinition {
        name: "SYSTEM_MMIO",
        start: 0xC000_0000,
        end: 0xC000_3FFF,
        size_kb: 16,
        perms: "rw-",
    },
    MemoryRegionDefinition {
        name: "AM34_OCD_MMIO",
        start: 0xC000_4000,
        end: 0xC000_40FF,
        size_kb: 1,
        perms: "rw-",
    },
    MemoryRegionDefinition {
        name: "ATA_IDE_MMIO",
        start: 0xD400_0000,
        end: 0xD400_0FFF,
        size_kb: 4,
        perms: "rw-",
    },
];

/// Returns all AM34 memory map regions as a JSON array value.
pub fn get_am34_memory_map_json() -> serde_json::Value {
    let list: Vec<serde_json::Value> = AM34_SYSTEM_MEMORY_REGIONS
        .iter()
        .map(|r| r.to_json_value())
        .collect();
    serde_json::Value::Array(list)
}

/// Formats the AM34 memory map architecture into a clean ASCII table.
pub fn format_memory_map_table() -> String {
    let mut out = String::new();
    out.push_str(
        "+------------------+---------------+---------------+-----------+-------------+\n",
    );
    out.push_str(
        "| Region           | Start Address | End Address   | Size (KB) | Permissions |\n",
    );
    out.push_str(
        "+------------------+---------------+---------------+-----------+-------------+\n",
    );
    for r in AM34_SYSTEM_MEMORY_REGIONS {
        out.push_str(&format!(
            "| {:<16} | 0x{:08X}    | 0x{:08X}    | {:>9} | {:<11} |\n",
            r.name, r.start, r.end, r.size_kb, r.perms
        ));
    }
    out.push_str(
        "+------------------+---------------+---------------+-----------+-------------+\n",
    );
    out
}
