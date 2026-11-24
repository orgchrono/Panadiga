use crate::cli::args::Cli;
use crate::cli::handlers::dispatch;
use clap::Parser;
use panadiga_core::ssot::{ModelProfileManifest, NorLayoutProfile};
use std::fs;

#[test]
fn cli_profiles_validate_show_authorize_and_export_schema_work() {
    let pid = std::process::id();
    let out_json = std::env::temp_dir().join(format!("panadiga_cli_profiles_{pid}.json"));
    let cli_show = Cli::try_parse_from([
        "panadiga",
        "profiles",
        "show",
        "DMR-EH55",
        "--json",
        out_json.to_str().unwrap(),
    ])
    .unwrap();
    dispatch(cli_show.command.unwrap()).unwrap();
    let body = fs::read_to_string(&out_json).unwrap();
    assert!(body.contains("S29JL064H70TFI00"));
    assert!(body.contains("0x847F0000"));
    let _ = fs::remove_file(&out_json);

    let cli_val = Cli::try_parse_from(["panadiga", "profiles", "validate"]).unwrap();
    dispatch(cli_val.command.unwrap()).unwrap();

    let auth_ok = Cli::try_parse_from([
        "panadiga",
        "profiles",
        "authorize",
        "DMR-EH55",
        "--capability",
        "nvram_sanitize_offline_cow",
        "--dump-size",
        "8388608",
        "--has-eemul-f1",
    ])
    .unwrap();
    dispatch(auth_ok.command.unwrap()).unwrap();

    let auth_denied = Cli::try_parse_from([
        "panadiga",
        "profiles",
        "authorize",
        "DMR-EZ485V",
        "--capability",
        "nvram_sanitize_offline_cow",
    ])
    .unwrap();
    assert!(dispatch(auth_denied.command.unwrap()).is_err());

    let schema_tmp = std::env::temp_dir().join(format!("panadiga_cli_schemas_{pid}"));
    let _ = fs::remove_dir_all(&schema_tmp);
    let exp_cli = Cli::try_parse_from([
        "panadiga",
        "profiles",
        "export-schema",
        "--out-dir",
        schema_tmp.to_str().unwrap(),
    ])
    .unwrap();
    dispatch(exp_cli.command.unwrap()).unwrap();
    assert!(schema_tmp.join("profiles_bundle.schema.json").is_file());
    let _ = fs::remove_dir_all(&schema_tmp);
}

