#![deny(unsafe_code)]

//! Sliced, SUM32-verified live NOR flash capture orchestrator over `PANAMON1>` / Pico `BRIDGE`.
//!
//! Replaces `capture_nor_dump.py` & `capture_all_8mb.py`: patches SDRAM uncached bounds
//! dynamically via [`Panamon1NorDumperOffsets`], verifies `BRIDGE: OK` telemetry
//! (`ctrl_edges == 0 && !overflow && ferr == 0`), persists optional per-slice checkpoints
//! with SHA-256 verification, and assembles contiguous NOR images.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::{decode_nor_dump_stream_slice, validate_nor_dump_range};
use crate::constants::{
    AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE, AM34_NVRAM_SIZE,
    AM34_NVRAM_UNCACHED_BASE,
};
use crate::nvram::calculate_hashes;
use crate::pico::panamon1::{
    BridgeExitSummary, PANAMON1_FULL_PROMPT, Panamon1Client, Panamon1Transport,
};
use crate::re::prog061_uart_monitor::{
    PROG061_UART_MONITOR_VA, Panamon1NorDumperOffsets, build_prog061_uart_monitor_with_offsets,
};

mod checkpoint;
#[cfg(test)]
mod tests;

use checkpoint::{save_checkpoint, try_load_checkpoint};

/// Default capture slice chunk size (1 MiB, matching `capture_all_8mb.py`).
pub const DEFAULT_NOR_CAPTURE_CHUNK_BYTES: u32 = 1024 * 1024;

/// Supported target slice presets for live `IC5001` NOR captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NorSlicePreset {
    Full8Mb,
    Loader256k,
    Nvram64k,
    Custom { start_va: u32, len: u32 },
}

impl NorSlicePreset {
    /// Resolves `(start_va, len)` and verifies read-only Spansion NOR window bounds.
    pub fn bounds(self) -> Result<(u32, u32)> {
        match self {
            Self::Full8Mb => Ok((AM34_FLASH_UNCACHED_BASE, AM34_FLASH_SIZE)),
            Self::Loader256k => Ok((AM34_FLASH_UNCACHED_BASE, AM34_FLASH_CRITICAL_IPL_SIZE)),
            Self::Nvram64k => Ok((AM34_NVRAM_UNCACHED_BASE, AM34_NVRAM_SIZE)),
            Self::Custom { start_va, len } => {
                validate_nor_dump_range(start_va, len, 4096)?;
                Ok((start_va, len))
            }
        }
    }
}

/// Configuration for [`NorCaptureOrchestrator`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NorCaptureConfig {
    pub chunk_size_bytes: u32,
    pub boot_mode: Option<String>,
    pub boot_timeout_ms: u32,
    pub checkpoint_dir: Option<PathBuf>,
    pub monitor_base_va: u32,
    pub offsets: Panamon1NorDumperOffsets,
    pub slice_timeout_ms: u64,
}

impl NorCaptureConfig {
    /// Builds configuration using dynamically resolved offsets from `build_prog061_uart_monitor_with_offsets`.
    pub fn try_default() -> Result<Self> {
        let (_, offsets) = build_prog061_uart_monitor_with_offsets(PROG061_UART_MONITOR_VA)?;
        Ok(Self {
            chunk_size_bytes: DEFAULT_NOR_CAPTURE_CHUNK_BYTES,
            boot_mode: Some("JIG:2500".to_string()),
            boot_timeout_ms: 6000,
            checkpoint_dir: None,
            monitor_base_va: PROG061_UART_MONITOR_VA,
            offsets,
            slice_timeout_ms: 120_000,
        })
    }
}

/// Per-slice capture audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NorSliceCaptureReport {
    pub start_va: u32,
    pub len: u32,
    pub sha256: String,
    pub bridge_summary: Option<BridgeExitSummary>,
    pub checkpoint_path: Option<String>,
    pub resumed_from_checkpoint: bool,
}

/// Final assembled NOR capture result and provenance report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NorCaptureResult {
    pub start_va: u32,
    pub total_len: usize,
    pub sha256: String,
    pub slices: Vec<NorSliceCaptureReport>,
    #[serde(skip)]
    pub data: Vec<u8>,
}

/// Stateful orchestrator driving sliced `PANAMON1>` NOR captures with CoW checkpoints.
#[derive(Debug, Clone)]
pub struct NorCaptureOrchestrator {
    pub config: NorCaptureConfig,
}

impl NorCaptureOrchestrator {
    pub fn new(config: NorCaptureConfig) -> Result<Self> {
        if config.chunk_size_bytes == 0
            || !config
                .chunk_size_bytes
                .is_multiple_of(config.offsets.block_size)
        {
            bail!(
                "chunk_size_bytes ({}) must be a non-zero multiple of block_size ({})",
                config.chunk_size_bytes,
                config.offsets.block_size
            );
        }
        Ok(Self { config })
    }

