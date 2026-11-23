//! ALU flag arithmetic, condition codes, and `0xF0`/`0xF1`/`0xF2` prefixed handlers.

use super::{Mn103Fault, Mn103Vm, PSW_C, PSW_FLAGS_MASK, PSW_N, PSW_RESERVED_MASK, PSW_V, PSW_Z};

impl Mn103Vm {
    pub(super) fn set_zn_flags(&mut self, zero: bool, neg: bool) {
        let mut mask = 0u16;
        if neg {
            mask |= PSW_N;
        }
        if zero {
            mask |= PSW_Z;
        }
        self.psw = (self.psw & !(PSW_N | PSW_Z)) | mask;
        self.psw_undef &= !(PSW_N | PSW_Z);
    }

    pub(super) fn set_zn(&mut self, result: u32) {
        self.set_zn_flags(result == 0, (result & 0x8000_0000) != 0);
    }

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

    pub(super) fn alu_add_with_carry(&mut self, a: u32, b: u32, carry_in: bool) -> u32 {
        let wide = (a as u64) + (b as u64) + u64::from(carry_in);
        let res = wide as u32;
        let overflow = ((!(a ^ b) & (a ^ res)) & 0x8000_0000) != 0;
        self.set_nzvc(overflow, (wide >> 32) != 0, res);
        res
    }

    pub(super) fn alu_add(&mut self, a: u32, b: u32) -> u32 {
        self.alu_add_with_carry(a, b, false)
    }

    pub(super) fn alu_addc(&mut self, a: u32, b: u32) -> u32 {
        self.alu_add_with_carry(a, b, (self.psw & PSW_C) != 0)
    }

    pub(super) fn alu_sub_with_borrow(&mut self, a: u32, b: u32, borrow_in: bool) -> u32 {
        let b_in = u32::from(borrow_in);
        let res = a.wrapping_sub(b).wrapping_sub(b_in);
        let borrow = (a as u64) < (b as u64) + u64::from(b_in);
        let overflow = (((a ^ b) & (a ^ res)) & 0x8000_0000) != 0;
        self.set_nzvc(overflow, borrow, res);
        res
    }

    pub(super) fn alu_sub(&mut self, a: u32, b: u32) -> u32 {
        self.alu_sub_with_borrow(a, b, false)
    }

    pub(super) fn alu_subc(&mut self, a: u32, b: u32) -> u32 {
        self.alu_sub_with_borrow(a, b, (self.psw & PSW_C) != 0)
    }

    pub(super) fn alu_mul(&mut self, a: u32, b: u32, signed: bool) -> (u32, u32) {
        let wide: u64 = if signed {
            ((a as i32) as i64).wrapping_mul((b as i32) as i64) as u64
        } else {
            (a as u64) * (b as u64)
        };
        let lo = wide as u32;
        let hi = (wide >> 32) as u32;
        self.set_zn(lo);
        (lo, hi)
    }

    pub(super) fn alu_div(
        &mut self,
        lo: u32,
        hi: u32,
        divisor: u32,
        signed: bool,
    ) -> Result<(u32, u32), Mn103Fault> {
        if divisor == 0 {
            return Err(Mn103Fault::InvalidOpcode(0x60));
        }
        let (quot, rem) = if signed {
            let dividend = ((hi as i64) << 32) | (lo as u64 as i64);
            let div = (divisor as i32) as i64;
            let q = dividend.wrapping_div(div) as u32;
            let r = dividend.wrapping_rem(div) as u32;
            (q, r)
        } else {
            let dividend = ((hi as u64) << 32) | (lo as u64);
            let div = divisor as u64;
            ((dividend / div) as u32, (dividend % div) as u32)
        };
        self.set_zn(quot);
        Ok((quot, rem))
    }

    pub(super) fn alu_shift(&mut self, orig: u32, k_raw: u32, kind: u8) -> u32 {
        let k = k_raw & 0x1F;
        let (res, carry) = if k == 0 {
            (orig, (self.psw & PSW_C) != 0)
        } else {
            match kind {
                0 => (orig.wrapping_shl(k), ((orig >> (32 - k)) & 1) != 0),
                1 => (orig.wrapping_shr(k), ((orig >> (k - 1)) & 1) != 0),
                _ => (((orig as i32) >> k) as u32, ((orig >> (k - 1)) & 1) != 0),
            }
        };
        self.set_nzvc(false, carry, res);
        res
    }

    pub(super) fn alu_ror(&mut self, val: u32) -> u32 {
        let old_cf = u32::from((self.psw & PSW_C) != 0);
        let new_cf = (val & 1) != 0;
        let res = (val >> 1) | (old_cf << 31);
        self.set_nzvc(false, new_cf, res);
        res
    }

    pub(super) fn alu_rol(&mut self, val: u32) -> u32 {
        let old_cf = u32::from((self.psw & PSW_C) != 0);
        let new_cf = (val & 0x8000_0000) != 0;
        let res = (val << 1) | old_cf;
        self.set_nzvc(false, new_cf, res);
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

    pub(super) fn step_f1(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        match p >> 4 {
            0x0 => self.d[n] = self.alu_sub(self.d[n], self.d[m]),
            0x1 => self.d[n] = self.alu_sub(self.d[n], self.a[m]),
            0x2 => self.a[n] = self.alu_sub(self.a[n], self.d[m]),
            0x3 => self.a[n] = self.alu_sub(self.a[n], self.a[m]),
            0x4 => self.d[n] = self.alu_addc(self.d[n], self.d[m]),
            0x5 => self.d[n] = self.alu_add(self.d[n], self.a[m]),
            0x6 => self.a[n] = self.alu_add(self.a[n], self.d[m]),
            0x7 => self.a[n] = self.alu_add(self.a[n], self.a[m]),
            0x8 => self.d[n] = self.alu_subc(self.d[n], self.d[m]),
            0x9 => {
                let _ = self.alu_sub(self.d[n], self.a[m]);
            }
            0xA => {
                let _ = self.alu_sub(self.a[n], self.d[m]);
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
            0x40..=0x5F => {
                let (lo, hi) = self.alu_mul(self.d[n], self.d[m], p < 0x50);
                self.d[n] = lo;
                self.mdr = hi;
                None
            }
            0x60..=0x7F => {
                let (q, r) = self.alu_div(self.d[n], self.mdr, self.d[m], p < 0x70)?;
                self.d[n] = q;
                self.mdr = r;
                None
            }
            0x80..=0x83 => {
                self.d[n] = self.alu_rol(self.d[n]);
                None
            }
            0x84..=0x87 => {
                self.d[n] = self.alu_ror(self.d[n]);
                None
            }
            0x90..=0x9F => {
                self.d[n] = self.alu_shift(self.d[n], self.d[m], 0);
                None
            }
            0xA0..=0xAF => {
                self.d[n] = self.alu_shift(self.d[n], self.d[m], 1);
                None
            }
            0xB0..=0xBF => {
                self.d[n] = self.alu_shift(self.d[n], self.d[m], 2);
                None
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
            0xEC..=0xEF => {
                self.d[n] = self.epsw | (self.psw as u32);
                None
            }
            0xF0 | 0xF4 | 0xF8 | 0xFC => {
                self.sp = self.a[m];
                None
            }
            0xF1 | 0xF5 | 0xF9 | 0xFD => {
                self.epsw = self.d[m];
                self.psw = (self.d[m] & 0xFFFF) as u16;
                self.psw_undef = 0;
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
}
