#![deny(unsafe_code)]

//! AM33/AM34 extended prefix handlers (`0xF3`, `0xF4`, `0xF5`, `0xF9`, `0xFB`, `0xFD`, `0xFE`).

use super::{Mn103Fault, Mn103Vm};

impl Mn103Vm {
    #[inline]
    fn read_xr(&self, idx: usize, op: u8) -> Result<u32, Mn103Fault> {
        if (idx & 3) == 0 {
            Ok(self.sp)
        } else {
            Err(Mn103Fault::InvalidOpcode(op))
        }
    }

    #[inline]
    fn write_xr(&mut self, idx: usize, val: u32, op: u8) -> Result<(), Mn103Fault> {
        if (idx & 3) == 0 {
            self.sp = val;
            Ok(())
        } else {
            Err(Mn103Fault::InvalidOpcode(op))
        }
    }

    #[inline]
    fn load_to_reg16(&mut self, dst: usize, addr: u32, width: u8) -> Result<(), Mn103Fault> {
        let val = match width {
            1 => self.read_u8(addr)? as u32,
            2 => self.read_u16(addr)? as u32,
            _ => self.read_u32(addr)?,
        };
        self.write_reg16(dst, val);
        Ok(())
    }

    #[inline]
    fn store_from_reg16(&mut self, src: usize, addr: u32, width: u8) -> Result<(), Mn103Fault> {
        let val = self.read_reg16(src);
        match width {
            1 => self.write_u8(addr, val as u8),
            2 => self.write_u16(addr, val as u16),
            _ => self.write_u32(addr, val),
        }
    }

    #[inline]
    fn decode_dm_an(&self, p: u8) -> (u8, usize, u32) {
        let u = ((p >> 4) & 3) as usize;
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        (p >> 6, u, self.d[m].wrapping_add(self.a[n]))
    }

