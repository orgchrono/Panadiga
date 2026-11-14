//! Read-only MN103S `DIGADIAG` UART NOR flash dumper stub builder and stream decoder.
//!
//! Direct pure-Rust consolidation of `scripts/bench/nor_dumper.py`: emits position-coupled
//! MN103S machine code strictly guarded to read-only cached (`0x80000000..0x80800000`) and
//! uncached (`0x84000000..0x84800000`) NOR windows, and verifies `PANADMP1..PANADONE` dumps.

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use crate::nvram::calculate_hashes;

/// Magic header emitted at the start of every UART NOR flash dump stream.
pub const NOR_MAGIC_HEADER: &[u8; 8] = b"PANADMP1";
/// Magic footer emitted at the end of every UART NOR flash dump stream.
pub const NOR_MAGIC_FOOTER: &[u8; 8] = b"PANADONE";
/// Strictly enforced read-only MN103S NOR flash windows (cached & uncached).
pub const NOR_ALLOWED_WINDOWS: [(u32, u32); 2] =
    [(0x8000_0000, 0x8080_0000), (0x8400_0000, 0x8480_0000)];
/// Safe free SDRAM load address used by `DIGADIAG.FRM` or Pico RAM injection (`$M` / `$G`).
pub const DEFAULT_NOR_STUB_LOAD_ADDR: u32 = 0x5180_0000;

const HEADER_PREFIX_LEN: usize = NOR_MAGIC_HEADER.len() + 12;

/// Verified MN103S Sleigh subroutine: polls `SC0STR` (`0xD400200C`) bit mask `0xA0`,
/// then writes `D3` to `SC0TXB` (`0xD4002008`).
const PUTC_D3_BYTES: [u8; 24] = [
    0xFC, 0xAC, 0x0C, 0x20, 0x00, 0xD4, // MOV (0xD400200C), D0
    0x1C, // MOV D0, D1? / shift setup
    0xF8, 0xE0, 0xA0, // AND 0xA0, D0
    0xA0, 0x00, // CMP 0, D0
    0xC9, 0xF4, // BNE -12
    0x8C, // MOV D3, D0
    0xFC, 0x82, 0x08, 0x20, 0x00, 0xD4, // MOVB D0, (0xD4002008)
    0xDF, 0x00, 0x00, // RET [], 0
];

/// Verified binary NOR dump decoded from a `PANADMP1..PANADONE` UART stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NorDumpResult {
    pub start_addr: u32,
    pub total_len: u32,
    pub block_size: u32,
    pub data: Vec<u8>,
    pub sha256: String,
    pub block_checksums: Vec<u32>,
}

/// Validates strictly read-only NOR address bounds (`0x80000000..` / `0x84000000..`) and alignment.
pub fn validate_nor_dump_range(start_addr: u32, total_len: u32, block_size: u32) -> Result<()> {
    if total_len == 0 {
        bail!("total_len must be > 0 (got 0)");
    }
    if block_size == 0 {
        bail!("block_size must be > 0 (got 0)");
    }
    if !total_len.is_multiple_of(block_size) {
        bail!("total_len ({total_len}) must be an exact multiple of block_size ({block_size})");
    }
    let start_u64 = start_addr as u64;
    let end_u64 = start_u64 + total_len as u64;
    let in_window = NOR_ALLOWED_WINDOWS
        .iter()
        .any(|&(lo, hi)| start_u64 >= lo as u64 && end_u64 <= hi as u64);
    if !in_window {
        bail!(
            "Read-only NOR guard rejected [0x{start_addr:08X}, 0x{end_u64:08X}); allowed windows: 0x80000000..0x80800000 or 0x84000000..0x84800000"
        );
    }
    Ok(())
}

fn emit_mov_imm32_reg(buf: &mut Vec<u8>, sub_op: u8, imm: u32) -> usize {
    let off = buf.len();
    buf.push(0xFC);
    buf.push(sub_op);
    buf.extend_from_slice(&imm.to_le_bytes());
    off + 2
}

