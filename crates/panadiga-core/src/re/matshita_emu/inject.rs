//! ATAPI write injection (`0x3B` WRITE BUFFER / `0xED` vendor channel + `0x3C` readback) for [`MatshitaOddEmulator`].

use super::{
    MatshitaEmulatorSnapshot, MatshitaOddEmulator, WriteInjectionRecord, encode_be24,
    is_protected_ipl_address, sha256_hex,
};
use anyhow::{Result, bail};

impl MatshitaOddEmulator {
    pub(super) fn record_injection(
        &mut self,
        source: &str,
        cdb0_or_cmd: &str,
        mode: u8,
        target: u32,
        payload: &[u8],
        status: &str,
        note: String,
    ) {
        let rec = WriteInjectionRecord {
            seq: self.injections.len() + 1,
            source: source.to_string(),
            cdb0_or_cmd: cdb0_or_cmd.to_string(),
            mode,
            target_va_or_lba: target,
            len_bytes: payload.len(),
            payload_sha256: sha256_hex(payload),
            status: status.to_string(),
            note: note.clone(),
        };
        self.injections.push(rec);
        self.log_event(note);
    }

    /// High-level helper: injects `payload` via ATAPI `0x3B` (or `0xED`) and verifies readback via `0x3C`.
    pub fn inject_write_buffer_and_verify(
        &mut self,
        offset: usize,
        mode: u8,
        use_vendor_ed: bool,
        payload: &[u8],
    ) -> Result<WriteInjectionRecord> {
        if use_vendor_ed {
            let mut prep = [0u8; 12];
            prep[0] = 0xED;
            prep[1] = 0x91;
            let tgt = self.staged_write_addr.wrapping_add(offset as u32);
            prep[2..6].copy_from_slice(&tgt.to_be_bytes());
            self.issue_atapi_cdb(&prep, None)?;

            let len_be = encode_be24(payload.len());
            let mut commit = [0u8; 12];
            commit[0] = 0xED;
            commit[1] = 0x93;
            commit[6..9].copy_from_slice(&len_be);
            self.issue_atapi_cdb(&commit, Some(payload))?;

            let mut rb = [0u8; 12];
            rb[0] = 0x3C;
            rb[6..9].copy_from_slice(&len_be);
            let (_, readback) = self.issue_atapi_cdb(&rb, None)?;
            if readback != payload {
                bail!("0xED + 0x3C readback mismatch!");
            }
        } else {
            let off_be = encode_be24(offset);
            let len_be = encode_be24(payload.len());
            let mut wb = [0u8; 12];
            wb[0] = 0x3B;
            wb[1] = mode & 0x1F;
            wb[3..6].copy_from_slice(&off_be);
            wb[6..9].copy_from_slice(&len_be);
            self.issue_atapi_cdb(&wb, Some(payload))?;

            let mut rb = [0u8; 12];
            rb[0] = 0x3C;
            rb[1] = mode & 0x1F;
            rb[3..9].copy_from_slice(&wb[3..9]);
            let (_, readback) = self.issue_atapi_cdb(&rb, None)?;
            if readback != payload {
                bail!("0x3B + 0x3C readback mismatch!");
            }
        }
        Ok(self.injections.last().expect("recorded injection").clone())
    }

    /// Optional target-VA-staged write injection (`0xED 0x91` + `0x3B`/`0xED 0x93` + `0x3C` verify).
    pub fn run_inject_write(
        &mut self,
        target_va: Option<u32>,
        offset: usize,
        mode: u8,
        use_vendor_ed: bool,
        payload: &[u8],
    ) -> Result<(WriteInjectionRecord, MatshitaEmulatorSnapshot)> {
        if let Some(va) = target_va {
            if is_protected_ipl_address(va, payload.len().max(1)) {
                bail!(
                    "Hardware Safety Lockout (E05): refusing target VA 0x{va:08X} in read-only IPL NOR"
                );
            }
            let b = va.to_be_bytes();
            self.issue_atapi_cdb(
                &[0xED, 0x91, b[0], b[1], b[2], b[3], 0, 0, 0, 0, 0, 0],
                None,
            )?;
        }
        let rec = self.inject_write_buffer_and_verify(offset, mode, use_vendor_ed, payload)?;
        Ok((rec, self.snapshot()))
    }
}
