//! MN103S / AM34 position-coupled bytecode emitters used by the `PANAMON1` UART0 monitor.

use anyhow::Result;

pub(super) use crate::re::mn10300_emit::{
    MONITOR_GETC_BYTES, MONITOR_PUTC_BYTES, PatchTable, emit_branch8_placeholder, emit_call16,
    emit_call16_leaf, emit_jmp16_placeholder, emit_mov_imm32, patch_s8_rel, patch_s16_rel,
};

pub(super) struct NorDumperSubLocations {
    pub sub_off: usize,
    pub start_imm_off: usize,
    pub end_imm_off: usize,
    pub block_size: u32,
}

pub(super) fn emit_uppercase_d0(buf: &mut Vec<u8>, tag: &str) -> Result<()> {
    buf.extend_from_slice(&[0xA0, b'a']);
    let blt_skip = emit_branch8_placeholder(buf, 0xC0);
    buf.extend_from_slice(&[0xA0, b'z']);
    let bgt_skip = emit_branch8_placeholder(buf, 0xC1);
    buf.extend_from_slice(&[0x28, 0xE0]);
    let done = buf.len();
    patch_s8_rel(buf, blt_skip, done, tag)?;
    patch_s8_rel(buf, bgt_skip, done, tag)
}

pub(super) fn emit_print_str(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    str_idx: usize,
    print_span_off: usize,
) {
    let a0 = emit_mov_imm32(buf, 0xDC, 0);
    let a1 = emit_mov_imm32(buf, 0xDD, 0);
    pt.str_refs.push((a0, a1, str_idx));
    emit_call16(buf, pt, print_span_off);
}

pub(super) fn emit_read_hex_u32_or_syntax(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    read_hex_u32_off: usize,
    syntax_err_off: usize,
) -> Result<()> {
    emit_call16(buf, pt, read_hex_u32_off);
    buf.extend_from_slice(&[0xA5, 0x00]); // cmp 0, d1
    emit_cond_jmp16(buf, pt, 0xC9, syntax_err_off)
}

pub(super) fn emit_cond_jmp16(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    skip_if_opcode: u8,
    target_off: usize,
) -> Result<()> {
    let skip = emit_branch8_placeholder(buf, skip_if_opcode);
    pt.j16s.push((emit_jmp16_placeholder(buf), target_off));
    let after = buf.len();
    patch_s8_rel(buf, skip, after, "COND_JMP16")
}

/// Emits 8 MB uncached NOR (`0x84000000..0x847FFFFF`) UART dumper leaf used by `PANAMON1` `'D'`.
pub(super) fn emit_nor_dumper_sub(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    print_span_off: usize,
    putc_off: usize,
    hdr_str_idx: usize,
    ftr_str_idx: usize,
) -> NorDumperSubLocations {
    const BLOCK_SIZE: u32 = 4096;
    let sub_off = buf.len();
    buf.extend_from_slice(&[0xF8, 0xFE, 0xFC]); // add -4, sp
    emit_print_str(buf, pt, hdr_str_idx, print_span_off);
    let start_imm_off = emit_mov_imm32(buf, 0xDC, 0x8400_0000);
    let end_imm_off = emit_mov_imm32(buf, 0xDD, 0x8480_0000);

    let blk_loop = buf.len();
    buf.extend_from_slice(&[0x8A, 0x00]); // mov 0, d2
    let _ = emit_mov_imm32(buf, 0xCD, BLOCK_SIZE); // mov 4096, d1

    let byte_loop = buf.len();
    buf.extend_from_slice(&[0xF0, 0x40, 0x83, 0xE2]); // movbu (a0), d0; mov d0, d3; add d0, d2
    emit_call16_leaf(buf, pt, putc_off);
    buf.extend_from_slice(&[0x20, 0x01, 0x29, 0xFF, 0xA5, 0x00]); // add 1,a0; add -1,d1; cmp 0,d1
    pt.bne8s
        .push((emit_branch8_placeholder(buf, 0xC9), byte_loop));

    buf.extend_from_slice(&[0x85, 0x04]); // mov 4, d1
    let csum_loop = buf.len();
    buf.extend_from_slice(&[0x88, 0x83]); // mov d2, d0; mov d0, d3
    emit_call16_leaf(buf, pt, putc_off);
    buf.extend_from_slice(&[0xF8, 0xC6, 0x08, 0x29, 0xFF, 0xA5, 0x00]); // lsr 8,d2; add -1,d1; cmp 0,d1
    pt.bne8s
        .push((emit_branch8_placeholder(buf, 0xC9), csum_loop));

    buf.push(0xB4); // cmp a1, a0
    pt.bne8s
        .push((emit_branch8_placeholder(buf, 0xC9), blk_loop));

    emit_print_str(buf, pt, ftr_str_idx, print_span_off);
    buf.extend_from_slice(&[0xF8, 0xFE, 0x04, 0xDF, 0x00, 0x00]); // add +4, sp; ret [], 0
    NorDumperSubLocations {
        sub_off,
        start_imm_off,
        end_imm_off,
        block_size: BLOCK_SIZE,
    }
}

