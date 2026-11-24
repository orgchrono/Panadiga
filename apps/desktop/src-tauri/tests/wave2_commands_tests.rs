#![deny(unsafe_code)]

use panadiga_core::bench::OpKind;
use panadiga_core::constants::{
    AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE, AM34_NVRAM_SIZE, AM34_NVRAM_UNCACHED_BASE,
    AM34_SDRAM_UNCACHED_BASE, PANA_DEV_BIN_MAGIC, PANASONIC_HDD_MBR_FULL_SIGNATURE,
};
use panadiga_core::hdd::writer::{BUS_TYPE_NVME, FileBackedMockFlashDevice};
use panadiga_core::pico::panamon1::MockPanamon1Transport;
use panadiga_core::re::bgo_unbrick::{
    OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ,
    ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PROG061_EXPECTED_SIZE,
};
use panadiga_core::re::eh55_bgo::{eh55_bgo_sector_offset, repair_plan::build_bgo_repair_plan};
use panadiga_core::re::prog061_uart_monitor::{
    PROG061_UART_MONITOR_VA, build_prog061_uart_monitor_with_offsets,
};
use panadiga_lib::commands::constants::{
    EVENT_BGO_REPAIR_PROGRESS, EVENT_DAVIS_PROBE_RESULT, EVENT_HDD_FLASH_PROGRESS,
    EVENT_NOR_DUMP_PROGRESS,
};
use panadiga_lib::commands::hardware_diag::{
    PicoEh55BgoAuditRequestDto, PicoEh55BgoRepairRequestDto, PicoEh55NorDumpRequestDto,
    PicoProbeDavisIdeRequestDto, RequestOperationNonceDto, consume_operation_nonce_with_state,
    issue_operation_nonce_with_state, run_davis_probe_with_transport,
    run_eh55_bgo_audit_with_transport, run_eh55_bgo_repair_with_transport,
    run_eh55_nor_dump_with_transport,
};
use panadiga_lib::commands::storage::{
    FlashEh55HddSafeRequestDto, inspect_safe_target_disks_command, run_flash_eh55_with_device,
};
use panadiga_lib::commands::unbrick::{
    ApplyMainTbcBypassesRequestDto, ApplyRegionCalibrationRequestDto, HardenSdUnbrickKitRequestDto,
    InspectRegionCalibrationRequestDto, apply_main_tbc_bypasses_command,
    apply_region_calibration_command, harden_sd_unbrick_kit_command,
    inspect_region_calibration_command,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("panadiga_tauri_w2_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn synthetic_prog061() -> Vec<u8> {
    let mut b = vec![0u8; PROG061_EXPECTED_SIZE];
    b[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
    b[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2].copy_from_slice(&ORIG_DEVINFO_BEQ);
    b[OFF_BGO_INIT_CALL..OFF_BGO_INIT_CALL + 7].copy_from_slice(&ORIG_A2BC_BGO_CALL);
    b[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20].copy_from_slice(&ORIG_A2CD_HOOK);
    b
}

fn build_mock_bgo_8mb() -> Vec<u8> {
    let mut nor = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    for sa in 26..=70 {
        let base = eh55_bgo_sector_offset(sa).unwrap();
        nor[base + 0x100..base + 0x200].fill(0);
    }
    for sa in [29usize, 41] {
        let base = eh55_bgo_sector_offset(sa).unwrap();
        for i in 0..4 {
            nor[base + 4 + i * 4..base + 6 + i * 4].copy_from_slice(&0xF101u16.to_le_bytes());
        }
    }
    let f2_sas: Vec<usize> = (26..=28).chain(30..=40).chain(42..=67).collect();
    let mut map: BTreeMap<u16, u16> = BTreeMap::new();
    for (idx, &sa) in f2_sas.iter().enumerate() {
        let fid = (idx + 1) as u16;
        let base = eh55_bgo_sector_offset(sa).unwrap();
        for i in 0..4 {
            nor[base + 4 + i * 4..base + 8 + i * 4].copy_from_slice(&[fid as u8, 0xF2, 0xFF, 0xFF]);
        }
        nor[base + 0xFC..base + 0x100].fill(0);
        let pid = if fid == 1 {
            1
        } else if fid <= 31 {
            0x0F05 + fid
        } else {
            0x0010 + fid
        };
        nor[base + 0x200..base + 0x202].copy_from_slice(&pid.to_le_bytes());
        if pid == 1 {
            nor[base + 0x400..base + 0x404].copy_from_slice(&PANA_DEV_BIN_MAGIC);
            nor[base + 0x404..base + 0x40C].copy_from_slice(b"M6930LB\x00");
        } else if (0x0F07..=0x0F24).contains(&pid) {
            nor[base + 0x400..base + 0x404].copy_from_slice(&[0xED, 0xFF, 0xFF, 0xFF]);
        }
        map.insert(pid, fid << 8);
    }
    let mut tids: BTreeMap<u16, [u16; 16]> = BTreeMap::new();
    for (pid, w) in map {
        tids.entry(pid / 16).or_insert([0xFFFF; 16])[usize::from(pid % 16)] = w;
    }
    for (idx, (tid, arr)) in tids.into_iter().enumerate() {
        let off = eh55_bgo_sector_offset(41).unwrap() + (2 + idx / 7) * 0x100;
        let s = idx % 7;
        if s == 0 {
            nor[off..off + 4].fill(0);
        }
        nor[off + 3] |= 1u8 << s;
        let soff = off + 4 + s * 0x24;
        nor[soff..soff + 2].copy_from_slice(&tid.to_le_bytes());
        nor[soff + 2..soff + 4].copy_from_slice(&0x0001u16.to_le_bytes());
        for (i, w) in arr.into_iter().enumerate() {
            nor[soff + 4 + i * 2..soff + 6 + i * 2].copy_from_slice(&w.to_le_bytes());
        }
    }
    for sa in std::iter::once(35).chain(56..=67) {
        let base = eh55_bgo_sector_offset(sa).unwrap();
        nor[base..base + 0x1_0000].fill(0xFF);
        nor[base + 0x100..base + 0x200].fill(0);
    }
    nor
}

#[test]
fn event_constants_and_dto_camelcase_serde_contracts_match() {
    assert_eq!(EVENT_HDD_FLASH_PROGRESS, "hdd://flash-progress");
    assert_eq!(EVENT_NOR_DUMP_PROGRESS, "nor://dump-progress");
    assert_eq!(EVENT_BGO_REPAIR_PROGRESS, "bgo://repair-progress");
    assert_eq!(EVENT_DAVIS_PROBE_RESULT, "davis://probe-result");

    let reg_req: InspectRegionCalibrationRequestDto = serde_json::from_value(json!({
        "preset": "br-pal-m",
        "tvStandard": "pal-m",
        "devBinPath": "DEV.BIN"
    }))
    .unwrap();
    assert_eq!(reg_req.tv_standard.as_deref(), Some("pal-m"));

    let flash_req: FlashEh55HddSafeRequestDto = serde_json::from_value(json!({
        "devicePath": "\\\\.\\PhysicalDrive1",
        "confirmDiskModelToken": "PhysicalDrive1",
        "backupDir": "bkp",
        "expectedMinGb": 240,
        "expectedMaxGb": 260,
        "allowExtended4tb": false
    }))
    .unwrap();
    assert_eq!(flash_req.confirm_disk_model_token, "PhysicalDrive1");
    assert_eq!(flash_req.allow_extended_4tb, Some(false));

    let bgo_rep: PicoEh55BgoRepairRequestDto = serde_json::from_value(json!({
        "norPath": "nor.bin",
        "dryRunOnly": true,
        "confirmToken": "CONFIRM_UPPER_NOR",
        "runBootTraceMs": 15000
    }))
    .unwrap();
    assert!(bgo_rep.dry_run_only);
    assert_eq!(bgo_rep.run_boot_trace_ms, Some(15000));
}

#[test]
fn region_calibration_sd_harden_and_tbc_lab_commands_enforce_cow_and_tokens() {
    let dir = temp_dir("unbrick");
    let mut ifo = vec![0u8; 2048];
    ifo[..4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
    ifo[4..12].copy_from_slice(b"EH55LB\x00\x00");
    ifo[0x20..0x2C].copy_from_slice(b"PANA_DVD_REC");
    ifo[0x400..0x408].copy_from_slice(b"IFO_RO!!");
    let ifo_path = dir.join("ifodev2.bin");
    fs::write(&ifo_path, &ifo).unwrap();

    let insp = inspect_region_calibration_command(InspectRegionCalibrationRequestDto {
        preset: Some("br-pal-m".into()),
        tv_standard: Some("pal-m".into()),
        dev_bin_path: Some(ifo_path.to_string_lossy().into()),
        main_path: None,
        frm_path: None,
        nvram_path: None,
    })
    .unwrap();
    assert!(insp.metadata_and_dev_oem_only);

    let out_cal = dir.join("cal_out");
    let bad_tok = apply_region_calibration_command(ApplyRegionCalibrationRequestDto {
        preset: Some("br-pal-m".into()),
        tv_standard: Some("pal-m".into()),
        output_dir: out_cal.to_string_lossy().into(),
        dev_bin_path: Some(ifo_path.to_string_lossy().into()),
        model: Some("EH55EG".into()),
        oem: Some("E".into()),
        main_path: None,
        main_compat_mode: None,
        frm_path: None,
        hw_family: None,
        model_group: None,
        target_flag: None,
        confirm_token: "WRONG".into(),
    });
    assert!(bad_tok.is_err());

    let cal_ok = apply_region_calibration_command(ApplyRegionCalibrationRequestDto {
        preset: Some("eu-pal".into()),
        tv_standard: Some("pal".into()),
        output_dir: out_cal.to_string_lossy().into(),
        dev_bin_path: Some(ifo_path.to_string_lossy().into()),
        model: Some("EH55EG".into()),
        oem: Some("E".into()),
        main_path: None,
        main_compat_mode: None,
        frm_path: None,
        hw_family: None,
        model_group: None,
        target_flag: None,
        confirm_token: "CALIBRATE_REGION_COW".into(),
    })
    .unwrap();
    assert!(cal_ok.metadata_and_dev_oem_only);
    let cow_ifo = fs::read(cal_ok.dev_bin_output_path.unwrap()).unwrap();
    assert_eq!(&cow_ifo[0x400..0x408], b"IFO_RO!!");

    let tbc_err = apply_main_tbc_bypasses_command(ApplyMainTbcBypassesRequestDto {
        input_main_path: None,
        output_main_path: None,
        passthrough_mode: None,
        appliance_mode: Some("experimental_sd_ram_chainload".into()),
        tv_standard: Some("ntsc".into()),
        experimental_ram_chainload: Some(false),
        confirm_token: "APPLY_MAIN_TBC_COW".into(),
    });
    assert!(tbc_err.is_err());
    let tbc_ok = apply_main_tbc_bypasses_command(ApplyMainTbcBypassesRequestDto {
        input_main_path: None,
        output_main_path: None,
        passthrough_mode: None,
        appliance_mode: Some("stage15_digadiag_tbc".into()),
        tv_standard: Some("pal-m".into()),
        experimental_ram_chainload: Some(false),
        confirm_token: "APPLY_MAIN_TBC_COW".into(),
    })
    .unwrap();
    assert!(tbc_ok.appliance_plan.nor_sa0_to_sa3_untouched);

    let prog_p = dir.join("PROG061.bin");
    let main_p = dir.join("MAIN.bin");
    let drv_p = dir.join("DRV.bin");
    fs::write(&prog_p, synthetic_prog061()).unwrap();
    fs::write(&main_p, vec![0xA5u8; 10_000_000]).unwrap();
    fs::write(&drv_p, vec![0x33u8; 128]).unwrap();

    let blocked_factory = harden_sd_unbrick_kit_command(HardenSdUnbrickKitRequestDto {
        output_dir: dir.join("sd_factory").to_string_lossy().into(),
        pana_dvd_path: None,
        prog061_path: Some(prog_p.to_string_lossy().into()),
        main_path: Some(main_p.to_string_lossy().into()),
        drv_path: Some(drv_p.to_string_lossy().into()),
        main_passthrough_mode: Some("factory_original".into()),
        unlock_hdd_dynamic: Some(false),
        hdd_target_gb: None,
        expected_main_build: None,
        allow_target_mismatch: Some(true),
        allow_factory_prog061: Some(false),
        write_extracted_sources: Some(false),
    });
    assert!(blocked_factory.is_ok());
    assert!(
        blocked_factory
            .unwrap()
            .dual_family_parity
            .unwrap()
            .dual_parity_ok
    );

    let reject_unpatched = harden_sd_unbrick_kit_command(HardenSdUnbrickKitRequestDto {
        output_dir: dir.join("sd_bad").to_string_lossy().into(),
        pana_dvd_path: None,
        prog061_path: Some(prog_p.to_string_lossy().into()),
        main_path: Some(main_p.to_string_lossy().into()),
        drv_path: Some(drv_p.to_string_lossy().into()),
        main_passthrough_mode: Some("factory_original".into()),
        unlock_hdd_dynamic: Some(false),
        hdd_target_gb: None,
        expected_main_build: None,
        allow_target_mismatch: Some(true),
        allow_factory_prog061: Some(true),
        write_extracted_sources: Some(false),
    });
    assert!(reject_unpatched.is_err());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn hdd_flash_safe_enforces_physicaldrive0_nvme_4tb_and_executes_mock_flash() {
    let _ = inspect_safe_target_disks_command().unwrap();
    let dir = temp_dir("hdd");
    let img_path = dir.join("eh55.img");
    let mut primary = vec![0x3Cu8; 1024 * 1024];
    primary[..512].fill(0);
    primary[..16].copy_from_slice(&PANASONIC_HDD_MBR_FULL_SIGNATURE);
    fs::write(&img_path, primary).unwrap();
    let mirror_p1 = dir.join("p1.bin");
    fs::write(&mirror_p1, vec![0x7Du8; 64 * 1024]).unwrap();
    let bkp_dir = dir.join("bkp");
    fs::create_dir_all(&bkp_dir).unwrap();

    let mock_path = dir.join("disk.bin");
    let mut ata_dev = FileBackedMockFlashDevice::create(&mock_path, 250_000_000_000, 3).unwrap();
    let pd0 = FlashEh55HddSafeRequestDto {
        device_path: r"\\.\PhysicalDrive0".into(),
        confirm_disk_model_token: "PhysicalDrive0".into(),
        image_path: Some(img_path.to_string_lossy().into()),
        mirror_p1_path: Some(mirror_p1.to_string_lossy().into()),
        backup_dir: bkp_dir.to_string_lossy().into(),
        expected_min_gb: Some(240),
        expected_max_gb: Some(260),
        allow_extended_4tb: Some(false),
        nonce_id: None,
    };
    assert!(run_flash_eh55_with_device(&mut ata_dev, &pd0, |_| {}).is_err());

    let mut nvme_dev =
        FileBackedMockFlashDevice::create(&mock_path, 250_000_000_000, BUS_TYPE_NVME).unwrap();
    let valid_dto = FlashEh55HddSafeRequestDto {
        device_path: r"\\.\PhysicalDrive99".into(),
        confirm_disk_model_token: "PhysicalDrive99".into(),
        ..pd0.clone()
    };
    assert!(run_flash_eh55_with_device(&mut nvme_dev, &valid_dto, |_| {}).is_err());

    let over_4tb = FlashEh55HddSafeRequestDto {
        expected_max_gb: Some(3000),
        allow_extended_4tb: Some(false),
        ..valid_dto.clone()
    };
    assert!(run_flash_eh55_with_device(&mut ata_dev, &over_4tb, |_| {}).is_err());

    let mut events = Vec::new();
    let rep = run_flash_eh55_with_device(&mut ata_dev, &valid_dto, |ev| events.push(ev)).unwrap();
    assert!(rep.readback_verified);
    assert!(!events.is_empty());
    let json_val = serde_json::to_value(&events[0]).unwrap();
    assert!(json_val.get("bytesDone").is_some());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn eh55_nor_dump_bgo_audit_repair_and_davis_probe_enforce_guards_with_mock() {
    let dir = temp_dir("diag");
    let mut nor = build_mock_bgo_8mb();
    let nv_off = (AM34_NVRAM_UNCACHED_BASE - AM34_FLASH_UNCACHED_BASE) as usize;
    nor[nv_off..nv_off + 8].copy_from_slice(b"TAURINOR");
    let nor_path = dir.join("nor.bin");
    fs::write(&nor_path, &nor).unwrap();

    let (_, offsets) = build_prog061_uart_monitor_with_offsets(PROG061_UART_MONITOR_VA).unwrap();
    let (mon_payload, _) =
        build_prog061_uart_monitor_with_offsets(PROG061_UART_MONITOR_VA).unwrap();
    let (entry_u, start_imm_u, end_imm_u) = offsets.uncached_patch_vas(PROG061_UART_MONITOR_VA);

    let mut cap_mock = MockPanamon1Transport::new();
    cap_mock.bridge_active = true;
    cap_mock.map_memory(
        (PROG061_UART_MONITOR_VA & 0x0FFF_FFFF) | AM34_SDRAM_UNCACHED_BASE,
        mon_payload,
    );
    cap_mock.map_memory(AM34_FLASH_UNCACHED_BASE, nor.clone());
    cap_mock.enable_dynamic_nor_dumper(entry_u, start_imm_u, end_imm_u, offsets.block_size);

    let mut dump_events = Vec::new();
    let cap_res = run_eh55_nor_dump_with_transport(
        cap_mock,
        &PicoEh55NorDumpRequestDto {
            port_name: "MOCK".into(),
            slice: Some("nvram".into()),
            custom_start_va_hex: None,
            custom_len_hex: None,
            output_path: Some(dir.join("nvram.bin").to_string_lossy().into()),
        },
        |ev| dump_events.push(ev),
    )
    .unwrap();
    assert_eq!(cap_res.total_len, AM34_NVRAM_SIZE as usize);
    assert!(!dump_events.is_empty());

    let audit_rep = run_eh55_bgo_audit_with_transport::<MockPanamon1Transport>(
        &PicoEh55BgoAuditRequestDto {
            source_mode: "DUMP_FILE".into(),
            nor_path: Some(nor_path.to_string_lossy().into()),
            port_name: None,
        },
        None,
    )
    .unwrap();
    assert!(!audit_rep.healthy);

    let cow_plan = run_eh55_bgo_repair_with_transport::<MockPanamon1Transport, _>(
        &PicoEh55BgoRepairRequestDto {
            nor_path: Some(nor_path.to_string_lossy().into()),
            plan_path: None,
            reference_nor_path: None,
            out_plan_path: Some(dir.join("plan.json").to_string_lossy().into()),
            cow_out_path: Some(dir.join("nor_cow.bin").to_string_lossy().into()),
            dry_run_only: true,
            port_name: None,
            backup_dir: None,
            confirm_token: None,
            run_boot_trace_ms: None,
            nonce_id: None,
        },
        None,
        |_| {},
    )
    .unwrap();
    assert!(cow_plan.sa0_to_sa3_locked_e05);
    assert!(cow_plan.plan.post_audit.healthy);

    let _ = build_bgo_repair_plan(&nor, None).unwrap();
    let mut mock_deny = MockPanamon1Transport::new();
    mock_deny.bridge_active = true;
    mock_deny.map_memory(AM34_FLASH_UNCACHED_BASE, nor.clone());
    assert!(
        run_eh55_bgo_repair_with_transport(
            &PicoEh55BgoRepairRequestDto {
                nor_path: None,
                plan_path: Some(dir.join("plan.json").to_string_lossy().into()),
                reference_nor_path: None,
                out_plan_path: None,
                cow_out_path: None,
                dry_run_only: false,
                port_name: Some("MOCK".into()),
                backup_dir: Some(dir.join("bkp").to_string_lossy().into()),
                confirm_token: Some("WRONG".into()),
                run_boot_trace_ms: None,
                nonce_id: None,
            },
            Some(mock_deny),
            |_| {},
        )
        .is_err()
    );

    let mut mock_ok = MockPanamon1Transport::new();
    mock_ok.bridge_active = true;
    mock_ok.map_memory(AM34_FLASH_UNCACHED_BASE, nor);
    mock_ok.boottrace_reply = "***** normal mode *****\r\nBOOTTRACE: OK\r\nPANADIGA> ".into();
    let mut bgo_events = Vec::new();
    let live_bgo = run_eh55_bgo_repair_with_transport(
        &PicoEh55BgoRepairRequestDto {
            nor_path: None,
            plan_path: Some(dir.join("plan.json").to_string_lossy().into()),
            reference_nor_path: None,
            out_plan_path: None,
            cow_out_path: None,
            dry_run_only: false,
            port_name: Some("MOCK".into()),
            backup_dir: Some(dir.join("bkp").to_string_lossy().into()),
            confirm_token: Some("CONFIRM_UPPER_NOR".into()),
            run_boot_trace_ms: Some(5000),
            nonce_id: None,
        },
        Some(mock_ok),
        |ev| bgo_events.push(ev),
    )
    .unwrap();
    assert!(live_bgo.sa0_to_sa3_locked_e05);
    assert_eq!(live_bgo.live_report.unwrap().boot_normal_ok, Some(true));
    assert!(!bgo_events.is_empty());

    let mut dmock = MockPanamon1Transport::new();
    dmock.bridge_active = true;
    dmock.map_memory(0x8900_0000, vec![0u8; 0x1000]);
    dmock.map_memory(0x504F_8234, vec![0u8; 0x40]);
    dmock.map_memory(0x9050_0000, vec![0u8; 0x3000]);
    let mut sys = vec![0u8; 0x0C];
    sys[8..10].copy_from_slice(&0x8002u16.to_le_bytes());
    dmock.map_memory(0xC000_1000, sys);
    let davis_res = run_davis_probe_with_transport(
        dmock,
        &PicoProbeDavisIdeRequestDto {
            port_name: "MOCK".into(),
            drive: Some("master".into()),
            boot_mode: None,
            boot_timeout_ms: None,
            nonce_id: None,
        },
    )
    .unwrap();
    assert!(davis_res.lethal_0xc000100c_blocked);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn operation_nonce_requires_single_use_exact_binding() {
    let state = panadiga_lib::state::PanadigaBackendState::new();
    assert!(
        consume_operation_nonce_with_state(
            &state,
            None,
            OpKind::NorWrite,
            "SA26..SA70",
            "live_plan",
            true
        )
        .is_err()
    );
    let bad_target = issue_operation_nonce_with_state(
        &state,
        &RequestOperationNonceDto {
            op_kind: OpKind::NorWrite,
            target: "SA26..SA70".into(),
            input_sha256: "live_plan".into(),
        },
    )
    .unwrap();
    // Mismatch fails closed AND burns the single-use challenge immediately.
    assert!(
        consume_operation_nonce_with_state(
            &state,
            Some(&bad_target.nonce_id),
            OpKind::NorWrite,
            "SA26..SA71",
            "live_plan",
            true
        )
        .is_err()
    );
    assert!(
        consume_operation_nonce_with_state(
            &state,
            Some(&bad_target.nonce_id),
            OpKind::NorWrite,
            "SA26..SA70",
            "live_plan",
            true
        )
        .is_err()
    );

    let bad_input = issue_operation_nonce_with_state(
        &state,
        &RequestOperationNonceDto {
            op_kind: OpKind::NorWrite,
            target: "SA26..SA70".into(),
            input_sha256: "live_plan".into(),
        },
    )
    .unwrap();
    assert!(
        consume_operation_nonce_with_state(
            &state,
            Some(&bad_input.nonce_id),
            OpKind::NorWrite,
            "SA26..SA70",
            "other_plan",
            true
        )
        .is_err()
    );

    let issued = issue_operation_nonce_with_state(
        &state,
        &RequestOperationNonceDto {
            op_kind: OpKind::NorWrite,
            target: "SA26..SA70".into(),
            input_sha256: "LIVE_PLAN".into(),
        },
    )
    .unwrap();
    let consumed = consume_operation_nonce_with_state(
        &state,
        Some(&issued.nonce_id),
        OpKind::NorWrite,
        "SA26..SA70",
        "live_plan",
        true,
    )
    .unwrap();
    assert_eq!(
        consumed.as_ref().map(|c| c.nonce_id.as_str()),
        Some(issued.nonce_id.as_str())
    );
    assert!(
        consume_operation_nonce_with_state(
            &state,
            Some(&issued.nonce_id),
            OpKind::NorWrite,
            "SA26..SA70",
            "live_plan",
            true
        )
        .is_err()
    );
    assert_eq!(state.bench_stream.snapshot().seq, 0);
}

#[test]
fn phase_b_security_invariants_csp_capabilities_gdb_and_nonces() {
    use panadiga_lib::commands::hardware_diag::require_destructive_nonce;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(manifest_dir.join("tauri.conf.json")).unwrap())
            .unwrap();
    let csp = tauri_conf["app"]["security"]["csp"]
        .as_str()
        .expect("CSP must be configured");
    assert!(!csp.contains("unsafe-inline"));
    for directive in [
        "object-src 'none'",
        "base-uri 'none'",
        "frame-ancestors 'none'",
        "form-action 'none'",
    ] {
        assert!(
            csp.contains(directive),
            "missing CSP directive: {directive}"
        );
    }

    let cap: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(manifest_dir.join("capabilities/default.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(cap["windows"], json!(["main"]));
    let perms: Vec<&str> = cap["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(perms.contains(&"core:default"));
    assert!(perms.contains(&"core:event:allow-listen"));
    assert!(perms.contains(&"core:event:allow-unlisten"));
    assert!(perms.iter().all(|p| {
        !p.contains("shell:") && !p.contains("fs:") && !p.contains("dialog:") && !p.contains('*')
    }));

    let state = panadiga_lib::state::PanadigaBackendState::new();
    assert!(
        panadiga_lib::pico::start_gdb_bridge(&state, "COM10".into(), "0.0.0.0:3333".into())
            .is_err()
    );
    assert!(
        panadiga_lib::pico::start_gdb_bridge(&state, "COM10".into(), "192.168.1.10:3333".into())
            .is_err()
    );

    for (kind, target, input_ref) in [
        (OpKind::NorWrite, "PhysicalDrive2", "hdd_img"),
        (
            OpKind::SdKitBuild,
            "PhysicalDrive9",
            "CONFIRM-PhysicalDrive9",
        ),
        (OpKind::RamProbe, "master", "COM10"),
    ] {
        assert!(require_destructive_nonce(&state, None, kind, target, input_ref).is_err());
        let ch = issue_operation_nonce_with_state(
            &state,
            &RequestOperationNonceDto {
                op_kind: kind,
                target: target.into(),
                input_sha256: input_ref.into(),
            },
        )
        .unwrap();
        assert!(
            require_destructive_nonce(&state, Some(&ch.nonce_id), kind, "WRONG_TARGET", input_ref)
                .is_err()
        );
        // Failed check consumes nonce; mint fresh valid one and verify single-use replay rejection.
        let fresh = issue_operation_nonce_with_state(
            &state,
            &RequestOperationNonceDto {
                op_kind: kind,
                target: target.into(),
                input_sha256: input_ref.into(),
            },
        )
        .unwrap();
        assert!(
            require_destructive_nonce(&state, Some(&fresh.nonce_id), kind, target, input_ref)
                .is_ok()
        );
        assert!(
            require_destructive_nonce(&state, Some(&fresh.nonce_id), kind, target, input_ref)
                .is_err()
        );
    }
}
