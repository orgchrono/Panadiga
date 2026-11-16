//! Pure Matsushita Kaseikyo 48-bit IR synthesizer, Philips Pronto Hex parser, and Flipper `.ir` decoder.

use anyhow::{Result, bail};

use crate::crypto::pana_fw_go::{build_kaseikyo_wire_frame, decode_flipper_kaseikyo_command};

/// Panasonic DIGA IR carrier frequency (Hz), matching Pronto divisor `0x0070`.
pub const CARRIER_MATSUSHITA_HZ: u32 = 37_010;
pub const PULSE_LEAD_IN_MARK_US: u32 = 3_459;
pub const PULSE_LEAD_IN_SPACE_US: u32 = 1_729;
pub const PULSE_BIT_MARK_US: u32 = 432;
pub const PULSE_BIT_0_SPACE_US: u32 = 432;
pub const PULSE_BIT_1_SPACE_US: u32 = 1_297;
pub const PULSE_TRAIL_MARK_US: u32 = 432;
pub const PULSE_TRAIL_SPACE_US: u32 = 74_720;

const PRONTO_TIMEBASE_CONST: f64 = 0.241_246;
pub const KASEIKYO_TRANSITION_COUNT: usize = 100;
pub const KASEIKYO_PRONTO_WORD_COUNT: usize = 104;

/// Combines remote channel mode (`0x00`, `0x10`, `0x20`) with subdevice group (`0x00` or `0x01`).
#[must_use]
pub const fn effective_subdevice(
    base_subdevice: u8,
    remote_mode: u8,
    respect_remote_mode: bool,
) -> u8 {
    if !respect_remote_mode {
        return base_subdevice;
    }
    let mode_bits = match remote_mode {
        2 => 0x10,
        3 => 0x20,
        _ => 0x00,
    };
    mode_bits | (base_subdevice & 0x0F)
}

/// Formats `[0x02, 0x20, dev, sub, obc, dev^sub^obc]` into spaced uppercase wire bytes.
#[must_use]
pub fn format_kaseikyo_wire_hex(device: u8, subdevice: u8, obc: u8) -> String {
    let f = build_kaseikyo_wire_frame(device, subdevice, obc);
    format!(
        "{:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
        f[0], f[1], f[2], f[3], f[4], f[5]
    )
}

/// Synthesizes the exact 100-element microsecond transition array for a 48-bit Kaseikyo frame.
#[must_use]
pub fn synthesize_kaseikyo_pattern(device: u8, subdevice: u8, obc: u8) -> Vec<u32> {
    let payload = build_kaseikyo_wire_frame(device, subdevice, obc);
    let mut pattern = Vec::with_capacity(KASEIKYO_TRANSITION_COUNT);
    pattern.push(PULSE_LEAD_IN_MARK_US);
    pattern.push(PULSE_LEAD_IN_SPACE_US);

    for byte_val in payload {
        for bit_pos in 0..8 {
            pattern.push(PULSE_BIT_MARK_US);
            let is_one = ((byte_val >> bit_pos) & 1) == 1;
            pattern.push(if is_one {
                PULSE_BIT_1_SPACE_US
            } else {
                PULSE_BIT_0_SPACE_US
            });
        }
    }
    pattern.push(PULSE_TRAIL_MARK_US);
    pattern.push(PULSE_TRAIL_SPACE_US);
    pattern
}

/// Synthesizes the exact 104-word Philips Pronto Hex representation of a Kaseikyo frame.
#[must_use]
pub fn synthesize_kaseikyo_pronto(device: u8, subdevice: u8, obc: u8) -> String {
    let payload = build_kaseikyo_wire_frame(device, subdevice, obc);
    let mut words = Vec::with_capacity(KASEIKYO_PRONTO_WORD_COUNT);
    words.extend(["0000", "0070", "0000", "0032", "0080", "0040"]);

    for byte_val in payload {
        for bit_pos in 0..8 {
            words.push("0010");
            let is_one = ((byte_val >> bit_pos) & 1) == 1;
            words.push(if is_one { "0030" } else { "0010" });
        }
    }
    words.extend(["0010", "0ACA"]);
    words.join(" ")
}