    pub fn try_default() -> Result<Self> {
        Self::new(NorCaptureConfig::try_default()?)
    }

    /// Captures the requested [`NorSlicePreset`], patching SDRAM bounds per sub-slice and verifying SUM32 + BRIDGE telemetry.
    pub fn capture_preset<T: Panamon1Transport>(
        &self,
        client: &mut Panamon1Client<T>,
        preset: NorSlicePreset,
        progress: &mut dyn FnMut(&str, usize, usize),
    ) -> Result<NorCaptureResult> {
        let (start_va, total_len) = preset.bounds()?;
        let mut plan = Vec::new();
        let mut cursor = 0u32;
        while cursor < total_len {
            let slice_len = (total_len - cursor).min(self.config.chunk_size_bytes);
            plan.push((start_va.wrapping_add(cursor), slice_len));
            cursor = cursor.wrapping_add(slice_len);
        }
        let (entry_u, start_imm_u, end_imm_u) = self
            .config
            .offsets
            .uncached_patch_vas(self.config.monitor_base_va);
        let insn_probe_va = start_imm_u.wrapping_sub(2);
        let mut assembled = Vec::with_capacity(total_len as usize);
        let mut reports = Vec::with_capacity(plan.len());
        let total_slices = plan.len();

        for (idx, (slice_va, slice_len)) in plan.into_iter().enumerate() {
            progress(
                &format!("Capturing NOR slice 0x{slice_va:08X} ({slice_len} B)"),
                idx,
                total_slices,
            );
            if let Some(dir) = self.config.checkpoint_dir.as_deref()
                && let Some((bytes, sha256, bin_path)) =
                    try_load_checkpoint(dir, slice_va, slice_len)?
            {
                assembled.extend_from_slice(&bytes);
                reports.push(NorSliceCaptureReport {
                    start_va: slice_va,
                    len: slice_len,
                    sha256,
                    bridge_summary: None,
                    checkpoint_path: Some(bin_path.display().to_string()),
                    resumed_from_checkpoint: true,
                });
                continue;
            }
            let _ = client.ensure_panamon1_bridge(
                self.config.boot_mode.as_deref(),
                self.config.boot_timeout_ms,
            )?;
            let op_bytes = client.mem_read_bytes(insn_probe_va, 12)?;
            if op_bytes[0..2] != [0xFC, 0xDC] || op_bytes[6..8] != [0xFC, 0xDD] {
                bail!(
                    "Unexpected SDRAM NOR dumper opcodes at 0x{insn_probe_va:08X}: {}",
                    hex::encode_upper(&op_bytes)
                );
            }
            let slice_end_va = slice_va.wrapping_add(slice_len);
            client.mem_write_bytes_burst(start_imm_u, &slice_va.to_le_bytes())?;
            client.mem_write_bytes_burst(end_imm_u, &slice_end_va.to_le_bytes())?;
            let verify = client.mem_read_bytes(insn_probe_va, 12)?;
            if verify[2..6] != slice_va.to_le_bytes() || verify[8..12] != slice_end_va.to_le_bytes()
            {
                bail!(
                    "SDRAM slice bound mismatch at 0x{insn_probe_va:08X}: {}",
                    hex::encode_upper(&verify)
                );
            }
            let raw_stream = client.jump_and_capture_raw(
                entry_u,
                &[b"PANADONEOK:DONE", PANAMON1_FULL_PROMPT.as_bytes()],
                Duration::from_millis(self.config.slice_timeout_ms),
            )?;
            let decoded = decode_nor_dump_stream_slice(&raw_stream, Some((slice_va, slice_len)))?;
            let bridge_summary = client.exit_bridge_checked()?;
            let checkpoint_path = self
                .config
                .checkpoint_dir
                .as_deref()
                .map(|dir| {
                    save_checkpoint(dir, slice_va, &decoded.data, &decoded.sha256)
                        .map(|p| p.display().to_string())
                })
                .transpose()?;
            assembled.extend_from_slice(&decoded.data);
            reports.push(NorSliceCaptureReport {
                start_va: slice_va,
                len: slice_len,
                sha256: decoded.sha256,
                bridge_summary: Some(bridge_summary),
                checkpoint_path,
                resumed_from_checkpoint: false,
            });
        }

        let _ = client.close_bridge_and_isolate()?;
        progress("NOR capture complete", total_slices, total_slices);
        let (_, sha256) = calculate_hashes(&assembled);
        Ok(NorCaptureResult {
            start_va,
            total_len: assembled.len(),
            sha256,
            slices: reports,
            data: assembled,
        })
    }
}
