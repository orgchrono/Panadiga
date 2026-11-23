use std::fs;
use std::io::Write;
use std::path::PathBuf;

use panadiga_bench::stream::{JournalAlertKind, OpKind};

use crate::lock::{VaultLock, VaultLockMode};

use super::{
    JOURNAL_FILENAME, JOURNAL_SUBDIR, Journal, JournalActor, JournalError, OpBeginParams, OpOutcome,
};

fn next_journal_test_dir(tag: &str) -> PathBuf {
    static JSEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = JSEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "panadiga_journal_{tag}_{}_{}",
        std::process::id(),
        seq
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn journal_chain_roundtrip_locking_eof_quarantine_and_crash_recovery() {
    let vault_dir = next_journal_test_dir("roundtrip");
    {
        let mut journal = Journal::open(&vault_dir, JournalActor::Cli, 1_000).unwrap();
        assert_eq!(journal.seq(), 0);

        assert!(matches!(
            Journal::open(&vault_dir, JournalActor::Desktop, 1_001),
            Err(JournalError::VaultLocked { .. })
        ));

        journal
            .append_op_begin(OpBeginParams {
                op_id: "op_1".into(),
                contract_hash: "A6F75ECE".into(),
                op_kind: OpKind::NorWrite,
                target: "SA26..SA70".into(),
                input_sha256: Some("abc123".into()),
                nonce_id: Some("n_01".into()),
                vdd_mv: Some(3280),
                nsw5v_high: Some(true),
                meta: Default::default(),
                ts_ms: 2_000,
            })
            .unwrap();
        journal
            .append_checkpoint("op_1", "A6F75ECE", "erase", 10, 45, false, 2_500)
            .unwrap();
        journal
            .append_op_end(
                "op_1",
                "A6F75ECE",
                OpOutcome::Ok,
                900,
                Some("def456".into()),
                None,
                None,
                2_900,
            )
            .unwrap();
        assert_eq!(journal.seq(), 3);
        assert!(journal.incomplete_ops().is_empty());
    }

    let s1 = VaultLock::try_acquire(&vault_dir, VaultLockMode::Shared, "r1").unwrap();
    let rep = Journal::verify_readonly(&vault_dir).unwrap();
    assert!(rep.valid);
    assert_eq!(rep.seq, 3);
    assert!(rep.incomplete_ops.is_empty());
    drop(s1);

    let ops_path = vault_dir.join(JOURNAL_SUBDIR).join(JOURNAL_FILENAME);
    {
        let mut f = fs::OpenOptions::new().append(true).open(&ops_path).unwrap();
        f.write_all(br#"{"v":1,"seq":4,"partial":true"#).unwrap();
    }
    let ro_partial = Journal::verify_readonly(&vault_dir).unwrap();
    assert!(!ro_partial.valid);
    assert!(ro_partial.truncated_tail_bytes.is_some());

    {
        let mut j = Journal::open(&vault_dir, JournalActor::Desktop, 5_000).unwrap();
        assert_eq!(j.seq(), 3);
        assert!(matches!(
            j.alerts().first(),
            Some(JournalAlertKind::TruncatedTailQuarantined { backup_file, bytes })
            if backup_file == "ops.jsonl.corrupt.5000" && *bytes > 0
        ));
        assert!(
            vault_dir
                .join(JOURNAL_SUBDIR)
                .join("ops.jsonl.corrupt.5000")
                .is_file()
        );

        j.append_op_begin(OpBeginParams {
            op_id: "op_crash".into(),
            contract_hash: "A6F75ECE".into(),
            op_kind: OpKind::NorDump,
            target: "84000000..847FFFFF".into(),
            ts_ms: 6_000,
            ..Default::default()
        })
        .unwrap();
    }

    {
        let mut j = Journal::open(&vault_dir, JournalActor::Mcp, 7_000).unwrap();
        assert_eq!(j.incomplete_ops().len(), 1);
        assert!(matches!(
            j.append_op_begin(OpBeginParams {
                op_id: "op_blocked".into(),
                contract_hash: "A6F75ECE".into(),
                op_kind: OpKind::SdKitBuild,
                target: "E:".into(),
                ts_ms: 7_100,
                ..Default::default()
            }),
            Err(JournalError::IncompleteOpBlocking { .. })
        ));
        j.acknowledge_incomplete_op("op_crash", "A6F75ECE", "operator inspected flash", 7_200)
            .unwrap();
        assert!(j.incomplete_ops().is_empty());
        j.append_op_begin(OpBeginParams {
            op_id: "op_after".into(),
            contract_hash: "A6F75ECE".into(),
            op_kind: OpKind::SdKitBuild,
            target: "E:".into(),
            ts_ms: 7_300,
            ..Default::default()
        })
        .unwrap();
        j.append_op_end(
            "op_after",
            "A6F75ECE",
            OpOutcome::Ok,
            100,
            None,
            None,
            None,
            7_400,
        )
        .unwrap();
    }

    let raw = fs::read_to_string(&ops_path).unwrap();
    let tampered = raw.replacen("\"seq\":1", "\"seq\":9", 1);
    fs::write(&ops_path, tampered).unwrap();
    assert!(matches!(
        Journal::open(&vault_dir, JournalActor::Cli, 8_000),
        Err(JournalError::ChainCorrupted {
            line: 1,
            expected_seq: 1
        })
    ));

    let _ = fs::remove_dir_all(&vault_dir);
}
