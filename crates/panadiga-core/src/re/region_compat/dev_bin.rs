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
    let (md5, sha256) = calculate_hashes(bytes);
    Ok(DevBinReport {
        size_bytes: bytes.len(),
        model: extract_c_ascii(&bytes[0x04..0x0C]),
        oem,
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
