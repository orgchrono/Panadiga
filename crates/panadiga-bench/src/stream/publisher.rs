#![deny(unsafe_code)]

use std::collections::BTreeMap;

use super::types::{
    BenchEvent, BenchEventEnvelope, BenchEventSink, BenchSnapshot, BenchState, IncompleteOpNotice,
    JournalAlertKind, LOG_BATCH_MAX_BYTES, LOG_BATCH_MAX_LINES, LOG_BUFFER_CAP_LINES,
    LOG_FLUSH_INTERVAL_MS, LogLine, LogStream, OpProgressSample, PROGRESS_FLUSH_INTERVAL_MS,
    TELEMETRY_FLUSH_INTERVAL_MS, TelemetrySample,
};

#[derive(Debug, Default)]
struct LogQueueState {
    lines: Vec<LogLine>,
    total_bytes: usize,
    dropped_lines: u64,
    last_flush_ms: Option<u64>,
}

/// Single publisher enforcing coalescing (20 Hz telemetry, 10 Hz + milestone progress)
/// and bounded batching (`50ms` / `128` lines / `32 KiB`, cap `512` lines).
pub struct BenchPublisher<S: BenchEventSink> {
    sink: S,
    seq: u64,
    ssot_contract_hash: String,
    state: BenchState,
    latest_telemetry: Option<TelemetrySample>,
    pending_telemetry: Option<TelemetrySample>,
    last_telemetry_flush_ms: Option<u64>,
    active_progress: Option<OpProgressSample>,
    pending_progress: Option<OpProgressSample>,
    last_progress_flush_ms: Option<u64>,
    incomplete_op: Option<IncompleteOpNotice>,
    log_queues: BTreeMap<LogStream, LogQueueState>,
}

impl<S: BenchEventSink> BenchPublisher<S> {
    /// Creates a new disconnected publisher bound to `ssot_contract_hash` and `sink`.
    pub fn new(ssot_contract_hash: impl Into<String>, sink: S) -> Self {
        Self {
            sink,
            seq: 0,
            ssot_contract_hash: ssot_contract_hash.into(),
            state: BenchState::Disconnected,
            latest_telemetry: None,
            pending_telemetry: None,
            last_telemetry_flush_ms: None,
            active_progress: None,
            pending_progress: None,
            last_progress_flush_ms: None,
            incomplete_op: None,
            log_queues: BTreeMap::new(),
        }
    }

    /// Replaces the sink while preserving current sequence numbers and snapshot state.
    pub fn set_sink(&mut self, sink: S) {
        self.sink = sink;
    }

    /// Returns a consistent pull snapshot representing all events up to `seq`.
    #[must_use]
    pub fn snapshot(&self) -> BenchSnapshot {
        BenchSnapshot {
            seq: self.seq,
            ssot_contract_hash: self.ssot_contract_hash.clone(),
            state: self.state.clone(),
            latest_telemetry: self.latest_telemetry.clone(),
            active_progress: self.active_progress.clone(),
            incomplete_op: self.incomplete_op.clone(),
        }
    }

    fn emit_now(&mut self, now_ms: u64, payload: BenchEvent) {
        self.seq = self.seq.saturating_add(1);
        let env = BenchEventEnvelope {
            seq: self.seq,
            ts_ms: now_ms,
            payload,
        };
        self.sink.emit(&env);
    }

    /// Immediately updates bench FSM state and emits `BenchEvent::StateChanged`.
    pub fn set_state(&mut self, state: BenchState, now_ms: u64) {
        self.state = state.clone();
        self.emit_now(now_ms, BenchEvent::StateChanged { state });
    }

    /// Updates crash-recovery notice in the snapshot and emits immediately when present.
    pub fn set_incomplete_op(&mut self, notice: Option<IncompleteOpNotice>, now_ms: u64) {
        self.incomplete_op = notice.clone();
        if let Some(n) = notice {
            self.emit_now(
                now_ms,
                BenchEvent::JournalAlert {
                    alert: JournalAlertKind::IncompleteOpDetected(n),
                },
            );
        }
    }

    /// Immediately emits a [`JournalAlertKind`] without dropping or coalescing.
    pub fn emit_journal_alert(&mut self, alert: JournalAlertKind, now_ms: u64) {
        if let JournalAlertKind::IncompleteOpDetected(ref notice) = alert {
            self.incomplete_op = Some(notice.clone());
        }
        self.emit_now(now_ms, BenchEvent::JournalAlert { alert });
    }

