//! USB CDC device classification and cross-platform serial port enumeration.

use anyhow::{Context, Result};
use serialport::SerialPortType;

use super::models::{
    ADAFRUIT_VID, ARDUINO_NANO_RP2040_BOOT_PID, ARDUINO_NANO_RP2040_CONNECT_PID, ARDUINO_VID,
    HostControllerPlatform, PICO_BOOTSEL_PID, PICO_RP2040_CDC_PID, PICO_RP2040_W_CDC_PID,
    PICO_RP2350_CDC_PID, PICO_RP2350_RISCV_CDC_PID, PICO_RP2350_W_CDC_PID, PicoDeviceInfo,
    RASPBERRY_PI_VID, SEEED_VID, SEEED_XIAO_RP2040_BOOT_PID, SEEED_XIAO_RP2040_PID,
    SEEED_XIAO_RP2350_PID, SPARKFUN_VID, WAVESHARE_TINYUSB_VID,
};

const EXACT_USB_DEVICES: &[(u16, u16, HostControllerPlatform, &str)] = &[
    (
        RASPBERRY_PI_VID,
        PICO_RP2040_CDC_PID,
        HostControllerPlatform::RaspberryPiPico,
        "Raspberry Pi Pico (RP2040) P7501 Async UART Host (264KB SRAM)",
    ),
    (
        RASPBERRY_PI_VID,
        PICO_RP2040_W_CDC_PID,
        HostControllerPlatform::RaspberryPiPicoW,
        "Raspberry Pi Pico W (RP2040) P7501 Async UART Host",
    ),
    (
        RASPBERRY_PI_VID,
        PICO_RP2350_CDC_PID,
        HostControllerPlatform::RaspberryPiPico2Arm,
        "Raspberry Pi Pico 2 ARM (RP2350) P7501 Async UART Host (520KB SRAM)",
    ),
    (
        RASPBERRY_PI_VID,
        PICO_RP2350_RISCV_CDC_PID,
        HostControllerPlatform::RaspberryPiPico2Riscv,
        "Raspberry Pi Pico 2 RISC-V (RP2350) P7501 Async UART Host",
    ),
    (
        RASPBERRY_PI_VID,
        PICO_RP2350_W_CDC_PID,
        HostControllerPlatform::RaspberryPiPico2W,
        "Raspberry Pi Pico 2 W (RP2350) P7501 Async UART Host",
    ),
    (
        RASPBERRY_PI_VID,
        PICO_BOOTSEL_PID,
        HostControllerPlatform::BootloaderMode,
        "Raspberry Pi BOOTSEL UF2 Mode (Ready for firmware flash)",
    ),
    (
        ARDUINO_VID,
        ARDUINO_NANO_RP2040_CONNECT_PID,
        HostControllerPlatform::ArduinoNanoRp2040,
        "Arduino Nano RP2040 Connect (Panadiga P7501 Compatible)",
    ),
    (
        ARDUINO_VID,
        ARDUINO_NANO_RP2040_BOOT_PID,
        HostControllerPlatform::BootloaderMode,
        "Arduino Nano RP2040 Connect (BOOTSEL Mode)",
    ),
    (
        SEEED_VID,
        SEEED_XIAO_RP2040_PID,
        HostControllerPlatform::SeeedXiaoRp2040,
        "Seeed Studio XIAO RP2040 (Panadiga P7501 Compatible)",
    ),
    (
        SEEED_VID,
        SEEED_XIAO_RP2040_BOOT_PID,
        HostControllerPlatform::BootloaderMode,
        "Seeed Studio XIAO RP2040 (BOOTSEL Mode)",
    ),
    (
        SEEED_VID,
        SEEED_XIAO_RP2350_PID,
        HostControllerPlatform::SeeedXiaoRp2350,
        "Seeed Studio XIAO RP2350 (Panadiga P7501 Compatible)",
    ),
];

