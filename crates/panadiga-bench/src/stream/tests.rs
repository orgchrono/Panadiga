use std::sync::{Arc, Mutex};

use super::*;

#[derive(Clone, Default)]
struct VecSink(Arc<Mutex<Vec<BenchEventEnvelope>>>);

impl VecSink {
    fn drained(&self) -> Vec<BenchEventEnvelope> {
        self.0.lock().unwrap().clone()
    }
}

impl BenchEventSink for VecSink {
    fn emit(&self, env: &BenchEventEnvelope) {
        self.0.lock().unwrap().push(env.clone());
    }
}

#[test]
fn serde_roundtrip_and_publisher_backpressure_work_deterministically() {
    let sink = VecSink::default();
    let mut pub_ = BenchPublisher::new("A6F75ECE", sink.clone());

    pub_.set_state(
        BenchState::Idle {
            port: "COM10".into(),
            fw_contract_hash: "A6F75ECE".into(),
        },
        100,
    );
    pub_.emit_journal_alert(
        JournalAlertKind::TruncatedTailQuarantined {
            backup_file: "ops.jsonl.corrupt.100".into(),
            bytes: 19,
        },
        101,
    );

    for (ts, mv) in [(110, 3250), (120, 3260), (140, 3290), (160, 3300)] {
        pub_.push_telemetry(
            TelemetrySample {
                ts_ms: ts,
                vdd_mv: mv,
                nsw5v_high: true,
                jig_asserted: false,
                uart_rx_bytes: ts,
            },
            ts,
        );
    }

    pub_.push_progress(
        OpProgressSample {
            op_id: "op_1".into(),
            phase: "erase".into(),
            done_units: 1,
            total_units: 45,
        },
        200,
    );
    pub_.push_progress(
        OpProgressSample {
            op_id: "op_1".into(),
            phase: "erase".into(),
            done_units: 2,
            total_units: 45,
        },
        220,
    );
    pub_.push_progress(
        OpProgressSample {
            op_id: "op_1".into(),
            phase: "verify".into(),
            done_units: 45,
            total_units: 45,
        },
        230,
    );

    for i in 0..700 {
        pub_.push_log(
            LogStream::Uart0,
            LogLine {
                ts_ms: 300,
                text: format!("line-{i}"),
            },
            300,
        );
    }
    pub_.flush(360);

    let snap = pub_.snapshot();
    let events = sink.drained();
    assert_eq!(snap.seq, events.last().unwrap().seq);
    assert_eq!(snap.latest_telemetry.unwrap().vdd_mv, 3300);

    let telemetry_mvs: Vec<u16> = events
        .iter()
        .filter_map(|e| match &e.payload {
            BenchEvent::Telemetry(t) => Some(t.vdd_mv),
            _ => None,
        })
        .collect();
    assert_eq!(telemetry_mvs, vec![3250, 3300]);

    let dropped_sum: u64 = events
        .iter()
        .filter_map(|e| match &e.payload {
            BenchEvent::LogBatch { dropped_lines, .. } => Some(*dropped_lines),
            _ => None,
        })
        .sum();
    assert_eq!(dropped_sum, 700 - 128 - 512);

    for env in &events {
        let encoded = serde_json::to_string(env).unwrap();
        let decoded: BenchEventEnvelope = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, *env);
    }
}
