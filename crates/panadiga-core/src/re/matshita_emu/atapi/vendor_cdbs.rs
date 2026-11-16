//! Matsushita vendor ATAPI CDB handlers (`0xED` buffer dispatcher and `0xFE` unlock/MMODE/SLOG).

use anyhow::{Result, bail};

use super::super::inject::InjectionLogEntry;
use super::super::{
    AtapiTransferDirection, DRV_ATAPI_VENDOR_ED_VA, DRV_ATAPI_VENDOR_FE_VA, DRV_PASS_HEX_TOKEN_0,
    DRV_PASS_HEX_TOKEN_1, MatshitaOddEmulator, is_protected_ipl_address,
};
use super::CdbOutcome;

impl MatshitaOddEmulator {
    pub(super) fn handle_cdb_vendor_ed(
        &mut self,
        cdb: &[u8; 12],
        data_out: Option<&[u8]>,
    ) -> Result<CdbOutcome> {
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
                    self.record_injection(InjectionLogEntry {
                        source: "ATAPI_VENDOR_ED",
                        cdb0_or_cmd: "0xED",
                        mode: sub,
                        target: self.staged_write_addr,
                        payload: &payload[..copy_len],
                        status: if (self.sram_0cd8_auth & 1) != 0 {
                            "COMMITTED_UNLOCKED"
                        } else {
                            "STAGED_UNAUTH_CHECKED"
                        },
                        note: format!(
                            "FUN_400209f3 sub=0x{sub:02X} staged {copy_len} B @ 0x{:08X}",
                            self.staged_write_addr
                        ),
                    });
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

    pub(super) fn handle_cdb_vendor_fe(&mut self, cdb: &[u8; 12]) -> Result<CdbOutcome> {
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