#[test]
fn cli_profiles_completeness_and_discover_nor_emit_valid_drafts() {
    let pid = std::process::id();
    let comp_json = std::env::temp_dir().join(format!("panadiga_cli_completeness_{pid}.json"));
    let cli_comp = Cli::try_parse_from([
        "panadiga",
        "profiles",
        "completeness",
        "--min-tier",
        "T1",
        "--json",
        comp_json.to_str().unwrap(),
    ])
    .unwrap();
    dispatch(cli_comp.command.unwrap()).unwrap();
    let comp_val: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&comp_json).unwrap()).unwrap();
    assert_eq!(comp_val["total_models"].as_u64(), Some(89));
    assert!(comp_val["collections"]["models"]["missing_fields"]["board_ref"].is_u64());
    let _ = fs::remove_file(&comp_json);

    let work_dir = std::env::temp_dir().join(format!("panadiga_cli_discover_nor_{pid}"));
    let _ = fs::remove_dir_all(&work_dir);
    fs::create_dir_all(&work_dir).unwrap();
    let mut synth_nor = vec![0xFFu8; 0x20_0000];
    let banner = b"Loader ver0.99 [07/01/02]\0";
    synth_nor[0x100..0x100 + banner.len()].copy_from_slice(banner);
    synth_nor[0x8_0000..0x8_0008].copy_from_slice(b"EXTRHEAD");
    synth_nor[0x10_0000..0x10_0002].copy_from_slice(&0xF101u16.to_le_bytes());
    synth_nor[0x1F_0005] = 0xF1;
    let nor_path = work_dir.join("synth_eh99.bin");
    let rep_path = work_dir.join("discover_report.json");
    let draft_dir = work_dir.join("drafts");
    fs::write(&nor_path, &synth_nor).unwrap();

    let cli_disc = Cli::try_parse_from([
        "panadiga",
        "profiles",
        "discover-nor",
        "--input",
        nor_path.to_str().unwrap(),
        "--model-id",
        "DMR-EH99",
        "--out-dir",
        draft_dir.to_str().unwrap(),
        "--json",
        rep_path.to_str().unwrap(),
    ])
    .unwrap();
    dispatch(cli_disc.command.unwrap()).unwrap();

    let rep: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&rep_path).unwrap()).unwrap();
    assert_eq!(rep["loader_version"].as_str(), Some("0.99"));
    assert_eq!(rep["loader_date"].as_str(), Some("07/01/02"));
    assert_eq!(rep["bgo_sector_count_64k"].as_u64(), Some(1));
    assert_eq!(rep["extrhead_entries"].as_array().unwrap().len(), 1);

    let draft_layout: NorLayoutProfile = serde_json::from_str(
        &fs::read_to_string(draft_dir.join("dmr_eh99_nor_layout_draft.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(draft_layout.regions.len(), 4);
    let draft_model: ModelProfileManifest =
        serde_json::from_str(&fs::read_to_string(draft_dir.join("dmr_eh99.draft.json")).unwrap())
            .unwrap();
    assert_eq!(draft_model.model_id, "DMR-EH99");
    let _ = fs::remove_dir_all(&work_dir);
}

#[test]
fn cli_profiles_review_proposal_verifies_hash_and_reports_json() {
    let pid = std::process::id();
    let root = std::env::temp_dir().join(format!("panadiga_cli_review_prop_{pid}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("fixtures")).unwrap();

    let fix_rel = "fixtures/sample.bin";
    fs::write(root.join(fix_rel), b"PANADIGA-FIXTURE-V1").unwrap();
    let real_sha = panadiga_core::ssot::compute_file_sha256_hex(&root.join(fix_rel)).unwrap();

    let ledger_path = root.join("ledger.json");
    fs::write(
        &ledger_path,
        serde_json::json!({
            "schema_version": 1,
            "entries": [{
                "id": "EV-CLI-1",
                "sha256_assertions": [{ "path": fix_rel, "sha256": real_sha }]
            }]
        })
        .to_string(),
    )
    .unwrap();

    let prop_path = root.join("proposal.json");
    let prop = panadiga_core::ssot::ManualClaimProposal {
        proposal_id: "CLI-PROP-1".to_string(),
        model_id: "DMR-EH55".to_string(),
        axis: panadiga_core::ssot::ClaimAxis::Eeprom,
        proposed_tier: panadiga_core::ssot::EvidenceTier::PhysicalDumpVerified,
        fixture_rel_path: fix_rel.to_string(),
        bench_log_rel_path: None,
        declared_sha256: real_sha.clone(),
        notes: None,
        submitted_by: Some("cli_test".to_string()),
        ledger_entry_id: Some("EV-CLI-1".to_string()),
    };
    fs::write(&prop_path, serde_json::to_string_pretty(&prop).unwrap()).unwrap();

    let out_rep = root.join("review_out.json");
    let cli = Cli::try_parse_from([
        "panadiga",
        "profiles",
        "review-proposal",
        "--proposal",
        prop_path.to_str().unwrap(),
        "--workspace-root",
        root.to_str().unwrap(),
        "--profiles-dir",
        "profiles",
        "--ledger",
        ledger_path.to_str().unwrap(),
        "--json",
        out_rep.to_str().unwrap(),
    ])
    .unwrap();
    dispatch(cli.command.unwrap()).unwrap();

    let rep: panadiga_core::ssot::ProposalReviewReport =
        serde_json::from_str(&fs::read_to_string(&out_rep).unwrap()).unwrap();
    assert!(rep.valid);
    assert!(rep.sha256_verified);
    assert!(rep.ledger_synced);
    assert!(!rep.promoted);
    let _ = fs::remove_dir_all(&root);
}
