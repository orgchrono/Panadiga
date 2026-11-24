#![deny(unsafe_code)]

//! Tauri `Plano B` snapshot/stream sink wiring and ephemeral operation nonce issuance/consumption.

use std::sync::Arc;

use panadiga_core::bench::{
    BenchEventEnvelope, BenchEventSink, BenchSnapshot, OpKind, OperationNonceChallenge,
};
use serde::Deserialize;
use tauri::Emitter;

use crate::commands::constants::EVENT_BENCH_STREAM;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::state::{PanadigaBackendState, epoch_ms_now};

/// Live Tauri emitter forwarding `BenchEventEnvelope` updates to `"panadiga://bench-event"`.
pub struct TauriBenchSink {
    app: tauri::AppHandle,
}

impl BenchEventSink for TauriBenchSink {
    fn emit(&self, env: &BenchEventEnvelope) {
        let _ = self.app.emit(EVENT_BENCH_STREAM, env);
    }
}

/// Attaches the Tauri `"panadiga://bench-event"` sink to backend stream state at startup.
pub fn attach_bench_stream_sink(app: tauri::AppHandle, backend: &PanadigaBackendState) {
    backend
        .bench_stream
        .attach_sink(Arc::new(TauriBenchSink { app }));
}

/// Request DTO for minting a single-use short-lived authorization nonce bound to operation params.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestOperationNonceDto {
    pub op_kind: OpKind,
    pub target: String,
    pub input_sha256: String,
}

/// Issues a single-use ephemeral nonce challenge against managed backend state.
pub fn issue_operation_nonce_with_state(
    state: &PanadigaBackendState,
    request: &RequestOperationNonceDto,
) -> IpcResult<OperationNonceChallenge> {
    let target = request.target.trim();
    if target.is_empty() {
        return Err(PanadigaIpcError::validation(
            "target obrigatório para emissão de nonce efêmero.",
        ));
    }
    let input_ref = request.input_sha256.trim();
    if input_ref.is_empty() {
        return Err(PanadigaIpcError::validation(
            "inputSha256 obrigatório para emissão de nonce efêmero.",
        ));
    }
    let now_ms = epoch_ms_now();
    Ok(state
        .nonces
        .with_lock(|store| store.issue(request.op_kind, target, input_ref, now_ms)))
}

/// Verifies and atomically consumes a single-use ephemeral nonce matching operation parameters.
pub fn consume_operation_nonce_with_state(
    state: &PanadigaBackendState,
    nonce_id: Option<&str>,
    op_kind: OpKind,
    target: &str,
    input_ref: &str,
    required: bool,
) -> IpcResult<Option<OperationNonceChallenge>> {
    let Some(clean_nonce) = nonce_id.map(str::trim).filter(|s| !s.is_empty()) else {
        if required {
            return Err(PanadigaIpcError::validation(
                "nonceId obrigatório: solicite request_operation_nonce para autorizar escrita destrutiva.",
            ));
        }
        return Ok(None);
    };
    let now_ms = epoch_ms_now();
    state
        .nonces
        .with_lock(|store| store.consume(clean_nonce, op_kind, target, input_ref, now_ms))
        .map(Some)
        .map_err(|e| PanadigaIpcError::validation(format!("Nonce efêmero inválido: {e}")))
}

/// Enforces mandatory single-use nonce authorization on every destructive Tauri IPC command.
pub fn require_destructive_nonce(
    state: &PanadigaBackendState,
    nonce_id: Option<&str>,
    op_kind: OpKind,
    target: &str,
    input_ref: &str,
) -> IpcResult<()> {
    consume_operation_nonce_with_state(state, nonce_id, op_kind, target, input_ref, true)
        .map(|_| ())
}

/// Issues a single-use WebView confirmation nonce bound to `(opKind, target, inputSha256)`.
#[tauri::command]
pub fn request_operation_nonce(
    state: tauri::State<'_, PanadigaBackendState>,
    request: RequestOperationNonceDto,
) -> IpcResult<OperationNonceChallenge> {
    issue_operation_nonce_with_state(state.inner(), &request)
}

/// Pulls the consistent `Plano B` hardware bench state snapshot (`get_bench_snapshot`).
#[tauri::command]
pub fn get_bench_snapshot(
    state: tauri::State<'_, PanadigaBackendState>,
) -> IpcResult<BenchSnapshot> {
    Ok(state.inner().bench_stream.snapshot())
}
