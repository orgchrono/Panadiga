//! Complete deterministic memory region table for AM34 / VEP79138K.

use super::{MAIN_APP_REGION_SIZE, ModelMemoryRegionSpec};
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

    if let Some(layout) = &resolved.nor_layout {
        let nor_base = resolved
            .soc
            .as_ref()
            .and_then(|soc| soc.bus_windows.iter().find(|w| w.name == "nor_cached"))
            .map_or_else(
                || {
                    if resolved
                        .soc_core
                        .as_ref()
                        .is_some_and(|core| !core.isa.to_ascii_uppercase().contains("MN103"))
                    {
                        0x0000_0000
                    } else {
                        0x8000_0000
                    }
                },
                |w| w.base,
            );
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
                nor_base.wrapping_add(reg.start_offset),
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
