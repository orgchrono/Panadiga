//! Spansion `S29JL064H` NOR flash JEDEC state machine and aperture policy helpers for [`Mn103Vm`].

use super::{CfiStage, Mn103Fault, Mn103Vm, NorBusyOp};
use crate::constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_IPL_SIZE, AM34_FLASH_SIZE,
    AM34_FLASH_UNCACHED_BASE, SPANSION_MAIN_SECTOR_SIZE_BYTES, SPANSION_MICROSECTOR_SIZE,
};
use crate::pico::rsp_codec::policy::NorWritePolicy;

pub(super) fn flash_aperture_and_offset(addr: u32) -> Option<(u32, u32)> {
    for base in [AM34_FLASH_BASE, AM34_FLASH_UNCACHED_BASE] {
        if let Some(off) = addr.checked_sub(base)
            && off < AM34_FLASH_SIZE
        {
            return Some((base, off));
        }
    }
    None
}

/// Returns `true` when `addr` lies in the permanent read-only boot IPL (`SA0..SA10`, first 256 KiB).
pub fn is_critical_ipl_boot_va(addr: u32) -> bool {
    flash_aperture_and_offset(addr).is_some_and(|(_, off)| off < AM34_FLASH_CRITICAL_IPL_SIZE)
}

/// Returns `true` when `addr` lies in cached/uncached IPL NOR flash (`0x80000000..0x807EFFFF` / `0x84..`).
pub fn is_ipl_readonly_va(addr: u32) -> bool {
    flash_aperture_and_offset(addr).is_some_and(|(_, off)| off < AM34_FLASH_IPL_SIZE)
}

fn is_nor_range_writable(policy: NorWritePolicy, start_off: u32, len: u32) -> bool {
    let end_off = start_off.saturating_add(len.saturating_sub(1));
    if start_off < AM34_FLASH_CRITICAL_IPL_SIZE {
        return false;
    }
    if start_off < AM34_FLASH_IPL_SIZE || end_off < AM34_FLASH_IPL_SIZE {
        return policy == NorWritePolicy::UpperNorAppUnlocked;
    }
    true
}

/// Dual-boot `S29JL064H` sector boundaries (`8x8 KiB` bottom `SA0..SA7`, `126x64 KiB` `SA8..SA133`, `8x8 KiB` top `SA134..SA141`).
pub(super) fn nor_sector_bounds(flash_off: u32) -> (u32, u32) {
    let main_sz = SPANSION_MAIN_SECTOR_SIZE_BYTES as u32;
    let micro_sz = SPANSION_MICROSECTOR_SIZE as u32;
    if flash_off < main_sz || flash_off >= AM34_FLASH_IPL_SIZE {
        (flash_off & !(micro_sz - 1), micro_sz)
    } else {
        (flash_off & !(main_sz - 1), main_sz)
    }
}

fn jedec_fail(vm: &mut Mn103Vm, addr: u32) -> Option<Result<(), Mn103Fault>> {
    vm.cfi_stage = CfiStage::Normal;
    Some(Err(Mn103Fault::IplWriteProtected(addr)))
}

