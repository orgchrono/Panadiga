//! Deterministic Matsushita MN10300 / MN103S / AM33-AM34 integer VM for offline
//! patch and firmware-subsystem verification.
//!
//! Models base MN103 (`D0..D3`, `A0..A3`, `SP`, `PC`, `MDR`, `LIR`, `LAR`, `PSW` +
//! undefined-flag poison mask) and AM33/AM34 extensions (`R0..R7`/`E0..E7`, `MDRQ`,
//! `MCRH`, `MCRL`, `MCVF`, `EPSW`, `EX0`/`EX1`/`EX2` stack masks) with zero `unsafe`.
//!
//! ## Honest ISA Scope (Audited vs `mn103.sinc` + `am33.sinc`)
//! - **Supported**: core `0x00..=0xEF` (incl. `MOV`/`MOVBU`/`MOVHU` `(u16)`/`(u8, SP)`,
//!   `MOV imm16/simm16`, `ASL2`, `CALL`/`RET`/`SETLB`/`Lcc`), `MOVM` (`0xCE`/`0xCF` incl.
//!   `EX0`/`EX1`/`EX2`), `0xF0`..`0xF5` (indexed `(Dm, An)` + AM33 `A/D <-> R0..R15`),
//!   `0xF8`..`0xFE` (incl. AM33 `0xF9` reg/indirect/mul/div, `0xFB` 3-op/indexed/`simm8`,
//!   `0xFD` `imm24`, and `0xFE` bitops/`imm32`).
//! - **Unimplemented / Out of Scope**: FPU coprocessor (`FMOV`/`FMUL`), hardware MMIO
//!   peripherals beyond `SC0` UART and Spansion CFI NOR (`nfrom` DMA, timers), interrupts
//!   (`RTI`), caches, and cycle-accurate pipeline timing.

mod memory;
mod ops;
mod ops_am33;
mod ops_ext;
mod spansion_nor;
mod step_core;

#[cfg(test)]
mod tests;

pub use spansion_nor::{is_critical_ipl_boot_va, is_ipl_readonly_va};

use crate::pico::rsp_codec::policy::NorWritePolicy;
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
    CfiQuery,
    EraseSetup,
    EraseUnlock555,
    EraseUnlock2AA,
    ProgramPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct NorBusyOp {
    pub(super) va_base: u32,
    pub(super) target_off: u32,
    pub(super) len: u32,
    pub(super) is_erase: bool,
    pub(super) prog_val: u16,
    pub(super) prog_width: usize,
    pub(super) expected_dq7: u8,
    pub(super) remaining_polls: u32,
    pub(super) toggle_phase: bool,
    pub(super) force_dq5: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum Mn103Fault {
    #[error("Unmapped read @ 0x{0:08X}")]
    UnmappedRead(u32),
    #[error("Unmapped write @ 0x{0:08X}")]
    UnmappedWrite(u32),
    #[error("Spansion IPL read-only flash write protection triggered @ 0x{0:08X} (E05)")]
    IplWriteProtected(u32),
    #[error("FORBIDDEN lethal AM34 register access @ 0x{0:08X} (0xC000100C halts WDT/bus)")]
    LethalRegisterRead(u32),
    #[error("Unaligned memory access @ 0x{0:08X}")]
    AlignError(u32),
    #[error("Attempt to write reserved PSW bits 0x{0:04X}")]
    ReservedPswBits(u16),
    #[error("Invalid/unimplemented MN103S opcode 0x{0:02X}")]
    InvalidOpcode(u8),
    #[error("Undefined PSW flag read")]
    UndefinedFlagRead,
    #[error("VM execution step limit reached")]
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
    /// AM33/AM34 extended general-purpose registers `R0..R7` (`E0..E7`).
    pub r: [u32; 8],
    pub sp: u32,
    pub pc: u32,
    pub mdr: u32,
    pub mdrq: u32,
    pub mcrh: u32,
    pub mcrl: u32,
    pub mcvf: u32,
    pub lir: u32,
    pub lar: u32,
    pub psw: u16,
    pub epsw: u32,
    pub psw_undef: u16,
    pub steps: u64,
    pub sc0_enabled: bool,
    pub cfi_stage: CfiStage,
    pub nor_write_policy: NorWritePolicy,
    /// Optional Spansion erase/program poll cycles before DQ7 transitions (`0` = instant).
    pub nor_busy_cycles_config: u32,
    /// Injects Spansion `DQ5` exceeded timing limit fault on NOR erase/program operations.
    pub nor_force_dq5_timeout: bool,
    pub(super) nor_busy_state: Option<NorBusyOp>,
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
            r: [0; 8],
            sp: 0,
            pc: 0,
            mdr: 0,
            mdrq: 0,
            mcrh: 0,
            mcrl: 0,
            mcvf: 0,
            lir: 0,
            lar: 0,
            psw: 0,
            epsw: 0,
            psw_undef: 0,
            steps: 0,
            sc0_enabled: false,
            cfi_stage: CfiStage::Normal,
            nor_write_policy: NorWritePolicy::NvramOnly,
            nor_busy_cycles_config: 0,
            nor_force_dq5_timeout: false,
            nor_busy_state: None,
            uart_rx: VecDeque::new(),
            uart_tx: Vec::new(),
            segments: Vec::new(),
        }
    }

    /// Reads one of the 16 AM33 registers (`0..=7 => R0..R7`, `8..=11 => A0..A3`, `12..=15 => D0..D3`).
    #[inline]
    pub(super) fn read_reg16(&self, idx: usize) -> u32 {
        match idx {
            0..=7 => self.r[idx],
            8..=11 => self.a[idx - 8],
            12..=15 => self.d[idx - 12],
            _ => unreachable!(),
        }
    }

    /// Writes one of the 16 AM33 registers (`0..=7 => R0..R7`, `8..=11 => A0..A3`, `12..=15 => D0..D3`).
    #[inline]
    pub(super) fn write_reg16(&mut self, idx: usize, val: u32) {
        match idx {
            0..=7 => self.r[idx] = val,
            8..=11 => self.a[idx - 8] = val,
            12..=15 => self.d[idx - 12] = val,
            _ => unreachable!(),
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

    pub fn segment_bytes(&self, base: u32) -> Option<&[u8]> {
        self.segments
            .iter()
            .find(|s| s.base == base)
            .map(|s| s.data.as_slice())
    }
}
