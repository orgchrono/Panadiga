#![deny(unsafe_code)]

//! ATA/ATAPI `IDENTIFY DEVICE` (`0xEC`) 256-word parser and Panasonic mirror LBA calculator.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Expected raw `IDENTIFY DEVICE` payload length in bytes (256 × 16-bit words).
pub const ATA_IDENTIFY_BYTES: usize = 512;
/// Expected 32-bit `DAVIS` PIO capture buffer length in bytes (256 × 32-bit MMIO reads).
pub const DAVIS_IDENTIFY_PIO_BYTES: usize = 1024;

/// Decoded ATA `IDENTIFY DEVICE` (`0xEC`) drive identity, LBA geometry, and DMA capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtaIdentifyInfo {
    pub word0: u16,
    pub is_atapi: bool,
    pub model: String,
    pub serial_number: String,
    pub firmware_revision: String,
    pub lba28_sectors: u32,
    pub lba48_supported: bool,
    pub lba48_sectors: u64,
    pub effective_sectors: u64,
    pub capacity_bytes: u64,
    pub mirror_main_lba: Option<u64>,
    pub word63_mwdma: u16,
    pub mwdma2_supported: bool,
    pub mwdma_active_mode: Option<u8>,
    pub word88_udma: u16,
    pub udma_supported_mask: u8,
    pub udma_active_mode: Option<u8>,
}

fn decode_ata_swapped_ascii(words: &[u16]) -> String {
    let mut raw = Vec::with_capacity(words.len() * 2);
    for &w in words {
        raw.push((w >> 8) as u8);
        raw.push((w & 0xFF) as u8);
    }
    String::from_utf8_lossy(&raw)
        .trim_matches(|c: char| c == '\0' || c.is_ascii_whitespace())
        .to_string()
}

fn highest_active_bit_mode(word: u16, shift: u8, max_mode: u8) -> Option<u8> {
    let active = (word >> shift) & 0xFF;
    (0..=max_mode)
        .rev()
        .find(|&bit| (active & (1u16 << bit)) != 0)
}

/// Converts a 1024-byte `DAVIS` 32-bit MMIO PIO capture into a standard 512-byte ATA IDENTIFY buffer.
pub fn extract_ata_identify_512_from_pio_1024(pio_1024: &[u8]) -> Result<[u8; ATA_IDENTIFY_BYTES]> {
    if pio_1024.len() != DAVIS_IDENTIFY_PIO_BYTES {
        bail!(
            "Expected {DAVIS_IDENTIFY_PIO_BYTES}-byte DAVIS PIO capture, got {} bytes",
            pio_1024.len()
        );
    }
    let mut out = [0u8; ATA_IDENTIFY_BYTES];
    for i in 0..256usize {
        out[i * 2] = pio_1024[i * 4];
        out[i * 2 + 1] = pio_1024[i * 4 + 1];
    }
    Ok(out)
}

/// Parses a 512-byte ATA `IDENTIFY DEVICE` (`0xEC`) word table into [`AtaIdentifyInfo`].
pub fn parse_ata_identify_words(raw_512: &[u8]) -> Result<AtaIdentifyInfo> {
    if raw_512.len() != ATA_IDENTIFY_BYTES {
        bail!(
            "Expected {ATA_IDENTIFY_BYTES}-byte ATA IDENTIFY buffer, got {} bytes",
            raw_512.len()
        );
    }
    let mut words = [0u16; 256];
    for (idx, chunk) in raw_512.as_chunks::<2>().0.iter().enumerate() {
        words[idx] = u16::from_le_bytes([chunk[0], chunk[1]]);
    }

    let word0 = words[0];
    if word0 == 0x0000 || word0 == 0xFFFF {
        bail!("Invalid ATA IDENTIFY word0 signature: 0x{word0:04X}");
    }
    if words.iter().all(|&w| w == word0) {
        bail!("Invalid uniform ATA IDENTIFY payload (all words 0x{word0:04X})");
    }

    let is_atapi = (word0 & 0xC000) == 0x8000;
    let serial_number = decode_ata_swapped_ascii(&words[10..20]);
    let firmware_revision = decode_ata_swapped_ascii(&words[23..27]);
    let model = decode_ata_swapped_ascii(&words[27..47]);

    let lba28_sectors = u32::from(words[60]) | (u32::from(words[61]) << 16);
    let lba48_supported = (words[83] & (1 << 10)) != 0;
    let lba48_sectors = u64::from(words[100])
        | (u64::from(words[101]) << 16)
        | (u64::from(words[102]) << 32)
        | (u64::from(words[103]) << 48);

    let effective_sectors = if lba48_supported && lba48_sectors > 0 {
        lba48_sectors
    } else {
        u64::from(lba28_sectors)
    };
    let capacity_bytes = effective_sectors.saturating_mul(512);
    let mirror_main_lba = ((capacity_bytes / 512) & !3).checked_sub(0x20000);

    let word63_mwdma = words[63];
    let mwdma2_supported = (word63_mwdma & (1 << 2)) != 0;
    let mwdma_active_mode = highest_active_bit_mode(word63_mwdma, 8, 2);

    let word88_udma = words[88];
    let udma_supported_mask = (word88_udma & 0x00FF) as u8;
    let udma_active_mode = highest_active_bit_mode(word88_udma, 8, 6);

    Ok(AtaIdentifyInfo {
        word0,
        is_atapi,
        model,
        serial_number,
        firmware_revision,
        lba28_sectors,
        lba48_supported,
        lba48_sectors,
        effective_sectors,
        capacity_bytes,
        mirror_main_lba,
        word63_mwdma,
        mwdma2_supported,
        mwdma_active_mode,
        word88_udma,
        udma_supported_mask,
        udma_active_mode,
    })
}

