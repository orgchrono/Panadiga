//! Tauri 2 IPC Commands for Hot Bare-Metal Silicon Explorer (`ndb>` over PicoFW `BRIDGE`).

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

use panadiga_core::bench::OpKind;
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::pico::decode_captured_log_bytes;
use panadiga_core::pico::ndb::{
    NdbAccessKind, NdbDdResult, NdbLogReplaySummary, NdbSession, RollbackReport,
    SerialNdbTransport, VolatileDiffEntry, VolatileDiffReport, VolatilePatchOp, diff_dd_snapshots,
    parse_ndb_dd_output, replay_ndb_evidence_log, validate_ndb_access,
};

use crate::state::PanadigaBackendState;

use super::error::{IpcResult, PanadigaIpcError};
use super::hardware::run_blocking;
use super::hardware_diag::require_destructive_nonce;
use super::ssot_studio::resolve_workspace_with_marker;

static SILICON_UNDO_HISTORY: OnceLock<Mutex<Vec<VolatilePatchOp>>> = OnceLock::new();

fn undo_history_mutex() -> &'static Mutex<Vec<VolatilePatchOp>> {
    SILICON_UNDO_HISTORY.get_or_init(|| Mutex::new(Vec::new()))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiliconBarrierCheckDto {
    pub allowed: bool,
    pub normalized_addr_hex: String,
    pub reason: String,
}

fn explorer_workspace_root() -> PathBuf {
    resolve_workspace_with_marker("docs/evidence")
}

fn resolve_evidence_log_path(root: &Path, log_kind: &str) -> IpcResult<PathBuf> {
    let clean = log_kind.trim();
    let rel = match clean {
        "volatile_patch" => "docs/evidence/com10_live_volatile_patch_20261124.log",
        "video_osd_diag" => "docs/evidence/com10_live_video_osd_diag_20261124.log",
        other => {
            if other.is_empty()
                || other.contains("..")
                || (!other.starts_with("docs/") && !other.starts_with("docs\\"))
            {
                return Err(PanadigaIpcError::validation(
                    "Invalid evidence log identifier; allowed keys: volatile_patch, video_osd_diag, or docs/*.log relative path.",
                ));
            }
            other
        }
    };
    Ok(root.join(rel))
}

fn decode_patch_hex_bytes(hex_raw: &str, width: &str) -> IpcResult<(VolatilePatchOp, u32)> {
    let clean: String = hex_raw
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .collect();
    if clean.is_empty()
        || !clean.len().is_multiple_of(2)
        || !clean.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(PanadigaIpcError::validation(
            "Invalid volatile hex patch payload: expected even hex byte string (e.g. '40F0' or '40 80').",
        ));
    }
    let bytes = hex::decode(&clean)
        .map_err(|e| PanadigaIpcError::validation(format!("Hex decode failed: {e}")))?;
    let len = bytes.len() as u32;
    let dummy_op = if width.eq_ignore_ascii_case("u16") && bytes.len() == 2 {
        VolatilePatchOp::write_u16(0, u16::from_le_bytes([bytes[0], bytes[1]]))
    } else {
        VolatilePatchOp::write_bytes(0, bytes)
    };
    Ok((dummy_op, len))
}

/// Deterministically replays a saved COM10 `ndb>` evidence capture (`volatile_patch` or `video_osd_diag`).
#[tauri::command]
pub fn silicon_explorer_replay_log(log_kind: String) -> IpcResult<NdbLogReplaySummary> {
    let root = explorer_workspace_root();
    let path = resolve_evidence_log_path(&root, &log_kind)?;
    let raw_bytes = std::fs::read(&path).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Failed reading evidence log '{}': {e}",
            path.display()
        ))
    })?;
    let raw_text = decode_captured_log_bytes(&raw_bytes);
    let source_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("evidence.log");
    replay_ndb_evidence_log(&raw_text, source_name)
        .map_err(|e| PanadigaIpcError::validation(e.to_string()))
}

