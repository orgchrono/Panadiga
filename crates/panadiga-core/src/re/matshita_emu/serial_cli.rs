//! `R6-MV4` `DRV` serial console (`PASS`, `MMODE`, `SLOGON`, `WB`, `WW`, `RB`) and unlock media mount.

use super::inject::InjectionLogEntry;
use super::{
    ATAPI_BUFFER_SIZE_BYTES, ATAPI_SECTOR_SIZE_BYTES, DRV_DECOMP_MMODE_HANDLER_VA,
    DRV_DECOMP_PASS_HANDLER_VA, DRV_DECOMP_SLOGOFF_HANDLER_VA, DRV_DECOMP_SLOGON_HANDLER_VA,
    DRV_PASS_HEX_TOKEN_0, DRV_PASS_HEX_TOKEN_1, FrmUnlockVerificationSummary, MatshitaBootScenario,
    MatshitaOddEmulator, SerialCliResponse, format_hex_bytes, is_protected_ipl_address,
    parse_u32_any, verify_unlock_frm_container,
};
use anyhow::{Result, bail};

impl MatshitaOddEmulator {
    /// Mounts a verified CD/DVD (`PANA_DVD.FRM`), `DIGADIAG.FRM`, or SD (`PANA__SD.FRM`) unlock image.
    pub fn mount_unlock_frm(
        &mut self,
        container_kind: &str,
        frm_bytes: &[u8],
    ) -> Result<FrmUnlockVerificationSummary> {
        let verification = verify_unlock_frm_container(container_kind, frm_bytes)?;
        match container_kind.trim().to_ascii_lowercase().as_str() {
            "dvd" | "cd" | "pana_dvd" => {
                self.disc_frm_bytes = Some(frm_bytes.to_vec());
                let needed_sectors = 16 + frm_bytes.len().div_ceil(ATAPI_SECTOR_SIZE_BYTES);
                if self.disc_image.len() < needed_sectors * ATAPI_SECTOR_SIZE_BYTES {
                    self.disc_image
                        .resize(needed_sectors * ATAPI_SECTOR_SIZE_BYTES, 0);
                }
                let start = 16 * ATAPI_SECTOR_SIZE_BYTES;
                self.disc_image[start..start + frm_bytes.len()].copy_from_slice(frm_bytes);
                self.scenario = MatshitaBootScenario::UnlockDiscFrm;
                self.log_event(format!(
                    "Mounted {} ({} B) at virtual ISO LBA 16",
                    verification.container_kind,
                    frm_bytes.len()
                ));
            }
            "sd" | "pana_sd" | "digadiag" => {
                self.staged_sd_container = Some(frm_bytes.to_vec());
                self.log_event(format!(
                    "Staged SD unlock media {} ({} B)",
                    verification.container_kind,
                    frm_bytes.len()
                ));
            }
            other => bail!("Unsupported unlock media kind '{other}'"),
        }
        Ok(verification)
    }

