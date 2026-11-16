//! USB VID/PID constants and architecture/platform models for RP2-family probes.

use serde::{Deserialize, Serialize};

/// Raspberry Pi USB Vendor ID
pub const RASPBERRY_PI_VID: u16 = 0x2E8A;
/// Standard Raspberry Pi Pico (RP2040) USB CDC Serial Product ID
pub const PICO_RP2040_CDC_PID: u16 = 0x000A;
/// Raspberry Pi Pico W (RP2040 + CYW43439) USB CDC Serial Product ID
pub const PICO_RP2040_W_CDC_PID: u16 = 0x0014;
/// Standard Raspberry Pi Pico 2 (RP2350 ARM Cortex-M33) USB CDC Serial Product ID
pub const PICO_RP2350_CDC_PID: u16 = 0x000F;
/// Raspberry Pi Pico 2 W (RP2350 + Wireless) USB CDC Serial Product ID
pub const PICO_RP2350_W_CDC_PID: u16 = 0x001F;
/// Raspberry Pi Pico 2 (RP2350 Hazard3 RISC-V) USB CDC Serial Product ID
pub const PICO_RP2350_RISCV_CDC_PID: u16 = 0x000E;
/// Raspberry Pi BOOTSEL Mode Product ID
pub const PICO_BOOTSEL_PID: u16 = 0x0003;

/// Arduino USB Vendor ID (Arduino Nano RP2040 Connect)
pub const ARDUINO_VID: u16 = 0x2341;
pub const ARDUINO_NANO_RP2040_CONNECT_PID: u16 = 0x005E;
pub const ARDUINO_NANO_RP2040_BOOT_PID: u16 = 0x805E;

/// Seeed Studio USB Vendor ID (XIAO RP2040 / XIAO RP2350)
pub const SEEED_VID: u16 = 0x2886;
pub const SEEED_XIAO_RP2040_PID: u16 = 0x0042;
pub const SEEED_XIAO_RP2040_BOOT_PID: u16 = 0x8042;
pub const SEEED_XIAO_RP2350_PID: u16 = 0x0044;

/// Adafruit Industries USB Vendor ID (Feather, QT Py, Trinkey RP2040/RP2350)
pub const ADAFRUIT_VID: u16 = 0x239A;

/// SparkFun Electronics USB Vendor ID (Pro Micro, Thing Plus RP2040/RP2350)
pub const SPARKFUN_VID: u16 = 0x1B4F;

/// Waveshare / OpenMoko / TinyUSB Generic USB Vendor ID
pub const WAVESHARE_TINYUSB_VID: u16 = 0x1209;

/// Architecture classification of connected Pico controller (Backwards-compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PicoArchitecture {
    Rp2040,
    Rp2350,
    Bootloader,
    Unknown,
}

impl std::fmt::Display for PicoArchitecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rp2040 => write!(f, "Raspberry Pi Pico (RP2040)"),
            Self::Rp2350 => write!(f, "Raspberry Pi Pico 2 (RP2350)"),
            Self::Bootloader => write!(f, "Raspberry Pi BOOTSEL Mode"),
            Self::Unknown => write!(f, "Unknown USB Serial Device"),
        }
    }
}

/// Detailed multi-platform host controller hardware classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostControllerPlatform {
    /// Official Raspberry Pi Pico (RP2040 Dual Cortex-M0+ @ 133MHz, 264KB SRAM)
    RaspberryPiPico,
    /// Official Raspberry Pi Pico W (RP2040 + Wi-Fi)
    RaspberryPiPicoW,
    /// Official Raspberry Pi Pico 2 (RP2350 Dual Cortex-M33 @ 150MHz, 520KB SRAM)
    RaspberryPiPico2Arm,
    /// Official Raspberry Pi Pico 2 (RP2350 Dual Hazard3 RISC-V @ 150MHz, 520KB SRAM)
    RaspberryPiPico2Riscv,
    /// Official Raspberry Pi Pico 2 W
    RaspberryPiPico2W,
    /// Arduino Nano RP2040 Connect
    ArduinoNanoRp2040,
    /// Seeed Studio XIAO RP2040
    SeeedXiaoRp2040,
    /// Seeed Studio XIAO RP2350
    SeeedXiaoRp2350,
    /// Adafruit RP2040 Board (Feather, QT Py, ItsyBitsy)
    AdafruitRp2040,
    /// Adafruit RP2350 Board (Feather, Metro)
    AdafruitRp2350,
    /// SparkFun RP2040 Board (Pro Micro)
    SparkfunRp2040,
    /// SparkFun RP2350 Board
    SparkfunRp2350,
    /// Waveshare RP2040 Board (Zero, One, Plus)
    WaveshareRp2040,
    /// Waveshare RP2350 Board (Plus, ETH)
    WaveshareRp2350,
    /// Generic RP2040 Microcontroller
    GenericRp2040,
    /// Generic RP2350 Microcontroller
    GenericRp2350,
    /// Generic USB CDC Device Running Panadiga P7501 UART Firmware
    GenericCompatibleCdc,
    /// RP2-family BOOTSEL UF2 Bootloader Mode
    BootloaderMode,
    /// Unknown Serial Device
    UnknownDevice,
}