    /// `0xF3`: base indexed `(Dm, An)` 32-bit load/store.
    pub(super) fn step_f3(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let (op, u, addr) = self.decode_dm_an(p);
        match op {
            0 => self.d[u] = self.read_u32(addr)?,
            1 => self.write_u32(addr, self.d[u])?,
            2 => self.a[u] = self.read_u32(addr)?,
            _ => self.write_u32(addr, self.a[u])?,
        }
        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    /// `0xF4`: base indexed `(Dm, An)` byte/halfword unsigned load/store.
    pub(super) fn step_f4(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let (op, u, addr) = self.decode_dm_an(p);
        match op {
            0 => self.d[u] = self.read_u8(addr)? as u32,
            1 => self.write_u8(addr, self.d[u] as u8)?,
            2 => self.d[u] = self.read_u16(addr)? as u32,
            _ => self.write_u16(addr, self.d[u] as u16)?,
        }
        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    /// `0xF5`: AM33 2-byte move between `A0..A3`/`D0..D3` and `R0..R15`.
    pub(super) fn step_f5(&mut self, p: u8) -> Result<(), Mn103Fault> {
        match p >> 6 {
            0 => self.write_reg16((p & 0x0F) as usize, self.a[((p >> 4) & 3) as usize]),
            1 => self.write_reg16((p & 0x0F) as usize, self.d[((p >> 4) & 3) as usize]),
            2 => self.a[(p & 3) as usize] = self.read_reg16(((p >> 2) & 0x0F) as usize),
            _ => self.d[(p & 3) as usize] = self.read_reg16(((p >> 2) & 0x0F) as usize),
        }
        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    /// `0xF9`: AM33 3-byte register-register, unary, and indirect/post-inc instructions.
    pub(super) fn step_f9(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let reg = self.read_u8(self.pc.wrapping_add(2))?;
        let hi = (reg >> 4) as usize;
        let lo = (reg & 0x0F) as usize;
        let ptr_lo = self.read_reg16(lo);
        match p {
            0x08 => self.write_reg16(lo, self.read_reg16(hi)),
            0xE8 => {
                let xr = self.read_xr(hi, p)?;
                self.write_reg16(lo, xr);
            }
            0xF8 => {
                let v = self.read_reg16(hi);
                self.write_xr(lo, v, p)?;
            }
            0x0A => self.load_to_reg16(hi, ptr_lo, 4)?,
            0x1A => self.store_from_reg16(hi, ptr_lo, 4)?,
            0x2A => self.load_to_reg16(hi, ptr_lo, 1)?,
            0x3A => self.store_from_reg16(hi, ptr_lo, 1)?,
            0x4A => self.load_to_reg16(hi, ptr_lo, 2)?,
            0x5A => self.store_from_reg16(hi, ptr_lo, 2)?,
            0x6A => {
                let val = self.read_u32(ptr_lo)?;
                self.write_reg16(lo, ptr_lo.wrapping_add(4));
                self.write_reg16(hi, val);
            }
            0x7A => {
                self.store_from_reg16(hi, ptr_lo, 4)?;
                self.write_reg16(lo, ptr_lo.wrapping_add(4));
            }
            0xEA => {
                let val = self.read_u16(ptr_lo)? as u32;
                self.write_reg16(lo, ptr_lo.wrapping_add(2));
                self.write_reg16(hi, val);
            }
            0xFA => {
                self.store_from_reg16(hi, ptr_lo, 2)?;
                self.write_reg16(lo, ptr_lo.wrapping_add(2));
            }
            0x8A => self.load_to_reg16(hi, self.sp, 4)?,
            0x9A => self.store_from_reg16(hi, self.sp, 4)?,
            0xAA => self.load_to_reg16(hi, self.sp, 1)?,
            0xBA => self.store_from_reg16(hi, self.sp, 1)?,
            0xCA => self.load_to_reg16(hi, self.sp, 2)?,
            0xDA => self.store_from_reg16(hi, self.sp, 2)?,
            0x18 => self.mdr = ((ptr_lo as i32) >> 31) as u32,
            0x28 => self.write_reg16(lo, (ptr_lo as i8) as i32 as u32),
            0x38 => self.write_reg16(lo, ptr_lo & 0xFF),
            0x48 => self.write_reg16(lo, (ptr_lo as i16) as i32 as u32),
            0x58 => self.write_reg16(lo, ptr_lo & 0xFFFF),
            0x68 => {
                self.write_reg16(lo, 0);
                self.set_nzvc(false, false, 0);
            }
            0x78 => {
                let v = self.alu_add(ptr_lo, self.read_reg16(hi));
                self.write_reg16(lo, v);
            }
            0x88 => {
                let v = self.alu_addc(ptr_lo, self.read_reg16(hi));
                self.write_reg16(lo, v);
            }
            0x98 => {
                let v = self.alu_sub(ptr_lo, self.read_reg16(hi));
                self.write_reg16(lo, v);
            }
            0xA8 => {
                let v = self.alu_subc(ptr_lo, self.read_reg16(hi));
                self.write_reg16(lo, v);
            }
            0xB8 => {
                let next = if (8..=11).contains(&lo) {
                    ptr_lo.wrapping_add(1)
                } else {
                    self.alu_add(ptr_lo, 1)
                };
                self.write_reg16(lo, next);
            }
            0xC8 => self.write_reg16(lo, ptr_lo.wrapping_add(4)),
            0xD8 => {
                let _ = self.alu_sub(ptr_lo, self.read_reg16(hi));
            }
            0x09 => {
                let r = ptr_lo & self.read_reg16(hi);
                self.write_reg16(lo, r);
                self.set_nzvc(false, false, r);
            }
            0x19 => {
                let r = ptr_lo | self.read_reg16(hi);
                self.write_reg16(lo, r);
                self.set_nzvc(false, false, r);
            }
            0x29 => {
                let r = ptr_lo ^ self.read_reg16(hi);
                self.write_reg16(lo, r);
                self.set_nzvc(false, false, r);
            }
            0x39 => {
                let r = !ptr_lo;
                self.write_reg16(lo, r);
                self.set_nzvc(false, false, r);
            }
            0x49 => {
                let v = self.alu_shift(ptr_lo, self.read_reg16(hi), 2);
                self.write_reg16(lo, v);
            }
            0x59 => {
                let v = self.alu_shift(ptr_lo, self.read_reg16(hi), 1);
                self.write_reg16(lo, v);
            }
            0x69 => {
                let v = self.alu_shift(ptr_lo, self.read_reg16(hi), 0);
                self.write_reg16(lo, v);
            }
            0x79 => self.write_reg16(lo, ptr_lo.wrapping_shl(2)),
            0x89 => {
                let v = self.alu_ror(ptr_lo);
                self.write_reg16(lo, v);
            }
            0x99 => {
                let v = self.alu_rol(ptr_lo);
                self.write_reg16(lo, v);
            }
            0xA9 | 0xB9 => {
                let (low, high) = self.alu_mul(ptr_lo, self.read_reg16(hi), p == 0xA9);
                self.write_reg16(lo, low);
                self.mdr = high;
            }
            0xC9 | 0xD9 => {
                let (q, rem) = self.alu_div(ptr_lo, self.mdr, self.read_reg16(hi), p == 0xC9)?;
                self.write_reg16(lo, q);
                self.mdr = rem;
            }
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
        self.pc = self.pc.wrapping_add(3);
        Ok(())
    }

    pub(super) fn exec_am33_imm_or_mem(
        &mut self,
        p: u8,
        reg: u8,
        uval: u32,
        sval: u32,
        insn_len: u32,
    ) -> Result<(), Mn103Fault> {
        let hi = (reg >> 4) as usize;
        let lo = (reg & 0x0F) as usize;
        let cur_lo = self.read_reg16(lo);
        let disp_rm = cur_lo.wrapping_add(sval);
        let disp_sp = self.sp.wrapping_add(uval);
        match p {
            0x0A => self.load_to_reg16(hi, disp_rm, 4)?,
            0x1A => self.store_from_reg16(hi, disp_rm, 4)?,
            0x2A => self.load_to_reg16(hi, disp_rm, 1)?,
            0x3A => self.store_from_reg16(hi, disp_rm, 1)?,
            0x4A => self.load_to_reg16(hi, disp_rm, 2)?,
            0x5A => self.store_from_reg16(hi, disp_rm, 2)?,
            0x6A => {
                let val = self.read_u32(cur_lo)?;
                self.write_reg16(lo, cur_lo.wrapping_add(sval));
                self.write_reg16(hi, val);
            }
            0x7A => {
                self.store_from_reg16(hi, cur_lo, 4)?;
                self.write_reg16(lo, cur_lo.wrapping_add(sval));
            }
            0x8A => self.load_to_reg16(hi, disp_sp, 4)?,
            0x9A => self.store_from_reg16(hi, disp_sp, 4)?,
            0xAA => self.load_to_reg16(hi, disp_sp, 1)?,
            0xBA => self.store_from_reg16(hi, disp_sp, 1)?,
            0xCA => self.load_to_reg16(hi, disp_sp, 2)?,
            0xDA => self.store_from_reg16(hi, disp_sp, 2)?,
            0x0E => self.load_to_reg16(hi, uval, 4)?,
            0x1E => self.store_from_reg16(hi, uval, 4)?,
            0x2E => self.load_to_reg16(hi, uval, 1)?,
            0x3E => self.store_from_reg16(hi, uval, 1)?,
            0x4E => self.load_to_reg16(hi, uval, 2)?,
            0x5E => self.store_from_reg16(hi, uval, 2)?,
            0x08 => self.write_reg16(lo, sval),
            0x18 => self.write_reg16(lo, uval),
            0xF8 => self.write_xr(lo, sval, p)?,
            0x78 => {
                let v = self.alu_add(cur_lo, sval);
                self.write_reg16(lo, v);
            }
            0x88 => {
                let v = self.alu_addc(cur_lo, sval);
                self.write_reg16(lo, v);
            }
            0x98 => {
                let v = self.alu_sub(cur_lo, sval);
                self.write_reg16(lo, v);
            }
            0xA8 => {
                let v = self.alu_subc(cur_lo, sval);
                self.write_reg16(lo, v);
            }
            0xD8 => {
                let _ = self.alu_sub(cur_lo, sval);
            }
            0x09 => {
                let r = cur_lo & uval;
                self.write_reg16(lo, r);
                self.set_nzvc(false, false, r);
            }
            0x19 => {
                let r = cur_lo | uval;
                self.write_reg16(lo, r);
                self.set_nzvc(false, false, r);
            }
            0x29 => {
                let r = cur_lo ^ uval;
                self.write_reg16(lo, r);
                self.set_nzvc(false, false, r);
            }
            0xE9 => {
                let r = cur_lo & uval;
                self.set_nzvc(false, false, r);
            }
            0x49 => {
                let v = self.alu_shift(cur_lo, uval, 2);
                self.write_reg16(lo, v);
            }
            0x59 => {
                let v = self.alu_shift(cur_lo, uval, 1);
                self.write_reg16(lo, v);
            }
            0x69 => {
                let v = self.alu_shift(cur_lo, uval, 0);
                self.write_reg16(lo, v);
            }
            0xA9 | 0xB9 => {
                let rhs = if p == 0xA9 { sval } else { uval };
                let (low, high) = self.alu_mul(cur_lo, rhs, p == 0xA9);
                self.write_reg16(lo, low);
                self.mdr = high;
            }
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
        self.pc = self.pc.wrapping_add(insn_len);
        Ok(())
    }

    /// `0xFB`: AM33 3-operand ALU, 3-register indexed memory, and `imm8`/`d8` operations.
    pub(super) fn step_fb(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let reg = self.read_u8(self.pc.wrapping_add(2))?;
        let b3 = self.read_u8(self.pc.wrapping_add(3))?;
        let hi = (reg >> 4) as usize;
        let lo = (reg & 0x0F) as usize;
        let rd2 = (b3 >> 4) as usize;
        let rd0 = (b3 & 0x0F) as usize;
        let idx_addr = self.read_reg16(hi).wrapping_add(self.read_reg16(lo));
        match p {
            0x7C => {
                let v = self.alu_add(self.read_reg16(hi), self.read_reg16(lo));
                self.write_reg16(rd2, v);
            }
            0x8C => {
                let v = self.alu_addc(self.read_reg16(lo), self.read_reg16(hi));
                self.write_reg16(rd2, v);
            }
            0x9C => {
                let v = self.alu_sub(self.read_reg16(lo), self.read_reg16(hi));
                self.write_reg16(rd2, v);
            }
            0xAC => {
                let v = self.alu_subc(self.read_reg16(lo), self.read_reg16(hi));
                self.write_reg16(rd2, v);
            }
            0x0D | 0x1D | 0x2D => {
                let a = self.read_reg16(hi);
                let b = self.read_reg16(lo);
                let r = match p {
                    0x0D => a & b,
                    0x1D => a | b,
                    _ => a ^ b,
                };
                self.write_reg16(rd2, r);
                self.set_nzvc(false, false, r);
            }
            0x4D | 0x5D | 0x6D => {
                let kind = match p {
                    0x6D => 0,
                    0x5D => 1,
                    _ => 2,
                };
                let v = self.alu_shift(self.read_reg16(lo), self.read_reg16(hi), kind);
                self.write_reg16(rd2, v);
            }
            0xAD | 0xBD => {
                let (low, high) = self.alu_mul(self.read_reg16(hi), self.read_reg16(lo), p == 0xAD);
                self.write_reg16(rd2, low);
                self.write_reg16(rd0, high);
                self.set_zn_flags(low == 0 && high == 0, (high & 0x8000_0000) != 0);
            }
            0x8E => self.load_to_reg16(rd2, idx_addr, 4)?,
            0x9E => self.store_from_reg16(rd2, idx_addr, 4)?,
            0xAE => self.load_to_reg16(rd2, idx_addr, 1)?,
            0xBE => self.store_from_reg16(rd2, idx_addr, 1)?,
            0xCE => self.load_to_reg16(rd2, idx_addr, 2)?,
            0xDE => self.store_from_reg16(rd2, idx_addr, 2)?,
            _ => {
                return self.exec_am33_imm_or_mem(p, reg, b3 as u32, (b3 as i8) as i32 as u32, 4);
            }
        }
        self.pc = self.pc.wrapping_add(4);
        Ok(())
    }

    /// `0xFD`: AM33 6-byte `imm24`/`d24` operations.
    pub(super) fn step_fd(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let reg = self.read_u8(self.pc.wrapping_add(2))?;
        let u24 = self.fetch_u24(self.pc.wrapping_add(3))?;
        let s24 = (((u24 << 8) as i32) >> 8) as u32;
        self.exec_am33_imm_or_mem(p, reg, u24, s24, 6)
    }

    /// `0xFE` AM33 subset: 7-byte `imm32`/`d32` operations.
    pub(super) fn step_fe_am33(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let reg = self.read_u8(self.pc.wrapping_add(2))?;
        let imm32 = self.fetch_u32(self.pc.wrapping_add(3))?;
        self.exec_am33_imm_or_mem(p, reg, imm32, imm32, 7)
    }
}
