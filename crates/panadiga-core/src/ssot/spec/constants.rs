//! Typed hardware, timing, USB identity, and template token SSOT constants.
//!
//! Eliminates inline magic literals across `ssot/spec` by synchronizing C11 firmware
//! headers (`p7501_types.h`, `p7501_wdt_logic.h`), AM34 bus constants (`hardware_am34.rs`),
//! Spansion geometry (`flash_hdd.rs`), and RE protocol tokens (`main_passthrough`, `matshita_emu`).

use crate::constants::{
    AM34_FLASH_BASE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE, AM34_NVRAM_BASE,
    AM34_NVRAM_UNCACHED_BASE, AM34_SRAM_LOADER_BASE, DEFAULT_BAUD_RATE,
};
use crate::re::main_passthrough::DEV_SERIAL_STATE_BASE_VA;
use crate::re::matshita_emu::{DRV_PASS_HEX_TOKEN_0, DRV_PASS_HEX_TOKEN_1};

// ============================================================================
// 1. P7501 Electrical, Clock & Watchdog Thresholds (Synced with C11 Firmware)
// ============================================================================

/// Default P7501 SWD/PIO probe bus frequency in Hz (1 MHz).
pub const P7501_SWD_BUS_FREQ_HZ: u32 = 1_000_000;
/// Nominal RP2040/RP2350 I/O reference rail voltage (`P7501_ADC_VREF`).
pub const P7501_NOMINAL_VDD_VOLTS: f32 = 3.30;
/// Minimum target sense voltage (`P7501_VDD_THRESHOLD_V`).
pub const P7501_MIN_VDD_THRESHOLD_VOLTS: f32 = 2.50;
/// Minimum safe operating rail voltage for P7501 LVTTL capture.
pub const P7501_SAFE_OPERATING_MIN_VOLTS: f32 = 3.00;
/// Maximum allowed floating rail ceiling (`P7501_VDD_FLOATING_CEIL_V`).
pub const P7501_MAX_VDD_CEILING_VOLTS: f32 = 3.45;
/// Non-overridable hardware watchdog max command window (`P7501_WDT_MAX_CMD_WINDOW_MS`).
pub const P7501_WDT_MAX_CMD_WINDOW_MS: u64 = 185_000;
/// Default 8 MB NOR dump capture window fitting safely below the WDT ceiling.
pub const WDT_DUMPNOR_CAPTURE_WINDOW_MS: u64 = 180_000;
/// Symbolic catalog timeout key for commands permitted up to the WDT ceiling.
pub const CATALOG_TIMEOUT_KEY_WDT_CEILING: &str = "wdt_ceiling";

/// Bytes per megabyte (`1024 * 1024`) for flash geometry calculation.
pub const MIB_BYTES: u32 = 1_048_576;
/// Flash size threshold (MB) above which top-boot NVRAM expands to 128 KB.
pub const FLASH_16MB_THRESHOLD_MB: u32 = 16;

/// Byte offset inside `_dev_SERIAL` state struct queried by default `PANARSP MEM`.
pub const DEV_SERIAL_MEM_PROBE_OFFSET: u32 = 0x38;

// ============================================================================
// 2. Official Raspberry Pi RP2040 / RP2350 USB Identities
// ============================================================================

pub const RASPBERRY_PI_VID: u16 = 0x2E8A;
pub const RP2040_CDC_PID: u16 = 0x000A;
pub const RP2040_W_CDC_PID: u16 = 0x0014;
pub const RP2350_CDC_PID: u16 = 0x000F;
pub const RP2350_W_CDC_PID: u16 = 0x001F;
pub const RP2350_RISCV_CDC_PID: u16 = 0x000E;
pub const RP_BOOTSEL_PID: u16 = 0x0003;

// ============================================================================
// 3. Command Taxonomy & C11 CLI Alias/Internal Exclusions
// ============================================================================