fn emit_call_rel16(buf: &mut Vec<u8>) -> usize {
    let insn_off = buf.len();
    buf.extend_from_slice(&[0xCD, 0x00, 0x00, 0x00, 0x00]);
    insn_off
}

fn emit_bne_back(buf: &mut Vec<u8>, target_off: usize) -> Result<()> {
    let insn_off = buf.len();
    let disp = (target_off as isize) - (insn_off as isize);
    if !(-128..=127).contains(&disp) {
        bail!("MN103S BNE displacement out of s8 range: {disp}");
    }
    buf.push(0xC9);
    buf.push((disp as i8) as u8);
    Ok(())
}

fn patch_call_rel16(buf: &mut [u8], call_insn_off: usize, target_off: usize) -> Result<()> {
    let disp = (target_off as isize) - (call_insn_off as isize);
    if !(-32768..=32767).contains(&disp) {
        bail!("MN103S CALL displacement out of s16 range: {disp}");
    }
    LittleEndian::write_i16(&mut buf[call_insn_off + 1..call_insn_off + 3], disp as i16);
    Ok(())
}

/// Assembles an executable MN103S read-only NOR UART dumper stub (`PANADMP1..PANADONE`).
pub fn build_readonly_nor_dumper_stub(
    start_addr: u32,
    total_len: u32,
    block_size: u32,
    load_addr: u32,
) -> Result<Vec<u8>> {
    validate_nor_dump_range(start_addr, total_len, block_size)?;
    if load_addr > 0xFFF0_0000 {
        bail!("Invalid MN103S load_addr: 0x{load_addr:08X}");
    }

    let mut buf = Vec::with_capacity(256);
    let mut header_data = Vec::with_capacity(HEADER_PREFIX_LEN);
    header_data.extend_from_slice(NOR_MAGIC_HEADER);
    header_data.extend_from_slice(&start_addr.to_le_bytes());
    header_data.extend_from_slice(&total_len.to_le_bytes());
    header_data.extend_from_slice(&block_size.to_le_bytes());

    // 1. Emit PANADMP1 + <start, total_len, block_size> header
    let hdr_start_patch = emit_mov_imm32_reg(&mut buf, 0xDC, 0);
    let hdr_end_patch = emit_mov_imm32_reg(&mut buf, 0xDD, 0);
    let call_emit_hdr = emit_call_rel16(&mut buf);

    // 2. Setup NOR read cursor A0 and end pointer A1
    let _ = emit_mov_imm32_reg(&mut buf, 0xDC, start_addr);
    let _ = emit_mov_imm32_reg(&mut buf, 0xDD, start_addr.wrapping_add(total_len));

    // 3. Outer block loop: reset D2 sum, load D1 = block_size
    let block_loop_off = buf.len();
    buf.extend_from_slice(&[0x8A, 0x00]); // MOV 0, D2
    let _ = emit_mov_imm32_reg(&mut buf, 0xCD, block_size); // MOV imm32, D1

    // 4. Inner read-only byte loop
    let byte_loop_off = buf.len();
    buf.extend_from_slice(&[0xF0, 0x40, 0x83, 0xE2]); // MOVBU (A0), D0; MOV D0, D3; ADD D0, D2
    let call_putc_data = emit_call_rel16(&mut buf);
    buf.extend_from_slice(&[0x20, 0x01, 0x29, 0xFF, 0xA5, 0x00]); // ADD 1, A0; ADD -1, D1; CMP 0, D1
    emit_bne_back(&mut buf, byte_loop_off)?;

    // 5. Emit 32-bit little-endian per-block sum in D2
    buf.extend_from_slice(&[0x85, 0x04]); // MOV 4, D1
    let csum_loop_off = buf.len();
    buf.extend_from_slice(&[0x88, 0x83]); // MOV D2, D0; MOV D0, D3
    let call_putc_csum = emit_call_rel16(&mut buf);
    buf.extend_from_slice(&[0xF8, 0xC6, 0x08, 0x29, 0xFF, 0xA5, 0x00]); // LSR 8, D2; ADD -1, D1; CMP 0, D1
    emit_bne_back(&mut buf, csum_loop_off)?;

    // 6. Continue until A0 == A1
    buf.push(0xB4); // CMP A1, A0
    emit_bne_back(&mut buf, block_loop_off)?;

    // 7. Emit PANADONE footer and park cleanly in BRA .
    let ftr_start_patch = emit_mov_imm32_reg(&mut buf, 0xDC, 0);
    let ftr_end_patch = emit_mov_imm32_reg(&mut buf, 0xDD, 0);
    let call_emit_ftr = emit_call_rel16(&mut buf);
    buf.extend_from_slice(&[0xCA, 0x00]); // BRA .

    // 8. Shared span emitter subroutine [A0..A1)
    let emit_span_off = buf.len();
    buf.extend_from_slice(&[0xF0, 0x40, 0x83]); // MOVBU (A0), D0; MOV D0, D3
    let call_putc_span = emit_call_rel16(&mut buf);
    buf.extend_from_slice(&[0x20, 0x01, 0xB4]); // ADD 1, A0; CMP A1, A0
    emit_bne_back(&mut buf, emit_span_off)?;
    buf.extend_from_slice(&[0xDF, 0x00, 0x00]); // RET [], 0

    // 9. Shared TX subroutine + trailing constant tables
    let putc_off = buf.len();
    buf.extend_from_slice(&PUTC_D3_BYTES);

    let hdr_off = buf.len();
    buf.extend_from_slice(&header_data);
    let ftr_off = buf.len();
    buf.extend_from_slice(NOR_MAGIC_FOOTER);
    let end_off = buf.len();

    for &(call_off, tgt) in &[
        (call_emit_hdr, emit_span_off),
        (call_emit_ftr, emit_span_off),
        (call_putc_data, putc_off),
        (call_putc_csum, putc_off),
        (call_putc_span, putc_off),
    ] {
        patch_call_rel16(&mut buf, call_off, tgt)?;
    }

    for &(patch_off, rel_off) in &[
        (hdr_start_patch, hdr_off),
        (hdr_end_patch, ftr_off),
        (ftr_start_patch, ftr_off),
        (ftr_end_patch, end_off),
    ] {
        LittleEndian::write_u32(
            &mut buf[patch_off..patch_off + 4],
            load_addr.wrapping_add(rel_off as u32),
        );
    }

    Ok(buf)
}

/// Parses and checksum-verifies a `PANADMP1..PANADONE` UART NOR capture stream.
pub fn decode_nor_dump_stream(stream: &[u8]) -> Result<NorDumpResult> {
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
    let start_addr = LittleEndian::read_u32(&meta[0..4]);
    let total_len = LittleEndian::read_u32(&meta[4..8]);
    let block_size = LittleEndian::read_u32(&meta[8..12]);
    validate_nor_dump_range(start_addr, total_len, block_size)?;

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
    use super::*;

    #[test]
    fn test_nor_dumper_stub_guard_and_stream_roundtrip() {
        let stub = build_readonly_nor_dumper_stub(0x8400_0000, 8192, 4096, 0x5180_0000).unwrap();
        assert!(stub.windows(8).any(|w| w == b"PANADMP1"));
        assert!(stub.windows(8).any(|w| w == b"PANADONE"));
        // Write window guard: reject RAM/MMIO addresses
        assert!(build_readonly_nor_dumper_stub(0x5000_0000, 4096, 4096, 0x5180_0000).is_err());

        // Construct synthetic PANADMP1..PANADONE stream with 2 blocks of 16 bytes
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
    }
}