/// Classifies a USB device based on its VID, PID, and product description
pub fn classify_usb_device(
    vid: u16,
    pid: u16,
    product_desc: Option<&str>,
) -> (HostControllerPlatform, String) {
    if let Some(&(_, _, platform, desc)) = EXACT_USB_DEVICES
        .iter()
        .find(|&&(v, p, _, _)| v == vid && p == pid)
    {
        return (platform, desc.to_string());
    }

    let desc_hint = product_desc.unwrap_or("").to_lowercase();
    match vid {
        RASPBERRY_PI_VID if desc_hint.contains("rp2350") || desc_hint.contains("pico 2") => (
            HostControllerPlatform::GenericRp2350,
            format!("Raspberry Pi RP2350 Custom PID 0x{pid:04X}"),
        ),
        RASPBERRY_PI_VID if desc_hint.contains("rp2040") || desc_hint.contains("pico") => (
            HostControllerPlatform::GenericRp2040,
            format!("Raspberry Pi RP2040 Custom PID 0x{pid:04X}"),
        ),
        RASPBERRY_PI_VID => (
            HostControllerPlatform::GenericCompatibleCdc,
            format!("Raspberry Pi Device PID 0x{pid:04X}"),
        ),
        ARDUINO_VID if desc_hint.contains("rp2040") => (
            HostControllerPlatform::ArduinoNanoRp2040,
            "Arduino RP2040 Board".to_string(),
        ),
        ARDUINO_VID => (
            HostControllerPlatform::UnknownDevice,
            format!("Arduino Device PID 0x{pid:04X}"),
        ),
        SEEED_VID if desc_hint.contains("rp2350") => (
            HostControllerPlatform::SeeedXiaoRp2350,
            "Seeed Studio RP2350 Board".to_string(),
        ),
        SEEED_VID if desc_hint.contains("rp2040") => (
            HostControllerPlatform::SeeedXiaoRp2040,
            "Seeed Studio RP2040 Board".to_string(),
        ),
        SEEED_VID => (
            HostControllerPlatform::UnknownDevice,
            format!("Seeed Device PID 0x{pid:04X}"),
        ),
        ADAFRUIT_VID if desc_hint.contains("rp2350") => (
            HostControllerPlatform::AdafruitRp2350,
            "Adafruit RP2350 Controller".to_string(),
        ),
        ADAFRUIT_VID if desc_hint.contains("rp2040") || pid >= 0x80F0 => (
            HostControllerPlatform::AdafruitRp2040,
            "Adafruit RP2040 Controller (Feather/QT Py)".to_string(),
        ),
        ADAFRUIT_VID => (
            HostControllerPlatform::UnknownDevice,
            format!("Adafruit Device PID 0x{pid:04X}"),
        ),
        SPARKFUN_VID if desc_hint.contains("rp2350") => (
            HostControllerPlatform::SparkfunRp2350,
            "SparkFun RP2350 Board".to_string(),
        ),
        SPARKFUN_VID if desc_hint.contains("rp2040") || pid == 0x0025 || pid == 0x0026 => (
            HostControllerPlatform::SparkfunRp2040,
            "SparkFun Pro Micro RP2040".to_string(),
        ),
        SPARKFUN_VID => (
            HostControllerPlatform::UnknownDevice,
            format!("SparkFun Device PID 0x{pid:04X}"),
        ),
        WAVESHARE_TINYUSB_VID if desc_hint.contains("rp2350") => (
            HostControllerPlatform::WaveshareRp2350,
            "Waveshare RP2350 Board".to_string(),
        ),
        WAVESHARE_TINYUSB_VID if desc_hint.contains("rp2040") || desc_hint.contains("p7501") => (
            HostControllerPlatform::WaveshareRp2040,
            "Waveshare RP2040 / TinyUSB Panadiga P7501 Probe".to_string(),
        ),
        WAVESHARE_TINYUSB_VID => (
            HostControllerPlatform::GenericCompatibleCdc,
            "USB CDC Serial Device (TinyUSB)".to_string(),
        ),
        _ if desc_hint.contains("rp2350") => (
            HostControllerPlatform::GenericRp2350,
            format!("RP2350 Controller (VID: {vid:04X} PID: {pid:04X})"),
        ),
        _ if desc_hint.contains("rp2040") => (
            HostControllerPlatform::GenericRp2040,
            format!("RP2040 Controller (VID: {vid:04X} PID: {pid:04X})"),
        ),
        _ if desc_hint.contains("p7501") || desc_hint.contains("panadiga") => (
            HostControllerPlatform::GenericCompatibleCdc,
            "Panadiga P7501 Async UART Compatible Probe".to_string(),
        ),
        _ => (
            HostControllerPlatform::UnknownDevice,
            product_desc.unwrap_or("USB Serial Device").to_string(),
        ),
    }
}

/// Discovers and enumerates all serial ports across Windows, Linux, and macOS,
/// classifying RP2040, RP2350, and compatible Panadiga P7501 Async UART hardware controllers.
pub fn list_pico_devices() -> Result<Vec<PicoDeviceInfo>> {
    let mut devices = Vec::new();
    let ports = serialport::available_ports().context("Failed to list serial ports on system")?;

    for p in ports {
        if let SerialPortType::UsbPort(info) = p.port_type {
            let (platform, desc) = classify_usb_device(info.vid, info.pid, info.product.as_deref());
            let arch = platform.architecture();

            devices.push(PicoDeviceInfo {
                port_name: p.port_name,
                architecture: arch,
                platform,
                vid: info.vid,
                pid: info.pid,
                description: desc,
            });
        }
    }

    Ok(devices)
}
