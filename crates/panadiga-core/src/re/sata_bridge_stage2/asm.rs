#![deny(unsafe_code)]

//! Minimal relocatable MN103S assembler used to build Stage-2 payloads and test stubs.

use std::collections::BTreeMap;

use anyhow::Result;
use byteorder::{ByteOrder, LittleEndian};

use super::super::mn10300_emit::{emit_call32_abs, emit_cmp_imm, emit_mov_imm32, patch_s16_rel};

pub(super) const LOC_MAX_LBA: i8 = -0x04;
pub(super) const LOC_LBA: i8 = -0x08;
pub(super) const LOC_SIZE: i8 = -0x0C;
pub(super) const LOC_ALIGNED_SIZE: i8 = -0x10;
pub(super) const LOC_ENTRY_ADDR: i8 = -0x14;
pub(super) const LOC_SRC_ADDR: i8 = -0x18;
pub(super) const LOC_ATTEMPT: i8 = -0x1C;
pub(super) const LOC_MOUNT_ID: i8 = -0x20;

const fn invert_cc(cc_op: u8) -> u8 {
    match cc_op {
        0xC0..=0xC7 => cc_op ^ 0x02,
        0xC8..=0xC9 => cc_op ^ 0x01,
        _ => cc_op ^ 0x01,
    }
}

pub(super) struct Stage2Asm {
    pub(super) base_va: u32,
    pub(super) printf_va: u32,
    pub(super) code: Vec<u8>,
    labels: BTreeMap<&'static str, usize>,
    jmp16_fixups: Vec<(usize, &'static str)>,
    imm32_fixups: Vec<(usize, &'static str)>,
    strings: Vec<(&'static str, &'static [u8])>,
}

impl Stage2Asm {
    pub(super) fn new(base_va: u32, printf_va: u32) -> Self {
        Self {
            base_va,
            printf_va,
            code: Vec::with_capacity(768),
            labels: BTreeMap::new(),
            jmp16_fixups: Vec::new(),
            imm32_fixups: Vec::new(),
            strings: Vec::new(),
        }
    }

    pub(super) fn label(&mut self, name: &'static str) {
        self.labels.insert(name, self.code.len());
    }

    pub(super) fn intern_cstr(&mut self, name: &'static str, s: &'static [u8]) {
        self.strings.push((name, s));
    }

    pub(super) fn clr_d(&mut self, d: u8) {
        self.code.push((d & 3) << 2);
    }

    pub(super) fn mov_d_to_d(&mut self, src_d: u8, dst_d: u8) {
        self.code.push(0x80 | ((src_d & 3) << 2) | (dst_d & 3));
    }

    pub(super) fn mov_imm8_d(&mut self, d: u8, imm8: i8) {
        let r = d & 3;
        self.code.push(0x80 | (r << 2) | r);
        self.code.push(imm8 as u8);
    }

    pub(super) fn mov_imm32_d(&mut self, d: u8, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xCC + (d & 3), imm);
    }

    #[cfg(test)]
    pub(super) fn mov_imm32_a(&mut self, a: u8, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xDC + (a & 3), imm);
    }

    pub(super) fn mov_str_d(&mut self, d: u8, str_label: &'static str) {
        let imm_off = emit_mov_imm32(&mut self.code, 0xCC + (d & 3), 0);
        self.imm32_fixups.push((imm_off, str_label));
    }

    pub(super) fn ld_local(&mut self, disp8: i8, d: u8) {
        self.code
            .extend_from_slice(&[0xF8, 0x03 | ((d & 3) << 2), disp8 as u8]);
    }

    pub(super) fn st_local(&mut self, d: u8, disp8: i8) {
        self.code
            .extend_from_slice(&[0xF8, 0x13 | ((d & 3) << 2), disp8 as u8]);
    }

    pub(super) fn lea_a3_disp_d(&mut self, disp8: i8, d: u8) {
        let r = d & 3;
        self.code
            .extend_from_slice(&[0xF1, 0xDC | r, 0x28 + r, disp8 as u8]);
    }

    pub(super) fn st_d_sp(&mut self, d: u8, u8_off: u8) {
        self.code
            .extend_from_slice(&[0x42 | ((d & 3) << 2), u8_off]);
    }

    pub(super) fn ld_abs32_d(&mut self, addr: u32, d: u8) {
        let _ = emit_mov_imm32(&mut self.code, 0xA4 + (d & 3), addr);
    }

    pub(super) fn cmp_imm8_d(&mut self, d: u8, imm8: i8) {
        emit_cmp_imm(&mut self.code, d, imm8);
    }

    pub(super) fn cmp_imm32_d0(&mut self, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xC8, imm);
    }

    pub(super) fn cmp_d_to_d0(&mut self, src_d: u8) {
        self.code.push(0xA0 | ((src_d & 3) << 2));
    }

    pub(super) fn add_imm8_d(&mut self, d: u8, imm8: i8) {
        self.code.extend_from_slice(&[0x28 + (d & 3), imm8 as u8]);
    }

    pub(super) fn add_imm32_d(&mut self, d: u8, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xC0 + (d & 3), imm);
    }

    pub(super) fn sub_imm32_d0(&mut self, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xC4, imm);
    }

    pub(super) fn asl_imm8_d(&mut self, d: u8, k: u8) {
        self.code.extend_from_slice(&[0xF8, 0xC0 | (d & 3), k]);
    }

    pub(super) fn lsr_imm8_d0(&mut self, k: u8) {
        self.code.extend_from_slice(&[0xF8, 0xC4, k]);
    }

    pub(super) fn call_abs(&mut self, target_va: u32) {
        emit_call32_abs(&mut self.code, self.base_va, target_va);
    }

    pub(super) fn printf0(&mut self, str_label: &'static str) {
        self.mov_str_d(0, str_label);
        self.call_abs(self.printf_va);
    }

    pub(super) fn jmp(&mut self, target: &'static str) {
        let off = self.code.len();
        self.code.extend_from_slice(&[0xCC, 0x00, 0x00]);
        self.jmp16_fixups.push((off, target));
    }

    /// Emits inverted short `B!cc +5` guarding a 3-byte `JMP16` so branches handle arbitrary spans.
    pub(super) fn jcc(&mut self, cc_op: u8, target: &'static str) {
        self.code.extend_from_slice(&[invert_cc(cc_op), 0x05]);
        self.jmp(target);
    }

    pub(super) fn finalize(mut self) -> Result<Vec<u8>> {
        for (name, bytes) in self.strings {
            let off = self.code.len();
            self.labels.insert(name, off);
            self.code.extend_from_slice(bytes);
            self.code.push(0x00);
        }
        while (self.code.len() & 3) != 0 {
            self.code.push(0x00);
        }
        for (insn_off, target) in self.jmp16_fixups {
            let tgt_off = *self
                .labels
                .get(target)
                .ok_or_else(|| anyhow::anyhow!("Unknown Stage-2 branch target '{target}'"))?;
            patch_s16_rel(&mut self.code, insn_off, tgt_off, target)?;
        }
        for (imm_off, target) in self.imm32_fixups {
            let tgt_off = *self
                .labels
                .get(target)
                .ok_or_else(|| anyhow::anyhow!("Unknown Stage-2 string label '{target}'"))?;
            let va = self.base_va + tgt_off as u32;
            LittleEndian::write_u32(&mut self.code[imm_off..imm_off + 4], va);
        }
        Ok(self.code)
    }
}
