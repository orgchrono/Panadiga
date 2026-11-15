//! Deterministic partial Matsushita MN10300 / MN103S Mini-VM for offline patch
//! and prologue/fast-path verification.
//!
//! Models D0..D3, A0..A3, SP, PC, MDR, LIR, LAR, PSW (plus undefined-flag poison mask)
//! and segmented RAM/ROM mappings with zero `unsafe` code.
//!
//! ## Honest ISA Scope (Audited vs `mn103.sinc`)
//! - **Supported**: core single-byte moves/ALU/branches (`0x00..=0xEF` subset incl.
//!   `CALL`/`RET`/`SETLB`/`Lcc`), `MOVM` (`0xCE`/`0xCF` for `D2/D3/A2/A3/OTHER`),
//!   `0xF0` subset (`CALLS`/`JMP`/`RETS`), `0xF1`/`0xF2` register ops (incl. `MOV Am, SP`),
//!   `0xF8` subset (`(d8, An)`/`(d8, SP)` moves, `ADD simm8, SP`, `AND/OR/BTST imm8`,
//!   `ASL/LSR/ASR imm8`, `BVC/BVS/BNC/BNS`), and `0xFC` subset (`(imm32)` abs moves,
//!   ALU/MOV `imm32`, `ADD simm32, SP`, `CALLS d32rel`).
//! - **Unimplemented / Out of Scope**: prefixes `0xF3..=0xF7`, `0xF9..=0xFB`, `0xFD..=0xFF`,
//!   `EX0..EX2` extended register masks in `MOVM`, hardware MMIO peripherals (`nfrom`, SDHC,
//!   DMA, timers), interrupts (`RTI`), caches, and cycle-accurate pipeline timing.

mod memory;
mod ops;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const PSW_V: u16 = 0x0001;
pub const PSW_C: u16 = 0x0002;
pub const PSW_N: u16 = 0x0004;
pub const PSW_Z: u16 = 0x0008;
pub const PSW_FLAGS_MASK: u16 = 0x000F;
pub const PSW_RESERVED_MASK: u16 = 0xC0F0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CfiStage {
    #[default]
    Normal,
    Unlock555,
    Unlock2AA,
    Autoselect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mn103Fault {
    UnmappedRead(u32),
    UnmappedWrite(u32),
    IplWriteProtected(u32),
    AlignError(u32),
    ReservedPswBits(u16),
    InvalidOpcode(u8),
    UndefinedFlagRead,
    StepLimitReached,
}

#[derive(Debug, Clone)]
pub(super) struct MemSegment {
    pub(super) base: u32,
    pub(super) writable: bool,
    pub(super) data: Vec<u8>,
}

/// Deterministic partial MN103S register + segmented memory state machine.
///
/// Intended for bounded verification of firmware patches and self-contained
/// fast-paths; unmapped MMIO/callees and unimplemented opcodes fail closed via
/// [`Mn103Fault`].
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
    pub sc0_enabled: bool,
    pub cfi_stage: CfiStage,
    pub uart_rx: VecDeque<u8>,
    pub uart_tx: Vec<u8>,
    pub(super) segments: Vec<MemSegment>,
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
            sc0_enabled: false,
            cfi_stage: CfiStage::Normal,
            uart_rx: VecDeque::new(),
            uart_tx: Vec::new(),
            segments: Vec::new(),
        }
    }

    /// Enables virtual AM34 `SC0` UART MMIO registers (`0xD4002008`/`2009`/`200C`).
    pub fn enable_sc0_uart(&mut self, initial_rx: &[u8]) {
        self.sc0_enabled = true;
        self.uart_rx.extend(initial_rx);
    }

    /// Queues additional RX bytes onto virtual `SC0` (`0xD4002009`).
    pub fn queue_uart_rx(&mut self, data: &[u8]) {
        self.uart_rx.extend(data);
    }

    /// Drains all bytes transmitted via `SC0TXB` (`0xD4002008`) as UTF-8 (lossy).
    pub fn take_uart_tx_utf8(&mut self) -> String {
        let bytes = std::mem::take(&mut self.uart_tx);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    pub fn map_segment(&mut self, base: u32, data: Vec<u8>, writable: bool) {
        self.segments.push(MemSegment {
            base,
            writable,
            data,
        });
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
                Self::validate_movm_mask(regs, op)?;
                let new_sp = self.sp.wrapping_add(Self::reg_mask_stack_bytes(regs));
                self.restore_regs(new_sp, regs)?;
                self.sp = new_sp;
                self.pc = pc.wrapping_add(2);
            }
            0xCF => {
                let regs = self.read_u8(pc + 1)?;
                Self::validate_movm_mask(regs, op)?;
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
            0xF8 => {
                let p = self.read_u8(pc + 1)?;
                self.step_f8(p)?;
            }
            0xFA => {
                let p = self.read_u8(pc + 1)?;
                self.step_fa(p)?;
            }
            0xFC => {
                let p = self.read_u8(pc + 1)?;
                self.step_fc(p)?;
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
