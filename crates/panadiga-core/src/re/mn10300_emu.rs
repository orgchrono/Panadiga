//! Deterministic Matsushita MN10300 / MN103S Mini-VM for offline patch verification.
//!
//! Models D0..D3, A0..A3, SP, PC, MDR, LIR, LAR, PSW (plus undefined-flag poison mask)
//! matching the verified Panasonic MN103 instruction specification. Zero `unsafe` code.

use serde::{Deserialize, Serialize};

pub const PSW_V: u16 = 0x0001;
pub const PSW_C: u16 = 0x0002;
pub const PSW_N: u16 = 0x0004;
pub const PSW_Z: u16 = 0x0008;
pub const PSW_FLAGS_MASK: u16 = 0x000F;
pub const PSW_RESERVED_MASK: u16 = 0xC0F0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mn103Fault {
    UnmappedRead(u32),
    UnmappedWrite(u32),
    AlignError(u32),
    ReservedPswBits(u16),
    InvalidOpcode(u8),
    UndefinedFlagRead,
    StepLimitReached,
}

#[derive(Debug, Clone)]
struct MemSegment {
    base: u32,
    writable: bool,
    data: Vec<u8>,
}

/// Deterministic MN103S register + segmented memory state machine.
#[derive(Debug, Clone)]
pub struct Mn103Vm {
    pub d: [u32; 4],
    pub a: [u32; 4],
    pub sp: u32,
    pub pc: u32,
    pub mdr: u32,
    pub lir: u32,
    pub lar: u32,
    pub psw: u16,
    pub psw_undef: u16,
    pub steps: u64,
    segments: Vec<MemSegment>,
}

impl Default for Mn103Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Mn103Vm {
    pub fn new() -> Self {
        Self {
            d: [0; 4],
            a: [0; 4],
            sp: 0,
            pc: 0,
            mdr: 0,
            lir: 0,
            lar: 0,
            psw: 0,
            psw_undef: 0,
            steps: 0,
            segments: Vec::new(),
        }
    }

    pub fn map_segment(&mut self, base: u32, data: Vec<u8>, writable: bool) {
        self.segments.push(MemSegment {
            base,
            writable,
            data,
        });
    }

    fn locate_slice(&self, addr: u32, len: usize) -> Option<(&MemSegment, usize)> {
        let addr_u64 = addr as u64;
        let end_u64 = addr_u64 + len as u64;
        for seg in &self.segments {
            let s = seg.base as u64;
            let e = s + seg.data.len() as u64;
            if addr_u64 >= s && end_u64 <= e {
                return Some((seg, (addr_u64 - s) as usize));
            }
        }
        None
    }

    fn locate_slice_mut(
        &mut self,
        addr: u32,
        len: usize,
    ) -> Result<(&mut [u8], usize), Mn103Fault> {
        let addr_u64 = addr as u64;
        let end_u64 = addr_u64 + len as u64;
        for seg in &mut self.segments {
            let s = seg.base as u64;
            let e = s + seg.data.len() as u64;
            if addr_u64 >= s && end_u64 <= e {
                if !seg.writable {
                    return Err(Mn103Fault::UnmappedWrite(addr));
                }
                return Ok((&mut seg.data, (addr_u64 - s) as usize));
            }
        }
        Err(Mn103Fault::UnmappedWrite(addr))
    }

    pub fn read_u8(&self, addr: u32) -> Result<u8, Mn103Fault> {
        let (seg, off) = self
            .locate_slice(addr, 1)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(seg.data[off])
    }

    pub fn read_u16(&self, addr: u32) -> Result<u16, Mn103Fault> {
        if (addr & 1) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        let (seg, off) = self
            .locate_slice(addr, 2)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(u16::from_le_bytes([seg.data[off], seg.data[off + 1]]))
    }

    pub fn read_u32(&self, addr: u32) -> Result<u32, Mn103Fault> {
        if (addr & 3) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        self.fetch_u32(addr)
    }

    /// Reads 4 little-endian instruction/literal bytes without data-alignment enforcement.
    fn fetch_u32(&self, addr: u32) -> Result<u32, Mn103Fault> {
        let (seg, off) = self
            .locate_slice(addr, 4)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(u32::from_le_bytes([
            seg.data[off],
            seg.data[off + 1],
            seg.data[off + 2],
            seg.data[off + 3],
        ]))
    }

