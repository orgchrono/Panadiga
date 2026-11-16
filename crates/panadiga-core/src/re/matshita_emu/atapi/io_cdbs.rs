//! Standard block/buffer ATAPI CDB handlers (`0x28 READ(10)`, `0x2A WRITE(10)`, `0x3B`, `0x3C`).

use anyhow::{Result, bail};

use super::super::inject::InjectionLogEntry;
use super::super::{
    ATAPI_SECTOR_SIZE_BYTES, AtapiTransferDirection, DRV_ATAPI_READ_BUFFER_3C_VA,
    DRV_ATAPI_WRITE_BUFFER_3B_VA, MAX_VIRTUAL_DISC_SECTORS, MatshitaOddEmulator,
    is_protected_ipl_address,
};
use super::CdbOutcome;

impl MatshitaOddEmulator {
    pub(super) fn handle_cdb_read10(&self, cdb: &[u8; 12]) -> Result<CdbOutcome> {
        let lba = u32::from_be_bytes([cdb[2], cdb[3], cdb[4], cdb[5]]) as usize;
        let blocks = u16::from_be_bytes([cdb[7], cdb[8]]) as usize;
        let off = lba.saturating_mul(ATAPI_SECTOR_SIZE_BYTES);
        let byte_len = blocks.saturating_mul(ATAPI_SECTOR_SIZE_BYTES);
        if off.saturating_add(byte_len) > self.disc_image.len() {
            bail!(
                "ATAPI READ(10) LBA {lba}+{blocks} exceeds virtual disc ({} sectors)",
                self.disc_image.len() / ATAPI_SECTOR_SIZE_BYTES
            );
        }
        Ok((
            "READ(10)".to_string(),
            0x4001_6000,
            AtapiTransferDirection::DataIn,
            self.disc_image[off..off + byte_len].to_vec(),
            0x00,
            0x00,
            0x00,
            format!("Read {blocks} sector(s) starting at LBA {lba}"),
        ))
    }

    pub(super) fn handle_cdb_write10(
        &mut self,
        cdb: &[u8; 12],
        data_out: Option<&[u8]>,
    ) -> Result<CdbOutcome> {
        let lba = u32::from_be_bytes([cdb[2], cdb[3], cdb[4], cdb[5]]) as usize;
        let blocks = u16::from_be_bytes([cdb[7], cdb[8]]) as usize;
        let payload = data_out.unwrap_or(&[]);
        let expected = blocks * ATAPI_SECTOR_SIZE_BYTES;
        if lba + blocks > MAX_VIRTUAL_DISC_SECTORS || payload.len() < expected {
            bail!(
                "ATAPI WRITE(10) invalid sector range LBA={lba} blocks={blocks} payload={}B",
                payload.len()
            );
        }
        let need = (lba + blocks) * ATAPI_SECTOR_SIZE_BYTES;
        if self.disc_image.len() < need {
            self.disc_image.resize(need, 0);
        }
        let off = lba * ATAPI_SECTOR_SIZE_BYTES;
        self.disc_image[off..off + expected].copy_from_slice(&payload[..expected]);
        self.record_injection(InjectionLogEntry {
            source: "ATAPI_WRITE10",
            cdb0_or_cmd: "0x2A",
            mode: 0,
            target: lba as u32,
            payload: &payload[..expected],
            status: "INJECTED_TO_VIRTUAL_DISC",
            note: format!("Wrote {blocks} virtual CD/DVD sectors at LBA {lba}"),
        });
        Ok((
            "WRITE(10)".to_string(),
            0x4001_6200,
            AtapiTransferDirection::DataOut,
            Vec::new(),
            0x00,
            0x00,
            0x00,
            format!("Injected {expected} B to virtual disc LBA {lba}"),
        ))
    }

    pub(super) fn handle_cdb_write_buffer(
        &mut self,
        cdb: &[u8; 12],
        data_out: Option<&[u8]>,
    ) -> Result<CdbOutcome> {
        let mode = cdb[1] & 0x1F;
        let offset = ((cdb[3] as usize) << 16) | ((cdb[4] as usize) << 8) | (cdb[5] as usize);
        let req_len = ((cdb[6] as usize) << 16) | ((cdb[7] as usize) << 8) | (cdb[8] as usize);
        let payload = data_out.unwrap_or(&[]);
        if payload.len() < req_len || offset.saturating_add(req_len) > self.buffer.len() {
            bail!(
                "ATAPI WRITE BUFFER out of range (offset=0x{offset:X}, len={req_len}, payload={}B)",
                payload.len()
            );
        }
        let target_va = self.staged_write_addr.wrapping_add(offset as u32);
        if is_protected_ipl_address(target_va, req_len) {
            bail!(
                "Hardware Safety Lockout (E05): ATAPI WRITE BUFFER cannot target IPL NOR at 0x{target_va:08X}"
            );
        }
        self.buffer[offset..offset + req_len].copy_from_slice(&payload[..req_len]);
        self.record_injection(InjectionLogEntry {
            source: "ATAPI_WRITE_BUFFER",
            cdb0_or_cmd: "0x3B",
            mode,
            target: target_va,
            payload: &payload[..req_len],
            status: "COMMITTED_BUFFER",
            note: format!(
                "0x3B WRITE BUFFER mode=0x{mode:02X} off=0x{offset:X} target=0x{target_va:08X}"
            ),
        });
        Ok((
            "WRITE BUFFER".to_string(),
            DRV_ATAPI_WRITE_BUFFER_3B_VA,
            AtapiTransferDirection::DataOut,
            Vec::new(),
            0x00,
            0x00,
            0x00,
            format!("Injected {req_len} B via 0x3B (FUN_4002168d) at +0x{offset:X}"),
        ))
    }

    pub(super) fn handle_cdb_read_buffer(&self, cdb: &[u8; 12]) -> Result<CdbOutcome> {
        let offset = ((cdb[3] as usize) << 16) | ((cdb[4] as usize) << 8) | (cdb[5] as usize);
        let req_len = ((cdb[6] as usize) << 16) | ((cdb[7] as usize) << 8) | (cdb[8] as usize);
        if offset.saturating_add(req_len) > self.buffer.len() {
            bail!("ATAPI READ BUFFER range exceeds 64 KiB drive buffer");
        }
        Ok((
            "READ BUFFER".to_string(),
            DRV_ATAPI_READ_BUFFER_3C_VA,
            AtapiTransferDirection::DataIn,
            self.buffer[offset..offset + req_len].to_vec(),
            0x00,
            0x00,
            0x00,
            format!("Read {req_len} B via 0x3C (FUN_40021476) from +0x{offset:X}"),
        ))
    }
}