/// Emits `(put_nibble_sub, put_hex_byte_sub)`: prints low byte of `D0` as two uppercase hex chars.
pub(super) fn emit_put_hex_byte_sub(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    putc_off: usize,
) -> Result<usize> {
    let put_nib_off = buf.len();
    buf.extend_from_slice(&[0xF8, 0xFE, 0xFC]); // add -4, sp
    buf.extend_from_slice(&[0xF8, 0xE0, 0x0F, 0x28, 0x30, 0xA0, 0x3A]); // and 0f,d0; add '0',d0; cmp '9'+1,d0
    let blt_digit = emit_branch8_placeholder(buf, 0xC0); // blt digit
    buf.extend_from_slice(&[0x28, 0x07]); // add 7, d0 ('A'..'F')
    let digit_off = buf.len();
    patch_s8_rel(buf, blt_digit, digit_off, "PUTHEX_BLT")?;
    buf.push(0x83); // mov d0, d3
    emit_call16_leaf(buf, pt, putc_off);
    buf.extend_from_slice(&[0xF8, 0xFE, 0x04, 0xDF, 0x00, 0x00]); // add +4, sp; ret [], 0

    let sub_off = buf.len();
    buf.extend_from_slice(&[0xF8, 0xFE, 0xFC, 0x81]); // add -4, sp; mov d0, d1
    buf.extend_from_slice(&[0xF8, 0xC4, 0x04]); // lsr 4, d0
    emit_call16(buf, pt, put_nib_off);
    buf.extend_from_slice(&[0x84, 0xF8, 0xFE, 0x04]); // mov d1, d0; add +4, sp
    pt.j16s.push((emit_jmp16_placeholder(buf), put_nib_off));
    Ok(sub_off)
}

