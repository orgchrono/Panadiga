//! Segmented memory accessors and MOVM/CALL register frame helpers for [`Mn103Vm`].

use super::{MemSegment, Mn103Fault, Mn103Vm};

impl Mn103Vm {
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

    pub fn read_u8(&self, addr: u32) -> Result<u8, Mn103Fault> {
        let (seg, off) = self
            .locate_slice(addr, 1)
            .ok_or(Mn103Fault::UnmappedRead(addr))?;
        Ok(seg.data[off])
    }

    pub fn read_u16(&self, addr: u32) -> Result<u16, Mn103Fault> {
        if (addr & 1) != 0 {
            return Err(Mn103Fault::AlignError(addr));
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
        let (slice, off) = self.locate_slice_mut(addr, 1)?;
        slice[off] = val;
        Ok(())
    }

    pub fn write_u16(&mut self, addr: u32, val: u16) -> Result<(), Mn103Fault> {
        if (addr & 1) != 0 {
            return Err(Mn103Fault::AlignError(addr));
        }
        let (slice, off) = self.locate_slice_mut(addr, 2)?;
        slice[off..off + 2].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) -> Result<(), Mn103Fault> {
        if (addr & 3) != 0 {
            return Err(Mn103Fault::AlignError(addr));
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
