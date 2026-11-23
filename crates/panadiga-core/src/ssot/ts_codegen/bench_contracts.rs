#![deny(unsafe_code)]

//! Deterministic TypeScript Bench Stream, Nonce, and Journal contract emitter.

use std::fmt::Write as _;

use crate::bench::nonce::DEFAULT_NONCE_TTL_MS;
use crate::bench::stream::{
    LOG_BATCH_MAX_BYTES, LOG_BATCH_MAX_LINES, LOG_BUFFER_CAP_LINES, LOG_FLUSH_INTERVAL_MS,
    PROGRESS_FLUSH_INTERVAL_MS, TELEMETRY_FLUSH_INTERVAL_MS,
};

pub(super) fn append_bench_stream_contracts(out: &mut String) {
    out.push_str(
        "export interface SsotBenchStreamLimits {\n\
         \x20 readonly telemetryFlushIntervalMs: number;\n\
         \x20 readonly progressFlushIntervalMs: number;\n\
         \x20 readonly logFlushIntervalMs: number;\n\
         \x20 readonly logBatchMaxLines: number;\n\
         \x20 readonly logBatchMaxBytes: number;\n\
         \x20 readonly logBufferCapLines: number;\n\
         \x20 readonly defaultNonceTtlMs: number;\n\
         }\n\n\
         export const SSOT_BENCH_STREAM_LIMITS: SsotBenchStreamLimits = {\n",
    );
    let _ = writeln!(
        out,
        "  telemetryFlushIntervalMs: {TELEMETRY_FLUSH_INTERVAL_MS},"
    );
    let _ = writeln!(
        out,
        "  progressFlushIntervalMs: {PROGRESS_FLUSH_INTERVAL_MS},"
    );
    let _ = writeln!(out, "  logFlushIntervalMs: {LOG_FLUSH_INTERVAL_MS},");
    let _ = writeln!(out, "  logBatchMaxLines: {LOG_BATCH_MAX_LINES},");
    let _ = writeln!(out, "  logBatchMaxBytes: {LOG_BATCH_MAX_BYTES},");
    let _ = writeln!(out, "  logBufferCapLines: {LOG_BUFFER_CAP_LINES},");
    let _ = writeln!(out, "  defaultNonceTtlMs: {DEFAULT_NONCE_TTL_MS},");
    out.push_str(
        "} as const;\n\n\
         export type BenchOpKind =\n\
         \x20 | 'nor_dump'\n\
         \x20 | 'nor_write'\n\
         \x20 | 'sd_kit_build'\n\
         \x20 | 'boottrace'\n\
         \x20 | 'ram_probe';\n\n\
         export interface OperationNonceChallenge {\n\
         \x20 readonly nonce_id: string;\n\
         \x20 readonly op_kind: BenchOpKind;\n\
         \x20 readonly target: string;\n\
         \x20 readonly input_sha256: string;\n\
         \x20 readonly ssot_contract_hash: string;\n\
         \x20 readonly issued_at_ms: number;\n\
         \x20 readonly expires_at_ms: number;\n\
         }\n\n\
         export type BenchState =\n\
         \x20 | { readonly kind: 'disconnected' }\n\
         \x20 | { readonly kind: 'idle'; readonly port: string; readonly fw_contract_hash: string }\n\
         \x20 | { readonly kind: 'busy'; readonly op_id: string; readonly op_kind: BenchOpKind }\n\
         \x20 | { readonly kind: 'fault'; readonly code: string; readonly message: string };\n\n\
         export interface TelemetrySample {\n\
         \x20 readonly ts_ms: number;\n\
         \x20 readonly vdd_mv: number;\n\
         \x20 readonly nsw5v_high: boolean;\n\
         \x20 readonly jig_asserted: boolean;\n\
         \x20 readonly uart_rx_bytes: number;\n\
         }\n\n\
         export interface OpProgressSample {\n\
         \x20 readonly op_id: string;\n\
         \x20 readonly phase: string;\n\
         \x20 readonly done_units: number;\n\
         \x20 readonly total_units: number;\n\
         }\n\n\
         export type LogStream = 'uart0' | 'pico_diag' | 'host_core';\n\n\
         export interface LogLine {\n\
         \x20 readonly ts_ms: number;\n\
         \x20 readonly text: string;\n\
         }\n\n\
         export interface IncompleteOpNotice {\n\
         \x20 readonly op_id: string;\n\
         \x20 readonly op_kind: BenchOpKind;\n\
         \x20 readonly target: string;\n\
         \x20 readonly started_ts_ms: number;\n\
         }\n\n\
         export type JournalAlertKind =\n\
         \x20 | { readonly kind: 'truncated_tail_quarantined'; readonly backup_file: string; readonly bytes: number }\n\
         \x20 | ({ readonly kind: 'incomplete_op_detected' } & IncompleteOpNotice);\n\n\
         export type BenchEvent =\n\
         \x20 | { readonly kind: 'state_changed'; readonly state: BenchState }\n\
         \x20 | ({ readonly kind: 'telemetry' } & TelemetrySample)\n\
         \x20 | ({ readonly kind: 'op_progress' } & OpProgressSample)\n\
         \x20 | { readonly kind: 'log_batch'; readonly stream: LogStream; readonly lines: readonly LogLine[]; readonly dropped_lines: number }\n\
         \x20 | { readonly kind: 'journal_alert'; readonly alert: JournalAlertKind };\n\n\
         export interface BenchEventEnvelope {\n\
         \x20 readonly seq: number;\n\
         \x20 readonly ts_ms: number;\n\
         \x20 readonly payload: BenchEvent;\n\
         }\n\n\
         export interface BenchSnapshot {\n\
         \x20 readonly seq: number;\n\
         \x20 readonly ssot_contract_hash: string;\n\
         \x20 readonly state: BenchState;\n\
         \x20 readonly latest_telemetry: TelemetrySample | null;\n\
         \x20 readonly active_progress: OpProgressSample | null;\n\
         \x20 readonly incomplete_op: IncompleteOpNotice | null;\n\
         }\n",
    );
}
