//! ATAPI CDB packet dispatch (`R6-MV4` `DRV`) for [`MatshitaOddEmulator`].

use super::{
    ATAPI_SECTOR_SIZE_BYTES, AtapiCommandRecord, AtapiTransferDirection,
    DRV_ATAPI_READ_BUFFER_3C_VA, DRV_ATAPI_VENDOR_ED_VA, DRV_ATAPI_VENDOR_FD_VA,
    DRV_ATAPI_VENDOR_FE_VA, DRV_ATAPI_WRITE_BUFFER_3B_VA, DRV_PASS_HEX_TOKEN_0,
    DRV_PASS_HEX_TOKEN_1, MAX_VIRTUAL_DISC_SECTORS, MatshitaBootScenario, MatshitaOddEmulator,
    format_hex_bytes, is_protected_ipl_address,
};
use anyhow::{Result, bail};

type CdbOutcome = (
    String,
    u32,
    AtapiTransferDirection,
    Vec<u8>,
    u8,
    u8,
    u8,
    String,
);

impl MatshitaOddEmulator {
    /// Executes an ATAPI CDB packet against the emulated `R6-MV4` `DRV` dispatcher and returns response bytes + trace.
    pub fn issue_atapi_cdb(
        &mut self,
        cdb: &[u8; 12],
        data_out: Option<&[u8]>,
    ) -> Result<(AtapiCommandRecord, Vec<u8>)> {
        let opcode = cdb[0];
        let cdb_hex = format_hex_bytes(cdb);
        let seq = self.history.len() + 1;

        let (name, handler_va, dir, data_in, sk, asc, ascq, note) = match opcode {
            0x08 => (
                "ATAPI SOFT RESET".to_string(),
                0x4400_9B11,
                AtapiTransferDirection::None,
                Vec::new(),
                0x06,
                0x29,
                0x00,
                "Device Reset Occurred (0x62900)".to_string(),
            ),
            0x12 => self.handle_cdb_inquiry(),
            0x00 => self.handle_cdb_test_unit_ready(),
            0x1B => self.handle_cdb_start_stop_unit(),
            0x1E => (
                "PREVENT ALLOW MEDIUM REMOVAL".to_string(),
                0x4001_5000,
                AtapiTransferDirection::None,
                Vec::new(),
                0x00,
                0x00,
                0x00,
                "Media lock updated".to_string(),
            ),
            0x28 => self.handle_cdb_read10(cdb)?,
            0x2A => self.handle_cdb_write10(cdb, data_out)?,
            0x3B => self.handle_cdb_write_buffer(cdb, data_out)?,
            0x3C => self.handle_cdb_read_buffer(cdb)?,
            0xED => self.handle_cdb_vendor_ed(cdb, data_out)?,
            0xFD => (
                "MATSUSHITA VENDOR 0xFD".to_string(),
                DRV_ATAPI_VENDOR_FD_VA,
                AtapiTransferDirection::None,
                Vec::new(),
                0x00,
                0x00,
                0x00,
                "Handled via FUN_40020147".to_string(),
            ),
            0xFE => self.handle_cdb_vendor_fe(cdb)?,
            _ => (
                format!("ATAPI_CMD_0x{opcode:02X}"),
                0x4001_4559,
                AtapiTransferDirection::None,
                Vec::new(),
                0x05,
                0x20,
                0x00,
                "Invalid/unimplemented CDB opcode".to_string(),
            ),
        };

        let pana_err = 0x8000_0000_u32
            | (u32::from(sk & 0x0F) << 16)
            | (u32::from(asc) << 8)
            | u32::from(ascq);
        let rec = AtapiCommandRecord {
            seq,
            cdb_hex,
            opcode,
            opcode_name: name,
            drv_handler_va: format!("0x{handler_va:08X}"),
            direction: dir,
            payload_len: data_in.len().max(data_out.map(|d| d.len()).unwrap_or(0)),
            sense_key: sk,
            asc,
            ascq,
            pana_error_hex: format!("0x{pana_err:08X}"),
            note,
        };
        self.log_event(format!(
            "CDB[0x{:02X}] {} sense={:02X}/{:02X}/{:02X}",
            opcode, rec.opcode_name, sk, asc, ascq
        ));
        self.history.push(rec.clone());
        Ok((rec, data_in))
    }

