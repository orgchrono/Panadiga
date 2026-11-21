//! MN103S Spansion `S29JL064H` JEDEC erase/program and runtime Upper NOR policy emitters.

use anyhow::Result;

use crate::constants::{
    AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_IPL_SIZE, AM34_FLASH_UNCACHED_BASE,
};

use super::mn103_emit::{
    PatchTable, emit_branch8_placeholder, emit_call16, emit_call16_leaf, emit_cond_jmp16,
    emit_jmp16_placeholder, emit_mov_imm32, emit_print_str, emit_read_hex_u32_or_syntax,
    emit_uppercase_d0, patch_s8_rel,
};

const JEDEC_UNLOCK1_VA: u32 = AM34_FLASH_UNCACHED_BASE + 0x0AAA;
const JEDEC_UNLOCK2_VA: u32 = AM34_FLASH_UNCACHED_BASE + 0x0554;

/// Checks `A2` against 2-layer Spansion protection (`D0` in = `nor_unlocked`).
/// Returns `D0 = -1` (E05), `0` (non-NOR RAM/MMIO), or `1` (authorized Spansion NOR).
pub(super) fn emit_check_nor_access_a2_sub(buf: &mut Vec<u8>) -> Result<usize> {
    let sub_off = buf.len();
    buf.extend_from_slice(&[0xF1, 0xD9]); // mov a2, d1
    let _ = emit_mov_imm32(buf, 0xC5, 0x8000_0000); // sub 0x80000000, d1
    let _ = emit_mov_imm32(buf, 0xC9, 0x0080_0000); // cmp 0x00800000, d1
    let is_nor_1 = emit_branch8_placeholder(buf, 0xC4); // bcs is_nor
    let _ = emit_mov_imm32(buf, 0xC5, 0x0400_0000); // sub 0x04000000, d1
    let _ = emit_mov_imm32(buf, 0xC9, 0x0080_0000); // cmp 0x00800000, d1
    let is_nor_2 = emit_branch8_placeholder(buf, 0xC4); // bcs is_nor
    buf.extend_from_slice(&[0x00, 0xDF, 0x00, 0x00]); // clr d0; ret [], 0

    let is_nor = buf.len();
    patch_s8_rel(buf, is_nor_1, is_nor, "NOR_CHK1")?;
    patch_s8_rel(buf, is_nor_2, is_nor, "NOR_CHK2")?;
    let _ = emit_mov_imm32(buf, 0xC9, AM34_FLASH_CRITICAL_IPL_SIZE); // SA0..SA3 permanent RO
    let blk_crit = emit_branch8_placeholder(buf, 0xC4);
    let _ = emit_mov_imm32(buf, 0xC9, AM34_FLASH_IPL_SIZE); // NVRAM SA134..SA141
    let allow_nvram = emit_branch8_placeholder(buf, 0xC6);
    buf.extend_from_slice(&[0xA0, 0x00]); // SA4..SA133 requires d0 != 0
    let blk_upper = emit_branch8_placeholder(buf, 0xC8);

    let allow_off = buf.len();
    patch_s8_rel(buf, allow_nvram, allow_off, "NOR_NVRAM_OK")?;
    buf.extend_from_slice(&[0x80, 0x01, 0xDF, 0x00, 0x00]);

    let blk_off = buf.len();
    patch_s8_rel(buf, blk_crit, blk_off, "NOR_BLK_CRIT")?;
    patch_s8_rel(buf, blk_upper, blk_off, "NOR_BLK_UPP")?;
    buf.extend_from_slice(&[0x80, 0xFF, 0xDF, 0x00, 0x00]);
    Ok(sub_off)
}

fn emit_align_a2_to_a1(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[0xF1, 0xD9]); // mov a2, d1
    let _ = emit_mov_imm32(buf, 0xCC, 0xFFFF_FFFE); // mov ~1, d0
    buf.extend_from_slice(&[0xF2, 0x01, 0xF1, 0xE5]); // and d0, d1; mov d1, a1
}

