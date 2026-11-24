#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

use panadiga_bench::{
    BenchEvent, BenchEventEnvelope, BenchEventSink, BenchPublisher, BenchState,
    DEFAULT_NONCE_TTL_MS, LOG_BUFFER_CAP_LINES, LogLine, LogStream, OpKind, OperationNonceStore,
};

#[derive(Clone, Default)]
struct RecordingSink(Arc<Mutex<Vec<BenchEventEnvelope>>>);

impl BenchEventSink for RecordingSink {
    fn emit(&self, env: &BenchEventEnvelope) {
        self.0.lock().expect("lock sink").push(env.clone());
    }
}

#[test]
fn nonce_store_enforces_expiry_binding_and_single_use_replay_lockout() {
    let mut store = OperationNonceStore::with_contract_hash("DEADBEEF");

    let n_op = store.issue(OpKind::NorWrite, "SA26..SA70", "abc123", 10_000);
    assert!(
        store
            .consume(
                &n_op.nonce_id,
                OpKind::NorDump,
                "SA26..SA70",
                "abc123",
                10_100
            )
            .is_err()
    );
    // Mismatched consume attempt must burn the nonce immediately.
    assert!(
        store
            .consume(
                &n_op.nonce_id,
                OpKind::NorWrite,
                "SA26..SA70",
                "abc123",
                10_101
            )
            .is_err()
    );

    let n_target = store.issue(OpKind::NorWrite, "SA26..SA70", "abc123", 10_200);
    assert!(
        store
            .consume(
                &n_target.nonce_id,
                OpKind::NorWrite,
                "SA27..SA70",
                "abc123",
                10_250
            )
            .is_err()
    );

    let n_hash = store.issue(OpKind::NorWrite, "SA26..SA70", "abc123", 10_300);
    assert!(
        store
            .consume(
                &n_hash.nonce_id,
                OpKind::NorWrite,
                "SA26..SA70",
                "def456",
                10_350
            )
            .is_err()
    );

    let expired = store.issue(OpKind::Boottrace, "COM10", "fff000", 20_000);
    assert!(
        store
            .consume(
                &expired.nonce_id,
                OpKind::Boottrace,
                "COM10",
                "fff000",
                20_000 + DEFAULT_NONCE_TTL_MS + 1,
            )
            .is_err()
    );

    let live = store.issue(OpKind::NorWrite, "SA26..SA70", "abc123", 30_000);
    let consumed = store
        .consume(
            &live.nonce_id,
            OpKind::NorWrite,
            "SA26..SA70",
            "abc123",
            30_500,
        )
        .expect("matching live challenge consumes once");
    assert_eq!(consumed.ssot_contract_hash, "DEADBEEF");

    assert!(
        store
            .consume(
                &live.nonce_id,
                OpKind::NorWrite,
                "SA26..SA70",
                "abc123",
                30_501
            )
            .is_err()
    );
}

#[test]
fn publisher_caps_burst_logs_at_512_lines_and_tracks_snapshot_state() {
    let sink = RecordingSink::default();
    let mut publisher = BenchPublisher::new("DEADBEEF", sink.clone());

    publisher.set_state(
        BenchState::Busy {
            op_id: "op_9".into(),
            op_kind: OpKind::RamProbe,
        },
        100,
    );
    assert!(matches!(
        publisher.snapshot().state,
        BenchState::Busy {
            op_kind: OpKind::RamProbe,
            ..
        }
    ));

    // Seed last_flush_ms for Uart0 so subsequent lines within < 50ms accumulate to capacity.
    publisher.push_log(
        LogStream::Uart0,
        LogLine {
            ts_ms: 100,
            text: "seed".into(),
        },
        100,
    );
    publisher.flush(100);

    for i in 0..(LOG_BUFFER_CAP_LINES + 25) {
        publisher.push_log(
            LogStream::Uart0,
            LogLine {
                ts_ms: 110,
                text: format!("line-{i}"),
            },
            110,
        );
    }
    publisher.flush(120);

    let events = sink.0.lock().expect("lock events").clone();
    let dropped_total: u64 = events
        .iter()
        .filter_map(|env| match &env.payload {
            BenchEvent::LogBatch { dropped_lines, .. } => Some(*dropped_lines),
            _ => None,
        })
        .sum();
    assert_eq!(dropped_total, 25);
}
