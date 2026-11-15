use panadiga_mcp::execute_tool;
use serde_json::json;
use std::fs;
#[test]
fn test_new_re_bgo_nor_and_capacity_tools() {
    let tmp = std::env::temp_dir().join(format!("panadiga_mcp_re_{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();

    // 1. Test panadiga_nor_dumper_stub
    let stub_res = execute_tool(
        "panadiga_nor_dumper_stub",
        &json!({
            "start_addr": "0x84000000",
            "total_len": 8192,
            "block_size": 4096,
            "output_path": "nor_stub.bin"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(stub_res["status"], "success");
    assert!(tmp.join("nor_stub.bin").is_file());

    // 2. Test panadiga_bgo_unbrick on synthetic factory PROG061
    let mut prog = vec![0u8; panadiga_core::re::PROG061_EXPECTED_SIZE];
    prog[panadiga_core::re::OFF_BGO_SUM_BEQ..panadiga_core::re::OFF_BGO_SUM_BEQ + 2]
        .copy_from_slice(&[0xC8, 0x0A]);
    prog[panadiga_core::re::OFF_DEVINFO_CHECK_BEQ..panadiga_core::re::OFF_DEVINFO_CHECK_BEQ + 2]
        .copy_from_slice(&[0xC8, 0x15]);
    prog[panadiga_core::re::OFF_BGO_INIT_FAIL_HOOK..panadiga_core::re::OFF_BGO_INIT_FAIL_HOOK + 20]
        .copy_from_slice(&[
            0xFC, 0xCC, 0xB0, 0x3E, 0x43, 0x50, 0xF8, 0x07, 0xFC, 0xDD, 0xF7, 0xAA, 0xFF, 0xFF,
            0x00, 0x00, 0x80, 0xFF, 0xCA, 0x32,
        ]);
    fs::write(tmp.join("prog061.bin"), &prog).unwrap();

    let blocked_auto = execute_tool(
        "panadiga_bgo_unbrick",
        &json!({
            "input_path": "prog061.bin",
            "action": "patch",
            "mode": "auto_format",
            "wrap_as_digadiag": true,
            "output_path": "DIGADIAG_BLOCKED.FRM"
        }),
        &tmp,
    )
    .unwrap_err();
    assert!(blocked_auto.to_string().contains("AutoFormatBgo"));

    let bgo_res = execute_tool(
        "panadiga_bgo_unbrick",
        &json!({
            "input_path": "prog061.bin",
            "action": "patch",
            "mode": "bypass",
            "wrap_as_digadiag": true,
            "output_path": "DIGADIAG.FRM"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(
        bgo_res["inspection_before"]["detected_mode"],
        "FACTORY_ORIGINAL"
    );
    assert_eq!(
        bgo_res["inspection_after"]["detected_mode"],
        "BYPASS_CHECKS_ONLY"
    );
    assert!(tmp.join("DIGADIAG.FRM").is_file());

    let mon_res = execute_tool(
        "panadiga_bgo_unbrick",
        &json!({
            "input_path": "prog061.bin",
            "action": "patch",
            "mode": "uart_monitor",
            "wrap_as_digadiag": true,
            "output_path": "DIGADIAG_MON.FRM"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(
        mon_res["inspection_after"]["detected_mode"],
        "UART_MONITOR_PANAMON1"
    );
    assert!(tmp.join("DIGADIAG_MON.FRM").is_file());

    // 3. Test panadiga_hdd_capacity reject in-place & error handling cleanly
    let cap_missing = execute_tool(
        "panadiga_hdd_capacity",
        &json!({ "input_path": "nonexistent_main.bin" }),
        &tmp,
    )
    .unwrap();
    assert!(cap_missing["error"].is_string());

    // 4. Test panadiga_main_passthrough in `rsp_plan` mode (no input image required)
    let rsp_plan = execute_tool(
        "panadiga_main_passthrough",
        &json!({
            "action": "rsp_plan",
            "callback_target_va": "0x50401F3C"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(rsp_plan["handshake_hex"], "0e0402");
    assert_eq!(rsp_plan["passthrough_frames"].as_array().unwrap().len(), 3);
    assert_eq!(rsp_plan["callback_write_frame"], "$M515B2E40,4:3c1f4050#95");

    // 5. Test panadiga_sd_unbrick_kit `status` and `verify_sd` actions
    let unbrick_status = execute_tool(
        "panadiga_sd_unbrick_kit",
        &json!({ "action": "status" }),
        &tmp,
    )
    .unwrap();
    assert_eq!(
        unbrick_status["hardware_facts"].as_array().unwrap().len(),
        5
    );

    let verify_res = execute_tool(
        "panadiga_sd_unbrick_kit",
        &json!({ "action": "verify_sd", "sd_root_dir": "." }),
        &tmp,
    )
    .unwrap();
    assert_eq!(verify_res["overall_status"], "READY_WITH_HW_WARNINGS");
    assert_eq!(verify_res["digadiag_bgo_mode"], "BYPASS_CHECKS_ONLY");

    let empty_sd = tmp.join("empty_sd");
    fs::create_dir_all(&empty_sd).unwrap();
    let verify_missing = execute_tool(
        "panadiga_sd_unbrick_kit",
        &json!({ "action": "verify_sd", "sd_root_dir": "empty_sd" }),
        &tmp,
    )
    .unwrap();
    assert_eq!(verify_missing["overall_status"], "INCOMPLETE_OR_INVALID");

    // Matshita R6-MV4 emulator MCP actions
    let emu_boot = execute_tool(
        "panadiga_sd_unbrick_kit",
        &json!({
            "action": "matshita_emu",
            "emu_action": "boot",
            "scenario": "unlock_disc_frm"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(emu_boot["startup_succeeded"], true);
    assert_eq!(emu_boot["unlock_media_verified"], true);
    assert_eq!(emu_boot["jump_target_va_hex"], "0x50400000");

    let emu_inject = execute_tool(
        "panadiga_sd_unbrick_kit",
        &json!({
            "action": "matshita_emu",
            "emu_action": "inject_write",
            "payload_hex": "deadbeef",
            "offset": 0,
            "mode": 2
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(emu_inject["record"]["len_bytes"], 4);
    assert!(
        emu_inject["snapshot"]["history_count"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );
    assert_eq!(
        emu_inject["snapshot"]["injections"]
            .as_array()
            .map(|v| v.len()),
        Some(1)
    );

    let emu_sdram_write = execute_tool(
        "panadiga_sd_unbrick_kit",
        &json!({
            "action": "matshita_emu",
            "emu_action": "serial_cmd",
            "serial_cmd": "WW 0x51800000 0x11223344",
            "unlock_pass": true
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(emu_sdram_write["status"], "WRITTEN");
    assert_eq!(emu_sdram_write["authorized_flag_cd8"], 1);

    let emu_ipl_block = execute_tool(
        "panadiga_sd_unbrick_kit",
        &json!({
            "action": "matshita_emu",
            "emu_action": "serial_cmd",
            "serial_cmd": "WW 0x84000000 0x11223344",
            "unlock_pass": true
        }),
        &tmp,
    );
    assert!(emu_ipl_block.is_err());
    assert!(emu_ipl_block.unwrap_err().to_string().contains("E05"));

    let packed_frm = execute_tool(
        "panadiga_sd_unbrick_kit",
        &json!({
            "action": "matshita_emu",
            "emu_action": "pack_unlock_frm",
            "container_kind": "cd",
            "output_path": "out/PANA_CD.FRM"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(packed_frm["verification"]["verified"], true);
    assert!(tmp.join("out/PANA_CD.FRM").is_file());

    // NOR dumper DIGADIAG defaults to 0x50400000
    let nor_digadiag = execute_tool(
        "panadiga_nor_dumper_stub",
        &json!({
            "wrap_digadiag": true,
            "output_path": "out/DIGADIAG_DMP.FRM"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(nor_digadiag["load_addr"], "0x50400000");
    assert_eq!(nor_digadiag["stub_size_bytes"], 131_072);

    // Guarded CoW NVRAM patch leaves original file untouched
    let flash_src = tmp.join("orig_8mb.bin");
    fs::write(&flash_src, vec![0xFFu8; 8 * 1024 * 1024]).unwrap();
    let patched = execute_tool(
        "panadiga_clean_nvram",
        &json!({
            "action": "patch",
            "input_file": "orig_8mb.bin",
            "output_file": "out/patched_8mb.bin",
            "nvram_offset": "0x10",
            "patch_hex": "cafe1234"
        }),
        &tmp,
    )
    .unwrap();
    assert_eq!(patched["sanitized_file_created"], true);
    let patched_bytes = fs::read(tmp.join("out/patched_8mb.bin")).unwrap();
    assert_eq!(
        &patched_bytes[0x7F_0010..0x7F_0014],
        &[0xCA, 0xFE, 0x12, 0x34]
    );

    let out_of_range = execute_tool(
        "panadiga_clean_nvram",
        &json!({
            "action": "patch",
            "input_file": "orig_8mb.bin",
            "nvram_offset": 65536,
            "patch_hex": "aa"
        }),
        &tmp,
    );
    assert!(out_of_range.is_err());

    let _ = fs::remove_dir_all(&tmp);
}
