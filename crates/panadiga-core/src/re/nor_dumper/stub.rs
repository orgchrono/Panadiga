//! MN103S machine-code stub assembler and `DIGADIAG.FRM` packager for read-only NOR dumps.

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use super::{HEADER_PREFIX_LEN, NOR_MAGIC_FOOTER, NOR_MAGIC_HEADER, validate_nor_dump_range};
use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::crypto::build_digadiag_frm;

/// Verified MN103S Sleigh subroutine: polls `SC0STR` (`0xD400200C`) bits `0xA0` (`TXBUSY|TXBF`,
/// matching `PROG061` `FUN_50404d15` @ `0x50404D37` & `BOOT.BIN` `uart0_putc` @ `0x440047DD`)
/// with a bounded `0x00040000`-poll timeout (preserving caller `D1` on stack), then writes `D3`
/// to `SC0TXB` (`0xD4002008`).
pub(crate) const PUTC_D3_BYTES: [u8; 46] = [
    0xF8, 0xFE, 0xFC, // ADD -4, SP (preserve return PC at (4, SP))
    0x46, 0x00, // MOV D1, (0, SP)
    0xFC, 0xCD, 0x00, 0x00, 0x04, 0x00, // MOV 0x00040000, D1 (bounded timeout)
    0xFC, 0xAC, 0x0C, 0x20, 0x00, 0xD4, // MOVHU (0xD400200C), D0
    0x1C, // EXTHU D0
    0xF8, 0xE0, 0xA0, // AND 0xA0, D0 (TXBUSY | TXBF)
    0xA0, 0x00, // CMP 0, D0
    0xC8, 0x08, // BEQ +8 -> ready
    0x29, 0xFF, // ADD -1, D1
    0xA5, 0x00, // CMP 0, D1
    0xC9, 0xEE, // BNE -18 -> poll loop
    0x8C, // MOV D3, D0
    0xFC, 0x82, 0x08, 0x20, 0x00, 0xD4, // MOVBU D0, (0xD4002008)
    0x59, 0x00, // MOV (0, SP), D1
    0xF8, 0xFE, 0x04, // ADD +4, SP
    0xDF, 0x00, 0x00, // RET [], 0
];

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

    let hdr_start_patch = emit_mov_imm32_reg(&mut buf, 0xDC, 0);
    let hdr_end_patch = emit_mov_imm32_reg(&mut buf, 0xDD, 0);
    let call_emit_hdr = emit_call_rel16(&mut buf);

    let _ = emit_mov_imm32_reg(&mut buf, 0xDC, start_addr);
    let _ = emit_mov_imm32_reg(&mut buf, 0xDD, start_addr.wrapping_add(total_len));

    let block_loop_off = buf.len();
    buf.extend_from_slice(&[0x8A, 0x00]); // MOV 0, D2
    let _ = emit_mov_imm32_reg(&mut buf, 0xCD, block_size); // MOV imm32, D1

    let byte_loop_off = buf.len();
    buf.extend_from_slice(&[0xF0, 0x40, 0x83, 0xE2]); // MOVBU (A0), D0; MOV D0, D3; ADD D0, D2
    let call_putc_data = emit_call_rel16(&mut buf);
    buf.extend_from_slice(&[0x20, 0x01, 0x29, 0xFF, 0xA5, 0x00]); // ADD 1, A0; ADD -1, D1; CMP 0, D1
    emit_bne_back(&mut buf, byte_loop_off)?;

    buf.extend_from_slice(&[0x85, 0x04]); // MOV 4, D1
    let csum_loop_off = buf.len();
    buf.extend_from_slice(&[0x88, 0x83]); // MOV D2, D0; MOV D0, D3
    let call_putc_csum = emit_call_rel16(&mut buf);
    buf.extend_from_slice(&[0xF8, 0xC6, 0x08, 0x29, 0xFF, 0xA5, 0x00]); // LSR 8, D2; ADD -1, D1; CMP 0, D1
    emit_bne_back(&mut buf, csum_loop_off)?;

    buf.push(0xB4); // CMP A1, A0
    emit_bne_back(&mut buf, block_loop_off)?;

    let ftr_start_patch = emit_mov_imm32_reg(&mut buf, 0xDC, 0);
    let ftr_end_patch = emit_mov_imm32_reg(&mut buf, 0xDD, 0);
    let call_emit_ftr = emit_call_rel16(&mut buf);
    buf.extend_from_slice(&[0xCA, 0x00]); // BRA .

    let emit_span_off = buf.len();
    buf.extend_from_slice(&[0xF8, 0xFE, 0xFC]); // ADD -4, SP (preserve return PC at (4, SP))
    let emit_span_loop = buf.len();
    buf.extend_from_slice(&[0xF0, 0x40, 0x83]); // MOVBU (A0), D0; MOV D0, D3
    let call_putc_span = emit_call_rel16(&mut buf);
    buf.extend_from_slice(&[0x20, 0x01, 0xB4]); // ADD 1, A0; CMP A1, A0
    emit_bne_back(&mut buf, emit_span_loop)?;
    buf.extend_from_slice(&[0xF8, 0xFE, 0x04, 0xDF, 0x00, 0x00]); // ADD +4, SP; RET [], 0

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

/// Builds either a raw AM34 read-only NOR dumper stub or a 128 KiB `DIGADIAG.FRM` container.
///
/// Returns `(raw_stub_bytes_len, final_payload_bytes)`.
pub fn build_nor_dumper_artifact(
    start_addr: u32,
    total_len: u32,
    block_size: u32,
    load_addr: u32,
    wrap_as_digadiag: bool,
) -> Result<(usize, Vec<u8>)> {
    let stub = build_readonly_nor_dumper_stub(start_addr, total_len, block_size, load_addr)?;
    let stub_len = stub.len();
    let payload = if wrap_as_digadiag {
        let (frm, _) = build_digadiag_frm(
            &stub,
            load_addr,
            "FVU ",
            "NOR_DMP",
            4,
            PANA_DIGADIAG_FRM_SIZE,
        )?;
        frm
    } else {
        stub
    };
    Ok((stub_len, payload))
}