/// Validates read/write intent against `0xC000100C`, `0xDC0F0000..0xDC0F0FFF`, BCU `0xD4004000`, and `CS0` NOR.
#[tauri::command]
pub fn silicon_explorer_validate_address(
    addr_hex: String,
    byte_len: usize,
    access_kind: String,
) -> IpcResult<SiliconBarrierCheckDto> {
    let addr = parse_u32_radix(&addr_hex)
        .map_err(|e| PanadigaIpcError::validation(format!("Invalid address '{addr_hex}': {e}")))?;
    let kind = if access_kind.eq_ignore_ascii_case("write") {
        NdbAccessKind::Write
    } else {
        NdbAccessKind::Read
    };
    let normalized = format!("0x{addr:08X}");
    match validate_ndb_access(addr, byte_len.max(1), kind) {
        Ok(()) => Ok(SiliconBarrierCheckDto {
            allowed: true,
            normalized_addr_hex: normalized.clone(),
            reason: format!(
                "SAFE: {kind:?} access permitted at {normalized} (+{} B).",
                byte_len.max(1)
            ),
        }),
        Err(err) => Ok(SiliconBarrierCheckDto {
            allowed: false,
            normalized_addr_hex: normalized,
            reason: err.to_string(),
        }),
    }
}

/// Computes an exact LE byte-level hot-diff between two raw `ndb> dd` captures.
#[tauri::command]
pub fn silicon_explorer_diff_snapshots(
    before_raw: String,
    after_raw: String,
) -> IpcResult<Vec<VolatileDiffEntry>> {
    let before = parse_ndb_dd_output(&before_raw)
        .map_err(|e| PanadigaIpcError::validation(format!("Invalid before snapshot: {e}")))?;
    let after = parse_ndb_dd_output(&after_raw)
        .map_err(|e| PanadigaIpcError::validation(format!("Invalid after snapshot: {e}")))?;
    diff_dd_snapshots(&before, &after).map_err(|e| PanadigaIpcError::validation(e.to_string()))
}

fn with_live_ndb_session<T, F>(port_name: &str, op: F) -> IpcResult<T>
where
    F: FnOnce(&mut NdbSession<SerialNdbTransport>) -> anyhow::Result<T>,
{
    let transport = SerialNdbTransport::connect_and_enter_bridge(port_name)
        .map_err(|e| PanadigaIpcError::serial(e.to_string()))?;
    let mut session = NdbSession::new(transport);
    let result = op(&mut session).map_err(|e| PanadigaIpcError::serial(e.to_string()));
    let mut tr = session.into_transport();
    let _ = tr.close_safely();
    result
}

/// Captures a read-only `ndb> dd` word dump from live AM34 hardware over `PicoFW` `BRIDGE` (obeying `STANDBY_GATE_STOP`).
#[tauri::command]
pub async fn silicon_explorer_live_snapshot(
    port_name: String,
    addr_hex: String,
    byte_len: usize,
) -> IpcResult<NdbDdResult> {
    let addr = parse_u32_radix(&addr_hex).map_err(|e| {
        PanadigaIpcError::validation(format!("Invalid hex address '{addr_hex}': {e}"))
    })?;
    let eff_len = byte_len.clamp(4, 256);
    validate_ndb_access(addr, eff_len, NdbAccessKind::Read)
        .map_err(|e| PanadigaIpcError::validation(e.to_string()))?;

    run_blocking(move || with_live_ndb_session(&port_name, |s| s.read_words(addr, eff_len))).await
}