/// Parses a raw learned Philips Pronto Hex (`0000`) string into `(carrier_hz, pattern_us)`.
pub fn parse_pronto_hex(pronto_hex: &str) -> Result<(u32, Vec<u32>)> {
    let words = parse_pronto_words(pronto_hex)?;
    if words.len() < 4 {
        bail!("Invalid Pronto Hex: missing minimum 4-word header");
    }
    if words[0] != 0 {
        bail!("Only raw learned Pronto signals (format 0000) are supported");
    }
    if words[1] == 0 {
        bail!("Invalid Pronto carrier frequency divisor 0x0000");
    }

    let carrier_hz = (1_000_000.0 / (f64::from(words[1]) * PRONTO_TIMEBASE_CONST)).round() as u32;
    if carrier_hz == 0 {
        bail!("Pronto carrier frequency evaluated to 0 Hz");
    }
    let cycle_to_us = 1_000_000.0 / f64::from(carrier_hz);
    let pattern = words[4..]
        .iter()
        .map(|&cycles| (f64::from(cycles) * cycle_to_us).round().max(1.0) as u32)
        .collect();
    Ok((carrier_hz, pattern))
}

/// Decodes a 48-bit Panasonic Kaseikyo Pronto Hex frame back to `(device, subdevice, obc)`.
#[must_use]
pub fn decode_kaseikyo_from_pronto(pronto_hex: &str) -> Option<(u8, u8, u8)> {
    let words = parse_pronto_words(pronto_hex).ok()?;
    if words.len() < KASEIKYO_PRONTO_WORD_COUNT || !words.len().is_multiple_of(2) {
        return None;
    }
    if words[0] != 0 || words[1] == 0 {
        return None;
    }
    let hdr_mark = words[4];
    let hdr_space = words[5];
    if hdr_mark < 50 || hdr_mark <= hdr_space {
        return None;
    }

    let mut bytes = [0u8; 6];
    for (byte_idx, slot) in bytes.iter_mut().enumerate() {
        let mut val = 0u8;
        for bit_idx in 0..8 {
            let p_idx = 6 + (byte_idx * 8 + bit_idx) * 2;
            let mark = words[p_idx];
            let space = words[p_idx + 1];
            if mark == 0 {
                return None;
            }
            if u32::from(space) > u32::from(mark) * 2 {
                val |= 1 << bit_idx;
            }
        }
        *slot = val;
    }
    if bytes[0] != 0x02 || bytes[1] != 0x20 {
        return None;
    }
    if (bytes[2] ^ bytes[3] ^ bytes[4]) != bytes[5] {
        return None;
    }
    Some((bytes[2], bytes[3], bytes[4]))
}

/// Decodes Flipper Zero `.ir` `address` / `command` hex fields into Kaseikyo `(device, subdevice, obc)`.
#[must_use]
pub fn decode_flipper_kaseikyo(address_hex: &str, command_hex: &str) -> Option<(u8, u8, u8)> {
    let addr = parse_hex_byte_list(address_hex)?;
    let cmd = parse_hex_byte_list(command_hex)?;
    if addr.len() < 4 || cmd.len() < 2 || addr[1] != 0x02 || addr[2] != 0x20 {
        return None;
    }
    let dev = addr[0] & 0xFE;
    let sub = (addr[0] & 0x01) | ((addr[3] >> 2) << 1);
    let obc = decode_flipper_kaseikyo_command(addr[3], cmd[0], cmd[1]);
    Some((dev, sub, obc))
}

fn parse_pronto_words(pronto_hex: &str) -> Result<Vec<u16>> {
    let mut out = Vec::new();
    for tok in pronto_hex.split_whitespace() {
        out.push(u16::from_str_radix(tok, 16)?);
    }
    Ok(out)
}

fn parse_hex_byte_list(raw: &str) -> Option<Vec<u8>> {
    raw.split_whitespace()
        .map(|tok| u8::from_str_radix(tok, 16).ok())
        .collect()
}
