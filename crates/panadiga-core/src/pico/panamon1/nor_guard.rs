#![deny(unsafe_code)]

//! RAII Upper NOR unlock guard (`U CONFIRM_UPPER_NOR` -> guaranteed `Drop` `U LOCK`).

use std::time::Duration;

use crate::constants::{
    AM34_FLASH_BASE, AM34_FLASH_IPL_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE,
};
use crate::pico::rsp_codec::policy::{NorWritePolicy, is_nor_write_blocked_with_policy};
use anyhow::{Result, bail};

use super::client::Panamon1Client;
use super::protocol::Panamon1Transport;

pub(super) fn flash_offset_from_va(addr: u32) -> Option<u32> {
    for base in [AM34_FLASH_BASE, AM34_FLASH_UNCACHED_BASE] {
        if let Some(off) = addr.checked_sub(base)
            && off < AM34_FLASH_SIZE
        {
            return Some(off);
        }
    }
    None
}

impl<T: Panamon1Transport> Panamon1Client<T> {
    /// Validates all `planned_targets` against `SA0..SA3` (`E05`) before sending `U CONFIRM_UPPER_NOR`.
    /// Returned guard unconditionally sends `U LOCK` on [`Drop`].
    pub fn unlock_upper_nor<'a>(
        &'a mut self,
        planned_targets: &[(u32, usize)],
    ) -> Result<Panamon1NorUnlockGuard<'a, T>> {
        for &(addr, len) in planned_targets {
            if is_nor_write_blocked_with_policy(
                addr,
                len.max(1),
                NorWritePolicy::UpperNorAppUnlocked,
            ) {
                bail!(
                    "Hardware Safety Lockout (E05): target 0x{addr:08X} (+{len} B) touches permanently read-only SA0..SA3 bootloader"
                );
            }
        }
        let _ = self.exec_target_cmd(
            "U CONFIRM_UPPER_NOR",
            &["OK:NOR_UNLOCK"],
            Duration::from_millis(3000),
        )?;
        Ok(Panamon1NorUnlockGuard::new(self))
    }
}

/// RAII Upper NOR unlock guard (`U CONFIRM_UPPER_NOR` -> guaranteed `U LOCK` on [`Drop`]).
pub struct Panamon1NorUnlockGuard<'a, T: Panamon1Transport> {
    client: &'a mut Panamon1Client<T>,
    locked: bool,
}

impl<'a, T: Panamon1Transport> Panamon1NorUnlockGuard<'a, T> {
    pub(super) fn new(client: &'a mut Panamon1Client<T>) -> Self {
        Self {
            client,
            locked: false,
        }
    }

    fn validate_upper_nor_target(va: u32, len: usize) -> Result<u32> {
        let off = flash_offset_from_va(va).ok_or_else(|| {
            anyhow::anyhow!("Address 0x{va:08X} is outside Spansion NOR flash aperture")
        })?;
        if is_nor_write_blocked_with_policy(va, len.max(1), NorWritePolicy::UpperNorAppUnlocked) {
            bail!(
                "Hardware Safety Lockout (E05): refusing operation on permanently protected SA0..SA3 at 0x{va:08X} (+{len} B)"
            );
        }
        Ok(off)
    }

    pub fn client_mut(&mut self) -> &mut Panamon1Client<T> {
        self.client
    }

    /// Issues `E <sector_va>` (outside `SA0..SA3`) and polls until sector reads `0xFF`.
    pub fn erase_sector(&mut self, sector_va: u32) -> Result<()> {
        let off = Self::validate_upper_nor_target(sector_va, 1)?;
        let is_micro = !(0x0001_0000..AM34_FLASH_IPL_SIZE).contains(&off);
        let cmd = format!("E {sector_va:08X}");
        let _ = self
            .client
            .exec_target_cmd(&cmd, &["OK:ERASE"], Duration::from_millis(10_000))?;

        let poll_len = 32usize;
        for _ in 0..32 {
            let head = self.client.mem_read_bytes(sector_va, poll_len)?;
            let tail_ok = if is_micro {
                true
            } else {
                self.client
                    .mem_read_bytes(sector_va.wrapping_add(0x100), poll_len)?
                    .iter()
                    .all(|&b| b == 0xFF)
            };
            if head.iter().all(|&b| b == 0xFF) && tail_ok {
                return Ok(());
            }
            self.client.sleep(Duration::from_millis(250));
        }
        bail!("Timeout polling Spansion erase completion at 0x{sector_va:08X}")
    }

    /// Writes `data` to unlocked Upper NOR / NVRAM (`SA4..SA141`) with permanent `SA0..SA3` `E05` block.
    pub fn write_nor_bytes(&mut self, va: u32, data: &[u8]) -> Result<()> {
        Self::validate_upper_nor_target(va, data.len().max(1))?;
        self.client
            .mem_write_bytes_burst_with_policy(va, data, NorWritePolicy::UpperNorAppUnlocked)
    }

    /// Explicitly issues `U LOCK` and consumes the guard.
    pub fn lock(mut self) -> Result<()> {
        self.locked = true;
        let _ =
            self.client
                .exec_target_cmd("U LOCK", &["OK:NOR_LOCK"], Duration::from_millis(3000))?;
        Ok(())
    }
}

impl<T: Panamon1Transport> Drop for Panamon1NorUnlockGuard<'_, T> {
    fn drop(&mut self) {
        if !self.locked {
            self.locked = true;
            let _ = self.client.exec_target_cmd(
                "U LOCK",
                &["OK:NOR_LOCK"],
                Duration::from_millis(2000),
            );
        }
    }
}