    fn handle_cdb_inquiry(&self) -> CdbOutcome {
        let mut inq = vec![0u8; 36];
        inq[0] = 0x05; // CD/DVD-ROM MMC
        inq[1] = 0x80; // Removable media
        inq[2] = 0x02;
        inq[3] = 0x32;
        inq[4] = 31;
        inq[8..16].copy_from_slice(b"MATSHITA");
        inq[16..32].copy_from_slice(b"R6-MV4          ");
        inq[32..36].copy_from_slice(b"S211");
        (
            "INQUIRY".to_string(),
            0x4001_4B00,
            AtapiTransferDirection::DataIn,
            inq,
            0x00,
            0x00,
            0x00,
            "Identified MATSHITAR6-MV4 S211".to_string(),
        )
    }

    fn handle_cdb_test_unit_ready(&self) -> CdbOutcome {
        if !self.is_tray_closed() {
            (
                "TEST UNIT READY".to_string(),
                0x4001_4C00,
                AtapiTransferDirection::None,
                Vec::new(),
                0x04,
                0x53,
                0x00,
                "can not close tray (80045300)".to_string(),
            )
        } else if !self.is_media_present() {
            (
                "TEST UNIT READY".to_string(),
                0x4001_4C00,
                AtapiTransferDirection::None,
                Vec::new(),
                0x02,
                0x3A,
                0x00,
                "Medium Not Present (80023a00)".to_string(),
            )
        } else {
            (
                "TEST UNIT READY".to_string(),
                0x4001_4C00,
                AtapiTransferDirection::None,
                Vec::new(),
                0x00,
                0x00,
                0x00,
                "Drive ready".to_string(),
            )
        }
    }

    fn handle_cdb_start_stop_unit(&self) -> CdbOutcome {
        match self.scenario {
            MatshitaBootScenario::TrayCannotClose80045300 => (
                "START STOP UNIT".to_string(),
                0x4400_9CEA,
                AtapiTransferDirection::None,
                Vec::new(),
                0x04,
                0x53,
                0x00,
                "startup_drive failed: 80045300 (can not close tray / 4-pin ribbon)".to_string(),
            ),
            MatshitaBootScenario::LaserApcFailure80040904 => (
                "START STOP UNIT".to_string(),
                0x4400_9CEA,
                AtapiTransferDirection::None,
                Vec::new(),
                0x04,
                0x09,
                0x04,
                "startup_drive failed: 80040904 / Mecha 0x83A7 (<0.5mW APC fault)".to_string(),
            ),
            _ => (
                "START STOP UNIT".to_string(),
                0x4400_9CEA,
                AtapiTransferDirection::None,
                Vec::new(),
                0x00,
                0x00,
                0x00,
                "Spindle spun up & laser APC calibrated OK".to_string(),
            ),
        }
    }

