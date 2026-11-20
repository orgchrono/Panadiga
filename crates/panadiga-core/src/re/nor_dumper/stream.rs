#![deny(unsafe_code)]

//! UART capture parser and per-block checksum verifier for `PANADMP1..PANADONE` dumps.

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use super::{
    HEADER_PREFIX_LEN, NOR_MAGIC_FOOTER, NOR_MAGIC_HEADER, NorDumpResult, validate_nor_dump_range,
};
use crate::constants::{AM34_FLASH_BASE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};
use crate::nvram::calculate_hashes;

/// Parses and checksum-verifies a `PANADMP1..PANADONE` UART NOR capture stream.
pub fn decode_nor_dump_stream(stream: &[u8]) -> Result<NorDumpResult> {
    decode_nor_dump_stream_slice(stream, None)
}

/// Parses and checksum-verifies a `PANADMP1..PANADONE` UART NOR stream, supporting
/// SDRAM-patched slice captures where the stub emits a static full 8 MiB header.
pub fn decode_nor_dump_stream_slice(
    stream: &[u8],
    expected_slice: Option<(u32, u32)>,
) -> Result<NorDumpResult> {
    let Some(hdr_pos) = stream
        .windows(NOR_MAGIC_HEADER.len())
        .position(|w| w == NOR_MAGIC_HEADER)
    else {
        bail!("Missing PANADMP1 header in UART stream");
    };
    if hdr_pos + HEADER_PREFIX_LEN > stream.len() {
        bail!("Truncated PANADMP1 header in UART stream");
    }

    let meta = &stream[hdr_pos + NOR_MAGIC_HEADER.len()..hdr_pos + HEADER_PREFIX_LEN];
    let hdr_start_addr = LittleEndian::read_u32(&meta[0..4]);
    let hdr_total_len = LittleEndian::read_u32(&meta[4..8]);
    let block_size = LittleEndian::read_u32(&meta[8..12]);
    validate_nor_dump_range(hdr_start_addr, hdr_total_len, block_size)?;

    let (start_addr, total_len) = match expected_slice {
        Some((slice_start, slice_len)) => {
            validate_nor_dump_range(slice_start, slice_len, block_size)?;
            let is_static_full_hdr = hdr_total_len == AM34_FLASH_SIZE
                && (hdr_start_addr == AM34_FLASH_UNCACHED_BASE
                    || hdr_start_addr == AM34_FLASH_BASE);
            let matches_slice_hdr = hdr_start_addr == slice_start && hdr_total_len == slice_len;
            if !is_static_full_hdr && !matches_slice_hdr {
                bail!(
                    "PANADMP1 header (0x{hdr_start_addr:08X}, {hdr_total_len} B) incompatible with expected slice (0x{slice_start:08X}, {slice_len} B)"
                );
            }
            (slice_start, slice_len)
        }
        None => (hdr_start_addr, hdr_total_len),
    };

    let block_count = (total_len / block_size) as usize;
    let payload_off = hdr_pos + HEADER_PREFIX_LEN;
    let expected_end =
        payload_off + (block_count * (block_size as usize + 4)) + NOR_MAGIC_FOOTER.len();
    if stream.len() < expected_end {
        bail!(
            "Truncated NOR dump stream: need {expected_end} bytes from header, got {}",
            stream.len()
        );
    }

    let mut data = Vec::with_capacity(total_len as usize);
    let mut checksums = Vec::with_capacity(block_count);
    let mut cursor = payload_off;

    for blk_idx in 0..block_count {
        let blk = &stream[cursor..cursor + block_size as usize];
        cursor += block_size as usize;
        let rx_sum = LittleEndian::read_u32(&stream[cursor..cursor + 4]);
        cursor += 4;
        let calc_sum: u32 = blk.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32));
        if rx_sum != calc_sum {
            bail!("Block {blk_idx} checksum mismatch: rx=0x{rx_sum:08X} != calc=0x{calc_sum:08X}");
        }
        data.extend_from_slice(blk);
        checksums.push(rx_sum);
    }

    if &stream[cursor..cursor + NOR_MAGIC_FOOTER.len()] != NOR_MAGIC_FOOTER {
        bail!("Missing PANADONE footer after payload blocks");
    }

    let (_, sha256) = calculate_hashes(&data);
    Ok(NorDumpResult {
        start_addr,
        total_len,
        block_size,
        data,
        sha256,
        block_checksums: checksums,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{
        NOR_MAGIC_FOOTER, NOR_MAGIC_HEADER, build_readonly_nor_dumper_stub,
        resolve_nor_stub_load_addr,
    };
    use super::*;

    #[test]
    fn test_nor_dumper_stub_guard_and_stream_roundtrip() {
        let stub = build_readonly_nor_dumper_stub(0x8400_0000, 8192, 4096, 0x5180_0000).unwrap();
        assert!(stub.windows(8).any(|w| w == b"PANADMP1"));
        assert!(stub.windows(8).any(|w| w == b"PANADONE"));
        assert!(build_readonly_nor_dumper_stub(0x5000_0000, 4096, 4096, 0x5180_0000).is_err());

        let b0 = vec![0xAAu8; 16];
        let b1 = vec![0x55u8; 16];
        let s0: u32 = b0.iter().map(|&b| b as u32).sum();
        let s1: u32 = b1.iter().map(|&b| b as u32).sum();
        let mut stream = Vec::new();
        stream.extend_from_slice(b"NOISE_BEFORE");
        stream.extend_from_slice(NOR_MAGIC_HEADER);
        stream.extend_from_slice(&0x8420_0000u32.to_le_bytes());
        stream.extend_from_slice(&32u32.to_le_bytes());
        stream.extend_from_slice(&16u32.to_le_bytes());
        stream.extend_from_slice(&b0);
        stream.extend_from_slice(&s0.to_le_bytes());
        stream.extend_from_slice(&b1);
        stream.extend_from_slice(&s1.to_le_bytes());
        stream.extend_from_slice(NOR_MAGIC_FOOTER);

        let parsed = decode_nor_dump_stream(&stream).unwrap();
        assert_eq!(parsed.start_addr, 0x8420_0000);
        assert_eq!(parsed.data.len(), 32);
        assert_eq!(parsed.block_checksums, vec![s0, s1]);
        assert_eq!(resolve_nor_stub_load_addr(None, true), 0x5040_0000);
        assert_eq!(resolve_nor_stub_load_addr(None, false), 0x5180_0000);
        assert_eq!(
            resolve_nor_stub_load_addr(Some(0x5100_0000), true),
            0x5100_0000
        );

        // Static 8 MiB header emitted by SDRAM-patched PANAMON1 stub for a 32-byte slice
        let mut slice_stream = Vec::new();
        slice_stream.extend_from_slice(NOR_MAGIC_HEADER);
        slice_stream.extend_from_slice(&AM34_FLASH_UNCACHED_BASE.to_le_bytes());
        slice_stream.extend_from_slice(&AM34_FLASH_SIZE.to_le_bytes());
        slice_stream.extend_from_slice(&16u32.to_le_bytes());
        slice_stream.extend_from_slice(&b0);
        slice_stream.extend_from_slice(&s0.to_le_bytes());
        slice_stream.extend_from_slice(&b1);
        slice_stream.extend_from_slice(&s1.to_le_bytes());
        slice_stream.extend_from_slice(b"PANADONEOK:DONE\r\n");

        let parsed_slice =
            decode_nor_dump_stream_slice(&slice_stream, Some((0x8410_0000, 32))).unwrap();
        assert_eq!(parsed_slice.start_addr, 0x8410_0000);
        assert_eq!(parsed_slice.total_len, 32);
        assert_eq!(parsed_slice.data[..16], b0);
        assert_eq!(parsed_slice.data[16..], b1);
    }
}