/// Emits `read_hex_u32_sub`: reads echoed ASCII hex into `D2`, digit count in `D1`, delimiter in `D0`.
pub(super) fn emit_read_hex_u32_sub(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    getc_off: usize,
    putc_off: usize,
) -> Result<usize> {
    let sub_off = buf.len();
    buf.extend_from_slice(&[0xF8, 0xFE, 0xFC, 0x8A, 0x00, 0x85, 0x00]); // add -4, sp; mov 0, d2; mov 0, d1

    let rx_loop = buf.len();
    emit_call16_leaf(buf, pt, getc_off);
    buf.push(0x83); // mov d0, d3
    emit_call16_leaf(buf, pt, putc_off);
    buf.push(0x8C); // mov d3, d0

    for &ws in b" \t" {
        buf.extend_from_slice(&[0xA0, ws]);
        pt.beq8s
            .push((emit_branch8_placeholder(buf, 0xC8), rx_loop));
    }

    let mut to_done = Vec::with_capacity(3);
    for &delim in b",\r\n" {
        buf.extend_from_slice(&[0xA0, delim]);
        to_done.push(emit_branch8_placeholder(buf, 0xC8));
    }

    buf.extend_from_slice(&[0xA0, b'a']);
    let blt_chk = emit_branch8_placeholder(buf, 0xC0);
    buf.extend_from_slice(&[0xA0, b'f']);
    let bgt_chk = emit_branch8_placeholder(buf, 0xC1);
    buf.extend_from_slice(&[0x28, 0xE0]); // add -0x20, d0
    let chk_hex = buf.len();
    patch_s8_rel(buf, blt_chk, chk_hex, "HEX_BLT")?;
    patch_s8_rel(buf, bgt_chk, chk_hex, "HEX_BGT")?;

    buf.extend_from_slice(&[0xA0, b'0']);
    let bad_1 = emit_branch8_placeholder(buf, 0xC0);
    buf.extend_from_slice(&[0xA0, b'9' + 1]);
    let is_dig = emit_branch8_placeholder(buf, 0xC0);
    buf.extend_from_slice(&[0xA0, b'A']);
    let bad_2 = emit_branch8_placeholder(buf, 0xC0);
    buf.extend_from_slice(&[0xA0, b'F' + 1]);
    let is_alpha = emit_branch8_placeholder(buf, 0xC0);
    let bad_3 = emit_branch8_placeholder(buf, 0xCA); // >= 'G'

    let dig_off = buf.len();
    patch_s8_rel(buf, is_dig, dig_off, "HEX_DIG")?;
    buf.extend_from_slice(&[0x28, 0xD0u8]); // add -0x30, d0
    let bra_shift = emit_branch8_placeholder(buf, 0xCA);

    let alpha_off = buf.len();
    patch_s8_rel(buf, is_alpha, alpha_off, "HEX_ALPHA")?;
    buf.extend_from_slice(&[0x28, 0xC9u8]); // add -0x37, d0

    let shift_off = buf.len();
    patch_s8_rel(buf, bra_shift, shift_off, "HEX_SHIFT")?;
    buf.extend_from_slice(&[0xF8, 0xC2, 0x04, 0xE2, 0x29, 0x01]); // asl 4,d2; add d0,d2; add 1,d1
    let br_next = emit_branch8_placeholder(buf, 0xCA);
    patch_s8_rel(buf, br_next, rx_loop, "HEX_NEXT")?;

    let done_off = buf.len();
    for br in to_done {
        patch_s8_rel(buf, br, done_off, "HEX_DONE")?;
    }
    buf.extend_from_slice(&[0xF8, 0xFE, 0x04, 0xDF, 0x00, 0x00]); // add +4, sp; ret [], 0

    let drain_off = buf.len();
    for br in [bad_1, bad_2, bad_3] {
        patch_s8_rel(buf, br, drain_off, "HEX_BAD")?;
    }
    let mut to_bad_ret = Vec::with_capacity(2);
    for &crlf in b"\r\n" {
        buf.extend_from_slice(&[0xA0, crlf]);
        to_bad_ret.push(emit_branch8_placeholder(buf, 0xC8));
    }
    emit_call16_leaf(buf, pt, getc_off);
    buf.push(0x83);
    emit_call16_leaf(buf, pt, putc_off);
    buf.push(0x8C);
    let br_drain = emit_branch8_placeholder(buf, 0xCA);
    patch_s8_rel(buf, br_drain, drain_off, "HEX_DRAIN")?;
    let bad_ret_off = buf.len();
    for br in to_bad_ret {
        patch_s8_rel(buf, br, bad_ret_off, "HEX_BAD_RET")?;
    }
    buf.extend_from_slice(&[0x85, 0x00, 0xF8, 0xFE, 0x04, 0xDF, 0x00, 0x00]); // mov 0, d1; add +4, sp; ret [], 0
    Ok(sub_off)
}
