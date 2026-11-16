//! Panasonic DMR-EH55 DIGA Suite - Hardware & Firmware Single Source of Truth (SSOT) DTO

mod catalog;
mod commands;
pub mod constants;
mod dto;
mod hardware;

#[cfg(test)]
mod tests;

pub use dto::{
    BreakInModeSpecDto, FlashGeometrySpecDto, HardwareSsotSpecDto, HardwareThresholdsDto,
    MemoryRegionDto, P7501PinMappingDto, PicoCommandArgOptionDto, PicoCommandSpecDto,
    PicoUsbIdentityDto,
};

use std::path::Path;

use crate::constants::CANDIDATE_MANUALS_DIRS;
use crate::manuals::count_pdf_files;

use commands::build_pico_commands;
use hardware::{
    build_breakin_modes, build_flash_geometries, build_memory_regions, build_pinout,
    build_thresholds, build_usb_ids,
};

/// Constructs the canonical SSOT hardware/firmware specification for UI consumers.
pub fn build_hardware_ssot_spec() -> HardwareSsotSpecDto {
    let indexed_manuals_count = crate::constants::find_workspace_subdir("manuals_and_schematics")
        .or_else(|| {
            CANDIDATE_MANUALS_DIRS
                .iter()
                .map(Path::new)
                .find(|p| p.is_dir())
                .map(Path::to_path_buf)
        })
        .as_deref()
        .map(count_pdf_files)
        .unwrap_or(0);

    HardwareSsotSpecDto {
        thresholds: build_thresholds(),
        usb_ids: build_usb_ids(),
        indexed_manuals_count,
        pinout: build_pinout(),
        breakin_modes: build_breakin_modes(),
        flash_geometries: build_flash_geometries(),
        memory_regions: build_memory_regions(),
        pico_commands: build_pico_commands(),
    }
}
