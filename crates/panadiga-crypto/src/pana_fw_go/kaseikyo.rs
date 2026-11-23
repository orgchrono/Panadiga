//! Panasonic Kaseikyo (`VendorID 0x2002`) service & front-panel IR command catalog.

use super::types::KaseikyoServiceCommand;

/// Verified Panasonic Kaseikyo (`VendorID 0x2002`) service & front-panel commands from `pana-fw/docs/IR.md`.
pub const PANA_KASEIKYO_SERVICE_COMMANDS: &[KaseikyoServiceCommand] = &[
    KaseikyoServiceCommand {
        name: "Service Key",
        fpc_bytes: [0x41, 0xB0, 0x00, 0xF2],
        device: 0xB,
        subdevice: 0x0,
        command: 0xF2,
        flipper_hint: "A:0x32002B0, C:0x320",
        notes: "Direct service remote trigger",
    },
    KaseikyoServiceCommand {
        name: "Panel INIT",
        fpc_bytes: [0x41, 0xB0, 0x00, 0xF6],
        device: 0xB,
        subdevice: 0x0,
        command: 0xF6,
        flipper_hint: "A:0x32002B0, C:0x360",
        notes: "Front panel initialization",
    },
    KaseikyoServiceCommand {
        name: "Panel ACCEPT",
        fpc_bytes: [0x41, 0xB0, 0x00, 0xFD],
        device: 0xB,
        subdevice: 0x0,
        command: 0xFD,
        flipper_hint: "A:0x32002B0, C:0x3D0",
        notes: "LINE 0x0fd",
    },
    KaseikyoServiceCommand {
        name: "AGING-RMC",
        fpc_bytes: [0x41, 0xB0, 0x01, 0x40],
        device: 0xB,
        subdevice: 0x1,
        command: 0x3F,
        flipper_hint: "A:0x12002B1, C:0x000",
        notes: "AGING-RMC 0x13f",
    },
    KaseikyoServiceCommand {
        name: "Service Disclosure mode",
        fpc_bytes: [0x40, 0xB0, 0x01, 0x80],
        device: 0xB,
        subdevice: 0x1,
        command: 0x80,
        flipper_hint: "A:0x22002B1, C:0x000",
        notes: "OK + Blue + Yellow",
    },
    KaseikyoServiceCommand {
        name: "Service Non-Disclosure mode 1",
        fpc_bytes: [0x40, 0xB0, 0x01, 0x81],
        device: 0xB,
        subdevice: 0x1,
        command: 0x81,
        flipper_hint: "A:0x22002B1, C:0x010",
        notes: "6 + 7 + Yellow",
    },
    KaseikyoServiceCommand {
        name: "Service Non-Disclosure mode 2",
        fpc_bytes: [0x40, 0xB0, 0x01, 0x82],
        device: 0xB,
        subdevice: 0x1,
        command: 0x82,
        flipper_hint: "A:0x22002B1, C:0x020",
        notes: "5 + 9 + Red (70 RET -> 80 SRV)",
    },
    KaseikyoServiceCommand {
        name: "Audio / Subtitles [A]",
        fpc_bytes: [0x41, 0xB0, 0x00, 0x33],
        device: 0xB,
        subdevice: 0x0,
        command: 0x33,
        flipper_hint: "A:B0 02 20 00, C:30 03 (wire 02 20 B0 00 33 83)",
        notes: "NA Combo remote AUDIO [A] != Teletext RED (0x141)",
    },
    KaseikyoServiceCommand {
        name: "Display [B]",
        fpc_bytes: [0x41, 0xB0, 0x00, 0x92],
        device: 0xB,
        subdevice: 0x0,
        command: 0x92,
        flipper_hint: "A:B0 02 20 02, C:20 01 (wire 02 20 B0 00 92 22, Panasonic_DVD_TV.ir)",
        notes: "NA Combo remote DISPLAY [B]",
    },
    KaseikyoServiceCommand {
        name: "Teletext Red [RED]",
        fpc_bytes: [0x41, 0xB0, 0x01, 0x41],
        device: 0xB,
        subdevice: 0x1,
        command: 0x41,
        flipper_hint: "wire 02 20 B0 01 41 F0 (DMR-BCT73/pana-fw)",
        notes: "EU/JDM Teletext Red button (absent on NA remotes)",
    },
    KaseikyoServiceCommand {
        name: "Power On/Off",
        fpc_bytes: [0x41, 0xB0, 0x00, 0x3D],
        device: 0xB,
        subdevice: 0x0,
        command: 0x3D,
        flipper_hint: "A:B0 02 20 00, C:D0 03 (wire 02 20 B0 00 3D 8D)",
        notes: "DVD/DIGA power toggle",
    },
];

/// Decodes Flipper Zero `.ir` Kaseikyo fields into the wire command byte (`byte 4`).
///
/// Matches `Flipper-IRDB/DVD_Players/Panasonic/Panasonic_DVD_TV.ir` (`address: <dev> 02 20 <hi>`,
/// `command: <c0> <c1> 00 00`) using `((hi & 0x03) << 6) | ((c1 & 0x0F) << 4) | (c0 >> 4)`.
pub const fn decode_flipper_kaseikyo_command(addr_hi: u8, cmd0: u8, cmd1: u8) -> u8 {
    ((addr_hi & 0x03) << 6) | ((cmd1 & 0x0F) << 4) | (cmd0 >> 4)
}

/// Decodes a Flipper Kaseikyo parsed signal into the 6-byte LSB-first wire frame
/// (`[0x02, 0x20, device, subdevice, cmd_byte, device ^ subdevice ^ cmd_byte]`).
pub const fn build_kaseikyo_wire_frame(device: u8, subdevice: u8, command: u8) -> [u8; 6] {
    [
        0x02,
        0x20,
        device,
        subdevice,
        command,
        device ^ subdevice ^ command,
    ]
}
