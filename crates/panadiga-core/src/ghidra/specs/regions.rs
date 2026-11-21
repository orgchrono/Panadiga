//! Complete deterministic memory region table for AM34 / VEP79138K.

use super::{MAIN_APP_REGION_SIZE, MemoryRegionSpec, ModelMemoryRegionSpec};
use crate::constants::*;
use crate::re::bgo_unbrick::{PROG061_BASE_VA, PROG061_EXPECTED_SIZE};
use crate::re::main_passthrough::{
    DEV_SERIAL_ISR_CTX_PTR_VA, DEV_SERIAL_STATE_BASE_VA, MAIN_BASE_VA,
};
use crate::ssot::{HardwareProfileRegistry, NorRegionPolicy};

/// Projects Ghidra memory regions dynamically from `HardwareProfileRegistry` (`soc.bus_windows`,
/// `firmware_train` load addresses, `nor_layout`, and `soc.mmio`) for any registered DIGA model.
pub fn memory_region_specs_for_model(model_id: &str) -> anyhow::Result<Vec<ModelMemoryRegionSpec>> {
    let resolved = HardwareProfileRegistry::builtin().resolve_model(model_id)?;
    let mut specs = Vec::new();
    let mut push = |name: String,
                    start: u32,
                    size: u32,
                    read: bool,
                    write: bool,
                    execute: bool,
                    is_volatile: bool| {
        if size > 0 {
            specs.push(ModelMemoryRegionSpec {
                name,
                start,
                size,
                read,
                write,
                execute,
                is_volatile,
            });
        }
    };

    if let Some(soc) = &resolved.soc {
        for win in &soc.bus_windows {
            let is_rom = win.name.contains("nor");
            push(
                format!("BUS_{}", win.name.to_ascii_uppercase()),
                win.base,
                win.size,
                true,
                !is_rom,
                true,
                false,
            );
            if let Some(alias) = win.uncached_alias_base
                && alias != win.base
            {
                push(
                    format!("BUS_{}_UNCACHED", win.name.to_ascii_uppercase()),
                    alias,
                    win.size,
                    true,
                    !is_rom,
                    true,
                    true,
                );
            }
        }
        for (mmio_name, &addr) in &soc.mmio {
            push(
                format!("MMIO_{}", mmio_name.to_ascii_uppercase()),
                addr,
                0x0000_1000,
                true,
                true,
                false,
                true,
            );
        }
    }

    if let (Some(soc), Some(layout)) = (&resolved.soc, &resolved.nor_layout)
        && let Some(nor_win) = soc.bus_windows.iter().find(|w| w.name == "nor_cached")
    {
        for reg in &layout.regions {
            let len = reg
                .end_offset
                .saturating_sub(reg.start_offset)
                .saturating_add(1);
            let write = !matches!(
                reg.policy,
                NorRegionPolicy::ReadOnlyPermanent | NorRegionPolicy::ReadOnlyReference
            );
            push(
                format!(
                    "NOR_{:?}_{}",
                    reg.kind,
                    reg.tag.as_deref().unwrap_or("REGION").replace(' ', "_")
                )
                .to_ascii_uppercase(),
                nor_win.base.wrapping_add(reg.start_offset),
                len,
                true,
                write,
                matches!(
                    reg.kind,
                    crate::ssot::NorRegionKind::Ipl | crate::ssot::NorRegionKind::MainXip
                ),
                false,
            );
        }
    }

    if let Some(train) = &resolved.firmware_train {
        if let Some(va) = train.prog_load_va {
            push(
                "TRAIN_PROG_RAM".to_string(),
                va,
                0x0008_0000,
                true,
                true,
                true,
                false,
            );
        }
        if let Some(va) = train.main_load_va {
            push(
                "TRAIN_MAIN_RAM".to_string(),
                va,
                MAIN_APP_REGION_SIZE,
                true,
                true,
                true,
                false,
            );
        }
        if let Some(va) = train.drv_load_va {
            push(
                "TRAIN_DRV_RAM".to_string(),
                va,
                0x0020_0000,
                true,
                true,
                true,
                false,
            );
        }
    }

    Ok(specs)
}

const fn region(
    name: &'static str,
    start: u32,
    size: u32,
    read: bool,
    write: bool,
    execute: bool,
    is_volatile: bool,
) -> MemoryRegionSpec {
    MemoryRegionSpec {
        name,
        start,
        size,
        read,
        write,
        execute,
        is_volatile,
    }
}