/// Nonce-gated volatile SDRAM/MMIO patch with pre/post `dd` verification and reverse-order undo journaling.
#[tauri::command]
pub async fn silicon_explorer_apply_volatile_patch(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    addr_hex: String,
    patch_hex: String,
    width: String,
    confirm_volatile: bool,
    nonce_id: Option<String>,
) -> IpcResult<VolatileDiffReport> {
    if !confirm_volatile {
        return Err(PanadigaIpcError::validation(
            "Explicit volatile write confirmation checkbox is required before touching AM34 SDRAM/MMIO.",
        ));
    }
    let addr = parse_u32_radix(&addr_hex).map_err(|e| {
        PanadigaIpcError::validation(format!("Invalid hex address '{addr_hex}': {e}"))
    })?;
    let (template_op, byte_len) = decode_patch_hex_bytes(&patch_hex, &width)?;
    validate_ndb_access(addr, byte_len as usize, NdbAccessKind::Write)
        .map_err(|e| PanadigaIpcError::validation(e.to_string()))?;

    let target_label = format!("0x{addr:08X}");
    require_destructive_nonce(
        state.inner(),
        nonce_id.as_deref(),
        OpKind::RamProbe,
        &target_label,
        &patch_hex,
    )?;

    let op = VolatilePatchOp {
        addr,
        width: template_op.width,
        bytes: template_op.bytes,
    };

    run_blocking(move || {
        let report = with_live_ndb_session(&port_name, |s| {
            s.apply_patches_with_diff(std::slice::from_ref(&op))
        })?;

        if let Ok(mut guard) = undo_history_mutex().lock() {
            let before_bytes: Vec<u8> = report
                .before_snapshots
                .first()
                .map(|snap| {
                    let aligned_base = addr & !3u32;
                    let off = (addr - aligned_base) as usize;
                    snap.bytes
                        .get(off..off + op.bytes.len())
                        .unwrap_or(&[])
                        .to_vec()
                })
                .unwrap_or_default();
            if before_bytes.len() == op.bytes.len() {
                guard.push(VolatilePatchOp {
                    addr,
                    width: op.width,
                    bytes: before_bytes,
                });
            }
        }
        Ok(report)
    })
    .await
}

/// Nonce-gated reverse-order rollback of volatile SDRAM/MMIO mutations recorded in the current session.
#[tauri::command]
pub async fn silicon_explorer_rollback(
    state: tauri::State<'_, PanadigaBackendState>,
    port_name: String,
    nonce_id: Option<String>,
) -> IpcResult<RollbackReport> {
    let pending_ops: Vec<VolatilePatchOp> = undo_history_mutex()
        .lock()
        .map_err(|_| PanadigaIpcError::internal("Rollback history mutex poisoned"))?
        .clone();
    if pending_ops.is_empty() {
        return Err(PanadigaIpcError::validation(
            "No live volatile SDRAM/MMIO patches recorded to rollback in this session.",
        ));
    }
    require_destructive_nonce(
        state.inner(),
        nonce_id.as_deref(),
        OpKind::RamProbe,
        "ndb_rollback",
        "reverse_undo",
    )?;

    run_blocking(move || {
        let reverse_ops: Vec<VolatilePatchOp> = pending_ops.iter().rev().cloned().collect();
        let diff = with_live_ndb_session(&port_name, |s| s.apply_patches_with_diff(&reverse_ops))?;
        let restored_bytes = reverse_ops.iter().map(|op| op.bytes.len()).sum();
        if let Ok(mut guard) = undo_history_mutex().lock() {
            guard.clear();
        }
        Ok(RollbackReport {
            restored_ops: reverse_ops.len(),
            restored_bytes,
            verified_clean: !diff.after_snapshots.is_empty(),
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{silicon_explorer_replay_log, silicon_explorer_validate_address};

    #[test]
    fn silicon_explorer_replays_live_logs_and_blocks_dangerous_regions() {
        let vol = silicon_explorer_replay_log("volatile_patch".to_string()).expect("volatile");
        assert!(vol.main3060np_osd_caveat.is_some());
        assert!(!vol.dd_reads.is_empty());

        let diag = silicon_explorer_replay_log("video_osd_diag".to_string()).expect("diag");
        assert!(diag.bcu_fault.is_some());
        assert!(diag.standby_gate_tripped);

        for (addr, kind) in [
            ("0xC000100C", "read"),
            ("0xDC0F0000", "write"),
            ("0xD4004000", "read"),
            ("0x80000000", "write"),
            ("0x84000000", "write"),
        ] {
            let chk = silicon_explorer_validate_address(addr.into(), 4, kind.into()).expect("chk");
            assert!(!chk.allowed, "must block {kind} at {addr}");
        }
    }
}
