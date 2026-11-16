//! `/BOOT.BIN` optical boot sequence verification (`startup_drive @ 0x44009cea`).

use super::{
    BootSequenceReport, BootSequenceStep, MatshitaBootScenario, MatshitaOddEmulator,
    build_sample_unlock_frm,
};
use crate::constants::PANA_DIGADIAG_DEFAULT_LOAD_ADDR;
use crate::crypto::{FrmContainerMode, decrypt_and_parse_frm_toc};
use anyhow::Result;

impl MatshitaOddEmulator {
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