fn emit_jedec_unlock_a0_a3(buf: &mut Vec<u8>) {
    let _ = emit_mov_imm32(buf, 0xDC, JEDEC_UNLOCK1_VA);
    let _ = emit_mov_imm32(buf, 0xDF, JEDEC_UNLOCK2_VA);
    buf.extend_from_slice(&[0x80, 0xAA, 0x14, 0xF0, 0x70]); // 0x00AA -> (0x84000AAA)
    buf.extend_from_slice(&[0x80, 0x55, 0xF0, 0x73]); // 0x0055 -> (0x84000554)
}

/// Polls `(a1)` DQ7 (`bit 7`) and DQ5 exceeded-timing flag until `(val & 0x80) == d3` or bounded timeout (~2.5-3.0s @ 100 MHz).
/// Resets flash array via `0x00F0` and returns `d0 = 0` on verified completion, or `d0 = 1` on timeout / DQ5 error.
pub(super) fn emit_jedec_poll_dq7_reset_sub(buf: &mut Vec<u8>) -> Result<usize> {
    let sub_off = buf.len();
    let _ = emit_mov_imm32(buf, 0xCD, 0x0400_0000); // d1 = poll budget
    let poll_loop = buf.len();
    buf.extend_from_slice(&[0xF0, 0x61, 0x82]); // movhu (a1), d0; mov d0, d2
    buf.extend_from_slice(&[0xF8, 0xE0, 0x80, 0xAC]); // and 0x80, d0; cmp d3, d0
    let br_ok1 = emit_branch8_placeholder(buf, 0xC8); // beq ok
    buf.extend_from_slice(&[0xF8, 0xEE, 0x20]); // btst 0x20, d2 (DQ5 exceeded timing)
    let br_no_dq5 = emit_branch8_placeholder(buf, 0xC8); // beq dec_d1
    buf.extend_from_slice(&[0xF0, 0x61, 0xF8, 0xE0, 0x80, 0xAC]); // final DQ7 re-read
    let br_ok2 = emit_branch8_placeholder(buf, 0xC8); // beq ok
    let br_dq5_fail = emit_branch8_placeholder(buf, 0xCA); // bra fail

    let dec_d1 = buf.len();
    patch_s8_rel(buf, br_no_dq5, dec_d1, "JEDEC_NO_DQ5")?;
    buf.extend_from_slice(&[0x29, 0xFF, 0xA5, 0x00]); // add -1,d1; cmp 0,d1
    let br_cont = emit_branch8_placeholder(buf, 0xC9);
    patch_s8_rel(buf, br_cont, poll_loop, "JEDEC_POLL")?;

    let fail_off = buf.len();
    patch_s8_rel(buf, br_dq5_fail, fail_off, "JEDEC_DQ5_FAIL")?;
    buf.extend_from_slice(&[0x8A, 0x01]); // mov 1, d2 (error status)
    let br_reset = emit_branch8_placeholder(buf, 0xCA);

    let ok_off = buf.len();
    patch_s8_rel(buf, br_ok1, ok_off, "JEDEC_OK1")?;
    patch_s8_rel(buf, br_ok2, ok_off, "JEDEC_OK2")?;
    buf.extend_from_slice(&[0x8A, 0x00]); // mov 0, d2 (ok status)

    let reset_off = buf.len();
    patch_s8_rel(buf, br_reset, reset_off, "JEDEC_RESET")?;
    let _ = emit_mov_imm32(buf, 0xDC, AM34_FLASH_UNCACHED_BASE);
    buf.extend_from_slice(&[0x80, 0xF0, 0x14, 0xF0, 0x70, 0x88, 0xDF, 0x00, 0x00]); // 0x00F0 -> 0x84000000; mov d2, d0; ret [], 0
    Ok(sub_off)
}