    fn handle_cdb_read10(&self, cdb: &[u8; 12]) -> Result<CdbOutcome> {
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

    fn handle_cdb_write10(
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
        self.record_injection(
            "ATAPI_WRITE10",
            "0x2A",
            0,
            lba as u32,
            &payload[..expected],
            "INJECTED_TO_VIRTUAL_DISC",
            format!("Wrote {blocks} virtual CD/DVD sectors at LBA {lba}"),
        );
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

    fn handle_cdb_write_buffer(
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
        self.record_injection(
            "ATAPI_WRITE_BUFFER",
            "0x3B",
            mode,
            target_va,
            &payload[..req_len],
            "COMMITTED_BUFFER",
            format!(
                "0x3B WRITE BUFFER mode=0x{mode:02X} off=0x{offset:X} target=0x{target_va:08X}"
            ),
        );
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

    fn handle_cdb_read_buffer(&self, cdb: &[u8; 12]) -> Result<CdbOutcome> {
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

    fn handle_cdb_vendor_ed(
        &mut self,
        cdb: &[u8; 12],
        data_out: Option<&[u8]>,
    ) -> Result<CdbOutcome> {
        // Matsushita FUN_400209f3 vendor buffer dispatcher (DAT_00000242 subcommands 0x91/0x12/0x93/0x13)
        let sub = cdb[1];
        self.sram_0242_ed_mode = sub;
        match sub {
            0x91 => {
                let addr = u32::from_be_bytes([cdb[2], cdb[3], cdb[4], cdb[5]]);
                if addr != 0 {
                    if is_protected_ipl_address(addr, 4) {
                        bail!("Hardware Safety Lockout (E05): 0xED target 0x{addr:08X} in IPL NOR");
                    }
                    self.staged_write_addr = addr;
                }
                Ok((
                    "MATSUSHITA VENDOR 0xED (SUB 0x91 PREP)".to_string(),
                    DRV_ATAPI_VENDOR_ED_VA,
                    AtapiTransferDirection::None,
                    Vec::new(),
                    0x00,
                    0x00,
                    0x00,
                    format!(
                        "DAT_00000242=0x91 armed target=0x{:08X}",
                        self.staged_write_addr
                    ),
                ))
            }
            0x12 => {
                let n = (cdb[8] as usize).clamp(16, 256);
                Ok((
                    "MATSUSHITA VENDOR 0xED (SUB 0x12 READ)".to_string(),
                    DRV_ATAPI_VENDOR_ED_VA,
                    AtapiTransferDirection::DataIn,
                    self.buffer[..n].to_vec(),
                    0x00,
                    0x00,
                    0x00,
                    format!("DAT_00000242=0x12 returned {n} B from buffer"),
                ))
            }
            0x93 | 0x13 => {
                let payload = data_out.unwrap_or(&[]);
                let copy_len = payload.len().min(self.buffer.len());
                if is_protected_ipl_address(self.staged_write_addr, copy_len) {
                    bail!("Hardware Safety Lockout (E05): 0xED commit blocked inside IPL");
                }
                if copy_len > 0 {
                    self.buffer[..copy_len].copy_from_slice(&payload[..copy_len]);
                    self.record_injection(
                        "ATAPI_VENDOR_ED",
                        "0xED",
                        sub,
                        self.staged_write_addr,
                        &payload[..copy_len],
                        if (self.sram_0cd8_auth & 1) != 0 {
                            "COMMITTED_UNLOCKED"
                        } else {
                            "STAGED_UNAUTH_CHECKED"
                        },
                        format!(
                            "FUN_400209f3 sub=0x{sub:02X} staged {copy_len} B @ 0x{:08X}",
                            self.staged_write_addr
                        ),
                    );
                }
                Ok((
                    format!("MATSUSHITA VENDOR 0xED (SUB 0x{sub:02X})"),
                    DRV_ATAPI_VENDOR_ED_VA,
                    AtapiTransferDirection::DataOut,
                    Vec::new(),
                    0x00,
                    0x00,
                    0x00,
                    format!("FUN_400209f3 sub=0x{sub:02X} processed {copy_len} B"),
                ))
            }
            other => {
                bail!("Unsupported 0xED subcommand 0x{other:02X} (expected 0x91/0x12/0x93/0x13)")
            }
        }
    }

    fn handle_cdb_vendor_fe(&mut self, cdb: &[u8; 12]) -> Result<CdbOutcome> {
        // Subcommand 0x01=MMODE, 0x02=SLOGON/SLOGOFF, 0x03=PASS 0x4d415453 0x48495441
        match cdb[1] {
            0x01 => {
                self.mmode = cdb[2];
                self.log_event(format!("ATAPI 0xFE MMODE -> {}", self.mmode));
            }
            0x02 => {
                self.slog_enabled = cdb[2] != 0;
                self.log_event(format!("ATAPI 0xFE SLOG -> {}", self.slog_enabled));
            }
            0x03 => {
                let k0 = u32::from_be_bytes([cdb[2], cdb[3], cdb[4], cdb[5]]);
                let k1 = u32::from_be_bytes([cdb[6], cdb[7], cdb[8], cdb[9]]);
                if k0 == DRV_PASS_HEX_TOKEN_0 && k1 == DRV_PASS_HEX_TOKEN_1 {
                    self.sram_0cd8_auth |= 0x01;
                    self.sram_0d78_s211 |= 0x02;
                } else {
                    bail!("0xFE PASS token mismatch: got 0x{k0:08X} 0x{k1:08X}");
                }
            }
            _ => {}
        }
        Ok((
            "MATSUSHITA VENDOR 0xFE".to_string(),
            DRV_ATAPI_VENDOR_FE_VA,
            AtapiTransferDirection::None,
            Vec::new(),
            0x00,
            0x00,
            0x00,
            format!(
                "FUN_4002025e sub=0x{:02X} mmode={} auth=0x{:02X}",
                cdb[1], self.mmode, self.sram_0cd8_auth
            ),
        ))
    }
}
