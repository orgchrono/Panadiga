//! Spansion `S29JL064H` NOR flash JEDEC state machine and aperture policy helpers for [`Mn103Vm`].

use super::{CfiStage, Mn103Fault, Mn103Vm};
use crate::constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_IPL_SIZE, AM34_FLASH_SIZE,
    AM34_FLASH_UNCACHED_BASE, SPANSION_MAIN_SECTOR_SIZE_BYTES,
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

/// Returns `true` when `addr` lies in the permanent read-only boot IPL (`SA0..SA3`, first 256 KiB).
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

fn nor_sector_bounds(flash_off: u32) -> (u32, u32) {
    if flash_off < AM34_FLASH_IPL_SIZE {
        let sector_bytes = SPANSION_MAIN_SECTOR_SIZE_BYTES as u32;
        (flash_off & !(sector_bytes - 1), sector_bytes)
    } else {
        (flash_off & !0x1FFF, 0x2000)
    }
}

fn jedec_fail(vm: &mut Mn103Vm, addr: u32) -> Option<Result<(), Mn103Fault>> {
    vm.cfi_stage = CfiStage::Normal;
    Some(Err(Mn103Fault::IplWriteProtected(addr)))
}

impl Mn103Vm {
    /// Models Spansion `S29JL064H` JEDEC autoselect (`0x90`), erase (`0x80->0x30`), and program (`0xA0`).
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

        if self.cfi_stage == CfiStage::ProgramPending {
            self.cfi_stage = CfiStage::Normal;
            if !is_nor_range_writable(self.nor_write_policy, off, width as u32) {
                return Some(Err(Mn103Fault::IplWriteProtected(addr)));
            }
            let bytes = raw_val.to_le_bytes();
            return Some(self.locate_slice_mut(addr, width).map(|(slice, idx)| {
                for (i, b) in bytes.iter().take(width).enumerate() {
                    slice[idx + i] &= *b;
                }
            }));
        }

        if cmd == 0xF0 && (self.cfi_stage != CfiStage::Normal || off < AM34_FLASH_IPL_SIZE) {
            self.cfi_stage = CfiStage::Normal;
            return Some(Ok(()));
        }

        match self.cfi_stage {
            CfiStage::Normal => {
                if is_unlock1 && cmd == 0xAA {
                    self.cfi_stage = CfiStage::Unlock555;
                    Some(Ok(()))
                } else if off >= AM34_FLASH_IPL_SIZE {
                    None
                } else {
                    Some(Err(Mn103Fault::IplWriteProtected(addr)))
                }
            }
            CfiStage::Autoselect | CfiStage::EraseSetup => {
                if is_unlock1 && cmd == 0xAA {
                    self.cfi_stage = if self.cfi_stage == CfiStage::Autoselect {
                        CfiStage::Unlock555
                    } else {
                        CfiStage::EraseUnlock555
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
                Some(
                    self.locate_slice_mut(va_base + sec_start, sec_len as usize)
                        .map(|(slice, idx)| {
                            slice[idx..idx + sec_len as usize].fill(0xFF);
                        }),
                )
            }
            CfiStage::ProgramPending => jedec_fail(self, addr),
        }
    }

    pub(super) fn read_cfi_autoselect_u16(&self, addr: u32) -> Option<u16> {
        if self.cfi_stage != CfiStage::Autoselect || flash_aperture_and_offset(addr).is_none() {
            return None;
        }
        match addr & 0xFE {
            0x00 => Some(0x0001), // Spansion/AMD manufacturer ID
            0x02 => Some(0x227E), // S29JL064H/S29GL family device ID
            _ => Some(0xFFFF),
        }
    }
}
