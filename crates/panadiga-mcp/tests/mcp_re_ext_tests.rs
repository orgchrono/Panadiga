#![deny(unsafe_code)]

use panadiga_core::constants::{AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE, PANA_DEV_BIN_MAGIC};
use panadiga_core::crypto::frm_pack::{FrmContainerMode, FrmPackModule, pack_frm};
use panadiga_core::nvram::calculate_hashes;
use panadiga_core::pico::panamon1::MockPanamon1Transport;
use panadiga_core::re::eh55_bgo::eh55_bgo_sector_offset;
use panadiga_core::re::eh55_bgo::repair_plan::{BgoRepairOp, BgoRepairPlan};
use panadiga_mcp::tools::re_ext_bench::{
    handle_davis_probe_with_policy, handle_eh55_bgo_with_policy,
};
use panadiga_mcp::{McpSecurityPolicy, execute_tool, execute_tool_with_policy, tool_definitions};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn temp_ws(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("panadiga_mcp_w2_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn all_24_mcp_tools_have_unique_names_and_valid_schemas() {
    let defs = tool_definitions();
    assert_eq!(defs.len(), 26);
    let mut names = HashSet::new();
    for d in &defs {
        let name = d["name"].as_str().unwrap();
        assert!(names.insert(name.to_string()), "duplicate tool {name}");
        assert_eq!(d["inputSchema"]["type"], "object");
        assert!(d["description"].as_str().unwrap().len() > 10);
    }
    for expected in [
        "panadiga_region_compat",
        "panadiga_hdd_flash_inspect",
        "panadiga_tbc_appliance_plan",
        "panadiga_nor_sector_audit",
        "panadiga_eh55_bgo",
        "panadiga_davis_probe",
        "panadiga_symbol_lookup",
    ] {
        assert!(names.contains(expected), "missing {expected}");
    }
}

#[test]
fn region_compat_and_ifodev2_tail_preserve_readonly_tail_cow() {
    let ws = temp_ws("region");
    let mut ifodev2 = vec![0u8; 2048];
    ifodev2[..4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
    ifodev2[4..12].copy_from_slice(b"EH55LB\x00\x00");
    ifodev2[0x20..0x2C].copy_from_slice(b"PANA_DVD_REC");
    ifodev2[0x400..0x408].copy_from_slice(b"TAIL_RO!");
    fs::write(ws.join("ifodev2.bin"), &ifodev2).unwrap();

    let rw_policy = McpSecurityPolicy {
        allow_file_write: true,
        ..McpSecurityPolicy::read_only()
    };
    let patched = execute_tool_with_policy(
        "panadiga_region_compat",
        &json!({
            "action": "patch_dev",
            "input_path": "ifodev2.bin",
            "output_path": "ifodev2_cow.bin",
            "model": "EH55EG",
            "oem": "E"
        }),
        &ws,
        &rw_policy,
    )
    .unwrap();
    assert_eq!(patched["metadata_and_dev_oem_only"], true);
    let out_bytes = fs::read(ws.join("ifodev2_cow.bin")).unwrap();
    assert_eq!(&out_bytes[0x400..0x408], b"TAIL_RO!");

    let (frm_bytes, _) = pack_frm(
        &[FrmPackModule::new("MAIN_EH55", vec![0x11u8; 512])],
        FrmContainerMode::Sd48,
        None,
    )
    .unwrap();
    fs::write(ws.join("PANA__SD.FRM"), frm_bytes).unwrap();

    let bundle = execute_tool(
        "panadiga_region_compat",
        &json!({
            "action": "inspect_tv_bundle",
            "preset": "br-pal-m",
            "tv_standard": "pal-m",
            "dev_bin_path": "ifodev2.bin",
            "frm_path": "PANA__SD.FRM"
        }),
        &ws,
    )
    .unwrap();
    assert_eq!(bundle["metadata_and_dev_oem_only"], true);
    assert_eq!(bundle["profile"]["preset"], "br_pal_m_r4");
    assert_eq!(bundle["profile"]["tv_standard"], "pal_m");
    let _ = fs::remove_dir_all(&ws);
}

#[test]
fn hdd_flash_inspect_is_readonly_and_tbc_and_nor_audit_work() {
    let ws = temp_ws("hdd_tbc_nor");
    let res = execute_tool(
        "panadiga_hdd_flash_inspect",
        &json!({
            "target_gb": 250,
            "disk_bytes": 250_000_000_000u64,
            "mock_disks": [
                { "device_path": "\\\\.\\PhysicalDrive0", "display_name": "OS", "size_bytes": 500_000_000_000u64, "is_system_drive": true, "bus_type": "NVMe" },
                { "device_path": "\\\\.\\PhysicalDrive1", "display_name": "Ext NVMe", "size_bytes": 250_000_000_000u64, "is_system_drive": false, "bus_type": "NVMe" },
                { "device_path": "\\\\.\\PhysicalDrive2", "display_name": "PATA USB", "size_bytes": 250_000_000_000u64, "is_system_drive": false, "bus_type": "USB" }
            ]
        }),
        &ws,
    )
    .unwrap();
    assert_eq!(res["read_only"], true);
    assert_eq!(res["dry_run_only"], true);
    assert_eq!(res["mirror_main_lba"], 488_150_176u64);
    assert_eq!(res["safe_target_count"], 1);
    assert_eq!(res["blocked_target_count"], 2);
    assert!(
        execute_tool(
            "panadiga_hdd_flash_inspect",
            &json!({ "target_gb": 2500, "allow_extended_4tb": false }),
            &ws
        )
        .is_err()
    );

    let tbc_ok = execute_tool(
        "panadiga_tbc_appliance_plan",
        &json!({ "mode": "stage15_digadiag_tbc", "tv_standard": "pal-m" }),
        &ws,
    )
    .unwrap();
    assert_eq!(tbc_ok["nor_sa0_to_sa3_untouched"], true);
    assert!(
        execute_tool(
            "panadiga_tbc_appliance_plan",
            &json!({ "mode": "experimental_sd_ram_chainload", "experimental_ram_chainload": false }),
            &ws
        )
        .is_err()
    );

    fs::write(ws.join("nor.bin"), vec![0xFFu8; AM34_FLASH_SIZE as usize]).unwrap();
    let nor_audit = execute_tool(
        "panadiga_nor_sector_audit",
        &json!({ "nor_path": "nor.bin" }),
        &ws,
    )
    .unwrap();
    assert_eq!(nor_audit["total_sectors"], 142);
    let _ = fs::remove_dir_all(&ws);
}

#[test]
fn eh55_bgo_and_davis_probe_tools_enforce_guards_with_mock_transport() {
    let ws = temp_ws("bgo_davis");
    let nor = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    fs::write(ws.join("corrupt_nor.bin"), &nor).unwrap();

    let audit = execute_tool(
        "panadiga_eh55_bgo",
        &json!({ "action": "audit", "nor_path": "corrupt_nor.bin" }),
        &ws,
    )
    .unwrap();
    assert_eq!(audit["healthy"], false);

    let sa70_off = eh55_bgo_sector_offset(70).unwrap();
    let sa70_va = AM34_FLASH_UNCACHED_BASE + sa70_off as u32;
    let mut after_nor = nor.clone();
    after_nor[sa70_off..sa70_off + 16].copy_from_slice(&[0x42u8; 16]);
    let pre_bgo_sha = calculate_hashes(&nor[sa70_off..sa70_off + 0x1_0000]).1;
    let post_bgo_sha = calculate_hashes(&after_nor[sa70_off..sa70_off + 0x1_0000]).1;
    let plan = BgoRepairPlan {
        bgo_window_start_va: 0x8413_0000,
        bgo_window_end_va: 0x8440_0000,
        target_sectors: vec![70],
        source_nor_sha256: calculate_hashes(&nor).1,
        source_bgo_sha256: pre_bgo_sha,
        expected_post_sha256: post_bgo_sha,
        expected_post_nor_sha256: calculate_hashes(&after_nor).1,
        missing_f2_ids: vec![],
        assigned_spare_sas: vec![70],
        erased_sas: vec![70],
        total_write_bytes: 16,
        ops: vec![
            BgoRepairOp::EraseSector {
                sa: 70,
                phys_va: sa70_va,
            },
            BgoRepairOp::WriteBytes {
                sa: 70,
                phys_va: sa70_va,
                data: vec![0x42u8; 16],
                requires_erase_first: true,
            },
        ],
        post_audit: serde_json::from_value(audit).unwrap(),
        repaired_nor_image: None,
    };
    fs::write(
        ws.join("plan.json"),
        serde_json::to_vec_pretty(&plan).unwrap(),
    )
    .unwrap();

    let hw_policy = McpSecurityPolicy {
        allow_file_write: true,
        allow_hardware_write: true,
        hitl_token: Some("test-hitl-token".to_string()),
        ..McpSecurityPolicy::read_only()
    };

    assert!(
        handle_eh55_bgo_with_policy::<MockPanamon1Transport>(
            &json!({
                "action": "live_apply",
                "port": "COM99",
                "plan_path": "plan.json",
                "backup_dir": "bkp",
                "confirm_upper_nor": false,
                "hitl_token": "test-hitl-token"
            }),
            &ws,
            None,
            &hw_policy,
        )
        .is_err()
    );

    let mut mock_bgo = MockPanamon1Transport::new();
    mock_bgo.bridge_active = true;
    mock_bgo.map_memory(AM34_FLASH_UNCACHED_BASE, nor);
    mock_bgo.boottrace_reply = "***** normal mode *****\r\nBOOTTRACE: OK\r\nPANADIGA> ".to_string();
    let live_bgo = handle_eh55_bgo_with_policy(
        &json!({
            "action": "live_apply",
            "port": "MOCK",
            "plan_path": "plan.json",
            "backup_dir": "bkp",
            "confirm_upper_nor": true,
            "hitl_token": "test-hitl-token",
            "boot_trace_ms": 5000
        }),
        &ws,
        Some(mock_bgo),
        &hw_policy,
    )
    .unwrap();
    assert_eq!(live_bgo["boot_normal_ok"], true);

    let stubs = execute_tool(
        "panadiga_davis_probe",
        &json!({ "action": "build_stubs", "drive": "master" }),
        &ws,
    )
    .unwrap();
    assert_eq!(stubs["lethal_0xc000100c_blocked"], true);
    assert_eq!(stubs["stubs"].as_array().unwrap().len(), 4);

    let mut ec = vec![0u8; 512];
    ec[0..2].copy_from_slice(&0x045Au16.to_le_bytes());
    ec[54..62].copy_from_slice(b"HDS72252");
    let ec_res = execute_tool(
        "panadiga_davis_probe",
        &json!({ "action": "parse_ec", "ec_hex": hex::encode(ec) }),
        &ws,
    )
    .unwrap();
    assert_eq!(ec_res["lethal_0xc000100c_blocked"], true);

    let mut dmock = MockPanamon1Transport::new();
    dmock.bridge_active = true;
    for (va, sz) in [
        (0x8900_0000u32, 0x1000usize),
        (0x504F_8234, 0x40),
        (0x9050_0000, 0x3000),
    ] {
        dmock.map_memory(va, vec![0u8; sz]);
    }
    let sys_bytes: Vec<u8> = [0u16, 0, 0, 0, 0x8002, 0]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    dmock.map_memory(0xC000_1000, sys_bytes);
    let live_davis = handle_davis_probe_with_policy(
        &json!({
            "action": "run_live",
            "port": "MOCK",
            "drive": "master",
            "hitl_token": "test-hitl-token"
        }),
        &ws,
        Some(dmock),
        &hw_policy,
    )
    .unwrap();
    assert_eq!(live_davis["lethal_0xc000100c_blocked"], true);
    let _ = fs::remove_dir_all(&ws);
}

#[test]
fn symbol_lookup_tool_and_resource_block_destructive_bgo_calls() {
    let real_ws = panadiga_mcp::resolve_workspace_root();
    if !real_ws
        .join("profiles/symbols/loader_ver061_toc48_main57/PROG061.json")
        .is_file()
    {
        return;
    }
    let res = execute_tool(
        "panadiga_symbol_lookup",
        &json!({
            "train_id": "loader_ver061_toc48_main57",
            "module": "PROG061",
            "va": "0x50409DF2"
        }),
        &real_ws,
    )
    .unwrap();
    assert_eq!(res["total_matches"], 1);
    assert_eq!(
        res["safety_gate"]["blocked_for_unattended_call"],
        json!(true)
    );
    assert_eq!(
        res["matches"][0]["symbol"]["side_effects"],
        json!("writes_nor_bgo")
    );

    let read_res = panadiga_mcp::resources::read_resource(
        "panadiga://symbols/loader_ver061_toc48_main57/PROG061",
        &real_ws,
    )
    .unwrap();
    let txt = read_res["contents"][0]["text"].as_str().unwrap();
    assert!(txt.contains("loader_ver061_toc48_main57"));
}
