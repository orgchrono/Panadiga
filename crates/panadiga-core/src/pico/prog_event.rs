#![deny(unsafe_code)]

//! Parsers and telemetry math for Pico `P7501` push progress frames (`@P7501_PROG`, `[NOR_MAP]`)
//! and live NOR/BGO throughput + ETA computation.

use serde::{Deserialize, Serialize};

use crate::constants::{AM34_FLASH_BASE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};

/// Optional real-time sector, byte-volume, throughput, and ETA telemetry attached to progress events.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamProgressTelemetry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_sa: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_done: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_total: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_per_sec: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u32>,
}

/// Real-time push progress frame emitted by Pico C11 firmware (`NOR_MAP`, `FAST_READ`, `FAST_WRITE`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PicoStreamProgressEvent {
    pub phase: String,
    pub sa: u16,
    pub done: u32,
    pub total: u32,
    pub crc32: u32,
    pub vdd_mv: u32,
}

/// Decoded 142-sector Spansion `S29JL064H` summary emitted by Pico `NOR_MAP`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PicoNorMapSummary {
    pub sectors: u16,
    pub ipl_crc32: u32,
    pub fe_extrhead: bool,
    pub norfrom_used: u16,
    pub bgo_p1: u16,
    pub bgo_p2: u16,
    pub bgo_spares: u16,
    pub bgo_corrupt: u16,
    pub eemul_active: u16,
    pub eemul_superseded: u16,
    pub eemul_empty: u16,
    pub eemul_spare: u16,
    pub nvram_active: u16,
    pub nvram_blank: u16,
    pub healthy: bool,
}

fn parse_kv_tokens(rest: &str) -> Vec<(&str, &str)> {
    rest.split_whitespace()
        .filter_map(|tok| tok.split_once('='))
        .collect()
}

fn kv_str<'a>(kvs: &[(&'a str, &'a str)], key: &str) -> Option<&'a str> {
    kvs.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

fn kv_u32(kvs: &[(&str, &str)], key: &str) -> Option<u32> {
    let raw = kv_str(kvs, key)?;
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        raw.parse::<u32>().ok()
    }
}

fn kv_u16(kvs: &[(&str, &str)], key: &str) -> Option<u16> {
    u16::try_from(kv_u32(kvs, key)?).ok()
}

fn kv_yes_no(kvs: &[(&str, &str)], key: &str) -> Option<bool> {
    match kv_str(kvs, key)? {
        "YES" => Some(true),
        "NO" => Some(false),
        _ => None,
    }
}

/// Parses an asynchronous `@P7501_PROG phase=... sa=... done=... total=... crc32=0x... vdd_mv=...` frame.
pub fn parse_p7501_prog_line(line: &str) -> Option<PicoStreamProgressEvent> {
    let body = line.trim().strip_prefix("@P7501_PROG ")?;
    let kvs = parse_kv_tokens(body);
    Some(PicoStreamProgressEvent {
        phase: kv_str(&kvs, "phase")?.to_string(),
        sa: kv_u16(&kvs, "sa")?,
        done: kv_u32(&kvs, "done")?,
        total: kv_u32(&kvs, "total")?,
        crc32: kv_u32(&kvs, "crc32")?,
        vdd_mv: kv_u32(&kvs, "vdd_mv")?,
    })
}

/// Parses a `[NOR_MAP] sectors=... healthy=YES|NO` summary line emitted by Pico `NOR_MAP`.
pub fn parse_p7501_nor_map_line(line: &str) -> Option<PicoNorMapSummary> {
    let body = line.trim().strip_prefix("[NOR_MAP] ")?;
    let kvs = parse_kv_tokens(body);
    Some(PicoNorMapSummary {
        sectors: kv_u16(&kvs, "sectors")?,
        ipl_crc32: kv_u32(&kvs, "ipl_crc32")?,
        fe_extrhead: kv_yes_no(&kvs, "fe_extrhead")?,
        norfrom_used: kv_u16(&kvs, "norfrom_used")?,
        bgo_p1: kv_u16(&kvs, "bgo_p1")?,
        bgo_p2: kv_u16(&kvs, "bgo_p2")?,
        bgo_spares: kv_u16(&kvs, "bgo_spares")?,
        bgo_corrupt: kv_u16(&kvs, "bgo_corrupt")?,
        eemul_active: kv_u16(&kvs, "eemul_active")?,
        eemul_superseded: kv_u16(&kvs, "eemul_superseded")?,
        eemul_empty: kv_u16(&kvs, "eemul_empty")?,
        eemul_spare: kv_u16(&kvs, "eemul_spare")?,
        nvram_active: kv_u16(&kvs, "nvram_active")?,
        nvram_blank: kv_u16(&kvs, "nvram_blank")?,
        healthy: kv_yes_no(&kvs, "healthy")?,
    })
}