    /// Executes a Matshita `DRV` serial console command (`PASS`, `MMODE`, `SLOGON`, `WB`, `WW`, `RB`, etc.).
    pub fn execute_drv_serial_command(&mut self, raw_line: &str) -> Result<SerialCliResponse> {
        let parts: Vec<&str> = raw_line.split_whitespace().collect();
        if parts.is_empty() {
            bail!("Empty DRV serial CLI command");
        }
        let cmd = parts[0].to_ascii_uppercase();
        let mut out_lines = Vec::new();
        let status;

        match cmd.as_str() {
            "VER" => {
                out_lines.push("MATSHITA R6-MV4 S211 (DRV_decomp emulated)".to_string());
                status = "OK".to_string();
            }
            "PASS" => {
                if parts.len() != 3 {
                    bail!("Usage: PASS 0x4d415453 0x48495441");
                }
                let p0 = parse_u32_any(parts[1])?;
                let p1 = parse_u32_any(parts[2])?;
                if p0 == DRV_PASS_HEX_TOKEN_0 && p1 == DRV_PASS_HEX_TOKEN_1 {
                    self.sram_0cd8_auth |= 0x01;
                    self.sram_0d78_s211 |= 0x02;
                    out_lines.push(format!(
                        "PASS OK -> DAT_00000cd8=0x{:02X} DAT_00000d78=0x{:02X} (FUN_0x{:08X})",
                        self.sram_0cd8_auth, self.sram_0d78_s211, DRV_DECOMP_PASS_HANDLER_VA
                    ));
                    status = "UNLOCKED".to_string();
                } else {
                    out_lines.push("PASS NG (invalid token)".to_string());
                    status = "DENIED".to_string();
                }
            }
            "MMODE" => {
                let val = parts
                    .get(1)
                    .map(|s| parse_u32_any(s))
                    .transpose()?
                    .unwrap_or(1);
                self.mmode = val as u8;
                out_lines.push(format!(
                    "MMODE={} (FUN_0x{:08X})",
                    self.mmode, DRV_DECOMP_MMODE_HANDLER_VA
                ));
                status = "OK".to_string();
            }
            "SLOGON" => {
                self.slog_enabled = true;
                out_lines.push(format!(
                    "SLOGON enabled (FUN_0x{:08X})",
                    DRV_DECOMP_SLOGON_HANDLER_VA
                ));
                status = "OK".to_string();
            }
            "SLOGOFF" => {
                self.slog_enabled = false;
                out_lines.push(format!(
                    "SLOGOFF disabled (FUN_0x{:08X})",
                    DRV_DECOMP_SLOGOFF_HANDLER_VA
                ));
                status = "OK".to_string();
            }
            "SLOGDUMP" => {
                out_lines.extend(self.slog_entries.clone());
                if out_lines.is_empty() {
                    out_lines.push("(slog ring empty)".to_string());
                }
                status = "OK".to_string();
            }
            "RB" | "RW" => {
                let off = parts
                    .get(1)
                    .map(|s| parse_u32_any(s))
                    .transpose()?
                    .unwrap_or(0) as usize
                    % self.buffer.len();
                let count = if cmd == "RW" { 4 } else { 16 }.min(self.buffer.len() - off);
                out_lines.push(format!(
                    "0x{off:04X}: {}",
                    format_hex_bytes(&self.buffer[off..off + count])
                ));
                status = "OK".to_string();
            }
            "WB" | "WW" => {
                if (self.sram_0cd8_auth & 0x01) == 0 {
                    bail!("DRV CLI '{cmd}' blocked until PASS 0x4d415453 0x48495441 is sent");
                }
                if parts.len() < 3 {
                    bail!("Usage: {cmd} <addr> <value>");
                }
                let addr = parse_u32_any(parts[1])?;
                let val = parse_u32_any(parts[2])?;
                let width = if cmd == "WW" { 4 } else { 1 };
                if is_protected_ipl_address(addr, width) {
                    bail!(
                        "Hardware Safety Lockout (E05): '{cmd}' rejected in IPL NOR 0x{addr:08X}"
                    );
                }
                let buf_off = (addr as usize) & (ATAPI_BUFFER_SIZE_BYTES - width);
                let written = if width == 4 {
                    val.to_le_bytes().to_vec()
                } else {
                    vec![(val & 0xFF) as u8]
                };
                self.buffer[buf_off..buf_off + width].copy_from_slice(&written);
                self.record_injection(InjectionLogEntry {
                    source: "DRV_SERIAL_CLI",
                    cdb0_or_cmd: &cmd,
                    mode: width as u8,
                    target: addr,
                    payload: &written,
                    status: "UNLOCKED_WRITE",
                    note: format!("{cmd} 0x{addr:08X} <- 0x{val:X}"),
                });
                out_lines.push(format!("{cmd} OK: 0x{addr:08X} <= 0x{val:X}"));
                status = "WRITTEN".to_string();
            }
            other => bail!("Unsupported emulated DRV CLI command '{other}'"),
        }

        self.log_event(format!("CLI '{raw_line}' -> {status}"));
        Ok(SerialCliResponse {
            command: raw_line.to_string(),
            authorized_flag_cd8: self.sram_0cd8_auth,
            s211_flag_d78: self.sram_0d78_s211,
            slog_enabled: self.slog_enabled,
            output_lines: out_lines,
            status,
        })
    }
}