impl HostControllerPlatform {
    /// Maps platform to high-level architecture category
    pub fn architecture(&self) -> PicoArchitecture {
        match self {
            Self::RaspberryPiPico
            | Self::RaspberryPiPicoW
            | Self::ArduinoNanoRp2040
            | Self::SeeedXiaoRp2040
            | Self::AdafruitRp2040
            | Self::SparkfunRp2040
            | Self::WaveshareRp2040
            | Self::GenericRp2040 => PicoArchitecture::Rp2040,

            Self::RaspberryPiPico2Arm
            | Self::RaspberryPiPico2Riscv
            | Self::RaspberryPiPico2W
            | Self::SeeedXiaoRp2350
            | Self::AdafruitRp2350
            | Self::SparkfunRp2350
            | Self::WaveshareRp2350
            | Self::GenericRp2350 => PicoArchitecture::Rp2350,

            Self::BootloaderMode => PicoArchitecture::Bootloader,
            Self::GenericCompatibleCdc | Self::UnknownDevice => PicoArchitecture::Unknown,
        }
    }

    /// Returns human-readable silicon processor family and frequency
    pub fn chip_family(&self) -> &'static str {
        match self.architecture() {
            PicoArchitecture::Rp2040 => "RP2040 (Dual ARM Cortex-M0+ @ 133 MHz)",
            PicoArchitecture::Rp2350 => match self {
                Self::RaspberryPiPico2Riscv => "RP2350 (Dual Hazard3 RISC-V @ 150 MHz)",
                _ => "RP2350 (Dual ARM Cortex-M33 @ 150 MHz)",
            },
            PicoArchitecture::Bootloader => "RP2 Family Bootloader (ROM UF2)",
            PicoArchitecture::Unknown => "Universal Microcontroller",
        }
    }

    /// Total on-chip SRAM capacity in bytes
    pub fn sram_size_bytes(&self) -> u32 {
        match self.architecture() {
            PicoArchitecture::Rp2040 => 264 * 1024, // 264 KB
            PicoArchitecture::Rp2350 => 520 * 1024, // 520 KB
            _ => 128 * 1024,
        }
    }

    /// Flash sector cache capacity allocated in firmware for P7501 UART / capture operations
    pub fn sector_cache_capacity_bytes(&self) -> u32 {
        match self.architecture() {
            PicoArchitecture::Rp2040 => 8 * 1024,  // 8 KB microsector cache
            PicoArchitecture::Rp2350 => 64 * 1024, // 64 KB full sector cache
            _ => 4 * 1024,
        }
    }

    /// True if the platform is known to natively support the PIO state machine (P7501 UART)
    pub fn supports_p7501_pio(&self) -> bool {
        matches!(
            self.architecture(),
            PicoArchitecture::Rp2040 | PicoArchitecture::Rp2350
        )
    }
}

impl std::fmt::Display for HostControllerPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::RaspberryPiPico => "Raspberry Pi Pico (RP2040)",
            Self::RaspberryPiPicoW => "Raspberry Pi Pico W (RP2040)",
            Self::RaspberryPiPico2Arm => "Raspberry Pi Pico 2 ARM (RP2350)",
            Self::RaspberryPiPico2Riscv => "Raspberry Pi Pico 2 RISC-V (RP2350)",
            Self::RaspberryPiPico2W => "Raspberry Pi Pico 2 W (RP2350)",
            Self::ArduinoNanoRp2040 => "Arduino Nano RP2040 Connect",
            Self::SeeedXiaoRp2040 => "Seeed Studio XIAO RP2040",
            Self::SeeedXiaoRp2350 => "Seeed Studio XIAO RP2350",
            Self::AdafruitRp2040 => "Adafruit RP2040 Controller",
            Self::AdafruitRp2350 => "Adafruit RP2350 Controller",
            Self::SparkfunRp2040 => "SparkFun RP2040 Pro Micro",
            Self::SparkfunRp2350 => "SparkFun RP2350 Controller",
            Self::WaveshareRp2040 => "Waveshare RP2040 Board",
            Self::WaveshareRp2350 => "Waveshare RP2350 Board",
            Self::GenericRp2040 => "Generic RP2040 Microcontroller",
            Self::GenericRp2350 => "Generic RP2350 Microcontroller",
            Self::GenericCompatibleCdc => "Compatible USB CDC Panadiga Probe",
            Self::BootloaderMode => "RP2 Family BOOTSEL UF2 Mode",
            Self::UnknownDevice => "Unknown USB Serial Device",
        };
        f.write_str(label)
    }
}

/// Metadata and hardware attributes of a detected serial port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicoDeviceInfo {
    pub port_name: String,
    pub architecture: PicoArchitecture,
    pub platform: HostControllerPlatform,
    pub vid: u16,
    pub pid: u16,
    pub description: String,
}

impl PicoDeviceInfo {
    /// Silicon chip family description
    pub fn chip_family(&self) -> &'static str {
        self.platform.chip_family()
    }

    /// On-chip SRAM size in bytes
    pub fn sram_size_bytes(&self) -> u32 {
        self.platform.sram_size_bytes()
    }

    /// Flash sector cache capacity in firmware
    pub fn sector_cache_capacity_bytes(&self) -> u32 {
        self.platform.sector_cache_capacity_bytes()
    }

    /// True if device belongs to the RP2 family (RP2040 or RP2350)
    pub fn is_rp2_family(&self) -> bool {
        self.platform.supports_p7501_pio()
    }
}
