#![forbid(unsafe_code)]

//! Panadiga L1 Hardware Bench Telemetry Stream, Snapshot & Ephemeral Operation Nonces.

pub mod nonce;
pub mod stream;

pub use nonce::{DEFAULT_NONCE_TTL_MS, OperationNonceChallenge, OperationNonceStore};
pub use stream::{
    BENCH_EVENT_CHANNEL, BenchEvent, BenchEventEnvelope, BenchEventSink, BenchPublisher,
    BenchSnapshot, BenchState, IncompleteOpNotice, JournalAlertKind, LOG_BATCH_MAX_BYTES,
    LOG_BATCH_MAX_LINES, LOG_BUFFER_CAP_LINES, LOG_FLUSH_INTERVAL_MS, LogLine, LogStream,
    NoopBenchSink, OpKind, OpProgressSample, PROGRESS_FLUSH_INTERVAL_MS,
    TELEMETRY_FLUSH_INTERVAL_MS, TelemetrySample,
};
