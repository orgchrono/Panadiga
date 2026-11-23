//! Extended prefixed MN103S opcode handlers (`0xF8`, `0xFA`, `0xFC`, `0xFE`).

use super::{Mn103Fault, Mn103Vm, PSW_N, PSW_RESERVED_MASK, PSW_V};

impl Mn103Vm {
    fn step_an_disp_mem(&mut self, p: u8, disp: u32) -> Result<(), Mn103Fault> {
        let m = ((p >> 2) & 3) as usize;
        match p >> 4 {
            0x0 => self.d[m] = self.read_u32(disp)?,
            0x1 => self.write_u32(disp, self.d[m])?,
            0x2 => self.a[m] = self.read_u32(disp)?,
            0x3 => self.write_u32(disp, self.a[m])?,
            0x4 => self.d[m] = self.read_u8(disp)? as u32,
            0x5 => self.write_u8(disp, self.d[m] as u8)?,
            0x6 => self.d[m] = self.read_u16(disp)? as u32,
            0x7 => self.write_u16(disp, self.d[m] as u16)?,
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
        Ok(())
    }

    fn step_sp_store(&mut self, p: u8, disp_sp: u32) -> Result<(), Mn103Fault> {
        let m = ((p >> 2) & 3) as usize;
        match p & 3 {
            0 => self.write_u32(disp_sp, self.a[m])?,
            1 => self.write_u32(disp_sp, self.d[m])?,
            2 => self.write_u8(disp_sp, self.d[m] as u8)?,
            _ => self.write_u16(disp_sp, self.d[m] as u16)?,
        }
        Ok(())
    }

    fn step_sp_load(&mut self, p: u8, disp_sp: u32) -> Result<(), Mn103Fault> {
        let n = (p & 3) as usize;
        match (p >> 2) & 3 {
            0 => self.a[n] = self.read_u32(disp_sp)?,
            1 => self.d[n] = self.read_u32(disp_sp)?,
            2 => self.d[n] = self.read_u8(disp_sp)? as u32,
            _ => self.d[n] = self.read_u16(disp_sp)? as u32,
        }
        Ok(())
    }

    pub(super) fn step_f8(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let op3 = self.read_u8(self.pc.wrapping_add(2))?;
        let n = (p & 3) as usize;
        let sext8 = (op3 as i8) as i32 as u32;
        let disp_an = self.a[n].wrapping_add(sext8);
        let disp_sp = self.sp.wrapping_add(op3 as u32);
        match p {
            0x00..=0x7F => self.step_an_disp_mem(p, disp_an)?,
            0x92 | 0x96 | 0x9A | 0x9E | 0x93 | 0x97 | 0x9B | 0x9F => {
                self.step_sp_store(p, disp_sp)?;
            }
            0xB8..=0xBF => self.step_sp_load(p, disp_sp)?,
            0xC0..=0xCB => self.d[n] = self.alu_shift(self.d[n], op3 as u32, (p >> 2) & 3),
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
            0xF0..=0xF3 => self.a[n] = self.read_u32(disp_sp)?,
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
            0x00..=0x7F => self.step_an_disp_mem(p, self.a[n].wrapping_add(imm32))?,
            0x80 | 0x84 | 0x88 | 0x8C => self.write_u32(imm32, self.a[m])?,
            0x81 | 0x85 | 0x89 | 0x8D => self.write_u32(imm32, self.d[m])?,
            0x82 | 0x86 | 0x8A | 0x8E => self.write_u8(imm32, self.d[m] as u8)?,
            0x83 | 0x87 | 0x8B | 0x8F => self.write_u16(imm32, self.d[m] as u16)?,
            0x90..=0x9F => self.step_sp_store(p, self.sp.wrapping_add(imm32))?,
            0xA0..=0xA3 => self.a[n] = self.read_u32(imm32)?,
            0xA4..=0xA7 => self.d[n] = self.read_u32(imm32)?,
            0xA8..=0xAB => self.d[n] = self.read_u8(imm32)? as u32,
            0xAC..=0xAF => self.d[n] = self.read_u16(imm32)? as u32,
            0xB0..=0xBF => self.step_sp_load(p, self.sp.wrapping_add(imm32))?,
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
            0xE0..=0xE3 => {
                let r = self.d[n] & imm32;
                self.d[n] = r;
                self.set_nzvc(false, false, r);
            }
            0xE4..=0xE7 => {
                let r = self.d[n] | imm32;
                self.d[n] = r;
                self.set_nzvc(false, false, r);
            }
            0xE8..=0xEB => {
                let r = self.d[n] ^ imm32;
                self.d[n] = r;
                self.set_nzvc(false, false, r);
            }
            0xEC..=0xEF => self.set_nzvc(false, false, self.d[n] & imm32),
            0xFC => {
                self.epsw &= imm32;
                self.psw &= (imm32 & 0xFFFF) as u16;
                self.psw_undef = 0;
            }
            0xFD => {
                self.epsw |= imm32;
                self.psw |= (imm32 & 0xFFFF) as u16;
                self.psw_undef = 0;
            }
            0xFE => self.sp = self.alu_add(self.sp, imm32),
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

    pub(super) fn step_fa(&mut self, p: u8) -> Result<(), Mn103Fault> {
        let m = ((p >> 2) & 3) as usize;
        let n = (p & 3) as usize;
        if (0xF0..=0xFB).contains(&p) {
            let mode = (p >> 2) & 3;
            let d8 = self.read_u8(self.pc.wrapping_add(2))?;
            let imm8 = self.read_u8(self.pc.wrapping_add(3))?;
            let disp = if mode == 2 {
                d8 as u32
            } else {
                (d8 as i8) as i32 as u32
            };
            self.exec_bitop(self.a[n].wrapping_add(disp), imm8, mode)?;
            self.pc = self.pc.wrapping_add(4);
            return Ok(());
        }
        let s16 = self.fetch_i16(self.pc.wrapping_add(2))?;
        let uval = (s16 as u16) as u32;
        let sval = (s16 as i32) as u32;
        match p {
            0x00..=0x7F => self.step_an_disp_mem(p, self.a[n].wrapping_add(sval))?,
            0x80 | 0x84 | 0x88 | 0x8C => self.write_u32(uval, self.a[m])?,
            0x90..=0x9F => self.step_sp_store(p, self.sp.wrapping_add(uval))?,
            0xA0..=0xA3 => self.a[n] = self.read_u32(uval)?,
            0xB0..=0xBF => self.step_sp_load(p, self.sp.wrapping_add(uval))?,
            0xC0..=0xC3 => self.d[n] = self.alu_add(self.d[n], sval),
            0xC8..=0xCB => {
                let _ = self.alu_sub(self.d[n], sval);
            }
            0xD0..=0xD3 => self.a[n] = self.alu_add(self.a[n], sval),
            0xD8..=0xDB => {
                let _ = self.alu_sub(self.a[n], sval);
            }
            0xE0..=0xE3 => {
                let r = self.d[n] & uval;
                self.d[n] = r;
                self.set_nzvc(false, false, r);
            }
            0xE4..=0xE7 => {
                let r = self.d[n] | uval;
                self.d[n] = r;
                self.set_nzvc(false, false, r);
            }
            0xE8..=0xEB => {
                let r = self.d[n] ^ uval;
                self.d[n] = r;
                self.set_nzvc(false, false, r);
            }
            0xEC..=0xEF => self.set_nzvc(false, false, self.d[n] & uval),
            0xFC | 0xFD => {
                let v = if p == 0xFC {
                    self.psw & (uval as u16)
                } else {
                    self.psw | (uval as u16)
                };
                if (v & PSW_RESERVED_MASK) != 0 {
                    return Err(Mn103Fault::ReservedPswBits(v));
                }
                self.psw = v;
                self.psw_undef = 0;
            }
            0xFE => self.sp = self.alu_add(self.sp, sval),
            0xFF => {
                let ret = self.pc.wrapping_add(4);
                self.write_u32(self.sp, ret)?;
                self.mdr = ret;
                self.pc = self.pc.wrapping_add(sval);
                return Ok(());
            }
            _ => return Err(Mn103Fault::InvalidOpcode(p)),
        }
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
            _ => self.step_fe_am33(p),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Mn103Vm, PSW_C, PSW_N, PSW_V, PSW_Z};

    #[test]
    fn executes_fc_e0_to_e3_and_imm32_with_nzvc_flags() {
        let mut vm = Mn103Vm::new();
        let code = vec![
            0xFC, 0xE0, 0x88, 0x00, 0x00, 0x00, // and 0x00000088, d0 -> 0x08 (Z=0, N=0)
            0xFC, 0xE1, 0x00, 0x00, 0x00, 0x80, // and 0x80000000, d1 -> 0x80000000 (N=1, Z=0)
            0xFC, 0xE2, 0x0F, 0x00, 0x00, 0x00, // and 0x0000000F, d2 -> 0 (Z=1, N=0)
            0xCB, // nop
        ];
        vm.map_segment(0x5000_0000, code, false);
        vm.d[0] = 0x58;
        vm.d[1] = 0x8000_0001;
        vm.d[2] = 0xF0;
        vm.psw = PSW_V | PSW_C;
        vm.pc = 0x5000_0000;

        vm.step().unwrap();
        assert_eq!(vm.d[0], 0x08);
        assert_eq!(vm.psw & (PSW_N | PSW_Z | PSW_V | PSW_C), 0);

        vm.step().unwrap();
        assert_eq!(vm.d[1], 0x8000_0000);
        assert_eq!(vm.psw & (PSW_N | PSW_Z | PSW_V | PSW_C), PSW_N);

        vm.step().unwrap();
        assert_eq!(vm.d[2], 0);
        assert_eq!(vm.psw & (PSW_N | PSW_Z | PSW_V | PSW_C), PSW_Z);
    }
}
