//! Pure immutable SSOT Data Transfer Objects and compact builder helpers.

use serde::{Deserialize, Serialize};

/// Canonical electrical, timing, and bus address thresholds synchronized with C firmware (`p7501_types.h`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareThresholdsDto {
    pub bus_freq_hz: u32,
    pub nominal_vdd_volts: f32,
    pub min_vdd_threshold_volts: f32,
    pub safe_operating_min_volts: f32,
    pub max_vdd_ceiling_volts: f32,
    pub default_baud_rate: u32,
    pub default_gdb_socket: String,
    pub ram_loader_base_hex: String,
    pub default_nvram_base_hex: String,
}

/// Official Raspberry Pi RP2040/RP2350 USB VID/PID constants for host discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicoUsbIdentityDto {
    pub raspberry_pi_vid: u16,
    pub rp2040_cdc_pid: u16,
    pub rp2040_w_cdc_pid: u16,
    pub rp2350_cdc_pid: u16,
    pub rp2350_w_cdc_pid: u16,
    pub rp2350_riscv_cdc_pid: u16,
    pub bootsel_pid: u16,
}

/// Physical and manual pin mapping entry for P7501 JST B6B-ZR-3R4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P7501PinMappingDto {
    pub manual_pin: u8,
    pub physical_pin: u8,
    pub pin_label: String,
    pub pico_gpio: String,
    pub pico_pin: String,
    pub signal_name: String,
    pub direction: String,
    pub voltage: String,
    pub critical_note: Option<String>,
}

/// Cold Break-In mode descriptor matching `protocol/p7501_bus_protocol.c`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakInModeSpecDto {
    pub mode: u8,
    pub label: String,
    pub badge: String,
    pub badge_variant: String,
    pub description: String,
    pub signal_diagram: String,
}

/// Supported Panasonic DIGA Top-Boot Spansion NOR Flash geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashGeometrySpecDto {
    pub size_mb: u32,
    pub size_bytes: u32,
    pub chip_name: String,
    pub nvram_base_hex: String,
    pub nvram_size_bytes: u32,
    pub microsector_range: String,
}

/// Structured AM34 memory map region DTO mapped from `AM34_SYSTEM_MEMORY_REGIONS`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegionDto {
    pub name: String,
    pub start: String,
    pub end: String,
    pub size_kb: usize,
    pub perms: String,
}

/// Option entry for parameterized Pico commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicoCommandArgOptionDto {
    pub label: String,
    pub value: String,
}

/// Granular Pico P7501 Async UART / FMU firmware command descriptor (`cli/p7501_cli.c`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicoCommandSpecDto {
    pub id: String,
    pub category: String,
    pub label: String,
    pub description: String,
    pub syntax: String,
    pub arg_label: Option<String>,
    pub arg_placeholder: Option<String>,
    pub default_arg: Option<String>,
    pub arg_options: Vec<PicoCommandArgOptionDto>,
    pub dangerous: bool,
    pub timeout_ms: u64,
}

/// Root SSOT payload delivered to Frontend UI and ViewModels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSsotSpecDto {
    pub thresholds: HardwareThresholdsDto,
    pub usb_ids: PicoUsbIdentityDto,
    pub indexed_manuals_count: usize,
    pub pinout: Vec<P7501PinMappingDto>,
    pub breakin_modes: Vec<BreakInModeSpecDto>,
    pub flash_geometries: Vec<FlashGeometrySpecDto>,
    pub memory_regions: Vec<MemoryRegionDto>,
    pub pico_commands: Vec<PicoCommandSpecDto>,
}