    /// Records telemetry in the latest slot and flushes at up to 20 Hz (`50 ms`).
    pub fn push_telemetry(&mut self, sample: TelemetrySample, now_ms: u64) {
        self.latest_telemetry = Some(sample.clone());
        self.pending_telemetry = Some(sample);
        let due = self
            .last_telemetry_flush_ms
            .is_none_or(|prev| now_ms.saturating_sub(prev) >= TELEMETRY_FLUSH_INTERVAL_MS);
        if due {
            self.flush_telemetry(now_ms);
        }
    }

    /// Records progress; flushes immediately on phase change/completion, else at `100 ms`.
    pub fn push_progress(&mut self, progress: OpProgressSample, now_ms: u64) {
        let phase_changed = self
            .active_progress
            .as_ref()
            .is_some_and(|prev| prev.phase != progress.phase || prev.op_id != progress.op_id);
        let completed = progress.done_units >= progress.total_units;
        self.active_progress = Some(progress.clone());
        self.pending_progress = Some(progress);

        let due = phase_changed
            || completed
            || self
                .last_progress_flush_ms
                .is_none_or(|prev| now_ms.saturating_sub(prev) >= PROGRESS_FLUSH_INTERVAL_MS);
        if due {
            self.flush_progress(now_ms);
        }
    }

    /// Buffers a log line under the `512`-line cap and enforces `>= 50ms` burst backpressure.
    pub fn push_log(&mut self, stream: LogStream, line: LogLine, now_ms: u64) {
        let should_flush = {
            let queue = self.log_queues.entry(stream).or_default();
            if queue.lines.len() >= LOG_BUFFER_CAP_LINES {
                queue.dropped_lines = queue.dropped_lines.saturating_add(1);
            } else {
                queue.total_bytes = queue.total_bytes.saturating_add(line.text.len());
                queue.lines.push(line);
            }
            let timer_due = queue
                .last_flush_ms
                .is_some_and(|prev| now_ms.saturating_sub(prev) >= LOG_FLUSH_INTERVAL_MS);
            let initial_batch_ready = queue.last_flush_ms.is_none()
                && (queue.lines.len() >= LOG_BATCH_MAX_LINES
                    || queue.total_bytes >= LOG_BATCH_MAX_BYTES);
            timer_due || initial_batch_ready
        };
        if should_flush {
            self.flush_log_stream(stream, now_ms);
        }
    }

    /// Flushes all pending telemetry, progress, and buffered log batches immediately.
    pub fn flush(&mut self, now_ms: u64) {
        self.flush_telemetry(now_ms);
        self.flush_progress(now_ms);
        let streams: Vec<LogStream> = self.log_queues.keys().copied().collect();
        for stream in streams {
            while self
                .log_queues
                .get(&stream)
                .is_some_and(|q| !q.lines.is_empty() || q.dropped_lines > 0)
            {
                self.flush_log_stream(stream, now_ms);
            }
        }
    }

    fn flush_telemetry(&mut self, now_ms: u64) {
        if let Some(sample) = self.pending_telemetry.take() {
            self.last_telemetry_flush_ms = Some(now_ms);
            self.emit_now(now_ms, BenchEvent::Telemetry(sample));
        }
    }

    fn flush_progress(&mut self, now_ms: u64) {
        if let Some(progress) = self.pending_progress.take() {
            self.last_progress_flush_ms = Some(now_ms);
            self.emit_now(now_ms, BenchEvent::OpProgress(progress));
        }
    }

    fn flush_log_stream(&mut self, stream: LogStream, now_ms: u64) {
        let batch_opt = {
            let Some(queue) = self.log_queues.get_mut(&stream) else {
                return;
            };
            if queue.lines.is_empty() && queue.dropped_lines == 0 {
                return;
            }
            let take_n = queue.lines.len().min(LOG_BATCH_MAX_LINES);
            let mut batch_lines: Vec<LogLine> = queue.lines.drain(..take_n).collect();
            let mut byte_sum: usize = batch_lines.iter().map(|l| l.text.len()).sum();
            while batch_lines.len() > 1 && byte_sum > LOG_BATCH_MAX_BYTES {
                if let Some(popped) = batch_lines.pop() {
                    byte_sum = byte_sum.saturating_sub(popped.text.len());
                    queue.lines.insert(0, popped);
                }
            }
            queue.total_bytes = queue.lines.iter().map(|l| l.text.len()).sum();
            let dropped = queue.dropped_lines;
            queue.dropped_lines = 0;
            queue.last_flush_ms = Some(now_ms);
            Some((batch_lines, dropped))
        };
        if let Some((lines, dropped_lines)) = batch_opt {
            self.emit_now(
                now_ms,
                BenchEvent::LogBatch {
                    stream,
                    lines,
                    dropped_lines,
                },
            );
        }
    }
}
