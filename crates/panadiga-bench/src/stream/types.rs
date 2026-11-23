#![deny(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

pub const TELEMETRY_FLUSH_INTERVAL_MS: u64 = 50;
pub const PROGRESS_FLUSH_INTERVAL_MS: u64 = 100;
pub const LOG_FLUSH_INTERVAL_MS: u64 = 50;
pub const LOG_BATCH_MAX_LINES: usize = 128;
pub const LOG_BATCH_MAX_BYTES: usize = 32 * 1024;
pub const LOG_BUFFER_CAP_LINES: usize = 512;

/// Latest consistent bench state pulled by UI on mount/reconnect (`get_bench_snapshot`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BenchSnapshot {
    /// Latest sequence number emitted; lets UI detect sequence gaps after reconnect.
    pub seq: u64,
    pub ssot_contract_hash: String,
    pub state: BenchState,
    pub latest_telemetry: Option<TelemetrySample>,
    pub active_progress: Option<OpProgressSample>,
    /// Populated when the audit journal detected `OpBegin` without matching `OpEnd`.
    pub incomplete_op: Option<IncompleteOpNotice>,
}

/// Hardware bench finite-state machine reported over `BenchSnapshot` and `BenchEvent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BenchState {
    Disconnected,
    Idle {
        port: String,
        fw_contract_hash: String,
    },
    Busy {
        op_id: String,
        op_kind: OpKind,
    },
    Fault {
        code: String,
        message: String,
    },
}

/// Closed set of audited physical/storage operations tracked across stream and journal.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    #[default]
    NorDump,
    NorWrite,
    SdKitBuild,
    Boottrace,
    RamProbe,
}

impl fmt::Display for OpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::NorDump => "nor_dump",
            Self::NorWrite => "nor_write",
            Self::SdKitBuild => "sd_kit_build",
            Self::Boottrace => "boottrace",
            Self::RamProbe => "ram_probe",
        };
        f.write_str(label)
    }
}

/// Instantaneous P7501 electrical and UART counter telemetry sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TelemetrySample {
    pub ts_ms: u64,
    pub vdd_mv: u16,
    pub nsw5v_high: bool,
    pub jig_asserted: bool,
    pub uart_rx_bytes: u64,
}

/// Bounded progress update for a running bench operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OpProgressSample {
    pub op_id: String,
    pub phase: String,
    pub done_units: u64,
    pub total_units: u64,
}

/// Origin stream classification for batched console output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    Uart0,
    PicoDiag,
    HostCore,
}

/// Single timestamped line captured from UART0, Pico diagnostics, or host core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LogLine {
    pub ts_ms: u64,
    pub text: String,
}

/// Monotonically sequenced envelope emitted on `BENCH_EVENT_CHANNEL`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BenchEventEnvelope {
    pub seq: u64,
    pub ts_ms: u64,
    pub payload: BenchEvent,
}

/// Closed unidirectional event payload emitted from Rust runtime to UI/subscribers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BenchEvent {
    StateChanged {
        state: BenchState,
    },
    Telemetry(TelemetrySample),
    OpProgress(OpProgressSample),
    LogBatch {
        stream: LogStream,
        lines: Vec<LogLine>,
        /// Number of lines discarded by backpressure since the previous batch.
        dropped_lines: u64,
    },
    JournalAlert {
        alert: JournalAlertKind,
    },
}

/// Forensic journal recovery notices surfaced immediately via stream and snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JournalAlertKind {
    TruncatedTailQuarantined { backup_file: String, bytes: usize },
    IncompleteOpDetected(IncompleteOpNotice),
}

/// Metadata identifying an `OpBegin` entry missing its corresponding `OpEnd`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IncompleteOpNotice {
    pub op_id: String,
    pub op_kind: OpKind,
    pub target: String,
    pub started_ts_ms: u64,
}

/// Sink abstraction decoupling runtime publishing from Tauri, CLI, and MCP transports.
pub trait BenchEventSink: Send + Sync {
    /// Delivers a sequenced [`BenchEventEnvelope`] to downstream observers.
    fn emit(&self, env: &BenchEventEnvelope);
}

/// Zero-op sink used when no live stream consumer is attached.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopBenchSink;

impl BenchEventSink for NoopBenchSink {
    fn emit(&self, _env: &BenchEventEnvelope) {}
}
