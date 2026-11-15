//! Matshita `R6-MV4` ATAPI/Serial ODD Emulator, Boot Sequence Verifier & Write Injector.
//!
//! Accurately models verified reverse-engineered behavior from `/BOOT.BIN`
//! (`startup_drive @ 0x44009cea`, `close_tray @ 0x44009c85`, `open_tray @ 0x44009bd7`,
//! `atapi_issue_pkt @ 0x44009b11`) and `DRV` (`0x3B WRITE BUFFER @ 0x4002168d`,
//! `0x3C READ BUFFER @ 0x40021476`, `0xED @ 0x400209f3`, `0xFE @ 0x4002025e`,
//! `0xFD @ 0x40020147`, `PASS @ 0x4013420a` / `0x40130efa`, `MMODE @ 0x40134382`,
//! `SLOGON`/`SLOGOFF @ 0x40133ed0`/`0x40133ede`), plus SD (`PANA__SD.FRM`/`DIGADIAG.FRM`)
//! and CD/DVD (`PANA_DVD.FRM`) unlock container packaging and verification.

mod atapi;
mod frm_unlock;
mod inject;
mod serial_boot;
pub mod types;

#[cfg(test)]
mod tests;

pub use frm_unlock::{
    build_sample_unlock_frm, parse_hex_payload, parse_unlock_container_mode,
    verify_unlock_frm_container,
};
pub use types::*;

use crate::constants::PANA_DIGADIAG_DEFAULT_LOAD_ADDR;
use crate::nvram::calculate_hashes;
pub(crate) use crate::re::mn10300_traps::{
    DRV_ATAPI_READ_BUFFER_3C_VA, DRV_ATAPI_VENDOR_ED_VA, DRV_ATAPI_VENDOR_FD_VA,
    DRV_ATAPI_VENDOR_FE_VA, DRV_ATAPI_WRITE_BUFFER_3B_VA, DRV_DECOMP_MMODE_HANDLER_VA,
    DRV_DECOMP_PASS_HANDLER_VA, DRV_DECOMP_SLOGOFF_HANDLER_VA, DRV_DECOMP_SLOGON_HANDLER_VA,
};
use anyhow::Result;

pub const ATAPI_BUFFER_SIZE_BYTES: usize = 64 * 1024;
pub const ATAPI_SECTOR_SIZE_BYTES: usize = 2048;
pub const MAX_VIRTUAL_DISC_SECTORS: usize = 4096;
pub const DRV_PASS_HEX_TOKEN_0: u32 = 0x4D41_5453; // "MATS"
pub const DRV_PASS_HEX_TOKEN_1: u32 = 0x4849_5441; // "HITA"

pub(super) const fn encode_be24(val: usize) -> [u8; 3] {
    [
        ((val >> 16) & 0xFF) as u8,
        ((val >> 8) & 0xFF) as u8,
        (val & 0xFF) as u8,
    ]
}

pub(super) fn is_protected_ipl_address(addr: u32, len: usize) -> bool {
    let end = addr.saturating_add(len.max(1) as u32);
    let in_cached_ipl = addr < 0x807F_0000 && end > 0x8000_0000;
    let in_uncached_ipl = addr < 0x847F_0000 && end > 0x8400_0000;
    in_cached_ipl || in_uncached_ipl
}

pub(super) fn format_hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    calculate_hashes(bytes).1
}

pub(super) fn parse_u32_any(raw: &str) -> Result<u32> {
    crate::crypto::codec::parse_u32_radix(raw)
}

/// Deterministic software emulator for the Panasonic / Matshita `R6-MV4` optical drive.
pub struct MatshitaOddEmulator {
    pub(super) scenario: MatshitaBootScenario,
    pub(super) buffer: Vec<u8>,
    pub(super) disc_image: Vec<u8>,
    pub(super) disc_frm_bytes: Option<Vec<u8>>,
    pub(super) sram_0cd8_auth: u8,
    pub(super) sram_0d78_s211: u8,
    pub(super) sram_0242_ed_mode: u8,
    pub(super) staged_write_addr: u32,
    pub(super) mmode: u8,
    pub(super) slog_enabled: bool,
    pub(super) slog_entries: Vec<String>,
    pub(super) history: Vec<AtapiCommandRecord>,
    pub(super) injections: Vec<WriteInjectionRecord>,
    pub(super) staged_sd_container: Option<Vec<u8>>,
}

impl Default for MatshitaOddEmulator {
    fn default() -> Self {
        Self::new(MatshitaBootScenario::HealthyWithDisc)
    }
}

impl MatshitaOddEmulator {
    /// Creates an initialized `R6-MV4` emulator instance for the chosen bench scenario.
    pub fn new(scenario: MatshitaBootScenario) -> Self {
        Self {
            scenario,
            buffer: vec![0u8; ATAPI_BUFFER_SIZE_BYTES],
            disc_image: vec![0u8; ATAPI_SECTOR_SIZE_BYTES * 32],
            disc_frm_bytes: None,
            sram_0cd8_auth: 0,
            sram_0d78_s211: 0,
            sram_0242_ed_mode: 0,
            staged_write_addr: PANA_DIGADIAG_DEFAULT_LOAD_ADDR,
            mmode: 0,
            slog_enabled: false,
            slog_entries: Vec::new(),
            history: Vec::new(),
            injections: Vec::new(),
            staged_sd_container: None,
        }
    }

    pub fn set_scenario(&mut self, scenario: MatshitaBootScenario) {
        self.scenario = scenario;
        self.log_event(format!("Scenario switched to {:?}", scenario));
    }

    pub(super) fn log_event(&mut self, msg: impl Into<String>) {
        if self.slog_enabled {
            let line = msg.into();
            if self.slog_entries.len() >= 128 {
                self.slog_entries.remove(0);
            }
            self.slog_entries.push(line);
        }
    }

    pub(super) fn is_tray_closed(&self) -> bool {
        !matches!(self.scenario, MatshitaBootScenario::TrayCannotClose80045300)
    }

    pub(super) fn is_media_present(&self) -> bool {
        self.is_tray_closed()
            && (self.disc_frm_bytes.is_some()
                || matches!(
                    self.scenario,
                    MatshitaBootScenario::HealthyWithDisc
                        | MatshitaBootScenario::UnlockDiscFrm
                        | MatshitaBootScenario::LaserApcFailure80040904
                ))
    }

    /// Exports an immutable snapshot of internal SRAM flags, buffer SHA-256, SLOG, and injections.
    pub fn snapshot(&self) -> MatshitaEmulatorSnapshot {
        MatshitaEmulatorSnapshot {
            inquiry_vendor: "MATSHITA".to_string(),
            inquiry_product: "R6-MV4".to_string(),
            inquiry_revision: "S211".to_string(),
            tray_closed: self.is_tray_closed(),
            media_present: self.is_media_present(),
            sram_0cd8_auth: self.sram_0cd8_auth,
            sram_0d78_s211: self.sram_0d78_s211,
            sram_0242_ed_mode: self.sram_0242_ed_mode,
            mmode: self.mmode,
            slog_enabled: self.slog_enabled,
            slog_entries: self.slog_entries.clone(),
            buffer_len: self.buffer.len(),
            buffer_sha256: sha256_hex(&self.buffer),
            disc_lba_sectors: self.disc_image.len() / ATAPI_SECTOR_SIZE_BYTES,
            history_count: self.history.len(),
            injections: self.injections.clone(),
        }
    }
}