/// Typed category classification for granular Pico CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PicoCommandCategory {
    Electrical,
    OcdFlash,
    UartFmu,
    Probe,
}

impl PicoCommandCategory {
    /// Returns the wire DTO string representation used by frontend filters and MCP tools.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Electrical => "electrical",
            Self::OcdFlash => "ocd_flash",
            Self::UartFmu => "uart_fmu",
            Self::Probe => "probe",
        }
    }
}

impl TryFrom<&str> for PicoCommandCategory {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "electrical" => Ok(Self::Electrical),
            "ocd_flash" => Ok(Self::OcdFlash),
            "uart_fmu" => Ok(Self::UartFmu),
            "probe" => Ok(Self::Probe),
            other => Err(format!("Invalid PicoCommandCategory '{other}' in catalog")),
        }
    }
}

/// C11 `k_cli_commands[]` names intentionally omitted from the interactive Studio catalog
/// (command aliases/dispatchers `VDD`, `HELP`, `?`, `BOOT_PROFILE`, `GDB`, `HIZ`, `PROBE`, `AUTOPROBE`, `IR`).
pub const INTERNAL_OR_ALIAS_CLI_COMMANDS: &[&str] = &[
    "VDD",
    "HELP",
    "?",
    "BOOT_PROFILE",
    "GDB",
    "HIZ",
    "PROBE",
    "AUTOPROBE",
    "IR",
    "FAST_ERASE",
    "FAULTACK",
];

// ============================================================================
// 4. Data-Driven Catalog Template Token Substitution
// ============================================================================

/// Builds canonical placeholder -> value substitutions from typed Rust hardware constants.
pub fn catalog_template_tokens() -> [(&'static str, String); 10] {
    let mem_probe_addr = DEV_SERIAL_STATE_BASE_VA + DEV_SERIAL_MEM_PROBE_OFFSET;
    [
        (
            "{{AM34_NVRAM_BASE_HEX}}",
            format!("0x{AM34_NVRAM_BASE:08X}"),
        ),
        (
            "{{AM34_FLASH_BASE_HEX}}",
            format!("0x{AM34_FLASH_BASE:08X}"),
        ),
        (
            "{{AM34_SRAM_LOADER_BASE_HEX}}",
            format!("0x{AM34_SRAM_LOADER_BASE:08X}"),
        ),
        (
            "{{AM34_FLASH_UNCACHED_ADDR}}",
            format!("{AM34_FLASH_UNCACHED_BASE:08X}"),
        ),
        (
            "{{AM34_FLASH_SIZE_HEX_NO_PREFIX}}",
            format!("{AM34_FLASH_SIZE:X}"),
        ),
        (
            "{{AM34_NVRAM_UNCACHED_ADDR}}",
            format!("{AM34_NVRAM_UNCACHED_BASE:08X}"),
        ),
        (
            "{{DEV_SERIAL_MEM_PROBE_ADDR}}",
            format!("{mem_probe_addr:08X}"),
        ),
        ("{{DEFAULT_BAUD_RATE}}", DEFAULT_BAUD_RATE.to_string()),
        (
            "{{MATSHITA_PASS_UNLOCK_CMD}}",
            format!("PASS 0x{DRV_PASS_HEX_TOKEN_0:08x} 0x{DRV_PASS_HEX_TOKEN_1:08x}"),
        ),
        (
            "{{WDT_DUMPNOR_CAPTURE_WINDOW_MS}}",
            WDT_DUMPNOR_CAPTURE_WINDOW_MS.to_string(),
        ),
    ]
}

/// Replaces all canonical `{{...}}` catalog tokens using typed Rust constants.
pub(super) fn resolve_catalog_tokens(raw: &str, tokens: &[(&str, String)]) -> String {
    if !raw.contains("{{") {
        return raw.to_string();
    }
    let mut resolved = raw.to_string();
    for (needle, replacement) in tokens {
        if resolved.contains(needle) {
            resolved = resolved.replace(needle, replacement);
        }
    }
    resolved
}
