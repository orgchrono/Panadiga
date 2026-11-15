//! `R6-MV4` `DRV` serial console, unlock media mounting, and `/BOOT.BIN` boot sequence verification.

use super::{
    ATAPI_BUFFER_SIZE_BYTES, ATAPI_SECTOR_SIZE_BYTES, BootSequenceReport, BootSequenceStep,
    DRV_DECOMP_MMODE_HANDLER_VA, DRV_DECOMP_PASS_HANDLER_VA, DRV_DECOMP_SLOGOFF_HANDLER_VA,
    DRV_DECOMP_SLOGON_HANDLER_VA, DRV_PASS_HEX_TOKEN_0, DRV_PASS_HEX_TOKEN_1,
    FrmUnlockVerificationSummary, MatshitaBootScenario, MatshitaOddEmulator, SerialCliResponse,
    build_sample_unlock_frm, format_hex_bytes, is_protected_ipl_address, parse_u32_any,
    verify_unlock_frm_container,
};
use crate::constants::PANA_DIGADIAG_DEFAULT_LOAD_ADDR;
use crate::crypto::{FrmContainerMode, decrypt_and_parse_frm_toc};
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
                self.record_injection(
                    "DRV_SERIAL_CLI",
                    &cmd,
                    width as u8,
                    addr,
                    &written,
                    "UNLOCKED_WRITE",
                    format!("{cmd} 0x{addr:08X} <- 0x{val:X}"),
                );
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

    /// Runs the full `/BOOT.BIN` optical boot sequence verification (`startup_drive @ 0x44009cea`).
    pub fn run_boot_sequence(&mut self) -> Result<BootSequenceReport> {
        let mut steps = Vec::new();
        let mut logs = vec![
            "***** Loader ver0.61 [06/03/06] *****".to_string(),
            match self.scenario {
                MatshitaBootScenario::UnlockDiscFrm => "***** serial/disc update mode *****",
                _ => "***** normal mode *****",
            }
            .to_string(),
        ];

        // 1. ATAPI Software Reset (0x08)
        let (r_rst, _) = self.issue_atapi_cdb(&[0x08, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], None)?;
        logs.push("[RTSD] ATAPI Software Reset ".to_string());
        logs.push("[RTSD] Device Reset Occurred(0x62900) ".to_string());
        steps.push(BootSequenceStep {
            step_index: 1,
            boot_fn_va: "0x44009B11".to_string(),
            boot_fn_name: "atapi_issue_pkt(SOFT_RESET)".to_string(),
            cdb_hex: r_rst.cdb_hex,
            sense_hex: r_rst.pana_error_hex,
            boot_log_line: "[RTSD] Device Reset Occurred(0x62900)".to_string(),
            passed: true,
        });

        // 2. INQUIRY (0x12)
        let (r_inq, inq_bytes) =
            self.issue_atapi_cdb(&[0x12, 0, 0, 0, 36, 0, 0, 0, 0, 0, 0, 0], None)?;
        let inquiry_id = String::from_utf8_lossy(&inq_bytes[8..36])
            .trim()
            .to_string();
        steps.push(BootSequenceStep {
            step_index: 2,
            boot_fn_va: "0x44009B11".to_string(),
            boot_fn_name: "atapi_issue_pkt(INQUIRY)".to_string(),
            cdb_hex: r_inq.cdb_hex,
            sense_hex: r_inq.pana_error_hex,
            boot_log_line: format!("INQUIRY={inquiry_id}"),
            passed: inquiry_id.contains("MATSHITA"),
        });

        // 3. close_tray (0x44009c85) + TEST UNIT READY (0x00)
        let (r_tur, _) = self.issue_atapi_cdb(&[0x00; 12], None)?;
        let tray_ok = self.is_tray_closed();
        if !tray_ok {
            logs.push("** can not close tray **".to_string());
        }
        steps.push(BootSequenceStep {
            step_index: 3,
            boot_fn_va: "0x44009C85".to_string(),
            boot_fn_name: "close_tray / TUR".to_string(),
            cdb_hex: r_tur.cdb_hex,
            sense_hex: r_tur.pana_error_hex,
            boot_log_line: if tray_ok {
                "** tray closed **".to_string()
            } else {
                "** can not close tray **".to_string()
            },
            passed: tray_ok,
        });

        // 4. startup_drive (0x44009cea) -> CDB [1B 00 00 00 03 00 01 00 00 00 00 00]
        logs.push("** startup drive **".to_string());
        let start_cdb = [
            0x1B, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let (r_su, _) = self.issue_atapi_cdb(&start_cdb, None)?;
        let startup_ok = r_su.sense_key == 0;
        if !startup_ok {
            logs.push(format!(
                "[RTSD] === DRIVE ACCESS ERROR === ( {} )",
                r_su.pana_error_hex
                    .trim_start_matches("0x")
                    .to_ascii_lowercase()
            ));
            logs.push(format!(
                "drive start error({})",
                r_su.pana_error_hex
                    .trim_start_matches("0x")
                    .to_ascii_lowercase()
            ));
        }
        steps.push(BootSequenceStep {
            step_index: 4,
            boot_fn_va: "0x44009CEA".to_string(),
            boot_fn_name: "startup_drive(START_STOP_UNIT)".to_string(),
            cdb_hex: r_su.cdb_hex,
            sense_hex: r_su.pana_error_hex.clone(),
            boot_log_line: r_su.note,
            passed: startup_ok,
        });

        // 5. Optical Unlock Container Verification (if UnlockDiscFrm or mounted disc_frm_bytes)
        let mut unlock_media_verified = false;
        let mut jump_target_va_hex = None;
        if startup_ok {
            if self.scenario == MatshitaBootScenario::UnlockDiscFrm && self.disc_frm_bytes.is_none()
            {
                let (demo_dvd, _) =
                    build_sample_unlock_frm(FrmContainerMode::Dvd, b"DEMO_UNLOCK_FVU")?;
                self.mount_unlock_frm("dvd", &demo_dvd)?;
            }
            if let Some(ref frm) = self.disc_frm_bytes {
                let toc = decrypt_and_parse_frm_toc(frm);
                unlock_media_verified = !toc.is_empty() && toc.iter().all(|e| e.list_sum_valid);
                if unlock_media_verified {
                    jump_target_va_hex = Some(format!("0x{PANA_DIGADIAG_DEFAULT_LOAD_ADDR:08X}"));
                    logs.push(format!(
                        "*** start fvu -> JMP 0x{PANA_DIGADIAG_DEFAULT_LOAD_ADDR:08X} ***"
                    ));
                    steps.push(BootSequenceStep {
                        step_index: 5,
                        boot_fn_va: "0x44006FDD".to_string(),
                        boot_fn_name: "boot_start_fvu(PANA_DVD.FRM)".to_string(),
                        cdb_hex: "28 00 00 00 00 10 00 00 01 00 00 00".to_string(),
                        sense_hex: "0x80000000".to_string(),
                        boot_log_line: format!(
                            "Verified {} TOC slot(s) -> JMP 0x{PANA_DIGADIAG_DEFAULT_LOAD_ADDR:08X}",
                            toc.len()
                        ),
                        passed: true,
                    });
                }
            }
        }

        let final_code = if startup_ok {
            "0x00000000".to_string()
        } else {
            r_su.pana_error_hex
        };

        Ok(BootSequenceReport {
            scenario: self.scenario,
            inquiry_id,
            steps,
            uart_boot_transcript: logs,
            startup_succeeded: startup_ok,
            unlock_media_verified,
            jump_target_va_hex,
            final_boot_code_hex: final_code,
        })
    }
}
