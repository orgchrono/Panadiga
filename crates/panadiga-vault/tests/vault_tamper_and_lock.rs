#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use panadiga_bench::stream::OpKind;
use panadiga_vault::{
    GENESIS_PREV_HASH, JOURNAL_FILENAME, JOURNAL_SUBDIR, Journal, JournalActor, JournalError,
    OpBeginParams, OpOutcome, VaultLock, VaultLockMode, compute_journal_entry_hash,
};

static VAULT_TEST_SEQ: AtomicU64 = AtomicU64::new(0);

fn scratch_vault_dir(label: &str) -> PathBuf {
    let id = VAULT_TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "panadiga_vault_int_{label}_{}_{}",
        std::process::id(),
        id
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn journal_chain_detects_single_byte_tamper() {
    let vault_dir = scratch_vault_dir("tamper");
    {
        let mut journal = Journal::open(&vault_dir, JournalActor::Cli, 1_000).expect("open vault");
        journal
            .append_op_begin(OpBeginParams {
                op_id: "op_t1".into(),
                contract_hash: "DEAD0001".into(),
                op_kind: OpKind::NorDump,
                target: "SA0..SA141".into(),
                ts_ms: 1_010,
                ..Default::default()
            })
            .expect("append begin");
        journal
            .append_op_end(
                "op_t1",
                "DEAD0001",
                OpOutcome::Ok,
                40,
                Some("012345".into()),
                None,
                None,
                1_050,
            )
            .expect("append end");
    }

    let ok_rep = Journal::verify_readonly(&vault_dir).expect("verify intact");
    assert!(ok_rep.valid);
    assert_eq!(ok_rep.seq, 2);

    let ops_path = vault_dir.join(JOURNAL_SUBDIR).join(JOURNAL_FILENAME);
    let original = fs::read_to_string(&ops_path).expect("read ops.jsonl");
    let tampered = original.replacen("SA0..SA141", "SA0..SA142", 1);
    assert_ne!(original, tampered);
    fs::write(&ops_path, tampered).expect("write tampered ops.jsonl");

    let bad_rep = Journal::verify_readonly(&vault_dir);
    assert!(matches!(
        bad_rep,
        Err(JournalError::ChainCorrupted {
            line: 1,
            expected_seq: 1
        })
    ));
    let _ = fs::remove_dir_all(&vault_dir);
}

#[test]
fn vault_lock_allows_shared_readers_blocks_exclusive_and_hashes_deterministically() {
    let vault_dir = scratch_vault_dir("locks");
    {
        let s1 = VaultLock::try_acquire(&vault_dir, VaultLockMode::Shared, "reader-1")
            .expect("first shared lock");
        let s2 = VaultLock::try_acquire(&vault_dir, VaultLockMode::Shared, "reader-2")
            .expect("second shared lock");
        assert_eq!(s1.mode(), VaultLockMode::Shared);
        assert_eq!(s2.mode(), VaultLockMode::Shared);
        assert!(matches!(
            VaultLock::try_acquire(&vault_dir, VaultLockMode::Exclusive, "writer"),
            Err(JournalError::VaultLocked { .. })
        ));
    }

    let ex = VaultLock::try_acquire(&vault_dir, VaultLockMode::Exclusive, "writer")
        .expect("exclusive lock after shared drop");
    assert_eq!(ex.mode(), VaultLockMode::Exclusive);
    drop(ex);

    let h1 = compute_journal_entry_hash(1, 1, GENESIS_PREV_HASH, br#"{"k":1}"#);
    let h2 = compute_journal_entry_hash(1, 2, GENESIS_PREV_HASH, br#"{"k":1}"#);
    let h3 = compute_journal_entry_hash(1, 1, &"a".repeat(64), br#"{"k":1}"#);
    let h4 = compute_journal_entry_hash(1, 1, GENESIS_PREV_HASH, br#"{"k":2}"#);
    assert_eq!(h1.len(), 64);
    assert!(
        h1.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    assert_ne!(h1, h2);
    assert_ne!(h1, h3);
    assert_ne!(h1, h4);
    let _ = fs::remove_dir_all(&vault_dir);
}