/// Emits 6-cycle Spansion JEDEC Sector Erase targeting `a2` (`0x30`) + DQ7 poll + `0xF0` reset.
pub(super) fn emit_jedec_erase_a2_sub(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    poll_reset_off: usize,
) -> usize {
    let sub_off = buf.len();
    emit_align_a2_to_a1(buf);
    emit_jedec_unlock_a0_a3(buf);
    buf.extend_from_slice(&[0x80, 0x80, 0x14, 0xF0, 0x70]); // 0x0080 -> unlock1
    emit_jedec_unlock_a0_a3(buf);
    buf.extend_from_slice(&[0x80, 0x30, 0xF0, 0x71]); // 0x0030 -> (a1)
    buf.extend_from_slice(&[0x8F, 0x80, 0x17]); // mov 0x80, d3; extbu d3
    pt.j16s.push((emit_jmp16_placeholder(buf), poll_reset_off));
    sub_off
}

/// Emits 4-cycle Spansion JEDEC Word Program (`0xA0`) preserving neighbor byte of `(a2 & !1)`.
pub(super) fn emit_jedec_prog_a2_d2_sub(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    poll_reset_off: usize,
) -> Result<usize> {
    let sub_off = buf.len();
    buf.push(0x16); // extbu d2
    emit_align_a2_to_a1(buf);
    buf.extend_from_slice(&[0xF0, 0x61, 0xF1, 0xD9, 0xF8, 0xED, 0x01]); // movhu (a1),d0; mov a2,d1; btst 1,d1
    let br_odd = emit_branch8_placeholder(buf, 0xC9);
    let _ = emit_mov_imm32(buf, 0xCD, 0x0000_FF00); // even byte: preserve upper byte
    buf.extend_from_slice(&[0xF2, 0x04, 0xF2, 0x12]); // and d1,d0; or d0,d2
    let br_prog = emit_branch8_placeholder(buf, 0xCA);

    let odd_off = buf.len();
    patch_s8_rel(buf, br_odd, odd_off, "JEDEC_PROG_ODD")?;
    buf.extend_from_slice(&[0x14, 0xF8, 0xC2, 0x08, 0xF2, 0x12]); // extbu d0; asl 8,d2; or d0,d2

    let prog_off = buf.len();
    patch_s8_rel(buf, br_prog, prog_off, "JEDEC_PROG_GO")?;
    emit_jedec_unlock_a0_a3(buf);
    buf.extend_from_slice(&[0x80, 0xA0, 0x14, 0xF0, 0x70, 0xF0, 0x79]); // 0x00A0 -> unlock1; d2 -> (a1)
    buf.extend_from_slice(&[0x8B, 0xF8, 0xE3, 0x80]); // mov d2,d3; and 0x80,d3
    pt.j16s.push((emit_jmp16_placeholder(buf), poll_reset_off));
    Ok(sub_off)
}

