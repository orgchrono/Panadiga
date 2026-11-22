#![deny(unsafe_code)]

//! Dynamic firmware anchor scanner and bench command/packet synthesizer for `bench_runtime`.

use serde::{Deserialize, Serialize};

use crate::constants::{AM34_SC0_BASE, PANA_FEISTEL_SBOX_256};
use crate::crypto::codec::read_u32_le;
use crate::re::bgo_unbrick::inspect_prog061_bgo_sites;
use crate::re::main_passthrough::{format_rsp_frame, format_rsp_write_u32_frame};
use crate::re::mn10300_traps::parse_drv_cli_table;
use crate::re::region_compat::{MmioWriteOp, inspect_main_region_compat};

/// Dynamically discovered firmware runtime anchors for a single unpacked stage binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchRuntimeStageDescriptor {
    pub stage_key: String,
    pub stage_kind: String,
    pub base_va_hex: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub source_container_kind: String,
    pub feistel_key_name: Option<String>,
    pub extrhead_build_name: Option<String>,
    pub compression_algo: Option<String>,
    pub bgo_layout_table_va_hex: Option<String>,
    pub bgo_partition_base_hex: Option<String>,
    pub panamon1_slack_va_hex: Option<String>,
    pub panamon1_slack_size_bytes: usize,
    pub bgo_init_hook_va_hex: Option<String>,
    pub cli_table_va_hex: Option<String>,
    pub cli_command_count: usize,
    pub feistel_sbox_vas_hex: Vec<String>,
    pub uart0_sc0_mmio_ref_count: usize,
    pub region_devinfo_va_hex: Option<String>,
    pub unpacked_path: Option<String>,
}

/// Finds the largest contiguous `0xFF` alignment/slack run `>= 4096` bytes, 4-byte aligned.
pub(super) fn find_ff_slack_window(raw: &[u8], base_va: u32) -> (Option<String>, usize) {
    let mut best_off = 0usize;
    let mut best_len = 0usize;
    let mut cur_start: Option<usize> = None;

    for (idx, &byte) in raw.iter().enumerate() {
        if byte == 0xFF {
            cur_start.get_or_insert(idx);
        } else if let Some(start) = cur_start.take() {
            let aligned_start = (start + 3) & !3;
            let len = idx.saturating_sub(aligned_start);
            if len > best_len {
                best_len = len;
                best_off = aligned_start;
            }
        }
    }
    if let Some(start) = cur_start {
        let aligned_start = (start + 3) & !3;
        let len = raw.len().saturating_sub(aligned_start);
        if len > best_len {
            best_len = len;
            best_off = aligned_start;
        }
    }
    if best_len < 4096 {
        return (None, 0);
    }
    (
        Some(format!("0x{:08X}", base_va.wrapping_add(best_off as u32))),
        best_len,
    )
}

fn find_bgo_layout_table(raw: &[u8], base_va: u32) -> (Option<String>, Option<String>) {
    for &bgo_base in &[0x8413_0000u32, 0x8420_0000u32] {
        let mut needle = [0u8; 8];
        needle[..4].copy_from_slice(&bgo_base.to_le_bytes());
        needle[4..8].copy_from_slice(&0x8440_0000u32.to_le_bytes());
        let mut pos = 0usize;
        while pos + 8 <= raw.len() {
            let Some(rel) = raw[pos..].windows(8).position(|w| w == needle) else {
                break;
            };
            let idx = pos + rel;
            if idx.is_multiple_of(4) && idx >= 44 && idx + 236 <= raw.len() {
                let tbl_off = idx - 44;
                if read_u32_le(raw, tbl_off + 32) == 0x10000
                    && read_u32_le(raw, tbl_off + 52) == 0x10000
                {
                    return (
                        Some(format!("0x{:08X}", base_va.wrapping_add(tbl_off as u32))),
                        Some(format!("0x{bgo_base:08X}")),
                    );
                }
            }
            pos = idx + 4;
        }
    }
    (None, None)
}

fn count_uart0_sc0_refs(raw: &[u8]) -> usize {
    let mut count = 0usize;
    let sc0_lo = AM34_SC0_BASE;
    let sc0_hi = AM34_SC0_BASE + 0x0B;
    for chunk in raw.windows(4).step_by(2) {
        let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if (sc0_lo..=sc0_hi).contains(&val) {
            count += 1;
        }
    }
    count
}

