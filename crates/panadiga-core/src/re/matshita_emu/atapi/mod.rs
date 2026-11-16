//! ATAPI CDB packet dispatch (`R6-MV4` `DRV`) for [`MatshitaOddEmulator`].

mod io_cdbs;
mod vendor_cdbs;

use anyhow::Result;

use super::{
    AtapiCommandRecord, AtapiTransferDirection, DRV_ATAPI_VENDOR_FD_VA, MatshitaBootScenario,
    MatshitaOddEmulator, format_hex_bytes,
};

pub(super) type CdbOutcome = (
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
        inq[0] = 0x05;
        inq[1] = 0x80;
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
}
