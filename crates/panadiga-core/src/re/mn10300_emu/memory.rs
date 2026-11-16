//! Segmented memory accessors and MOVM/CALL register frame helpers for [`Mn103Vm`].

use super::{CfiStage, MemSegment, Mn103Fault, Mn103Vm};
use crate::constants::{
    AM34_FLASH_BASE, AM34_FLASH_IPL_SIZE, AM34_FLASH_UNCACHED_BASE, AM34_SC0_RXB_VA,
    AM34_SC0_STR_VA, AM34_SC0_TXB_VA,
};

const SC0_TXB_VA: u32 = AM34_SC0_TXB_VA;
const SC0_RXB_VA: u32 = AM34_SC0_RXB_VA;
const SC0_STR_VA: u32 = AM34_SC0_STR_VA;

/// Returns `true` when `addr` lies in strictly read-only cached/uncached IPL NOR flash.
pub fn is_ipl_readonly_va(addr: u32) -> bool {
    let ipl_last = AM34_FLASH_IPL_SIZE - 1;
    (AM34_FLASH_BASE..=AM34_FLASH_BASE + ipl_last).contains(&addr)
        || (AM34_FLASH_UNCACHED_BASE..=AM34_FLASH_UNCACHED_BASE + ipl_last).contains(&addr)
}

impl Mn103Vm {
    fn try_cfi_write(&mut self, addr: u32, raw_val: u16) -> Option<Result<(), Mn103Fault>> {
        if !is_ipl_readonly_va(addr) {
            return None;
        }
        let cmd = (raw_val & 0xFF) as u8;
        let low = addr & 0x0FFF;
        if cmd == 0xF0 {
            self.cfi_stage = CfiStage::Normal;
            return Some(Ok(()));
        }
        if matches!(low, 0x554 | 0x555 | 0xAAA) && cmd == 0xAA && self.cfi_stage == CfiStage::Normal
        {
            self.cfi_stage = CfiStage::Unlock555;
            return Some(Ok(()));
        }
        if matches!(low, 0xAAA | 0xAAB | 0x554 | 0x555)
            && cmd == 0x55
            && self.cfi_stage == CfiStage::Unlock555
        {
            self.cfi_stage = CfiStage::Unlock2AA;
            return Some(Ok(()));
        }
        if cmd == 0x90 && self.cfi_stage == CfiStage::Unlock2AA {
            self.cfi_stage = CfiStage::Autoselect;
            return Some(Ok(()));
        }
        Some(Err(Mn103Fault::IplWriteProtected(addr)))
    }

    fn read_cfi_autoselect_u16(&self, addr: u32) -> Option<u16> {
        if self.cfi_stage != CfiStage::Autoselect || !is_ipl_readonly_va(addr) {
            return None;
        }
        match addr & 0xFE {
            0x00 => Some(0x0001), // Spansion/AMD manufacturer ID
            0x02 => Some(0x227E), // S29JL064H/S29GL family device ID
            _ => Some(0xFFFF),
        }
    }
    fn locate_slice(&self, addr: u32, len: usize) -> Option<(&MemSegment, usize)> {
        let addr_u64 = addr as u64;
        let end_u64 = addr_u64 + len as u64;
        for seg in &self.segments {
            let s = seg.base as u64;
            let e = s + seg.data.len() as u64;
            if addr_u64 >= s && end_u64 <= e {
                return Some((seg, (addr_u64 - s) as usize));
            }
        }
        None
    }

    fn locate_slice_mut(
        &mut self,
        addr: u32,
        len: usize,
    ) -> Result<(&mut [u8], usize), Mn103Fault> {
        let addr_u64 = addr as u64;
        let end_u64 = addr_u64 + len as u64;
        for seg in &mut self.segments {
            let s = seg.base as u64;
            let e = s + seg.data.len() as u64;
            if addr_u64 >= s && end_u64 <= e {
                if !seg.writable {
                    return Err(Mn103Fault::UnmappedWrite(addr));
                }
                return Ok((&mut seg.data, (addr_u64 - s) as usize));
            }
        }
        Err(Mn103Fault::UnmappedWrite(addr))
    }

    pub fn read_u8(&mut self, addr: u32) -> Result<u8, Mn103Fault> {
        if self.sc0_enabled {
            if addr == SC0_RXB_VA {
                return Ok(self.uart_rx.pop_front().unwrap_or(0));
            }
            if addr == SC0_STR_VA {
                return Ok(if self.uart_rx.is_empty() { 0x00 } else { 0x10 });
            }
        }
        if let Some(id16) = self.read_cfi_autoselect_u16(addr) {
            return Ok(if (addr & 1) == 0 {
                id16 as u8
            } else {
                (id16 >> 8) as u8
            });
        }
        let (seg, off) = self
            .locate_slice(addr, 1)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(seg.data[off])
    }

    pub fn read_u16(&self, addr: u32) -> Result<u16, Mn103Fault> {
        if (addr & 1) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        if self.sc0_enabled && addr == SC0_STR_VA {
            return Ok(if self.uart_rx.is_empty() {
                0x0000
            } else {
                0x0010
            });
        }
        if let Some(id16) = self.read_cfi_autoselect_u16(addr) {
            return Ok(id16);
        }
        let (seg, off) = self
            .locate_slice(addr, 2)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(u16::from_le_bytes([seg.data[off], seg.data[off + 1]]))
    }

