#![deny(unsafe_code)]

//! Reusable Matsushita MN10300 / MN103S position-coupled machine-code emitters.
//!
//! Shared as the Single Source of Truth across `prog061_uart_monitor` (`PANAMON1`)
//! and `davis_probe` (`CH0`/`CH1` & `IDENTIFY DEVICE 0xEC` SDRAM stubs).

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use crate::re::nor_dumper::PUTC_D3_BYTES;

/// Leaf `SC0` UART0 TX subroutine (`PUTC D3`), terminating with `RET [], 0`.
pub const MONITOR_PUTC_BYTES: [u8; 46] = PUTC_D3_BYTES;

/// Leaf `SC0` UART0 RX subroutine (`GETC -> D0`), terminating with `RET [], 0`.
pub const MONITOR_GETC_BYTES: [u8; 23] = [
    0xFC, 0xAC, 0x0C, 0x20, 0x00, 0xD4, 0x1C, 0xF8, 0xE0, 0x10, 0xA0, 0x00, 0xC8, 0xF4, 0xFC, 0xA8,
    0x09, 0x20, 0x00, 0xD4, 0xDF, 0x00, 0x00,
];

/// Deferred branch, call, and string-span relocation table for position-coupled stubs.
#[derive(Debug, Clone, Default)]
pub struct PatchTable {
    pub str_refs: Vec<(usize, usize, usize)>,
    pub calls16: Vec<(usize, usize)>,
    pub j16s: Vec<(usize, usize)>,
    pub beq8s: Vec<(usize, usize)>,
    pub bne8s: Vec<(usize, usize)>,
}

/// Emits `0xFC <sub_op> <imm32_le>` and returns the byte offset of the 32-bit immediate.
pub fn emit_mov_imm32(buf: &mut Vec<u8>, sub_op: u8, imm: u32) -> usize {
    let off = buf.len() + 2;
    buf.extend_from_slice(&[0xFC, sub_op]);
    buf.extend_from_slice(&imm.to_le_bytes());
    off
}

/// Emits `AND imm32, D0` (`0xFC 0xE0 <imm32_le>`).
pub fn emit_and_imm32_d0(buf: &mut Vec<u8>, imm: u32) {
    let _ = emit_mov_imm32(buf, 0xE0, imm);
}

/// Emits `MOV Dm, An` (`0xF1 0xE0 | (d_reg << 2) | a_reg`).
pub fn emit_mov_d_to_a(buf: &mut Vec<u8>, d_reg: u8, a_reg: u8) {
    buf.push(0xF1);
    buf.push(0xE0 | ((d_reg & 3) << 2) | (a_reg & 3));
}

/// Emits `MOVHU (An), Dm` (`0xF0 0x60 | (d_reg << 2) | a_reg`).
pub fn emit_movhu_mem(buf: &mut Vec<u8>, a_reg: u8, d_reg: u8) {
    buf.push(0xF0);
    buf.push(0x60 | ((d_reg & 3) << 2) | (a_reg & 3));
}

/// Emits `MOVBU (An), Dm` (`0xF0 0x40 | (d_reg << 2) | a_reg`).
pub fn emit_movbu_mem(buf: &mut Vec<u8>, a_reg: u8, d_reg: u8) {
    buf.push(0xF0);
    buf.push(0x40 | ((d_reg & 3) << 2) | (a_reg & 3));
}

/// Emits 32-bit store `MOV Dm, (An)` (`0x60 | (d_reg << 2) | a_reg`).
pub fn emit_st_w_an_dm(buf: &mut Vec<u8>, d_reg: u8, a_reg: u8) {
    buf.push(0x60 | ((d_reg & 3) << 2) | (a_reg & 3));
}

/// Emits 32-bit load `MOV (An), Dm` (`0x70 | (d_reg << 2) | a_reg`).
pub fn emit_ld_w_an_dm(buf: &mut Vec<u8>, a_reg: u8, d_reg: u8) {
    buf.push(0x70 | ((d_reg & 3) << 2) | (a_reg & 3));
}

/// Emits 32-bit absolute store `MOV D0, (abs32)` (`0xFC 0x81 <abs32_le>`).
pub fn emit_store_d0_abs32(buf: &mut Vec<u8>, addr: u32) {
    let _ = emit_mov_imm32(buf, 0x81, addr);
}

/// Emits `CMP simm8, Dn` (`0xA0 | (d_reg << 2) | d_reg, simm8`).
pub fn emit_cmp_imm(buf: &mut Vec<u8>, d_reg: u8, imm8: i8) {
    let r = d_reg & 3;
    buf.push(0xA0 | (r << 2) | r);
    buf.push(imm8 as u8);
}

/// Emits subroutine return `RETS` (`0xF0 0xFC`).
pub fn emit_rets(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[0xF0, 0xFC]);
}

/// Emits `ADD simm8, SP` (`0xF8 0xFE <simm8>`).
pub fn emit_add_sp_imm8(buf: &mut Vec<u8>, imm8: i8) {
    buf.extend_from_slice(&[0xF8, 0xFE, imm8 as u8]);
}

