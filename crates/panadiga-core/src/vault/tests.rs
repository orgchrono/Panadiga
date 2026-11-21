#![deny(unsafe_code)]

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::constants::{AM34_FLASH_SIZE, PANA_FEISTEL_ROUND_KEY_32B};
use crate::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, pack_extrhead, pack_frm_with_key,
};
use crate::re::unbrick_sd_kit::build_cleanroom_digadiag;

#[test]
fn vault_enforces_raw_vs_frm_vs_recompiled_segregation_immutability_and_nor_drv_extraction() {
    let root = std::env::temp_dir().join(format!("panadiga_vault_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let manifest = init_vault_session(
        &root,
        "DMR-EH55",
        SdLoaderProfile::Ver061RawSd48,
        Some("eh55_demo"),
    )
    .unwrap();
    let sdir = PathBuf::from(&manifest.session_dir);
    assert!(sdir.join("raw").is_dir());
    assert!(sdir.join("frm").is_dir());
    assert!(sdir.join("recompiled").is_dir());
    assert_eq!(manifest.current_step, StrictWorkflowStep::BuildCleanroomSd);

    // Raw/ rejects .FRM; Frm/ rejects .bin
    assert!(
        ingest_vault_artifact(&sdir, b"abc", "PANA_DVD.FRM", VaultBucket::Raw, "t", None).is_err()
    );
    assert!(
        ingest_vault_artifact(&sdir, b"abc", "nor_8mb.bin", VaultBucket::Frm, "t", None).is_err()
    );

    // Recompiled/ fails closed without passed PreWriteGateReport
    assert!(
        ingest_vault_artifact(
            &sdir,
            b"patched",
            "MAIN_PAT.bin",
            VaultBucket::Recompiled,
            "t",
            None
        )
        .is_err()
    );

    // Auto-classify .FRM vs .bin without altering originals
    let staging = root.join("src");
    fs::create_dir_all(&staging).unwrap();
    let frm_src = staging.join("PANA_DVD.FRM");
    let (frm_bytes, _) = pack_frm_with_key(
        &[FrmPackModule::new("MAIN", b"EH55".to_vec())],
        FrmContainerMode::Dvd,
        None,
        &PANA_FEISTEL_ROUND_KEY_32B,
    )
    .unwrap();
    fs::write(&frm_src, &frm_bytes).unwrap();
    let rec_frm = auto_classify_and_ingest_file(&sdir, &frm_src, None, "user_frm", None).unwrap();
    assert_eq!(rec_frm.bucket, VaultBucket::Frm);

    // Immutability conflict: overwriting raw dump with different content must fail!
    ingest_vault_artifact(
        &sdir,
        b"NOR1",
        "nor_8mb.bin",
        VaultBucket::Raw,
        "pico",
        None,
    )
    .unwrap();
    assert!(
        ingest_vault_artifact(
            &sdir,
            b"NOR2",
            "nor_8mb.bin",
            VaultBucket::Raw,
            "pico",
            None
        )
        .is_err()
    );

    // Recompiled artifact succeeds when Mn103Vm gate passes
    let cr = build_cleanroom_digadiag(SdLoaderProfile::Ver061RawSd48).unwrap();
    let rec_re = ingest_vault_artifact(
        &sdir,
        &cr.bytes,
        "DIGADIAG.FRM",
        VaultBucket::Recompiled,
        "cleanroom",
        Some(&cr.emu_gate),
    )
    .unwrap();
    assert!(rec_re.emu_verified);
    assert!(
        sdir.join("recompiled")
            .join("emu_gate_report.json")
            .is_file()
    );

    // Extract DRV EXTRHEAD directly from synthetic 8 MiB NOR capture
    let drv_eh = pack_extrhead(
        &vec![0x33u8; 2048],
        "DRV ",
        0x9020_2000,
        "S211",
        ExtrheadCryptMode::Drv,
        16,
        1,
    )
    .unwrap();
    let mut nor = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    nor[0x08_0000..0x08_0000 + drv_eh.len()].copy_from_slice(&drv_eh);
    let rec_drv = extract_nor_drv_into_vault(&sdir, &nor).unwrap();
    assert_eq!(rec_drv.filename, "drv_from_nor.bin");

    let sessions = list_vault_sessions(&root).unwrap();
    assert_eq!(sessions.len(), 1);
    let _ = fs::remove_dir_all(&root);
}