/// Emits `U <CONFIRM_UPPER_NOR|LOCK>` runtime handler mutating `(12, sp)` unlock state.
pub(super) fn emit_nor_unlock_handler(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    getc_off: usize,
    putc_off: usize,
    print_span_off: usize,
    targets: (usize, usize), // (prompt_loop, syntax_err_off)
    strs: (usize, usize, usize, usize, usize), // (confirm_rest, lock_rest, crlf, ok_u, ok_l)
) -> Result<()> {
    let (prompt_loop, syntax_err_off) = targets;
    let skip_ws = buf.len();
    emit_call16_leaf(buf, pt, getc_off);
    buf.push(0x83);
    emit_call16_leaf(buf, pt, putc_off);
    buf.push(0x8C);
    for ws in *b" \t" {
        buf.extend_from_slice(&[0xA0, ws]);
        pt.beq8s
            .push((emit_branch8_placeholder(buf, 0xC8), skip_ws));
    }
    for crlf in *b"\r\n" {
        buf.extend_from_slice(&[0xA0, crlf]);
        emit_cond_jmp16(buf, pt, 0xC9, syntax_err_off)?;
    }
    emit_uppercase_d0(buf, "U_UP1")?;

    buf.extend_from_slice(&[0xA0, b'C']);
    let not_c = emit_branch8_placeholder(buf, 0xC9);
    let a0_c = emit_mov_imm32(buf, 0xDC, 0);
    let a1_c = emit_mov_imm32(buf, 0xDD, 0);
    pt.str_refs.push((a0_c, a1_c, strs.0));
    buf.extend_from_slice(&[0x8A, 0x01]); // mov 1, d2
    let go_cmp = emit_branch8_placeholder(buf, 0xCA);

    let chk_l = buf.len();
    patch_s8_rel(buf, not_c, chk_l, "U_CHK_L")?;
    buf.extend_from_slice(&[0xA0, b'L']);
    let bad_first = emit_branch8_placeholder(buf, 0xC9);
    let a0_l = emit_mov_imm32(buf, 0xDC, 0);
    let a1_l = emit_mov_imm32(buf, 0xDD, 0);
    pt.str_refs.push((a0_l, a1_l, strs.1));
    buf.extend_from_slice(&[0x8A, 0x00]); // mov 0, d2

    let cmp_loop = buf.len();
    patch_s8_rel(buf, go_cmp, cmp_loop, "U_GO_CMP")?;
    emit_call16_leaf(buf, pt, getc_off);
    buf.push(0x83);
    emit_call16_leaf(buf, pt, putc_off);
    buf.push(0x8C);

    buf.push(0xB4); // cmp a1, a0
    let at_end = emit_branch8_placeholder(buf, 0xC8);
    for crlf in *b"\r\n" {
        buf.extend_from_slice(&[0xA0, crlf]);
        emit_cond_jmp16(buf, pt, 0xC9, syntax_err_off)?;
    }
    emit_uppercase_d0(buf, "U_UP2")?;
    buf.extend_from_slice(&[0xF0, 0x44, 0x20, 0x01, 0xA4]); // movbu (a0),d1; add 1,a0; cmp d1,d0
    let br_ok = emit_branch8_placeholder(buf, 0xC8);
    patch_s8_rel(buf, br_ok, cmp_loop, "U_CMP_NEXT")?;
    let br_mismatch = emit_branch8_placeholder(buf, 0xCA);

    let end_chk = buf.len();
    patch_s8_rel(buf, at_end, end_chk, "U_AT_END")?;
    let mut to_success = Vec::with_capacity(2);
    for crlf in *b"\r\n" {
        buf.extend_from_slice(&[0xA0, crlf]);
        to_success.push(emit_branch8_placeholder(buf, 0xC8));
    }
    for ws in *b" \t" {
        buf.extend_from_slice(&[0xA0, ws]);
        pt.beq8s
            .push((emit_branch8_placeholder(buf, 0xC8), cmp_loop));
    }

    let drain_cr = buf.len();
    patch_s8_rel(buf, bad_first, drain_cr, "U_BAD_1ST")?;
    patch_s8_rel(buf, br_mismatch, drain_cr, "U_MISMATCH")?;
    emit_call16_leaf(buf, pt, getc_off);
    buf.push(0x83);
    emit_call16_leaf(buf, pt, putc_off);
    buf.push(0x8C);
    for crlf in *b"\r\n" {
        buf.extend_from_slice(&[0xA0, crlf]);
        emit_cond_jmp16(buf, pt, 0xC9, syntax_err_off)?;
    }
    let br_drain = emit_branch8_placeholder(buf, 0xCA);
    patch_s8_rel(buf, br_drain, drain_cr, "U_DRAIN")?;

    let success_off = buf.len();
    for br in to_success {
        patch_s8_rel(buf, br, success_off, "U_SUCCESS")?;
    }
    buf.extend_from_slice(&[0x4A, 0x0C]); // mov d2, (12, sp)
    emit_print_str(buf, pt, strs.2, print_span_off);
    buf.extend_from_slice(&[0xAA, 0x00]); // cmp 0, d2
    let is_lock = emit_branch8_placeholder(buf, 0xC8);
    emit_print_str(buf, pt, strs.3, print_span_off); // OK:NOR_UNLOCK
    pt.j16s.push((emit_jmp16_placeholder(buf), prompt_loop));

    let lock_msg = buf.len();
    patch_s8_rel(buf, is_lock, lock_msg, "U_LOCK_MSG")?;
    emit_print_str(buf, pt, strs.4, print_span_off); // OK:NOR_LOCK
    pt.j16s.push((emit_jmp16_placeholder(buf), prompt_loop));
    Ok(())
}