impl Mn103Vm {
    fn commit_nor_busy_op(&mut self, op: NorBusyOp) -> Result<(), Mn103Fault> {
        let addr = op.va_base + op.target_off;
        if op.is_erase {
            let (slice, idx) = self.locate_slice_mut(addr, op.len as usize)?;
            slice[idx..idx + op.len as usize].fill(0xFF);
        } else {
            let bytes = op.prog_val.to_le_bytes();
            let (slice, idx) = self.locate_slice_mut(addr, op.prog_width)?;
            for (i, b) in bytes.iter().take(op.prog_width).enumerate() {
                slice[idx + i] &= *b;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn start_nor_busy_op(
        &mut self,
        va_base: u32,
        target_off: u32,
        len: u32,
        is_erase: bool,
        prog_val: u16,
        prog_width: usize,
        expected_dq7: u8,
    ) -> Option<Result<(), Mn103Fault>> {
        let op = NorBusyOp {
            va_base,
            target_off,
            len,
            is_erase,
            prog_val,
            prog_width,
            expected_dq7,
            remaining_polls: self.nor_busy_cycles_config,
            toggle_phase: false,
            force_dq5: self.nor_force_dq5_timeout,
        };
        if op.remaining_polls == 0 && !op.force_dq5 {
            return Some(self.commit_nor_busy_op(op));
        }
        self.nor_busy_state = Some(op);
        Some(Ok(()))
    }

    /// Models Spansion `S29JL064H` JEDEC autoselect (`0x90`), CFI Query (`0x98`), erase (`0x80->0x30`), and program (`0xA0`).
    pub(super) fn try_cfi_write(
        &mut self,
        addr: u32,
        raw_val: u16,
        width: usize,
    ) -> Option<Result<(), Mn103Fault>> {
        let (va_base, off) = flash_aperture_and_offset(addr)?;
        let cmd = (raw_val & 0xFF) as u8;
        let low = addr & 0x0FFF;
        let is_unlock1 = matches!(low, 0x554 | 0x555 | 0xAAA);
        let is_unlock2 = matches!(low, 0xAAA | 0xAAB | 0x554 | 0x555);
        let is_cfi_addr = matches!(low, 0xAA | 0x55 | 0x554 | 0x555 | 0xAAA);

        if self.cfi_stage == CfiStage::ProgramPending {
            self.cfi_stage = CfiStage::Normal;
            if !is_nor_range_writable(self.nor_write_policy, off, width as u32) {
                return Some(Err(Mn103Fault::IplWriteProtected(addr)));
            }
            return self.start_nor_busy_op(
                va_base,
                off,
                width as u32,
                false,
                raw_val,
                width,
                (raw_val & 0x80) as u8,
            );
        }

        if cmd == 0xF0
            && (self.cfi_stage != CfiStage::Normal
                || self.nor_busy_state.is_some()
                || off < AM34_FLASH_IPL_SIZE)
        {
            self.cfi_stage = CfiStage::Normal;
            self.nor_busy_state = None;
            return Some(Ok(()));
        }

        match self.cfi_stage {
            CfiStage::Normal => {
                if is_unlock1 && cmd == 0xAA {
                    self.cfi_stage = CfiStage::Unlock555;
                    Some(Ok(()))
                } else if is_cfi_addr && cmd == 0x98 {
                    self.cfi_stage = CfiStage::CfiQuery;
                    Some(Ok(()))
                } else if width == 2
                    && (addr & 0xFFFF) == 0
                    && off >= AM34_FLASH_CRITICAL_IPL_SIZE
                    && matches!(raw_val, 0x0090 | 0xFFFF)
                {
                    // Multi-vendor firmware probers emit Sharp/Intel 0x0090 / 0xFFFF at bank base
                    // before AMD/Spansion AA->55->90; Spansion S29JL064H ignores them in array read mode.
                    Some(Ok(()))
                } else if off >= AM34_FLASH_IPL_SIZE {
                    None
                } else {
                    Some(Err(Mn103Fault::IplWriteProtected(addr)))
                }
            }
            CfiStage::Autoselect | CfiStage::EraseSetup | CfiStage::CfiQuery => {
                if is_unlock1 && cmd == 0xAA {
                    self.cfi_stage = if self.cfi_stage == CfiStage::EraseSetup {
                        CfiStage::EraseUnlock555
                    } else {
                        CfiStage::Unlock555
                    };
                    Some(Ok(()))
                } else {
                    jedec_fail(self, addr)
                }
            }
            CfiStage::Unlock555 | CfiStage::EraseUnlock555 => {
                if is_unlock2 && cmd == 0x55 {
                    self.cfi_stage = if self.cfi_stage == CfiStage::Unlock555 {
                        CfiStage::Unlock2AA
                    } else {
                        CfiStage::EraseUnlock2AA
                    };
                    Some(Ok(()))
                } else {
                    jedec_fail(self, addr)
                }
            }
            CfiStage::Unlock2AA => match cmd {
                0x90 => {
                    self.cfi_stage = CfiStage::Autoselect;
                    Some(Ok(()))
                }
                0x98 => {
                    self.cfi_stage = CfiStage::CfiQuery;
                    Some(Ok(()))
                }
                0x80 => {
                    self.cfi_stage = CfiStage::EraseSetup;
                    Some(Ok(()))
                }
                0xA0 => {
                    self.cfi_stage = CfiStage::ProgramPending;
                    Some(Ok(()))
                }
                _ => jedec_fail(self, addr),
            },
            CfiStage::EraseUnlock2AA => {
                self.cfi_stage = CfiStage::Normal;
                if cmd != 0x30 {
                    return Some(Err(Mn103Fault::IplWriteProtected(addr)));
                }
                let (sec_start, sec_len) = nor_sector_bounds(off);
                if !is_nor_range_writable(self.nor_write_policy, sec_start, sec_len) {
                    return Some(Err(Mn103Fault::IplWriteProtected(addr)));
                }
                self.start_nor_busy_op(va_base, sec_start, sec_len, true, 0xFFFF, 2, 0x80)
            }
            CfiStage::ProgramPending => jedec_fail(self, addr),
        }
    }

    pub(super) fn read_cfi_autoselect_u16(&mut self, addr: u32) -> Result<Option<u16>, Mn103Fault> {
        if flash_aperture_and_offset(addr).is_none() {
            return Ok(None);
        }
        if let Some(mut op) = self.nor_busy_state {
            if op.remaining_polls > 0 {
                op.remaining_polls -= 1;
                op.toggle_phase = !op.toggle_phase;
                self.nor_busy_state = Some(op);
                let dq7 = op.expected_dq7 ^ 0x80;
                let dq6 = if op.toggle_phase { 0x40 } else { 0x00 };
                let dq5 = if op.force_dq5 { 0x20 } else { 0x00 };
                return Ok(Some(u16::from(dq7 | dq6 | dq5)));
            }
            if op.force_dq5 {
                op.toggle_phase = !op.toggle_phase;
                self.nor_busy_state = Some(op);
                let dq7 = op.expected_dq7 ^ 0x80;
                let dq6 = if op.toggle_phase { 0x40 } else { 0x00 };
                return Ok(Some(u16::from(dq7 | dq6 | 0x20)));
            }
            self.nor_busy_state = None;
            self.commit_nor_busy_op(op)?;
            return Ok(None);
        }

        match self.cfi_stage {
            CfiStage::Autoselect => Ok(Some(match addr & 0xFE {
                0x00 => 0x0001, // Spansion/AMD manufacturer ID
                0x02 => 0x227E, // S29JL064H/S29GL family device ID
                0x1C => 0x2202, // S29JL064H word extended ID 1
                0x1E => 0x2201, // S29JL064H dual-boot extended ID 2
                _ => 0xFFFF,
            })),
            CfiStage::CfiQuery => Ok(Some(match addr & 0xFE {
                0x20 => 0x0051, // 'Q'
                0x22 => 0x0052, // 'R'
                0x24 => 0x0059, // 'Y'
                0x26 => 0x0002, // AMD/Fujitsu standard command set
                0x4E => 0x0017, // 2^23 = 8 MiB
                0x58 => 0x0003, // 3 erase block regions (8x8K + 126x64K + 8x8K)
                0x5A => 0x0007, // Region 1 count - 1 = 7 (8 sectors)
                0x5C => 0x0000,
                0x5E => 0x0020, // Region 1 size / 256 = 0x20 (8 KiB)
                0x60 => 0x0000,
                0x62 => 0x007D, // Region 2 count - 1 = 125 (126 sectors)
                0x64 => 0x0000,
                0x66 => 0x0000, // Region 2 size / 256 = 0x100 (64 KiB)
                0x68 => 0x0001,
                0x6A => 0x0007, // Region 3 count - 1 = 7 (8 sectors)
                0x6C => 0x0000,
                0x6E => 0x0020, // Region 3 size / 256 = 0x20 (8 KiB)
                0x70 => 0x0000,
                _ => 0xFFFF,
            })),
            _ => Ok(None),
        }
    }
}