fn find_feistel_sbox_vas(raw: &[u8], base_va: u32) -> Vec<String> {
    let sig = &PANA_FEISTEL_SBOX_256[..13];
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + sig.len() <= raw.len() {
        let Some(rel) = raw[pos..].windows(sig.len()).position(|w| w == sig) else {
            break;
        };
        let idx = pos + rel;
        if idx + 256 <= raw.len() && raw[idx..idx + 256] == PANA_FEISTEL_SBOX_256 {
            out.push(format!("0x{:08X}", base_va.wrapping_add(idx as u32)));
        }
        pos = idx + 1;
    }
    out
}

/// Scans an unpacked firmware stage buffer for BGO tables, slack windows, CLI tables, S-boxes, and hooks.
#[allow(clippy::too_many_arguments)]
pub(super) fn scan_stage_anchors(
    stage_key: String,
    stage_kind: String,
    base_va: u32,
    raw: &[u8],
    sha256: String,
    source_container_kind: String,
    feistel_key_name: Option<String>,
    extrhead_build_name: Option<String>,
    compression_algo: Option<String>,
    unpacked_path: Option<String>,
) -> BenchRuntimeStageDescriptor {
    let (bgo_layout_table_va_hex, bgo_partition_base_hex) = find_bgo_layout_table(raw, base_va);
    let (panamon1_slack_va_hex, panamon1_slack_size_bytes) = if stage_kind == "PROG" {
        if raw.len() >= 0x3_9494 + 4096 && raw[0x3_9494..0x3_A494].iter().all(|&b| b == 0xFF) {
            let tail_len = raw[0x3_9494..].iter().take_while(|&&b| b == 0xFF).count();
            (Some("0x50439494".to_string()), tail_len)
        } else {
            find_ff_slack_window(raw, base_va)
        }
    } else {
        (None, 0)
    };

    let bgo_init_hook_va_hex = if stage_kind == "PROG" {
        inspect_prog061_bgo_sites(raw)
            .ok()
            .map(|rep| rep.bgo_init_hook_site.va_hex)
    } else {
        None
    };

    let (cli_table_va_hex, cli_command_count) =
        if let Some(off) = raw.windows(8).position(|w| w == b"SLOGON\0\0") {
            let entries = parse_drv_cli_table(raw, off, base_va, 160, false);
            (
                Some(format!("0x{:08X}", base_va.wrapping_add(off as u32))),
                entries.len(),
            )
        } else {
            (None, 0)
        };

    let region_devinfo_va_hex = if stage_kind == "MAIN" {
        inspect_main_region_compat(raw).ok().map(|r| r.va_hex)
    } else {
        None
    };

    BenchRuntimeStageDescriptor {
        stage_key,
        stage_kind,
        base_va_hex: format!("0x{base_va:08X}"),
        size_bytes: raw.len(),
        sha256,
        source_container_kind,
        feistel_key_name,
        extrhead_build_name,
        compression_algo,
        bgo_layout_table_va_hex,
        bgo_partition_base_hex,
        panamon1_slack_va_hex,
        panamon1_slack_size_bytes,
        bgo_init_hook_va_hex,
        cli_table_va_hex,
        cli_command_count,
        feistel_sbox_vas_hex: find_feistel_sbox_vas(raw, base_va),
        uart0_sc0_mmio_ref_count: count_uart0_sc0_refs(raw),
        region_devinfo_va_hex,
        unpacked_path,
    }
}

pub(super) fn build_panamon1_bench_script(
    bgo_base_hex: &str,
    tv_mmio_plan: &[MmioWriteOp],
) -> Vec<String> {
    let bgo_clean = bgo_base_hex
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    let mut lines = vec![
        "J".to_string(),
        format!("R {bgo_clean}"),
        "R 847F0000".to_string(),
    ];
    for op in tv_mmio_plan {
        lines.push(format!(
            "W {:08X} {:08X} ; {} ({}b)",
            op.addr, op.value, op.register_name, op.width_bits
        ));
    }
    lines
}

pub(super) fn build_gdb_rsp_packets(tv_mmio_plan: &[MmioWriteOp]) -> Vec<String> {
    let mut packets = vec![format_rsp_frame("m84000000,40")];
    for op in tv_mmio_plan {
        if op.width_bits == 16 {
            let bytes = (op.value as u16).to_le_bytes();
            let payload = format!("M{:08x},2:{:02x}{:02x}", op.addr, bytes[0], bytes[1]);
            packets.push(format_rsp_frame(&payload));
        } else {
            packets.push(format_rsp_write_u32_frame(op.addr, op.value));
        }
    }
    packets
}
