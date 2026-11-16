//! ALU flag arithmetic, condition codes, and `0xF0`/`0xF1`/`0xF2` prefixed handlers.

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
}