/// Emits guarded `'W'` (write/JEDEC program) or `'E'` (JEDEC sector erase) handler.
pub(super) fn emit_write_or_erase_handler(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    subs: (usize, usize, usize, usize), // (read_hex_u32, check_nor, jedec_op, print_span)
    targets: (usize, usize, usize),     // (prompt_loop, syntax_err, e05_err)
    is_erase: bool,
) -> Result<()> {
    emit_read_hex_u32_or_syntax(buf, pt, subs.0, targets.1)?;
    if !is_erase {
        buf.extend_from_slice(&[0xA0, b',']);
        emit_cond_jmp16(buf, pt, 0xC8, targets.1)?;
        buf.extend_from_slice(&[0xF1, 0xEA]);
        emit_read_hex_u32_or_syntax(buf, pt, subs.0, targets.1)?;
    } else {
        buf.extend_from_slice(&[0xF1, 0xEA]);
    }
    buf.extend_from_slice(&[0x58, 0x0C]); // mov (12, sp), d0
    emit_call16(buf, pt, subs.1);
    if is_erase {
        buf.extend_from_slice(&[0xA0, 0x01]);
        emit_cond_jmp16(buf, pt, 0xC8, targets.2)?;
        emit_print_str(buf, pt, 5, subs.3);
        emit_call16(buf, pt, subs.2);
        buf.extend_from_slice(&[0xA0, 0x00]); // cmp 0, d0
        let br_poll_err = emit_branch8_placeholder(buf, 0xC9);
        buf.extend_from_slice(&[0xF0, 0x61]); // movhu (a1), d0 (0xFFFF readback verify)
        let _ = emit_mov_imm32(buf, 0xCD, 0x0000_FFFF); // mov 0xFFFF, d1
        buf.extend_from_slice(&[0xA4]); // cmp d1, d0
        let br_rb_err = emit_branch8_placeholder(buf, 0xC9);
        emit_print_str(buf, pt, 8, subs.3); // OK:ERASE
        pt.j16s.push((emit_jmp16_placeholder(buf), targets.0));
        let err_erase = buf.len();
        patch_s8_rel(buf, br_poll_err, err_erase, "E_POLL_ERR")?;
        patch_s8_rel(buf, br_rb_err, err_erase, "E_RB_ERR")?;
        emit_print_str(buf, pt, 16, subs.3); // ERR:ERASE
    } else {
        buf.extend_from_slice(&[0xA0, 0xFF]);
        emit_cond_jmp16(buf, pt, 0xC9, targets.2)?;
        buf.extend_from_slice(&[0xA0, 0x00]); // cmp 0, d0
        let br_nor = emit_branch8_placeholder(buf, 0xC9);
        buf.extend_from_slice(&[0xF0, 0x5A]); // RAM/MMIO direct byte write
        let br_ok = emit_branch8_placeholder(buf, 0xCA);
        let nor_prog = buf.len();
        patch_s8_rel(buf, br_nor, nor_prog, "W_NOR_PROG")?;
        emit_call16(buf, pt, subs.2);
        buf.extend_from_slice(&[0xA0, 0x00]); // cmp 0, d0
        let br_w_err = emit_branch8_placeholder(buf, 0xC9);
        let wr_done = buf.len();
        patch_s8_rel(buf, br_ok, wr_done, "W_DONE")?;
        emit_print_str(buf, pt, 5, subs.3);
        emit_print_str(buf, pt, 7, subs.3);
        pt.j16s.push((emit_jmp16_placeholder(buf), targets.0));
        let err_write = buf.len();
        patch_s8_rel(buf, br_w_err, err_write, "W_ERR")?;
        emit_print_str(buf, pt, 5, subs.3);
        emit_print_str(buf, pt, 17, subs.3); // ERR:WRITE
    }
    pt.j16s.push((emit_jmp16_placeholder(buf), targets.0));
    Ok(())
}
