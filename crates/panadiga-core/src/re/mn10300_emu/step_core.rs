//! Primary instruction fetch/decode loop and bounded execution driver for [`Mn103Vm`].

use super::{Mn103Fault, Mn103Vm};

impl Mn103Vm {
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
            0x01 | 0x05 | 0x09 | 0x0D => {
                let addr = (self.fetch_i16(pc + 1)? as u16) as u32;
                self.write_u32(addr, self.d[m])?;
                self.pc = pc.wrapping_add(3);
            }
            0x02 | 0x06 | 0x0A | 0x0E => {
                let addr = (self.fetch_i16(pc + 1)? as u16) as u32;
                self.write_u8(addr, self.d[m] as u8)?;
                self.pc = pc.wrapping_add(3);
            }
            0x03 | 0x07 | 0x0B | 0x0F => {
                let addr = (self.fetch_i16(pc + 1)? as u16) as u32;
                self.write_u16(addr, self.d[m] as u16)?;
                self.pc = pc.wrapping_add(3);
            }
            0x10..=0x13 => {
                self.d[n] = (self.d[n] as i8) as i32 as u32;
                self.pc = pc.wrapping_add(1);
            }
            0x14..=0x17 => {
                self.d[n] &= 0xFF;
                self.pc = pc.wrapping_add(1);
            }
            0x18..=0x1B => {
                self.d[n] = (self.d[n] as i16) as i32 as u32;
                self.pc = pc.wrapping_add(1);
            }
            0x1C..=0x1F => {
                self.d[n] &= 0xFFFF;
                self.pc = pc.wrapping_add(1);
            }
            0x20..=0x23 => {
                let imm = (self.read_u8(pc + 1)? as i8) as i32 as u32;
                self.a[n] = self.alu_add(self.a[n], imm);
                self.pc = pc.wrapping_add(2);
            }
            0x24..=0x27 => {
                self.a[n] = (self.fetch_i16(pc + 1)? as u16) as u32;
                self.pc = pc.wrapping_add(3);
            }
            0x28..=0x2B => {
                let imm = (self.read_u8(pc + 1)? as i8) as i32 as u32;
                self.d[n] = self.alu_add(self.d[n], imm);
                self.pc = pc.wrapping_add(2);
            }
            0x2C..=0x2F => {
                self.d[n] = (self.fetch_i16(pc + 1)? as i32) as u32;
                self.pc = pc.wrapping_add(3);
            }
            0x30..=0x33 => {
                let addr = (self.fetch_i16(pc + 1)? as u16) as u32;
                self.d[n] = self.read_u32(addr)?;
                self.pc = pc.wrapping_add(3);
            }
            0x34..=0x37 => {
                let addr = (self.fetch_i16(pc + 1)? as u16) as u32;
                self.d[n] = self.read_u8(addr)? as u32;
                self.pc = pc.wrapping_add(3);
            }
            0x38..=0x3B => {
                let addr = (self.fetch_i16(pc + 1)? as u16) as u32;
                self.d[n] = self.read_u16(addr)? as u32;
                self.pc = pc.wrapping_add(3);
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
            0x43 | 0x47 | 0x4B | 0x4F => {
                let d8 = self.read_u8(pc + 1)? as u32;
                self.write_u32(self.sp.wrapping_add(d8), self.a[m])?;
                self.pc = pc.wrapping_add(2);
            }
            0x50..=0x53 => {
                self.a[n] = self.a[n].wrapping_add(4);
                self.pc = pc.wrapping_add(1);
            }
            0x54..=0x57 => {
                self.d[n] = self.d[n].wrapping_shl(2);
                self.pc = pc.wrapping_add(1);
            }
            0x58..=0x5B => {
                let d8 = self.read_u8(pc + 1)? as u32;
                self.d[n] = self.read_u32(self.sp.wrapping_add(d8))?;
                self.pc = pc.wrapping_add(2);
            }
            0x5C..=0x5F => {
                let d8 = self.read_u8(pc + 1)? as u32;
                self.a[n] = self.read_u32(self.sp.wrapping_add(d8))?;
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
            0xA0..=0xBF => {
                let is_addr = op >= 0xB0;
                let len = if m == n { 2 } else { 1 };
                let rhs = if m == n {
                    (self.read_u8(pc + 1)? as i8) as i32 as u32
                } else if is_addr {
                    self.a[m]
                } else {
                    self.d[m]
                };
                let lhs = if is_addr { self.a[n] } else { self.d[n] };
                let _ = self.alu_sub(lhs, rhs);
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
            0xCE => {
                let regs = self.read_u8(pc + 1)?;
                let new_sp = self.sp.wrapping_add(Self::reg_mask_stack_bytes(regs));
                self.restore_regs(new_sp, regs)?;
                self.sp = new_sp;
                self.pc = pc.wrapping_add(2);
            }
            0xCF => {
                let regs = self.read_u8(pc + 1)?;
                self.save_regs(self.sp, regs)?;
                self.sp = self.sp.wrapping_sub(Self::reg_mask_stack_bytes(regs));
                self.pc = pc.wrapping_add(2);
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
            0xF3 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f3(p)?;
            }
            0xF4 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f4(p)?;
            }
            0xF5 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f5(p)?;
            }
            0xF8 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f8(p)?;
            }
            0xF9 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f9(p)?;
            }
            0xFA => {
                let p = self.read_u8(pc + 1)?;
                self.step_fa(p)?;
            }
            0xFB => {
                let p = self.read_u8(pc + 1)?;
                self.step_fb(p)?;
            }
            0xFC => {
                let p = self.read_u8(pc + 1)?;
                self.step_fc(p)?;
            }
            0xFD => {
                let p = self.read_u8(pc + 1)?;
                self.step_fd(p)?;
            }
            0xFE => {
                let p = self.read_u8(pc + 1)?;
                self.step_fe(p)?;
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
