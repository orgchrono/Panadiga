//! Multi-platform RP2040 / RP2350 / TinyUSB hardware platform models and USB CDC discovery.

mod detection;
mod models;

#[cfg(test)]
mod tests;

pub use detection::{classify_usb_device, list_pico_devices};
pub use models::{
    ADAFRUIT_VID, ARDUINO_NANO_RP2040_BOOT_PID, ARDUINO_NANO_RP2040_CONNECT_PID, ARDUINO_VID,
    HostControllerPlatform, PICO_BOOTSEL_PID, PICO_RP2040_CDC_PID, PICO_RP2040_W_CDC_PID,
    PICO_RP2350_CDC_PID, PICO_RP2350_RISCV_CDC_PID, PICO_RP2350_W_CDC_PID, PicoArchitecture,
    PicoDeviceInfo, RASPBERRY_PI_VID, SEEED_VID, SEEED_XIAO_RP2040_BOOT_PID, SEEED_XIAO_RP2040_PID,
    SEEED_XIAO_RP2350_PID, SPARKFUN_VID, WAVESHARE_TINYUSB_VID,
};
