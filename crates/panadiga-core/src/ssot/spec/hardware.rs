//! Data-driven electrical thresholds, USB IDs, pinout, break-in modes, and AM34 memory map.

use crate::constants::{
    AM34_FLASH_BASE, AM34_NVRAM_BASE, AM34_SRAM_LOADER_BASE, DEFAULT_BAUD_RATE,
    DEFAULT_GDB_LISTEN_SOCKET, FLASH_16MB_NVRAM_SIZE_BYTES, SPANSION_NVRAM_SIZE_BYTES,
};

use super::catalog::{hardware_catalog, pt_br_ssot_locale};
use super::constants::{
    FLASH_16MB_THRESHOLD_MB, MIB_BYTES, P7501_MAX_VDD_CEILING_VOLTS, P7501_MIN_VDD_THRESHOLD_VOLTS,
    P7501_NOMINAL_VDD_VOLTS, P7501_SAFE_OPERATING_MIN_VOLTS, P7501_SWD_BUS_FREQ_HZ,
    RASPBERRY_PI_VID, RP_BOOTSEL_PID, RP2040_CDC_PID, RP2040_W_CDC_PID, RP2350_CDC_PID,
    RP2350_RISCV_CDC_PID, RP2350_W_CDC_PID,
};
use super::dto::{
    BreakInModeSpecDto, FlashGeometrySpecDto, HardwareThresholdsDto, MemoryRegionDto,
    P7501PinMappingDto, PicoUsbIdentityDto,
};

/// Derives physical byte aperture and top-boot NVRAM base address from Spansion NOR geometry.
pub(super) fn compute_flash_geometry_addresses(size_mb: u32) -> (u32, u32, u32) {
    let size_bytes = size_mb * MIB_BYTES;
    let nvram_size_bytes = if size_mb >= FLASH_16MB_THRESHOLD_MB {
        FLASH_16MB_NVRAM_SIZE_BYTES as u32
    } else {
        SPANSION_NVRAM_SIZE_BYTES as u32
    };
    let nvram_base_addr = AM34_FLASH_BASE + size_bytes - nvram_size_bytes;
    (size_bytes, nvram_base_addr, nvram_size_bytes)
}

pub(super) fn build_thresholds() -> HardwareThresholdsDto {
    HardwareThresholdsDto {
        bus_freq_hz: P7501_SWD_BUS_FREQ_HZ,
        nominal_vdd_volts: P7501_NOMINAL_VDD_VOLTS,
        min_vdd_threshold_volts: P7501_MIN_VDD_THRESHOLD_VOLTS,
        safe_operating_min_volts: P7501_SAFE_OPERATING_MIN_VOLTS,
        max_vdd_ceiling_volts: P7501_MAX_VDD_CEILING_VOLTS,
        default_baud_rate: DEFAULT_BAUD_RATE,
        default_gdb_socket: DEFAULT_GDB_LISTEN_SOCKET.to_string(),
        ram_loader_base_hex: format!("0x{AM34_SRAM_LOADER_BASE:08X}"),
        default_nvram_base_hex: format!("0x{AM34_NVRAM_BASE:08X}"),
    }
}

pub(super) fn build_usb_ids() -> PicoUsbIdentityDto {
    PicoUsbIdentityDto {
        raspberry_pi_vid: RASPBERRY_PI_VID,
        rp2040_cdc_pid: RP2040_CDC_PID,
        rp2040_w_cdc_pid: RP2040_W_CDC_PID,
        rp2350_cdc_pid: RP2350_CDC_PID,
        rp2350_w_cdc_pid: RP2350_W_CDC_PID,
        rp2350_riscv_cdc_pid: RP2350_RISCV_CDC_PID,
        bootsel_pid: RP_BOOTSEL_PID,
    }
}

pub(super) fn build_pinout() -> Vec<P7501PinMappingDto> {
    let catalog = hardware_catalog();
    let locale = pt_br_ssot_locale();
    catalog
        .pinout
        .iter()
        .map(|bp| {
            let text = locale
                .pins
                .get(&bp.key)
                .unwrap_or_else(|| panic!("Missing ssot.pins.{} in pt-BR.json", bp.key));
            P7501PinMappingDto {
                manual_pin: bp.manual_pin,
                physical_pin: bp.physical_pin,
                pin_label: text.pin_label.clone(),
                pico_gpio: bp.pico_gpio.clone(),
                pico_pin: text.pico_pin.clone(),
                signal_name: text.signal_name.clone(),
                direction: text.direction.clone(),
                voltage: text.voltage.clone(),
                critical_note: (!text.critical_note.is_empty()).then(|| text.critical_note.clone()),
            }
        })
        .collect()
}

pub(super) fn build_breakin_modes() -> Vec<BreakInModeSpecDto> {
    let catalog = hardware_catalog();
    let locale = pt_br_ssot_locale();
    catalog
        .breakin_modes
        .iter()
        .map(|bp| {
            let text = locale
                .breakin_modes
                .get(&bp.key)
                .unwrap_or_else(|| panic!("Missing ssot.breakinModes.{} in pt-BR.json", bp.key));
            BreakInModeSpecDto {
                mode: bp.mode,
                label: text.label.clone(),
                badge: text.badge.clone(),
                badge_variant: bp.badge_variant.clone(),
                description: text.description.clone(),
                signal_diagram: text.signal_diagram.clone(),
            }
        })
        .collect()
}

pub(super) fn build_flash_geometries() -> Vec<FlashGeometrySpecDto> {
    let catalog = hardware_catalog();
    let locale = pt_br_ssot_locale();
    catalog
        .flash_geometries
        .iter()
        .map(|bp| {
            let text = locale
                .flash_geometries
                .get(&bp.key)
                .unwrap_or_else(|| panic!("Missing ssot.flashGeometries.{} in pt-BR.json", bp.key));
            let (size_bytes, nvram_base_addr, nvram_size_bytes) =
                compute_flash_geometry_addresses(bp.size_mb);
            FlashGeometrySpecDto {
                size_mb: bp.size_mb,
                size_bytes,
                chip_name: text.chip_name.clone(),
                nvram_base_hex: format!("0x{nvram_base_addr:08X}"),
                nvram_size_bytes,
                microsector_range: bp.microsector_range.clone(),
            }
        })
        .collect()
}

pub(super) fn build_memory_regions() -> Vec<MemoryRegionDto> {
    hardware_catalog()
        .system_memory_regions
        .iter()
        .map(|r| MemoryRegionDto {
            name: r.name.clone(),
            start: r.start.clone(),
            end: r.end.clone(),
            size_kb: r.size_kb,
            perms: r.perms.clone(),
        })
        .collect()
}
