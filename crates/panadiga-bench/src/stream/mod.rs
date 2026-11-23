#![deny(unsafe_code)]

//! Unidirectional Snapshot + Throttled Telemetry/Progress/Log Stream contracts (`Plano B`).

pub mod publisher;
pub mod types;

#[cfg(test)]
mod tests;

pub use panadiga_constants::events::EVENT_BENCH_STREAM as BENCH_EVENT_CHANNEL;
pub use publisher::BenchPublisher;
pub use types::{
    BenchEvent, BenchEventEnvelope, BenchEventSink, BenchSnapshot, BenchState, IncompleteOpNotice,
    JournalAlertKind, LOG_BATCH_MAX_BYTES, LOG_BATCH_MAX_LINES, LOG_BUFFER_CAP_LINES,
    LOG_FLUSH_INTERVAL_MS, LogLine, LogStream, NoopBenchSink, OpKind, OpProgressSample,
    PROGRESS_FLUSH_INTERVAL_MS, TELEMETRY_FLUSH_INTERVAL_MS, TelemetrySample,
};