/// Emits `CALL d32rel, [], 0` (`0xDD <rel32_le> 0x00 0x00`) targeting `target_va` from `base_va + buf.len()`.
pub fn emit_call32_abs(buf: &mut Vec<u8>, base_va: u32, target_va: u32) {
    let cur_va = base_va.wrapping_add(buf.len() as u32);
    let rel = target_va.wrapping_sub(cur_va);
    buf.push(0xDD);
    buf.extend_from_slice(&rel.to_le_bytes());
    buf.extend_from_slice(&[0x00, 0x00]);
}

fn emit_call16_frame(buf: &mut Vec<u8>, pt: &mut PatchTable, target: usize, frame_bytes: u8) {
    let off = buf.len();
    buf.extend_from_slice(&[0xCD, 0x00, 0x00, 0x00, frame_bytes]);
    pt.calls16.push((off, target));
}

/// Emits `CALL16 target, [], 0` matching Panasonic MN103S compiler convention.
pub fn emit_call16(buf: &mut Vec<u8>, pt: &mut PatchTable, target: usize) {
    emit_call16_frame(buf, pt, target, 0x00);
}

/// Emits `CALL16 target, [], 0` to a leaf `RET [], 0` stub.
pub fn emit_call16_leaf(buf: &mut Vec<u8>, pt: &mut PatchTable, target: usize) {
    emit_call16_frame(buf, pt, target, 0x00);
}

/// Emits `JMP16` (`0xCC 0x00 0x00`) placeholder and returns its instruction offset.
pub fn emit_jmp16_placeholder(buf: &mut Vec<u8>) -> usize {
    let off = buf.len();
    buf.extend_from_slice(&[0xCC, 0x00, 0x00]);
    off
}

/// Emits 2-byte relative branch placeholder (`opcode 0x00`) and returns its instruction offset.
pub fn emit_branch8_placeholder(buf: &mut Vec<u8>, opcode: u8) -> usize {
    let off = buf.len();
    buf.extend_from_slice(&[opcode, 0x00]);
    off
}

fn compute_rel_disp(
    insn_off: usize,
    target_off: usize,
    min_disp: isize,
    max_disp: isize,
    width_label: &str,
    kind: &str,
) -> Result<isize> {
    let disp = (target_off as isize) - (insn_off as isize);
    if !(min_disp..=max_disp).contains(&disp) {
        bail!("MN103S {kind} displacement out of {width_label} range: {disp}");
    }
    Ok(disp)
}

/// Patches a 16-bit signed instruction-relative displacement (`CALL16` / `JMP16`).
pub fn patch_s16_rel(buf: &mut [u8], insn_off: usize, target_off: usize, kind: &str) -> Result<()> {
    let disp = compute_rel_disp(insn_off, target_off, -32768, 32767, "s16", kind)?;
    LittleEndian::write_i16(&mut buf[insn_off + 1..insn_off + 3], disp as i16);
    Ok(())
}

/// Patches an 8-bit signed instruction-relative displacement (`Bcc` / `BRA`).
pub fn patch_s8_rel(buf: &mut [u8], insn_off: usize, target_off: usize, kind: &str) -> Result<()> {
    let disp = compute_rel_disp(insn_off, target_off, -128, 127, "s8", kind)?;
    buf[insn_off + 1] = (disp as i8) as u8;
    Ok(())
}

/// Emits a backward `BNE` (`0xC9 <s8>`) targeting `target_off`.
pub fn emit_bne_back(buf: &mut Vec<u8>, target_off: usize, kind: &str) -> Result<()> {
    let insn_off = emit_branch8_placeholder(buf, 0xC9);
    patch_s8_rel(buf, insn_off, target_off, kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_verified_mn103s_opcodes_and_relocations() {
        let mut buf = Vec::new();
        let imm_off = emit_mov_imm32(&mut buf, 0xDC, 0x8900_0D20);
        assert_eq!(imm_off, 2);
        assert_eq!(buf, vec![0xFC, 0xDC, 0x20, 0x0D, 0x00, 0x89]);

        emit_and_imm32_d0(&mut buf, 0x88);
        assert_eq!(&buf[6..12], &[0xFC, 0xE0, 0x88, 0x00, 0x00, 0x00]);

        emit_mov_d_to_a(&mut buf, 2, 1); // F1 E9
        emit_movhu_mem(&mut buf, 1, 0); // F0 61
        emit_movbu_mem(&mut buf, 0, 1); // F0 44
        emit_st_w_an_dm(&mut buf, 0, 1); // 61
        emit_ld_w_an_dm(&mut buf, 0, 0); // 70
        emit_cmp_imm(&mut buf, 0, 0x08); // A0 08
        emit_add_sp_imm8(&mut buf, -0x10); // F8 FE F0
        emit_rets(&mut buf); // F0 FC

        assert_eq!(
            &buf[12..],
            &[
                0xF1, 0xE9, 0xF0, 0x61, 0xF0, 0x44, 0x61, 0x70, 0xA0, 0x08, 0xF8, 0xFE, 0xF0, 0xF0,
                0xFC,
            ]
        );
    }
}
