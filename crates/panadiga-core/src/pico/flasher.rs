//! Raspberry Pi Pico (RP2040 & RP2350) Automated Firmware Flasher
//!
//! Automatically flashes P7501 Async UART probe firmware to RP2040/RP2350 Pico devices.
//! 1. Detection of BOOTSEL USB Mass Storage drives (`RPI-RP2` / `RP2350` via `INFO_UF2.TXT`).
//! 2. Automatic 1200-baud touch reset to transition running Pico CDC devices into BOOTSEL mode.
//! 3. Binary UF2 validation, integrity hashing, and drag-and-drop file streaming.

mod io;
mod uf2;

#[cfg(test)]
mod tests;

pub use io::{
    detect_bootsel_drives, flash_embedded_firmware, flash_uf2_to_drive, merge_unified_uf2,
    trigger_1200_baud_bootsel,
};
pub use uf2::validate_uf2_buffer;

/// Magic numbers identifying Microsoft UF2 (USB Flashing Format) containers.
pub const UF2_MAGIC_START0: u32 = 0x0A32_4655; // "UF2\n" in little-endian
pub const UF2_MAGIC_START1: u32 = 0x9E5D_5157;
pub const UF2_MAGIC_END: u32 = 0x0AB1_6F30;
pub const UF2_BLOCK_SIZE: usize = 512;
pub const UF2_PAYLOAD_SIZE: usize = 256;

/// Bit flag in UF2 header flags (offset 8..12) indicating field 28..32 carries a Family ID.
pub const UF2_FLAG_FAMILY_ID_PRESENT: u32 = 0x0000_2000;

/// Raspberry Pi RP2040 and RP2350 UF2 Family IDs.
/// Microsoft UF2 Spec constants:
pub const RP2040_FAMILY_ID: u32 = 0xE484_9208;
pub const RP2350_ARM_S_FAMILY_ID: u32 = 0xE484_920A;
pub const RP2350_RISCV_FAMILY_ID: u32 = 0xE484_920B;

/// Official Raspberry Pi Pico SDK (elf2uf2 / picotool) Family IDs:
pub const RP2040_SDK_FAMILY_ID: u32 = 0xE48B_FF56;
pub const RP2350_ABS_SDK_FAMILY_ID: u32 = 0xE48B_FF57;
pub const RP2350_SDK_FAMILY_ID: u32 = RP2350_ABS_SDK_FAMILY_ID;
pub const RP2350_ARM_S_SDK_FAMILY_ID: u32 = 0xE48B_FF59;
pub const RP2350_RISCV_SDK_FAMILY_ID: u32 = 0xE48B_FF5A;

/// Pre-compiled P7501 Async UART firmware binary containers embedded into the application executable.
pub const EMBEDDED_RP2040_UF2: &[u8] =
    include_bytes!("firmware_bins/panasonic_p7501_uart_rp2040.uf2");
pub const EMBEDDED_RP2350_UF2: &[u8] =
    include_bytes!("firmware_bins/panasonic_p7501_uart_rp2350.uf2");
pub const EMBEDDED_UNIFIED_UF2: &[u8] =
    include_bytes!("firmware_bins/panasonic_p7501_uart_unified.uf2");

/// Result summary of an automated Pico flashing operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PicoFlashReport {
    pub target_drive: String,
    pub uf2_size_bytes: usize,
    pub blocks_written: usize,
    pub board_family: String,
    pub sha256_hash: String,
    pub success: bool,
    pub message: String,
}