    pub fn read_u32(&self, addr: u32) -> Result<u32, Mn103Fault> {
        if (addr & 3) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        self.fetch_u32(addr)
    }

    /// Reads 4 little-endian instruction/literal bytes without data-alignment enforcement.
    pub(super) fn fetch_u32(&self, addr: u32) -> Result<u32, Mn103Fault> {
        let (seg, off) = self
            .locate_slice(addr, 4)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(u32::from_le_bytes([
            seg.data[off],
            seg.data[off + 1],
            seg.data[off + 2],
            seg.data[off + 3],
        ]))
    }

    pub(super) fn fetch_i16(&self, addr: u32) -> Result<i16, Mn103Fault> {
        let (seg, off) = self
            .locate_slice(addr, 2)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(i16::from_le_bytes([seg.data[off], seg.data[off + 1]]))
    }

    pub fn write_u8(&mut self, addr: u32, val: u8) -> Result<(), Mn103Fault> {
        if self.sc0_enabled && addr == SC0_TXB_VA {
            self.uart_tx.push(val);
            return Ok(());
        }
        if let Some(res) = self.try_cfi_write(addr, val as u16) {
            return res;
        }
        let (slice, off) = self.locate_slice_mut(addr, 1)?;
        slice[off] = val;
        Ok(())
    }

    pub fn write_u16(&mut self, addr: u32, val: u16) -> Result<(), Mn103Fault> {
        if (addr & 1) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        if let Some(res) = self.try_cfi_write(addr, val) {
            return res;
        }
        let (slice, off) = self.locate_slice_mut(addr, 2)?;
        slice[off..off + 2].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) -> Result<(), Mn103Fault> {
        if (addr & 3) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        if is_ipl_readonly_va(addr) || is_ipl_readonly_va(addr.wrapping_add(3)) {
            return Err(Mn103Fault::IplWriteProtected(addr));
        }
        let (slice, off) = self.locate_slice_mut(addr, 4)?;
        slice[off..off + 4].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    pub(super) fn validate_movm_mask(regs_mask: u8, op: u8) -> Result<(), Mn103Fault> {
        if (regs_mask & 0x07) != 0 {
            return Err(Mn103Fault::InvalidOpcode(op));
        }
        Ok(())
    }

    pub(super) fn reg_mask_stack_bytes(regs_mask: u8) -> u32 {
        let mut bytes = 0u32;
        for bit in [0x80u8, 0x40, 0x20, 0x10] {
            if (regs_mask & bit) != 0 {
                bytes += 4;
            }
        }
        if (regs_mask & 0x08) != 0 {
            bytes += 32;
        }
        bytes
    }

    pub(super) fn save_regs(&mut self, base_sp: u32, regs_mask: u8) -> Result<(), Mn103Fault> {
        let mut cursor = base_sp;
        for (bit, reg_idx) in [(0x80u8, 2usize), (0x40, 3)] {
            if (regs_mask & bit) != 0 {
                cursor = cursor.wrapping_sub(4);
                self.write_u32(cursor, self.d[reg_idx])?;
            }
        }
        for (bit, reg_idx) in [(0x20u8, 2usize), (0x10, 3)] {
            if (regs_mask & bit) != 0 {
                cursor = cursor.wrapping_sub(4);
                self.write_u32(cursor, self.a[reg_idx])?;
            }
        }
        if (regs_mask & 0x08) != 0 {
            for val in [
                self.d[0], self.d[1], self.a[0], self.a[1], self.mdr, self.lir, self.lar, 0,
            ] {
                cursor = cursor.wrapping_sub(4);
                self.write_u32(cursor, val)?;
            }
        }
        Ok(())
    }

    pub(super) fn restore_regs(&mut self, new_sp: u32, regs_mask: u8) -> Result<(), Mn103Fault> {
        let mut cursor = new_sp;
        let mut d2 = self.d[2];
        let mut d3 = self.d[3];
        let mut a2 = self.a[2];
        let mut a3 = self.a[3];
        if (regs_mask & 0x80) != 0 {
            cursor = cursor.wrapping_sub(4);
            d2 = self.read_u32(cursor)?;
        }
        if (regs_mask & 0x40) != 0 {
            cursor = cursor.wrapping_sub(4);
            d3 = self.read_u32(cursor)?;
        }
        if (regs_mask & 0x20) != 0 {
            cursor = cursor.wrapping_sub(4);
            a2 = self.read_u32(cursor)?;
        }
        if (regs_mask & 0x10) != 0 {
            cursor = cursor.wrapping_sub(4);
            a3 = self.read_u32(cursor)?;
        }
        if (regs_mask & 0x08) != 0 {
            let mut vals = [0u32; 8];
            for slot in &mut vals {
                cursor = cursor.wrapping_sub(4);
                *slot = self.read_u32(cursor)?;
            }
            self.d[0] = vals[0];
            self.d[1] = vals[1];
            self.a[0] = vals[2];
            self.a[1] = vals[3];
            self.mdr = vals[4];
            self.lir = vals[5];
            self.lar = vals[6];
        }
        self.d[2] = d2;
        self.d[3] = d3;
        self.a[2] = a2;
        self.a[3] = a3;
        Ok(())
    }
}