    fn fetch_i16(&self, addr: u32) -> Result<i16, Mn103Fault> {
        let (seg, off) = self
            .locate_slice(addr, 2)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(i16::from_le_bytes([seg.data[off], seg.data[off + 1]]))
    }

    pub fn write_u8(&mut self, addr: u32, val: u8) -> Result<(), Mn103Fault> {
        let (slice, off) = self.locate_slice_mut(addr, 1)?;
        slice[off] = val;
        Ok(())
    }

    pub fn write_u16(&mut self, addr: u32, val: u16) -> Result<(), Mn103Fault> {
        if (addr & 1) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        let (slice, off) = self.locate_slice_mut(addr, 2)?;
        slice[off..off + 2].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) -> Result<(), Mn103Fault> {
        if (addr & 3) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        let (slice, off) = self.locate_slice_mut(addr, 4)?;
        slice[off..off + 4].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    fn set_nzvc(&mut self, v: bool, c: bool, result: u32) {
        let mut flags = 0u16;
        if v {
            flags |= PSW_V;
        }
        if c {
            flags |= PSW_C;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= PSW_N;
        }
        if result == 0 {
            flags |= PSW_Z;
        }
        self.psw = (self.psw & !PSW_FLAGS_MASK) | flags;
        self.psw_undef &= !PSW_FLAGS_MASK;
    }

    fn alu_add(&mut self, a: u32, b: u32) -> u32 {
        let wide = (a as u64) + (b as u64);
        let res = wide as u32;
        let overflow = ((!(a ^ b) & (a ^ res)) & 0x8000_0000) != 0;
        self.set_nzvc(overflow, (wide >> 32) != 0, res);
        res
    }

    fn alu_sub(&mut self, a: u32, b: u32) -> u32 {
        let res = a.wrapping_sub(b);
        let borrow = b > a;
        let overflow = (((a ^ b) & (a ^ res)) & 0x8000_0000) != 0;
        self.set_nzvc(overflow, borrow, res);
        res
    }

    fn eval_cc(&self, cc: u8) -> Result<bool, Mn103Fault> {
        let need_mask: u16 = match cc {
            0..=3 => PSW_N | PSW_V | if cc == 1 || cc == 3 { PSW_Z } else { 0 },
            4 | 6 => PSW_C,
            5 | 7 => PSW_C | PSW_Z,
            8 | 9 => PSW_Z,
            _ => return Err(Mn103Fault::InvalidOpcode(cc)),
        };
        if (self.psw_undef & need_mask) != 0 {
            return Err(Mn103Fault::UndefinedFlagRead);
        }
        let v = (self.psw & PSW_V) != 0;
        let c = (self.psw & PSW_C) != 0;
        let n = (self.psw & PSW_N) != 0;
        let z = (self.psw & PSW_Z) != 0;
        Ok(match cc {
            0 => n ^ v,
            1 => !(z || (n ^ v)),
            2 => !(n ^ v),
            3 => z || (n ^ v),
            4 => c,
            5 => !(c || z),
            6 => !c,
            7 => c || z,
            8 => z,
            9 => !z,
            _ => unreachable!(),
        })
    }

    fn save_regs(&mut self, base_sp: u32, regs_mask: u8) -> Result<(), Mn103Fault> {
        let mut cursor = base_sp;
        for (bit, reg_idx) in [(0x80u8, 2usize), (0x40, 3)] {
            if (regs_mask & bit) != 0 {
                cursor = cursor.wrapping_sub(4);
                self.write_u32(cursor, self.d[reg_idx])?;
            }
        }
        for (bit, reg_idx) in [(0x20u8, 2usize), (0x10, 3)] {
            if (regs_mask & bit) != 0 {
                cursor = cursor.wrapping_sub(4);
                self.write_u32(cursor, self.a[reg_idx])?;
            }
        }
        if (regs_mask & 0x08) != 0 {
            for val in [
                self.d[0], self.d[1], self.a[0], self.a[1], self.mdr, self.lir, self.lar, 0,
            ] {
                cursor = cursor.wrapping_sub(4);
                self.write_u32(cursor, val)?;
            }
        }
        Ok(())
    }

    fn restore_regs(&mut self, new_sp: u32, regs_mask: u8) -> Result<(), Mn103Fault> {
        let mut cursor = new_sp;
        let mut d2 = self.d[2];
        let mut d3 = self.d[3];
        let mut a2 = self.a[2];
        let mut a3 = self.a[3];
        if (regs_mask & 0x80) != 0 {
            cursor = cursor.wrapping_sub(4);
            d2 = self.read_u32(cursor)?;
        }
        if (regs_mask & 0x40) != 0 {
            cursor = cursor.wrapping_sub(4);
            d3 = self.read_u32(cursor)?;
        }
        if (regs_mask & 0x20) != 0 {
            cursor = cursor.wrapping_sub(4);
            a2 = self.read_u32(cursor)?;
        }
        if (regs_mask & 0x10) != 0 {
            cursor = cursor.wrapping_sub(4);
            a3 = self.read_u32(cursor)?;
        }
        if (regs_mask & 0x08) != 0 {
            let mut vals = [0u32; 8];
            for slot in &mut vals {
                cursor = cursor.wrapping_sub(4);
                *slot = self.read_u32(cursor)?;
            }
            self.d[0] = vals[0];
            self.d[1] = vals[1];
            self.a[0] = vals[2];
            self.a[1] = vals[3];
            self.mdr = vals[4];
            self.lir = vals[5];
            self.lar = vals[6];
        }
        self.d[2] = d2;
        self.d[3] = d3;
        self.a[2] = a2;
        self.a[3] = a3;
        Ok(())
    }

    fn step_f0(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        match p {
            0x00..=0x0F => self.a[m] = self.read_u32(self.a[n])?,
            0x10..=0x1F => self.write_u32(self.a[n], self.a[m])?,
            0x40..=0x4F => self.d[m] = self.read_u8(self.a[n])? as u32,
            0x50..=0x5F => self.write_u8(self.a[n], self.d[m] as u8)?,
            0xF0..=0xF3 => {
                let ret = self.pc.wrapping_add(2);
                self.write_u32(self.sp, ret)?;
                self.mdr = ret;
                self.pc = self.a[n];
                return Ok(());
            }
            0xF4..=0xF7 => {
                self.pc = self.a[n];
                return Ok(());
            }
            0xFC => {
                self.pc = self.read_u32(self.sp)?;
                return Ok(());
            }
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    fn step_f1(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        match p >> 4 {
            0x0 => self.d[n] = self.alu_sub(self.d[n], self.d[m]),
            0x1 => self.d[n] = self.alu_sub(self.d[n], self.a[m]),
            0x5 => self.d[n] = self.alu_add(self.d[n], self.a[m]),
            0x6 => self.a[n] = self.alu_add(self.a[n], self.d[m]),
            0x7 => self.a[n] = self.alu_add(self.a[n], self.a[m]),
            0x9 => {
                let _ = self.alu_sub(self.d[n], self.a[m]);
            }
            0xD => self.d[n] = self.a[m],
            0xE => self.a[n] = self.d[m],
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    fn step_f2(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        let res = match p {
            0x00..=0x0F => {
                let r = self.d[n] & self.d[m];
                self.d[n] = r;
                Some(r)
            }
            0x10..=0x1F => {
                let r = self.d[n] | self.d[m];
                self.d[n] = r;
                Some(r)
            }
            0x20..=0x2F => {
                let r = self.d[n] ^ self.d[m];
                self.d[n] = r;
                Some(r)
            }
            0x30..=0x33 => {
                let r = !self.d[n];
                self.d[n] = r;
                Some(r)
            }
            0xD0..=0xD3 => {
                self.mdr = if (self.d[n] & 0x8000_0000) != 0 {
                    0xFFFF_FFFF
                } else {
                    0
                };
                None
            }
            0xE0..=0xE3 => {
                self.d[n] = self.mdr;
                None
            }
            0xE4..=0xE7 => {
                self.d[n] = self.psw as u32;
                None
            }
            0xF2 | 0xF6 | 0xFA | 0xFE => {
                self.mdr = self.d[m];
                None
            }
            0xF3 | 0xF7 | 0xFB | 0xFF => {
                let v = (self.d[m] & 0xFFFF) as u16;
                if (v & PSW_RESERVED_MASK) != 0 {
                    return Err(Mn103Fault::ReservedPswBits(v));
                }
                self.psw = v;
                self.psw_undef = 0;
                None
            }
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        };
        if let Some(val) = res {
            self.set_nzvc(false, false, val);
        }
        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    fn step_fc(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let imm32 = self.fetch_u32(self.pc.wrapping_add(2))?;
        let n = (p & 3) as usize;
        match p {
            0xC0..=0xC3 => self.d[n] = self.alu_add(self.d[n], imm32),
            0xC4..=0xC7 => self.d[n] = self.alu_sub(self.d[n], imm32),
            0xC8..=0xCB => {
                let _ = self.alu_sub(self.d[n], imm32);
            }
            0xCC..=0xCF => self.d[n] = imm32,
            0xD0..=0xD3 => self.a[n] = self.alu_add(self.a[n], imm32),
            0xD4..=0xD7 => self.a[n] = self.alu_sub(self.a[n], imm32),
            0xD8..=0xDB => {
                let _ = self.alu_sub(self.a[n], imm32);
            }
            0xDC..=0xDF => self.a[n] = imm32,
            0xFF => {
                let ret = self.pc.wrapping_add(6);
                self.write_u32(self.sp, ret)?;
                self.mdr = ret;
                self.pc = self.pc.wrapping_add(imm32);
                return Ok(());
            }
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
        self.pc = self.pc.wrapping_add(6);
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), Mn103Fault> {
        let pc = self.pc;
        let op = self.read_u8(pc)?;
        let m = ((op >> 2) & 3) as usize;
        let n = (op & 3) as usize;
        match op {
            0x00 | 0x04 | 0x08 | 0x0C => {
                self.d[m] = 0;
                self.set_nzvc(false, false, 0);
                self.pc = pc.wrapping_add(1);
            }
            0x10..=0x13 => {
                self.d[n] = (self.d[n] as i8) as i32 as u32;
                self.pc = pc.wrapping_add(1);
            }
            0x14..=0x17 => {
                self.d[n] &= 0xFF;
                self.pc = pc.wrapping_add(1);
            }
            0x20..=0x23 => {
                let imm = (self.read_u8(pc + 1)? as i8) as i32 as u32;
                self.a[n] = self.alu_add(self.a[n], imm);
                self.pc = pc.wrapping_add(2);
            }
            0x28..=0x2B => {
                let imm = (self.read_u8(pc + 1)? as i8) as i32 as u32;
                self.d[n] = self.alu_add(self.d[n], imm);
                self.pc = pc.wrapping_add(2);
            }
            0x3C..=0x3F => {
                self.a[n] = self.sp;
                self.pc = pc.wrapping_add(1);
            }
            0x40 | 0x44 | 0x48 | 0x4C => {
                self.d[m] = self.alu_add(self.d[m], 1);
                self.pc = pc.wrapping_add(1);
            }
            0x41 | 0x45 | 0x49 | 0x4D => {
                self.a[m] = self.a[m].wrapping_add(1);
                self.pc = pc.wrapping_add(1);
            }
            0x42 | 0x46 | 0x4A | 0x4E => {
                let d8 = self.read_u8(pc + 1)? as u32;
                self.write_u32(self.sp.wrapping_add(d8), self.d[m])?;
                self.pc = pc.wrapping_add(2);
            }
            0x50..=0x53 => {
                self.a[n] = self.a[n].wrapping_add(4);
                self.pc = pc.wrapping_add(1);
            }
            0x58..=0x5B => {
                let d8 = self.read_u8(pc + 1)? as u32;
                self.d[n] = self.read_u32(self.sp.wrapping_add(d8))?;
                self.pc = pc.wrapping_add(2);
            }
            0x60..=0x6F => {
                self.write_u32(self.a[n], self.d[m])?;
                self.pc = pc.wrapping_add(1);
            }
            0x70..=0x7F => {
                self.d[m] = self.read_u32(self.a[n])?;
                self.pc = pc.wrapping_add(1);
            }
            0x80..=0x8F => {
                if m == n {
                    self.d[n] = (self.read_u8(pc + 1)? as i8) as i32 as u32;
                    self.pc = pc.wrapping_add(2);
                } else {
                    self.d[n] = self.d[m];
                    self.pc = pc.wrapping_add(1);
                }
            }
            0x90..=0x9F => {
                if m == n {
                    self.a[n] = self.read_u8(pc + 1)? as u32;
                    self.pc = pc.wrapping_add(2);
                } else {
                    self.a[n] = self.a[m];
                    self.pc = pc.wrapping_add(1);
                }
            }
            0xA0..=0xAF => {
                let len = if m == n { 2 } else { 1 };
                let rhs = if m == n {
                    (self.read_u8(pc + 1)? as i8) as i32 as u32
                } else {
                    self.d[m]
                };
                let _ = self.alu_sub(self.d[n], rhs);
                self.pc = pc.wrapping_add(len);
            }
            0xC0..=0xC9 => {
                let d8 = (self.read_u8(pc + 1)? as i8) as i32 as u32;
                self.pc = if self.eval_cc(op & 0x0F)? {
                    pc.wrapping_add(d8)
                } else {
                    pc.wrapping_add(2)
                };
            }
            0xCA => {
                let d8 = (self.read_u8(pc + 1)? as i8) as i32 as u32;
                self.pc = pc.wrapping_add(d8);
            }
            0xCB => self.pc = pc.wrapping_add(1),
            0xCC => {
                let d16 = self.fetch_i16(pc + 1)? as i32 as u32;
                self.pc = pc.wrapping_add(d16);
            }
            0xCD | 0xDD => {
                let is_32 = op == 0xDD;
                let insn_len: u32 = if is_32 { 7 } else { 5 };
                let disp = if is_32 {
                    self.fetch_u32(pc + 1)?
                } else {
                    self.fetch_i16(pc + 1)? as i32 as u32
                };
                let regs = self.read_u8(pc + insn_len - 2)?;
                let imm8 = self.read_u8(pc + insn_len - 1)? as u32;
                let ret = pc.wrapping_add(insn_len);
                let orig_sp = self.sp;
                self.write_u32(orig_sp, ret)?;
                self.save_regs(orig_sp, regs)?;
                self.sp = orig_sp.wrapping_sub(imm8);
                self.mdr = ret;
                self.pc = pc.wrapping_add(disp);
            }
            0xD0..=0xD9 => {
                self.pc = if self.eval_cc(op & 0x0F)? {
                    self.lar.wrapping_sub(4)
                } else {
                    pc.wrapping_add(1)
                };
            }
            0xDA => self.pc = self.lar.wrapping_sub(4),
            0xDB => {
                self.lir = self.fetch_u32(pc + 1)?;
                self.lar = pc.wrapping_add(5);
                self.pc = pc.wrapping_add(1);
            }
            0xDC => {
                let d32 = self.fetch_u32(pc + 1)?;
                self.pc = pc.wrapping_add(d32);
            }
            0xDE | 0xDF => {
                let is_retf = op == 0xDE;
                let regs = self.read_u8(pc + 1)?;
                let imm8 = self.read_u8(pc + 2)? as u32;
                let saved_mdr = self.mdr;
                let new_sp = self.sp.wrapping_add(imm8);
                self.restore_regs(new_sp, regs)?;
                let ret_pc = if is_retf {
                    saved_mdr
                } else {
                    self.read_u32(new_sp)?
                };
                self.sp = new_sp;
                self.pc = ret_pc;
            }
            0xE0..=0xEF => {
                self.d[n] = self.alu_add(self.d[n], self.d[m]);
                self.pc = pc.wrapping_add(1);
            }
            0xF0 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f0(p)?;
            }
            0xF1 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f1(p)?;
            }
            0xF2 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f2(p)?;
            }
            0xFC => {
                let p = self.read_u8(pc + 1)?;
                self.step_fc(p)?;
            }
            _ => return Err(Mn103Fault::InvalidOpcode(op)),
        }
        self.steps += 1;
        Ok(())
    }

    pub fn run_until(&mut self, stop_pc: u32, max_steps: u64) -> Result<(), Mn103Fault> {
        while self.pc != stop_pc {
            if self.steps >= max_steps {
                return Err(Mn103Fault::StepLimitReached);
            }
            self.step()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::re::bgo_unbrick::{PATCH_DEVINFO_BRA, PATCH_SUM_BRA, PROG061_BASE_VA};
    use crate::re::main_passthrough::{
        MAIN_BASE_VA, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
        PATCH_FPC_STATUS_CTRL, PATCH_ODD_INIT_STATUS, PATCH_ODD_STATE_BRANCH,
    };

    #[test]
    fn test_main_passthrough_and_bgo_patches_execute_as_designed() {
        // 1. FPC status control: `mov 1, d0; rets; nop; nop` returns to stack-top LR with D0=1.
        let mut vm = Mn103Vm::new();
        let fpc_va = MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32;
        vm.map_segment(fpc_va, PATCH_FPC_STATUS_CTRL.to_vec(), false);
        vm.map_segment(0x5000_0F00, vec![0; 0x200], true);
        vm.sp = 0x5000_1000;
        vm.write_u32(vm.sp, 0x5700_1234).unwrap();
        vm.pc = fpc_va;
        vm.run_until(0x5700_1234, 16).unwrap();
        assert_eq!(vm.d[0], 1);
        assert_eq!(vm.sp, 0x5000_1000);

        // 2. ODD state branch + init status: `jmp16 +0x00D0` -> 0x574F7C5C -> `clr d0 (80 00)` -> D0=0.
        let mut odd_vm = Mn103Vm::new();
        let br_va = MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32;
        let init_va = MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32;
        let span = (init_va - br_va) as usize + PATCH_ODD_INIT_STATUS.len();
        let mut odd_mem = vec![0xCBu8; span];
        odd_mem[..PATCH_ODD_STATE_BRANCH.len()].copy_from_slice(&PATCH_ODD_STATE_BRANCH);
        let rel = (init_va - br_va) as usize;
        odd_mem[rel..rel + PATCH_ODD_INIT_STATUS.len()].copy_from_slice(&PATCH_ODD_INIT_STATUS);
        odd_vm.map_segment(br_va, odd_mem, false);
        odd_vm.d[0] = 0xDEAD_BEEF;
        odd_vm.pc = br_va;
        odd_vm
            .run_until(init_va + PATCH_ODD_INIT_STATUS.len() as u32, 16)
            .unwrap();
        assert_eq!(odd_vm.d[0], 0);

        // 3. BGO unconditional branches: PATCH_SUM_BRA (+0x0A) -> PATCH_DEVINFO_BRA (+0x15 -> 0x1F).
        let mut bgo_vm = Mn103Vm::new();
        bgo_vm.map_segment(PROG061_BASE_VA, vec![0xCB; 0x40], false);
        bgo_vm.segments[0].data[0..2].copy_from_slice(&PATCH_SUM_BRA);
        bgo_vm.segments[0].data[0x0A..0x0C].copy_from_slice(&PATCH_DEVINFO_BRA);
        bgo_vm.pc = PROG061_BASE_VA;
        bgo_vm.run_until(PROG061_BASE_VA + 0x1F, 16).unwrap();
        assert_eq!(bgo_vm.steps, 2);
    }

    #[test]
    fn test_call_ret_setlb_and_inc_an_preserve_flags() {
        // Verify CALL16 / RET, SETLB..LNE countdown, and INC4 An not clobbering PSW flags.
        let code_base = 0x4400_0000u32;
        let mut code = vec![0xCB; 0x60];
        // 0x00: CALL16 +0x20, regs=0x80 (d2), frame=0x08 -> target 0x20, ret 0x05
        code[0x00] = 0xCD;
        code[0x01..0x03].copy_from_slice(&(0x0020i16).to_le_bytes());
        code[0x03] = 0x80;
        code[0x04] = 0x08;
        // 0x20 (callee): MOV 3, D0; SETLB; DEC D0 via ADD -1,D0; INC4 A0; LNE
        code[0x20..0x22].copy_from_slice(&[0x80, 0x03]); // mov 3, d0
        code[0x22] = 0xDB; // setlb -> LAR = 0x27, loop body = 0x23
        code[0x23..0x25].copy_from_slice(&[0x28, 0xFF]); // add -1, d0 (updates Z)
        code[0x25] = 0x50; // inc4 a0 (must NOT overwrite PSW flags!)
        code[0x26] = 0xCB; // nop (fills 4 bytes after setlb)
        code[0x27] = 0xD9; // lne -> if !Z jump LAR-4 = 0x23
        // 0x28: RET 0x80, 0x08
        code[0x28..0x2B].copy_from_slice(&[0xDF, 0x80, 0x08]);

        let mut vm = Mn103Vm::new();
        vm.map_segment(code_base, code, false);
        vm.map_segment(0x4401_0000, vec![0; 0x200], true);
        vm.sp = 0x4401_0100;
        vm.d[2] = 0x1234_5678;
        vm.a[0] = 0x1000;
        vm.pc = code_base;
        vm.run_until(code_base + 0x05, 64).unwrap();
        assert_eq!(vm.d[0], 0);
        assert_eq!(vm.a[0], 0x100C);
        assert_eq!(vm.d[2], 0x1234_5678);
        assert_eq!(vm.sp, 0x4401_0100);
    }
}
