#![deny(unsafe_code)]

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_panadiga(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_panadiga"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("spawn panadiga CLI binary");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "panadiga {:?} failed (status={}):\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
        args,
        out.status
    );
    stdout
}

fn run_panadiga_json(prefix_args: &[&str], json_path: &Path) -> Value {
    let mut cmd_args: Vec<&str> = prefix_args.to_vec();
    cmd_args.push("--json");
    let json_arg = json_path.to_str().unwrap();
    cmd_args.push(json_arg);
    run_panadiga(&cmd_args);
    serde_json::from_str(&fs::read_to_string(json_path).unwrap()).unwrap()
}

#[test]
fn cli_profiles_validate_and_completeness_parity() {
    let validate_out = run_panadiga(&["profiles", "validate"]);
    assert!(validate_out.contains("valid=true"));
    assert!(validate_out.contains("models=89"));

    let stdout = run_panadiga(&["profiles", "completeness", "--json", "-"]);
    let payload: Value = serde_json::from_str(&stdout).expect("completeness JSON");
    assert_eq!(payload["total_models"], 89);
    assert!(payload["models_by_max_tier"].is_object());
    assert!(payload["collections"]["models"]["total"].as_u64() == Some(89));
}

#[test]
fn cli_discover_nor_emits_drafts_without_touching_ssot() {
    let dir = std::env::temp_dir().join(format!("panadiga_cli_disc_e2e_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let nor_path = dir.join("synthetic_eh88_nor.bin");
    let out_dir = dir.join("drafts");
    let json_path = dir.join("discover.json");
    let mut nor = vec![0xFFu8; 0x20_0000];
    nor[0x120..0x13A].copy_from_slice(b"Loader ver0.88 [06/11/09]\x00");
    nor[0x8_0000..0x8_000C].copy_from_slice(b"EXTRHEADMAIN");
    nor[0x10_0000..0x10_0002].copy_from_slice(&0xF101u16.to_le_bytes());
    nor[0x1F_0005] = 0xF1;
    fs::write(&nor_path, &nor).unwrap();

    let report = run_panadiga_json(
        &[
            "profiles",
            "discover-nor",
            "--input",
            nor_path.to_str().unwrap(),
            "--model-id",
            "DMR-EH88",
            "--out-dir",
            out_dir.to_str().unwrap(),
        ],
        &json_path,
    );
    assert_eq!(report["loader_version"], "0.88");
    assert_eq!(report["bgo_sector_count_64k"], 1);
    assert!(out_dir.join("dmr_eh88_nor_layout_draft.json").is_file());
    assert!(out_dir.join("dmr_eh88.draft.json").is_file());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_verify_sd_and_ghidra_export_all_parity() {
    let dir = std::env::temp_dir().join(format!("panadiga_cli_ghidra_e2e_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let json_path = dir.join("verify_sd.json");
    let ghidra_dir = dir.join("ghidra_out");

    let report = run_panadiga_json(
        &[
            "unbrick",
            "verify-sd",
            "--audit-profile",
            "hardened_recovery",
            "firmware_and_images/sd_unbrick_kit",
        ],
        &json_path,
    );
    assert_eq!(report["dual_family_parity"]["dual_parity_ok"], true);
    assert_eq!(report["factory_unpatched_prog061_detected"], false);

    run_panadiga(&[
        "ghidra",
        "export-all",
        "--dir",
        ghidra_dir.to_str().unwrap(),
    ]);
    let checked_in = fs::read_to_string(repo_root().join("tools/ghidra/ghidra_ssot_maps.json"))
        .unwrap()
        .replace("\r\n", "\n");
    let generated = fs::read_to_string(ghidra_dir.join("ghidra_ssot_maps.json"))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(generated, checked_in);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_unbrick_rebuild_from_dump_help_and_optional_nor_flow() {
    let help = run_panadiga(&["unbrick", "rebuild-from-dump", "--help"]);
    assert!(help.contains("--nor-dump"));
    assert!(help.contains("--hdd-main"));
    assert!(help.contains("--target-gb"));

    let nor = repo_root().join("firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
    let frm = repo_root().join("firmware_and_images/PANA_DVD.FRM");
    let main_bin = repo_root()
        .join("firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin");
    if nor.is_file() && frm.is_file() && main_bin.is_file() {
        let dir =
            std::env::temp_dir().join(format!("panadiga_cli_rebuild_e2e_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let out_sd = dir.join("sd");
        let rep_json = dir.join("rebuild.json");

        let rep = run_panadiga_json(
            &[
                "unbrick",
                "rebuild-from-dump",
                "--model",
                "DMR-EH55",
                "--nor-dump",
                nor.to_str().unwrap(),
                "--hdd-main",
                main_bin.to_str().unwrap(),
                "--frm",
                frm.to_str().unwrap(),
                "--target-gb",
                "250",
                "--out-dir",
                out_sd.to_str().unwrap(),
            ],
            &rep_json,
        );
        assert_eq!(rep["model_id"], "DMR-EH55");
        assert_eq!(rep["resolved_loader_profile"], "ver061");
        assert_eq!(rep["kit"]["hdd_capacity_applied"], "cap_250gb");
        assert!(out_sd.join("DIGADIAG.FRM").is_file());
        assert!(out_sd.join("PANA__SD.FRM").is_file());

        let _ = fs::remove_dir_all(&dir);
    }
}
