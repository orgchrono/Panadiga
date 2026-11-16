//! Unit tests for `pico::platforms` models and USB CDC fallback/bootloader classification.

use super::*;

#[test]
fn platform_display_and_device_info_delegators_match_silicon_specs() {
    let info = PicoDeviceInfo {
        port_name: "COM7".to_string(),
        architecture: PicoArchitecture::Rp2350,
        platform: HostControllerPlatform::RaspberryPiPico2Riscv,
        vid: RASPBERRY_PI_VID,
        pid: PICO_RP2350_RISCV_CDC_PID,
        description: "Test probe".to_string(),
    };
    assert!(info.chip_family().contains("Hazard3 RISC-V"));
    assert_eq!(info.sram_size_bytes(), 520 * 1024);
    assert_eq!(info.sector_cache_capacity_bytes(), 64 * 1024);
    assert!(info.is_rp2_family());
    assert_eq!(
        info.platform.to_string(),
        "Raspberry Pi Pico 2 RISC-V (RP2350)"
    );
    assert_eq!(
        PicoArchitecture::Unknown.to_string(),
        "Unknown USB Serial Device"
    );
}

#[test]
fn classify_usb_device_handles_bootsel_and_vendor_fallbacks() {
    let cases = [
        (
            RASPBERRY_PI_VID,
            PICO_BOOTSEL_PID,
            None,
            HostControllerPlatform::BootloaderMode,
            PicoArchitecture::Bootloader,
        ),
        (
            ARDUINO_VID,
            ARDUINO_NANO_RP2040_BOOT_PID,
            None,
            HostControllerPlatform::BootloaderMode,
            PicoArchitecture::Bootloader,
        ),
        (
            SEEED_VID,
            SEEED_XIAO_RP2040_BOOT_PID,
            None,
            HostControllerPlatform::BootloaderMode,
            PicoArchitecture::Bootloader,
        ),
        (
            RASPBERRY_PI_VID,
            0x9999,
            Some("Custom Pico 2 Board"),
            HostControllerPlatform::GenericRp2350,
            PicoArchitecture::Rp2350,
        ),
        (
            ADAFRUIT_VID,
            0x0001,
            Some("Adafruit Metro RP2350"),
            HostControllerPlatform::AdafruitRp2350,
            PicoArchitecture::Rp2350,
        ),
        (
            SPARKFUN_VID,
            0x0099,
            Some("SparkFun Thing Plus RP2350"),
            HostControllerPlatform::SparkfunRp2350,
            PicoArchitecture::Rp2350,
        ),
        (
            WAVESHARE_TINYUSB_VID,
            0x0001,
            Some("Generic TinyUSB CDC"),
            HostControllerPlatform::GenericCompatibleCdc,
            PicoArchitecture::Unknown,
        ),
        (
            0xCAFE,
            0x0001,
            None,
            HostControllerPlatform::UnknownDevice,
            PicoArchitecture::Unknown,
        ),
    ];

    for (vid, pid, desc, exp_plat, exp_arch) in cases {
        let (plat, label) = classify_usb_device(vid, pid, desc);
        assert_eq!(plat, exp_plat);
        assert_eq!(plat.architecture(), exp_arch);
        assert!(!label.is_empty());
    }
}
