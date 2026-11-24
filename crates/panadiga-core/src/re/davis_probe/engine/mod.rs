#![deny(unsafe_code)]

//! Live `DAVIS` `CH0`/`CH1` & `IDENTIFY DEVICE (0xEC)` diagnostic types, guards, and runner.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::ata_identify::AtaIdentifyInfo;
use super::stub_builder::{DAVIS_IDENTIFY_BUF_DEFAULT_VA, DAVIS_STUB_DEFAULT_VA};
use crate::constants::AM34_FORBIDDEN_LETHAL_REG;

pub mod runner;

#[cfg(test)]
mod tests;

pub use runner::DavisProbeEngine;

/// Diagnostic root-cause classification for a probed `DAVIS` IDE channel (`CH1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DavisHangClassification {
    FloatingBus,
    ResetStuckOrUnclocked,
    BsyIordyHang,
    CsJumperOrSlaveContention,
    AtapiSignatureDetected,
    AtaReady,
    Unknown,
}

/// 10-word ATA taskfile + `ALTSTAT` + sentinel snapshot returned by `DAVIS` SDRAM probe stubs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DavisTaskfileSnapshot {
    pub data: u32,
    pub err: u32,
    pub seccnt: u32,
    pub lbalo: u32,
    pub lbamid: u32,
    pub lbahi: u32,
    pub devsel: u32,
    pub status: u32,
    pub altstat: u32,
    pub sentinel: u32,
}

impl DavisTaskfileSnapshot {
    /// Decodes a 40-byte little-endian `DAVIS` taskfile capture buffer.
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 40 {
            bail!(
                "Expected >= 40 bytes for DavisTaskfileSnapshot, got {}",
                buf.len()
            );
        }
        let w = |idx: usize| -> u32 {
            let o = idx * 4;
            u32::from_le_bytes([buf[o], buf[o + 1], buf[o + 2], buf[o + 3]])
        };
        Ok(Self {
            data: w(0),
            err: w(1),
            seccnt: w(2),
            lbalo: w(3),
            lbamid: w(4),
            lbahi: w(5),
            devsel: w(6),
            status: w(7),
            altstat: w(8),
            sentinel: w(9),
        })
    }

    #[must_use]
    pub fn is_atapi_signature(&self) -> bool {
        (self.lbamid & 0xFF) == 0x14 && (self.lbahi & 0xFF) == 0xEB
    }

    /// Mirrors `is_valid_ata(tf, check_scratch)` from `probe_ch1_live_v2.py`.
    #[must_use]
    pub fn is_valid_ata(&self, check_scratch: bool) -> bool {
        let st_low = self.status & 0xFF;
        let alt_low = self.altstat & 0xFF;
        let st_16 = self.status & 0xFFFF;
        let alt_16 = self.altstat & 0xFFFF;
        let drdy_ok =
            matches!(st_16, 0x40 | 0x50 | 0x51 | 0x58) || matches!(alt_16, 0x40 | 0x50 | 0x58);
        let scratch_ok = check_scratch
            && (self.seccnt & 0xFF) == 0x55
            && (self.lbalo & 0xFF) == 0xAA
            && self.data != 0x2004;
        let bsy_live = st_low == 0x80 || alt_low == 0x80;
        drdy_ok || scratch_ok || bsy_live
    }
}

const fn default_run_identify() -> bool {
    true
}

/// Configuration for [`DavisProbeEngine`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DavisProbeConfig {
    pub sdram_stub_va: u32,
    pub identify_out_va: u32,
    pub drive_sel: u8,
    pub boot_mode: Option<String>,
    pub boot_timeout_ms: u32,
    #[serde(default = "default_run_identify")]
    pub run_identify: bool,
}

impl Default for DavisProbeConfig {
    fn default() -> Self {
        Self {
            sdram_stub_va: DAVIS_STUB_DEFAULT_VA,
            identify_out_va: DAVIS_IDENTIFY_BUF_DEFAULT_VA,
            drive_sel: 0xA0,
            boot_mode: Some("NORMAL".to_string()),
            boot_timeout_ms: 15_000,
            run_identify: true,
        }
    }
}

/// Structured telemetry report produced by [`DavisProbeEngine::run_probe`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DavisProbeReport {
    pub c0001008_before: u16,
    pub c0001008_after: u16,
    pub fe_released: bool,
    pub ch0_taskfile: DavisTaskfileSnapshot,
    pub ch1_raw: DavisTaskfileSnapshot,
    pub ch1_master: DavisTaskfileSnapshot,
    pub ch1_slave: DavisTaskfileSnapshot,
    pub ch1_2b_reset: Option<DavisTaskfileSnapshot>,
    pub ch1_2c_reset: Option<DavisTaskfileSnapshot>,
    pub fun_cd4d_rc: u32,
    pub ch1_struct_hex: String,
    pub identify_sentinel: Option<u32>,
    pub identify_post_altstat: Option<u32>,
    pub identify_info: Option<AtaIdentifyInfo>,
    #[serde(skip)]
    pub identify_raw_512: Option<Vec<u8>>,
    pub classification: DavisHangClassification,
    pub step_logs: Vec<String>,
}

/// Fail-closed read guard prohibiting any touch of lethal register `0xC000_100C`.
pub fn validate_safe_mmio_read_range(addr: u32, len: usize) -> Result<()> {
    let start = u64::from(addr);
    let end = start + len.max(1) as u64;
    let lethal_lo = u64::from(AM34_FORBIDDEN_LETHAL_REG);
    let lethal_hi = lethal_lo + 4;
    if start < lethal_hi && end > lethal_lo {
        bail!(
            "Blocked read of lethal AM34 register 0xC000100C (requested 0x{addr:08X}..+0x{len:X})"
        );
    }
    Ok(())
}

/// Classifies the observed `DAVIS CH1` bus state from taskfile snapshots and optional `0xEC` info.
#[must_use]
pub fn classify_davis_ch1(
    raw: &DavisTaskfileSnapshot,
    master: &DavisTaskfileSnapshot,
    slave: &DavisTaskfileSnapshot,
    identify_info: Option<&AtaIdentifyInfo>,
) -> DavisHangClassification {
    if identify_info.is_some()
        || (!master.is_atapi_signature()
            && matches!(master.status & 0xFF, 0x40 | 0x50 | 0x51 | 0x58))
    {
        return DavisHangClassification::AtaReady;
    }
    if master.is_atapi_signature() || raw.is_atapi_signature() {
        return DavisHangClassification::AtapiSignatureDetected;
    }
    if (master.status & 0xFF) == 0x80 || (master.altstat & 0xFF) == 0x80 {
        return DavisHangClassification::BsyIordyHang;
    }
    let master_scratch = (master.seccnt & 0xFF) == 0x55 && (master.lbalo & 0xFF) == 0xAA;
    let slave_scratch = (slave.seccnt & 0xFF) == 0x55 && (slave.lbalo & 0xFF) == 0xAA;
    if slave_scratch && !master_scratch {
        return DavisHangClassification::CsJumperOrSlaveContention;
    }
    if (raw.status & 0xFF) == 0xFF && (master.status & 0xFF) == 0xFF {
        return DavisHangClassification::FloatingBus;
    }
    if (raw.status & 0xFFFF) == 0 && (master.status & 0xFFFF) == 0 && (master.seccnt & 0xFF) == 0 {
        return DavisHangClassification::ResetStuckOrUnclocked;
    }
    DavisHangClassification::Unknown
}