/// Decodes a 1024-byte `DAVIS` PIO capture directly into [`AtaIdentifyInfo`].
pub fn parse_ata_identify_pio_buffer(pio_1024: &[u8]) -> Result<AtaIdentifyInfo> {
    let raw_512 = extract_ata_identify_512_from_pio_1024(pio_1024)?;
    parse_ata_identify_words(&raw_512)
}

#[cfg(test)]
pub(super) fn build_mock_identify_512(
    model: &str,
    serial: &str,
    fw: &str,
    lba48_sectors: u64,
) -> [u8; ATA_IDENTIFY_BYTES] {
    let mut words = [0u16; 256];
    words[0] = 0x045A;
    let mut encode_ata = |start: usize, count: usize, s: &str| {
        let mut padded = s.as_bytes().to_vec();
        padded.resize(count * 2, b' ');
        for (i, pair) in padded.as_chunks::<2>().0.iter().enumerate() {
            words[start + i] = (u16::from(pair[0]) << 8) | u16::from(pair[1]);
        }
    };
    encode_ata(10, 10, serial);
    encode_ata(23, 4, fw);
    encode_ata(27, 20, model);
    let lba28 = lba48_sectors.min(0x0FFF_FFFF) as u32;
    words[60] = (lba28 & 0xFFFF) as u16;
    words[61] = (lba28 >> 16) as u16;
    words[63] = 0x0407; // MWDMA 0..2 supported, mode 2 active
    words[83] = 1 << 10; // LBA48 supported
    words[88] = 0x203F; // UDMA 0..5 supported, mode 5 active
    words[100] = (lba48_sectors & 0xFFFF) as u16;
    words[101] = ((lba48_sectors >> 16) & 0xFFFF) as u16;
    words[102] = ((lba48_sectors >> 32) & 0xFFFF) as u16;
    words[103] = ((lba48_sectors >> 48) & 0xFFFF) as u16;

    let mut raw = [0u8; ATA_IDENTIFY_BYTES];
    for (i, w) in words.into_iter().enumerate() {
        raw[i * 2..i * 2 + 2].copy_from_slice(&w.to_le_bytes());
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ata_identify_words_lba48_dma_and_rejects_invalid_signatures() {
        let raw = build_mock_identify_512("SAMSUNG HM251JI", "S123456789", "2AC101C4", 488_397_168);
        let info = parse_ata_identify_words(&raw).unwrap();
        assert_eq!(info.word0, 0x045A);
        assert!(!info.is_atapi);
        assert_eq!(info.model, "SAMSUNG HM251JI");
        assert_eq!(info.serial_number, "S123456789");
        assert_eq!(info.firmware_revision, "2AC101C4");
        assert!(info.lba48_supported);
        assert_eq!(info.lba48_sectors, 488_397_168);
        assert_eq!(info.mirror_main_lba, Some(488_266_096));
        assert!(info.mwdma2_supported);
        assert_eq!(info.mwdma_active_mode, Some(2));
        assert_eq!(info.udma_active_mode, Some(5));

        assert!(parse_ata_identify_words(&[0x00u8; 512]).is_err());
        assert!(parse_ata_identify_words(&[0xFFu8; 512]).is_err());
    }
}