/// Complete deterministic memory region specification for AM34 / VEP79138K.
pub fn memory_region_specs() -> Vec<MemoryRegionSpec> {
    vec![
        region(
            "INTERNAL_SRAM",
            AM34_INTERNAL_SRAM_BASE,
            AM34_INTERNAL_SRAM_SIZE,
            true,
            true,
            true,
            false,
        ),
        region(
            "CORE_MMIO",
            AM34_MMIO_BASE,
            AM34_MMIO_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "MAIN_SDRAM",
            AM34_SDRAM_BASE,
            AM34_SDRAM_SIZE,
            true,
            true,
            true,
            false,
        ),
        region(
            "BOOT_BIN_RAM",
            AM34_BOOT_BIN_BASE,
            AM34_BOOT_BIN_SIZE,
            true,
            true,
            true,
            false,
        ),
        region(
            "PROG061_FVU_RAM",
            PROG061_BASE_VA,
            PROG061_EXPECTED_SIZE as u32,
            true,
            true,
            true,
            false,
        ),
        region(
            "DEV_SERIAL_DATA",
            DEV_SERIAL_STATE_BASE_VA & 0xFFFF_F000,
            0x0000_2000,
            true,
            true,
            false,
            true,
        ),
        region(
            "DEV_SERIAL_ISR_CTX",
            DEV_SERIAL_ISR_CTX_PTR_VA & 0xFFFF_F000,
            0x0000_2000,
            true,
            true,
            false,
            true,
        ),
        region(
            "MAIN_APP_RAM",
            MAIN_BASE_VA,
            MAIN_APP_REGION_SIZE,
            true,
            true,
            true,
            false,
        ),
        region(
            "FLASH_NOR_IPL",
            AM34_FLASH_BASE,
            AM34_FLASH_IPL_SIZE,
            true,
            false,
            true,
            false,
        ),
        region(
            "NVRAM_EEMUL",
            AM34_NVRAM_BASE,
            AM34_NVRAM_SIZE,
            true,
            true,
            false,
            false,
        ),
        region(
            "FLASH_NOR_CS1_IPL",
            AM34_FLASH_UNCACHED_BASE,
            AM34_EH55_FLASH_BGO_BASE - AM34_FLASH_UNCACHED_BASE,
            true,
            false,
            true,
            false,
        ),
        region(
            "FLASH_NOR_CS1_EH55_BGO",
            AM34_EH55_FLASH_BGO_BASE,
            AM34_EH55_FLASH_BGO_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "FLASH_NOR_CS1_APP",
            AM34_FLASH_UNCACHED_APP_BASE,
            AM34_FLASH_UNCACHED_APP_SIZE,
            true,
            false,
            true,
            false,
        ),
        region(
            "NVRAM_UNCACHED",
            AM34_NVRAM_UNCACHED_BASE,
            AM34_NVRAM_SIZE,
            true,
            true,
            false,
            false,
        ),
        region(
            "AIOF_TBC_MMIO",
            AM34_AIOF_BASE,
            0x0000_1000,
            true,
            true,
            false,
            true,
        ),
        region(
            "DAVIS_CH0_RESET_MMIO",
            0x8900_0400,
            AM34_DAVIS_MMIO_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "DAVIS_IDE_PRI_MMIO",
            AM34_DAVIS_PRI_MMIO_BASE,
            AM34_DAVIS_MMIO_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "DAVIS_CH1_RESET_MMIO",
            0x8900_0C00,
            AM34_DAVIS_MMIO_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "DAVIS_IDE_SEC_MMIO",
            AM34_DAVIS_SEC_MMIO_BASE,
            AM34_DAVIS_MMIO_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "SDCD_HOST_MMIO",
            AM34_SDCD_MMIO_BASE,
            AM34_SDCD_MMIO_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "AVDD_DNR_MMIO",
            AM34_AVDD_BASE,
            0x0000_1000,
            true,
            true,
            false,
            true,
        ),
        region(
            "SDRAM_UNCACHED_LO",
            AM34_SDRAM_UNCACHED_BASE,
            AM34_PROG061_SDRAM_UNCACHED_BASE - AM34_SDRAM_UNCACHED_BASE,
            true,
            true,
            true,
            false,
        ),
        region(
            "PROG061_SDRAM_UNCACHED",
            AM34_PROG061_SDRAM_UNCACHED_BASE,
            PROG061_EXPECTED_SIZE as u32,
            true,
            true,
            true,
            false,
        ),
        region(
            "SDRAM_UNCACHED_HI",
            AM34_PROG061_SDRAM_UNCACHED_BASE + PROG061_EXPECTED_SIZE as u32,
            AM34_SDRAM_SIZE
                - (AM34_PROG061_SDRAM_UNCACHED_BASE - AM34_SDRAM_UNCACHED_BASE)
                - PROG061_EXPECTED_SIZE as u32,
            true,
            true,
            true,
            false,
        ),
        region(
            "ASIC_MMIO",
            AM34_ASIC_MMIO_BASE,
            0x0000_1000,
            true,
            true,
            false,
            true,
        ),
        region(
            "WDBC_SYS_STATUS_MMIO",
            AM34_WDBC_REG,
            0x0000_1000,
            true,
            true,
            false,
            true,
        ),
        region(
            "ASIC_MMIO_HI",
            AM34_WDBC_REG + 0x0000_1000,
            AM34_ASIC_MMIO_SIZE - 0x0000_2000,
            true,
            true,
            false,
            true,
        ),
        region(
            "ATA_IDE_MMIO",
            AM34_PATA_MMIO_BASE,
            AM34_PATA_MMIO_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "SC0_UART0_MMIO",
            AM34_SC0_BASE,
            AM34_SC0_SIZE,
            true,
            true,
            false,
            true,
        ),
        region(
            "IIC0_MMIO",
            AM34_IIC0_BASE,
            0x0000_0100,
            true,
            true,
            false,
            true,
        ),
        region(
            "IIC1_MMIO",
            AM34_IIC1_BASE,
            0x0000_0100,
            true,
            true,
            false,
            true,
        ),
        region(
            "TBUS_IC7501_MMIO",
            AM34_IC7501_TBUS_BASE,
            0x0000_0100,
            true,
            true,
            false,
            true,
        ),
    ]
}
