//! `DEV.BIN` (`512`-byte) and NFROM `ifodev2` (`2048`-byte) `AA 55 FF FF` / `PANA_DVD_REC` descriptor parser and patcher.

use anyhow::{Result, bail};

use crate::constants::{PANA_DEV_BIN_MAGIC, PANA_DEV_BIN_SIG, PANA_DEV_BIN_SIZE};
use crate::crypto::codec::{extract_c_ascii, write_padded_ascii};
use crate::nvram::calculate_hashes;

use super::types::{DevBinReport, validate_ascii_field};

const DEV_BIN_SIZE: usize = PANA_DEV_BIN_SIZE;
/// Size of real Panasonic DMR-EH55 NFROM `ifodev2` file (4 × 512-byte BGO FTL blocks).
pub const PANA_IFODEV2_SIZE: usize = 2048;
const DEV_BIN_MAGIC: [u8; 4] = PANA_DEV_BIN_MAGIC;
const DEV_BIN_SIG: &[u8; 12] = PANA_DEV_BIN_SIG;

/// Decodes the Panasonic DIGA `market_code` byte (`ifodev2+0x31`) using the 19-entry `MAIN` LUT.
#[must_use]
pub fn decode_market_code_name(code: u8) -> &'static str {
    match code {
        0x00 => "JP",
        0x01 => "P",
        0x02 => "PC",
        0x03 => "PP",
        0x04 => "PL",
        0x05 => "PX",
        0x06 => "GT",
        0x80 => "EG",
        0x81 => "GCS",
        0x82 => "GN",
        0x83 => "EB",
        0x85 => "EF",
        0x86 => "EBL",
        0x87 => "EE",
        0x88 => "EP",
        0x89 => "GC",
        0x8A => "GK",
        0x8B => "EC",
        0xFF => "--",
        _ => "UNKNOWN",
    }
}

/// Parses and validates a 512-byte `DEV.BIN` or 2048-byte NFROM `ifodev2` Panasonic service descriptor.
pub fn inspect_dev_bin_bytes(bytes: &[u8]) -> Result<DevBinReport> {
    if bytes.len() != DEV_BIN_SIZE && bytes.len() != PANA_IFODEV2_SIZE {
        bail!(
            "DEV.BIN / ifodev2 must be {DEV_BIN_SIZE} or {PANA_IFODEV2_SIZE} bytes, got {}",
            bytes.len()
        );
    }
    if bytes[0..4] != DEV_BIN_MAGIC {
        bail!(
            "Invalid DEV.BIN magic (expected AA55FFFF, got {})",
            hex::encode(&bytes[0..4])
        );
    }
    if &bytes[0x20..0x2C] != DEV_BIN_SIG {
        bail!("Missing DEV.BIN 'PANA_DVD_REC' signature at offset 0x20");
    }
    let oem_byte = bytes[0x0C];
    let oem = if oem_byte.is_ascii_graphic() {
        (oem_byte as char).to_string()
    } else {
        format!("0x{oem_byte:02X}")
    };
    let market_code = bytes[0x31];
    let main_option_btn = [
        u16::from_be_bytes([bytes[0x50], bytes[0x51]]),
        u16::from_be_bytes([bytes[0x52], bytes[0x53]]),
        u16::from_be_bytes([bytes[0x54], bytes[0x55]]),
        u16::from_be_bytes([bytes[0x56], bytes[0x57]]),
    ];
    let (md5, sha256) = calculate_hashes(bytes);
    Ok(DevBinReport {
        size_bytes: bytes.len(),
        model: extract_c_ascii(&bytes[0x04..0x0C]),
        oem,
        option_code_hex: hex::encode(&bytes[0x0D..0x13]),
        hw_version_u16: u16::from_be_bytes([bytes[0x18], bytes[0x19]]),
        model_code: bytes[0x30],
        market_code,
        market_code_name: decode_market_code_name(market_code).to_string(),
        main_option_btn,
        main_option_rcmode: bytes[0x58],
        main_option_gui: [bytes[0x59], bytes[0x5A], bytes[0x5B], bytes[0x5C]],
        main_option_chmode: bytes[0x5D],
        signature: extract_c_ascii(&bytes[0x20..0x30]),
        md5,
        sha256,
    })
}

/// Edits `model` (bytes `0x04..0x0C`) and optional `oem` char (`0x0C`) in a validated `DEV.BIN`.
pub fn patch_dev_bin_bytes(
    bytes: &[u8],
    new_model: &str,
    new_oem: Option<char>,
) -> Result<(Vec<u8>, DevBinReport)> {
    let _ = inspect_dev_bin_bytes(bytes)?;
    let model = new_model.trim();
    validate_ascii_field("DEV.BIN model", model, 8)?;
    let mut out = bytes.to_vec();
    write_padded_ascii(&mut out[0x04..0x0C], model, 0);
    if let Some(ch) = new_oem {
        if !ch.is_ascii_graphic() {
            bail!("DEV.BIN oem must be a printable ASCII character");
        }
        out[0x0C] = ch as u8;
    }
    let rep = inspect_dev_bin_bytes(&out)?;
    Ok((out, rep))
}
