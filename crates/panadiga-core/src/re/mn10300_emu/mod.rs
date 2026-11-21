//! Deterministic partial Matsushita MN10300 / MN103S Mini-VM for offline patch
//! and prologue/fast-path verification.
//!
//! Models D0..D3, A0..A3, SP, PC, MDR, LIR, LAR, PSW (plus undefined-flag poison mask)
//! and segmented RAM/ROM mappings with zero `unsafe` code.
//!
//! ## Honest ISA Scope (Audited vs `mn103.sinc`)
//! - **Supported**: core single-byte moves/ALU/branches (`0x00..=0xEF` subset incl.
//!   `CALL`/`RET`/`SETLB`/`Lcc`), `MOVM` (`0xCE`/`0xCF` for `D2/D3/A2/A3/OTHER`),
//!   `0xF0` subset (`CALLS`/`JMP`/`RETS`), `0xF1`/`0xF2` register ops (incl. `MOV Am, SP` and `CMP Dm, An`),
//!   `0xF8` subset (`(d8, An)`/`(d8, SP)` moves, `ADD simm8, SP`, `AND/OR/BTST imm8`,
//!   `ASL/LSR/ASR imm8`, `BVC/BVS/BNC/BNS`), `0xFB 0xAA` (`MOVBU (d8, SP), Rn2`), and `0xFC` subset
//!   (`(imm32)` abs moves, ALU/MOV `imm32`, `ADD simm32, SP`, `CALLS d32rel`).
//! - **Unimplemented / Out of Scope**: prefixes `0xF3..=0xF7`, `0xF9`, `0xFB` (outside `0xAA`), `0xFD`, `0xFF`,
//!   `EX0..EX2` extended register masks in `MOVM`, hardware MMIO peripherals (`nfrom`, SDHC,
//!   DMA, timers), interrupts (`RTI`), caches, and cycle-accurate pipeline timing.

mod memory;
mod ops;
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
            nor_write_policy: NorWritePolicy::NvramOnly,
            nor_busy_cycles_config: 0,
            nor_force_dq5_timeout: false,
            nor_busy_state: None,
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
}