/// Maps an AM34 cached (`0x80000000..`) or uncached (`0x84000000..`) Spansion `S29JL064H` VA to its sector index `SA0..SA141`.
pub fn flash_va_to_sector_sa(va: u32) -> Option<usize> {
    let off =
        if (AM34_FLASH_UNCACHED_BASE..AM34_FLASH_UNCACHED_BASE + AM34_FLASH_SIZE).contains(&va) {
            va - AM34_FLASH_UNCACHED_BASE
        } else if (AM34_FLASH_BASE..AM34_FLASH_BASE + AM34_FLASH_SIZE).contains(&va) {
            va - AM34_FLASH_BASE
        } else {
            return None;
        };
    if off < 0x10000 {
        Some((off / 0x2000) as usize)
    } else if off < 0x7F0000 {
        Some(8 + ((off - 0x10000) / 0x10000) as usize)
    } else {
        Some(134 + ((off - 0x7F0000) / 0x2000) as usize)
    }
}

/// Computes live `(bytes_per_sec, eta_seconds)` from elapsed milliseconds and completed/total byte counts.
pub fn compute_rate_and_eta(
    elapsed_ms: u128,
    bytes_done: usize,
    bytes_total: usize,
) -> (Option<u32>, Option<u32>) {
    if elapsed_ms == 0 || bytes_done == 0 {
        return (None, None);
    }
    let bps_u128 = (bytes_done as u128).saturating_mul(1_000) / elapsed_ms;
    let bps = u32::try_from(bps_u128).unwrap_or(u32::MAX);
    if bps == 0 {
        return (None, None);
    }
    let remaining = bytes_total.saturating_sub(bytes_done) as u128;
    let eta_u128 = remaining.div_ceil(u128::from(bps));
    let eta = u32::try_from(eta_u128).unwrap_or(u32::MAX);
    (Some(bps), Some(eta))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_p7501_prog_and_nor_map_frames() {
        let prog = parse_p7501_prog_line(
            "@P7501_PROG phase=READ sa=26 done=4096 total=65536 crc32=0x3D2D3D4A vdd_mv=3295",
        )
        .expect("valid @P7501_PROG frame");
        assert_eq!(prog.phase, "READ");
        assert_eq!(prog.sa, 26);
        assert_eq!(prog.done, 4096);
        assert_eq!(prog.total, 65536);
        assert_eq!(prog.crc32, 0x3D2D_3D4A);
        assert_eq!(prog.vdd_mv, 3295);
        assert!(parse_p7501_prog_line("PONG").is_none());

        let normap = parse_p7501_nor_map_line(
            "[NOR_MAP] sectors=142 ipl_crc32=0x1234ABCD fe_extrhead=YES norfrom_used=2 \
             bgo_p1=1 bgo_p2=40 bgo_spares=4 bgo_corrupt=0 \
             eemul_active=2 eemul_superseded=61 eemul_empty=0 eemul_spare=0 \
             nvram_active=8 nvram_blank=0 healthy=YES",
        )
        .expect("valid [NOR_MAP] frame");
        assert_eq!(normap.sectors, 142);
        assert_eq!(normap.ipl_crc32, 0x1234_ABCD);
        assert!(normap.fe_extrhead);
        assert_eq!(normap.bgo_p1, 1);
        assert_eq!(normap.bgo_p2, 40);
        assert_eq!(normap.bgo_spares, 4);
        assert_eq!(normap.bgo_corrupt, 0);
        assert!(normap.healthy);
    }

    #[test]
    fn maps_flash_va_to_sector_and_computes_rate_eta() {
        assert_eq!(flash_va_to_sector_sa(0x8400_0000), Some(0));
        assert_eq!(flash_va_to_sector_sa(0x8401_0000), Some(8));
        assert_eq!(flash_va_to_sector_sa(0x8413_0000), Some(26));
        assert_eq!(flash_va_to_sector_sa(0x847F_0000), Some(134));
        assert_eq!(flash_va_to_sector_sa(0x807F_E000), Some(141));
        assert_eq!(flash_va_to_sector_sa(0x9000_0000), None);

        assert_eq!(compute_rate_and_eta(0, 1024, 4096), (None, None));
        assert_eq!(
            compute_rate_and_eta(1000, 10_240, 30_720),
            (Some(10_240), Some(2))
        );
        assert_eq!(
            compute_rate_and_eta(2000, 30_720, 30_720),
            (Some(15_360), Some(0))
        );
    }
}
