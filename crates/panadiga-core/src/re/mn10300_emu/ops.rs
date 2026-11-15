//! ALU flag arithmetic, condition codes, and prefixed opcode handlers (`0xF0`/`F1`/`F2`/`F8`/`FC`).

use super::{Mn103Fault, Mn103Vm, PSW_C, PSW_FLAGS_MASK, PSW_N, PSW_RESERVED_MASK, PSW_V, PSW_Z};

impl Mn103Vm {
    pub(super) fn set_nzvc(&mut self, v: bool, c: bool, result: u32) {
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

    pub(super) fn alu_add(&mut self, a: u32, b: u32) -> u32 {
        let wide = (a as u64) + (b as u64);
        let res = wide as u32;
        let overflow = ((!(a ^ b) & (a ^ res)) & 0x8000_0000) != 0;
        self.set_nzvc(overflow, (wide >> 32) != 0, res);
        res
    }

    pub(super) fn alu_sub(&mut self, a: u32, b: u32) -> u32 {
        let res = a.wrapping_sub(b);
        let borrow = b > a;
        let overflow = (((a ^ b) & (a ^ res)) & 0x8000_0000) != 0;
        self.set_nzvc(overflow, borrow, res);
        res
    }

    pub(super) fn eval_cc(&self, cc: u8) -> Result<bool, Mn103Fault> {
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

    pub(super) fn exec_bitop(&mut self, addr: u32, mask: u8, mode: u8) -> Result<(), Mn103Fault> {
        let orig = self.read_u8(addr)?;
        let zero = (orig & mask) == 0;
        self.psw = if zero {
            self.psw | PSW_Z
        } else {
            self.psw & !PSW_Z
        };
        self.psw_undef &= !PSW_Z;
        match mode {
            0 => self.write_u8(addr, orig | mask)?,
            1 => self.write_u8(addr, orig & !mask)?,
            2 => {}
            _ => return Err(Mn103Fault::InvalidOpcode(mode)),
        }
        Ok(())
    }

    pub(super) fn step_f0(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        match p {
            0x00..=0x0F => self.a[m] = self.read_u32(self.a[n])?,
            0x10..=0x1F => self.write_u32(self.a[n], self.a[m])?,
            0x40..=0x4F => self.d[m] = self.read_u8(self.a[n])? as u32,
            0x50..=0x5F => self.write_u8(self.a[n], self.d[m] as u8)?,
            0x60..=0x6F => self.d[m] = self.read_u16(self.a[n])? as u32,
            0x70..=0x7F => self.write_u16(self.a[n], self.d[m] as u16)?,
            0x80..=0x8F => self.exec_bitop(self.a[n], self.d[m] as u8, 0)?,
            0x90..=0x9F => self.exec_bitop(self.a[n], self.d[m] as u8, 1)?,
            0xF0..=0xF3 => {
                let ret = self.pc.wrapping_add(2);
                self.sp = self.sp.wrapping_sub(4);
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
                self.sp = self.sp.wrapping_add(4);
                return Ok(());
            }
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    pub(super) fn step_f1(&mut self, p: u8) -> Result<(), Mn103Fault> {
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

    pub(super) fn step_f2(&mut self, p: u8) -> Result<(), Mn103Fault> {
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
            0xF0 | 0xF4 | 0xF8 | 0xFC => {
                self.sp = self.a[m];
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

    fn step_f8_shift(&mut self, p: u8, n: usize, imm8: u8) {
        let k = (imm8 & 0x1F) as u32;
        let orig = self.d[n];
        let (res, carry) = if k == 0 {
            (orig, (self.psw & PSW_C) != 0)
        } else {
            match (p >> 2) & 3 {
                0 => (orig.wrapping_shl(k), ((orig >> (32 - k)) & 1) != 0),
                1 => (orig.wrapping_shr(k), ((orig >> (k - 1)) & 1) != 0),
                _ => (
                    ((orig as i32).wrapping_shr(k)) as u32,
                    ((orig >> (k - 1)) & 1) != 0,
                ),
            }
        };
        self.d[n] = res;
        self.set_nzvc(false, carry, res);
    }

    pub(super) fn step_f8(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let op3 = self.read_u8(self.pc.wrapping_add(2))?;
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        let sext8 = (op3 as i8) as i32 as u32;
        let disp_an = self.a[n].wrapping_add(sext8);
        let disp_sp_sext = self.sp.wrapping_add(sext8);
        let disp_sp_zext = self.sp.wrapping_add(op3 as u32);
        match p {
            0x00..=0x0F => self.d[m] = self.read_u32(disp_an)?,
            0x10..=0x1F => self.write_u32(disp_an, self.d[m])?,
            0x20..=0x2F => self.a[m] = self.read_u32(disp_an)?,
            0x30..=0x3F => self.write_u32(disp_an, self.a[m])?,
            0x40..=0x4F => self.d[m] = self.read_u8(disp_an)? as u32,
            0x50..=0x5F => self.write_u8(disp_an, self.d[m] as u8)?,
            0x60..=0x6F => self.d[m] = self.read_u16(disp_an)? as u32,
            0x70..=0x7F => self.write_u16(disp_an, self.d[m] as u16)?,
            0x92 | 0x96 | 0x9A | 0x9E => self.write_u8(disp_sp_zext, self.d[m] as u8)?,
            0x93 | 0x97 | 0x9B | 0x9F => self.write_u16(disp_sp_zext, self.d[m] as u16)?,
            0xB8..=0xBB => self.d[n] = self.read_u8(disp_sp_sext)? as u32,
            0xBC..=0xBF => self.d[n] = self.read_u16(disp_sp_sext)? as u32,
            0xC0..=0xCB => self.step_f8_shift(p, n, op3),
            0xE0..=0xE3 => {
                let r = self.d[n] & (op3 as u32);
                self.d[n] = r;
                self.set_nzvc(false, false, r);
            }
            0xE4..=0xE7 => {
                let r = self.d[n] | (op3 as u32);
                self.d[n] = r;
                self.set_nzvc(false, false, r);
            }
            0xE8..=0xEB => {
                let flag = if p <= 0xE9 { PSW_V } else { PSW_N };
                if (self.psw_undef & flag) != 0 {
                    return Err(Mn103Fault::UndefinedFlagRead);
                }
                let want_set = (p & 1) != 0;
                let is_set = (self.psw & flag) != 0;
                self.pc = if is_set == want_set {
                    self.pc.wrapping_add(sext8)
                } else {
                    self.pc.wrapping_add(3)
                };
                return Ok(());
            }
            0xEC..=0xEF => self.set_nzvc(false, false, self.d[n] & (op3 as u32)),
            0xF0..=0xF3 => self.a[n] = self.read_u32(disp_sp_sext)?,
            0xFE => self.sp = self.alu_add(self.sp, sext8),
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
        self.pc = self.pc.wrapping_add(3);
        Ok(())
    }

    pub(super) fn step_fc(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let imm32 = self.fetch_u32(self.pc.wrapping_add(2))?;
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        match p {
            0x80 | 0x84 | 0x88 | 0x8C => self.write_u32(imm32, self.a[m])?,
            0x81 | 0x85 | 0x89 | 0x8D => self.write_u32(imm32, self.d[m])?,
            0x82 | 0x86 | 0x8A | 0x8E => self.write_u8(imm32, self.d[m] as u8)?,
            0x83 | 0x87 | 0x8B | 0x8F => self.write_u16(imm32, self.d[m] as u16)?,
            0xA0..=0xA3 => self.a[n] = self.read_u32(imm32)?,
            0xA4..=0xA7 => self.d[n] = self.read_u32(imm32)?,
            0xA8..=0xAB => self.d[n] = self.read_u8(imm32)? as u32,
            0xAC..=0xAF => self.d[n] = self.read_u16(imm32)? as u32,
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
            0xFE => self.sp = self.alu_add(self.sp, imm32),
            0xFF => {
                let ret = self.pc.wrapping_add(6);
                self.sp = self.sp.wrapping_sub(4);
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

    pub(super) fn step_fa(&mut self, p: u8) -> Result<(), Mn103Fault> {
        if !(0xF0..=0xFB).contains(&p) {
            return Err(Mn103Fault::InvalidOpcode(p));
        }
        let mode = (p >> 2) & 3;
        let n = (p & 3) as usize;
        let d8 = self.read_u8(self.pc.wrapping_add(2))?;
        let imm8 = self.read_u8(self.pc.wrapping_add(3))?;
        let disp = if mode == 2 {
            d8 as u32
        } else {
            (d8 as i8) as i32 as u32
        };
        self.exec_bitop(self.a[n].wrapping_add(disp), imm8, mode)?;
        self.pc = self.pc.wrapping_add(4);
        Ok(())
    }

    pub(super) fn step_fe(&mut self, p: u8) -> Result<(), Mn103Fault> {
        match p {
            0x00..=0x02 => {
                let addr = self.fetch_u32(self.pc.wrapping_add(2))?;
                let imm8 = self.read_u8(self.pc.wrapping_add(6))?;
                self.exec_bitop(addr, imm8, p & 3)?;
                self.pc = self.pc.wrapping_add(7);
                Ok(())
            }
            0x80..=0x82 => {
                let addr = (self.fetch_i16(self.pc.wrapping_add(2))? as u16) as u32;
                let imm8 = self.read_u8(self.pc.wrapping_add(4))?;
                self.exec_bitop(addr, imm8, p & 3)?;
                self.pc = self.pc.wrapping_add(5);
                Ok(())
            }
            _ => Err(Mn103Fault::InvalidOpcode(p)),
        }
    }
}
