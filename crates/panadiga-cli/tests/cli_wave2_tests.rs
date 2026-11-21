#![deny(unsafe_code)]

use clap::Parser;
use panadiga_cli::cli::args::hdd::{FlashEh55Args, HddExpectedRangeArgs};
use panadiga_cli::cli::args::pico::{CaptureNorArgs, DavisProbeArgs};
use panadiga_cli::cli::args::unbrick::BgoLiveApplyArgs;
use panadiga_cli::cli::handlers::hdd::flash::{run_flash_eh55_on_device, run_format_meihdfs};
use panadiga_cli::cli::handlers::pico::live_ops::{
    run_capture_nor_with_transport, run_davis_probe_with_transport,
};
use panadiga_cli::cli::handlers::unbrick::bgo_tbc::handle_bgo_live_apply_with_transport;
use panadiga_cli::cli::{Cli, dispatch};
use panadiga_core::constants::{
    AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE, AM34_NVRAM_SIZE, AM34_NVRAM_UNCACHED_BASE,
    AM34_SDRAM_UNCACHED_BASE, LBA_MIRROR_MBR_OFFSET, LBA_PARTITION_1_OFFSET,
    LBA_PARTITION_2_OFFSET, PANA_DEV_BIN_MAGIC, PANASONIC_HDD_MBR_FULL_SIGNATURE,
};
use panadiga_core::crypto::frm_pack::{FrmContainerMode, FrmPackModule, pack_frm};
use panadiga_core::hdd::writer::{BUS_TYPE_NVME, FileBackedMockFlashDevice};
use panadiga_core::pico::panamon1::MockPanamon1Transport;
use panadiga_core::re::davis_probe::{
    DAVIS_IDENTIFY_BUF_DEFAULT_VA, DAVIS_IDENTIFY_SENTINEL, DAVIS_STUB_DEFAULT_VA,
};
use panadiga_core::re::eh55_bgo::eh55_bgo_sector_offset;
use panadiga_core::re::nor_dumper::NorCaptureConfig;
use panadiga_core::re::prog061_uart_monitor::{
    PROG061_UART_MONITOR_VA, build_prog061_uart_monitor_with_offsets,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("panadiga_cli_w2_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn synthetic_corrupt_bgo_nor() -> Vec<u8> {
    let mut nor = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    for sa in 26..=70 {
        let base = eh55_bgo_sector_offset(sa).unwrap();
        nor[base + 0x100..base + 0x200].fill(0x00);
    }
    for sa in [29usize, 41] {
        let base = eh55_bgo_sector_offset(sa).unwrap();
        for i in 0..4 {
            nor[base + 0x04 + i * 4..base + 0x06 + i * 4].copy_from_slice(&0xF101u16.to_le_bytes());
        }
    }
    let mut pid_to_word: BTreeMap<u16, u16> = BTreeMap::new();
    let f2_sectors = (26usize..=67).filter(|sa| ![29, 41].contains(sa));
    for (idx, sa) in f2_sectors.enumerate() {
        let fid = u16::try_from(idx + 1).unwrap();
        let sec_base = eh55_bgo_sector_offset(sa).unwrap();
        let hdr = [fid as u8, 0xF2, 0xFF, 0xFF];
        for chunk in nor[sec_base + 4..sec_base + 20].as_chunks_mut::<4>().0 {
            chunk.copy_from_slice(&hdr);
        }
        nor[sec_base + 0xFC..sec_base + 0x100].fill(0);
        let pid = match fid {
            1 => 1,
            2..=31 => 0x0F05 + fid,
            _ => 0x0010 + fid,
        };
        nor[sec_base + 0x200..sec_base + 0x202].copy_from_slice(&pid.to_le_bytes());
        if pid == 1 {
            nor[sec_base + 0x400..sec_base + 0x404].copy_from_slice(&PANA_DEV_BIN_MAGIC);
            nor[sec_base + 0x404..sec_base + 0x40C].copy_from_slice(b"M6930LB\x00");
        } else if (0x0F07..=0x0F24).contains(&pid) {
            nor[sec_base + 0x400..sec_base + 0x404].copy_from_slice(&[0xED, 0xFF, 0xFF, 0xFF]);
        }
        pid_to_word.insert(pid, fid << 8);
    }
    let mut tids: BTreeMap<u16, [u16; 16]> = BTreeMap::new();
    for (pid, w) in pid_to_word {
        tids.entry(pid / 16).or_insert([0xFFFF; 16])[usize::from(pid % 16)] = w;
    }
    for (idx, (tid, arr)) in tids.into_iter().enumerate() {
        let off = eh55_bgo_sector_offset(41).unwrap() + (2 + idx / 7) * 0x100;
        let s = idx % 7;
        if s == 0 {
            nor[off..off + 4].copy_from_slice(&[0x00; 4]);
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
        nor[base + 0x100..base + 0x200].fill(0x00);
    }
    let sa70 = eh55_bgo_sector_offset(70).unwrap();
    nor[sa70 + 0xE100..sa70 + 0xE200].fill(0x00);
    nor
}

#[test]
fn wave2_cli_new_flags_and_subcommands_parse_and_preserve_defaults() {
    let cases = [
        vec![
            "panadiga",
            "unbrick",
            "prepare-sd",
            "--tv-standard",
            "pal-m",
            "--lbs-preset",
            "br-pal-m",
            "--hardened-sd",
        ],
        vec![
            "panadiga",
            "unbrick",
            "verify-sd",
            "--audit-profile",
            "hardened",
        ],
        vec![
            "panadiga",
            "unbrick",
            "tbc-plan",
            "--mode",
            "stage15_digadiag_tbc",
            "--standard",
            "pal-m",
        ],
        vec!["panadiga", "unbrick", "bgo-audit", "--nor", "nor.bin"],
        vec![
            "panadiga",
            "unbrick",
            "bgo-repair-plan",
            "--nor",
            "nor.bin",
            "--out-plan",
            "plan.json",
            "--cow-out",
            "cow.bin",
        ],
        vec![
            "panadiga",
            "unbrick",
            "bgo-live-apply",
            "--port",
            "COM10",
            "--plan",
            "plan.json",
            "--backup-dir",
            "bkp",
            "--confirm-upper-nor",
            "--no-boottrace",
        ],
        vec![
            "panadiga",
            "unbrick",
            "region-compat",
            "inspect-tv-bundle",
            "--preset",
            "br-pal-m",
            "--tv-standard",
            "pal-m",
        ],
        vec![
            "panadiga",
            "hdd",
            "flash-eh55",
            "--drive",
            r"\\.\PhysicalDrive1",
            "--image",
            "hdd.img",
            "--backup-dir",
            "bkp",
            "--confirm-physical-drive",
            "PhysicalDrive1",
            "--expected-min-gb",
            "240",
            "--expected-max-gb",
            "260",
            "--allow-extended-4tb",
        ],
        vec![
            "panadiga",
            "pico",
            "capture-nor",
            "--port",
            "COM10",
            "--slice",
            "bgo",
            "--out",
            "bgo.bin",
        ],
        vec![
            "panadiga",
            "pico",
            "davis-probe",
            "--port",
            "COM10",
            "--drive",
            "slave",
            "--no-identify",
        ],
        vec![
            "panadiga",
            "nvram",
            "audit-nor-sectors",
            "--image",
            "nor.bin",
        ],
    ];
    for argv in cases {
        let parsed = Cli::try_parse_from(&argv);
        assert!(parsed.is_ok(), "Failed parsing {argv:?}: {parsed:?}");
    }
}

#[test]
fn wave2_tbc_plan_and_nor_sector_audit_execute_offline() {
    let dir = temp_dir("tbc_nor");
    let tbc_json = dir.join("tbc_plan.json");
    let ok_cli = Cli::try_parse_from([
        "panadiga",
        "unbrick",
        "tbc-plan",
        "--mode",
        "stage15_digadiag_tbc",
        "--standard",
        "pal-m",
        "--json",
        tbc_json.to_str().unwrap(),
    ])
    .unwrap();
    dispatch(ok_cli.command.unwrap()).unwrap();
    assert!(
        fs::read_to_string(&tbc_json)
            .unwrap()
            .contains("stage15_digadiag_tbc")
    );

    let gated_cli = Cli::try_parse_from([
        "panadiga",
        "unbrick",
        "tbc-plan",
        "--mode",
        "experimental_sd_ram_chainload",
        "--standard",
        "ntsc",
    ])
    .unwrap();
    assert!(dispatch(gated_cli.command.unwrap()).is_err());

    let nor_path = dir.join("nor_8mb.bin");
    fs::write(&nor_path, vec![0xFFu8; AM34_FLASH_SIZE as usize]).unwrap();
    let nor_json = dir.join("nor_sectors.json");
    let nvram_cli = Cli::try_parse_from([
        "panadiga",
        "nvram",
        "audit-nor-sectors",
        "--image",
        nor_path.to_str().unwrap(),
        "--json",
        nor_json.to_str().unwrap(),
    ])
    .unwrap();
    dispatch(nvram_cli.command.unwrap()).unwrap();
    assert!(
        fs::read_to_string(&nor_json)
            .unwrap()
            .contains("\"total_sectors\": 142")
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn wave2_bgo_audit_repair_plan_and_mock_live_apply_enforce_e05() {
    let dir = temp_dir("bgo_live");
    let corrupt = synthetic_corrupt_bgo_nor();
    let nor_path = dir.join("corrupt.bin");
    let plan_path = dir.join("bgo_plan.json");
    let cow_path = dir.join("cow_repaired.bin");
    fs::write(&nor_path, &corrupt).unwrap();

    dispatch(
        Cli::try_parse_from([
            "panadiga",
            "unbrick",
            "bgo-audit",
            "--nor",
            nor_path.to_str().unwrap(),
        ])
        .unwrap()
        .command
        .unwrap(),
    )
    .unwrap();

    dispatch(
        Cli::try_parse_from([
            "panadiga",
            "unbrick",
            "bgo-repair-plan",
            "--nor",
            nor_path.to_str().unwrap(),
            "--out-plan",
            plan_path.to_str().unwrap(),
            "--cow-out",
            cow_path.to_str().unwrap(),
        ])
        .unwrap()
        .command
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::metadata(&cow_path).unwrap().len(),
        AM34_FLASH_SIZE as u64
    );

    let mut mock_deny = MockPanamon1Transport::new();
    mock_deny.map_memory(AM34_FLASH_UNCACHED_BASE, corrupt.clone());
    assert!(
        handle_bgo_live_apply_with_transport(
            mock_deny,
            BgoLiveApplyArgs {
                port: "MOCK".to_string(),
                plan: plan_path.clone(),
                backup_dir: dir.join("bkp"),
                confirm_upper_nor: false,
                no_boottrace: true,
                json: None,
            },
        )
        .is_err()
    );

    let mut mock_ok = MockPanamon1Transport::new();
    mock_ok.bridge_active = true;
    mock_ok.map_memory(AM34_FLASH_UNCACHED_BASE, corrupt);
    mock_ok.boottrace_reply = "***** normal mode *****\r\nPANADIGA> ".to_string();
    let out_json = dir.join("live_rep.json");
    handle_bgo_live_apply_with_transport(
        mock_ok,
        BgoLiveApplyArgs {
            port: "MOCK".to_string(),
            plan: plan_path,
            backup_dir: dir.join("bkp"),
            confirm_upper_nor: true,
            no_boottrace: false,
            json: Some(out_json.clone()),
        },
    )
    .unwrap();
    assert!(
        fs::read_to_string(&out_json)
            .unwrap()
            .contains("backup_sha256")
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn wave2_pico_capture_nor_and_davis_probe_mock_offline() {
    let dir = temp_dir("pico_live");
    let mut nor = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    let nvram_off = (AM34_NVRAM_UNCACHED_BASE - AM34_FLASH_UNCACHED_BASE) as usize;
    nor[nvram_off..nvram_off + 16].copy_from_slice(b"PANANVRAM_TEST_1");

    let cfg = NorCaptureConfig::try_default().unwrap();
    let (mon_payload, offsets) =
        build_prog061_uart_monitor_with_offsets(PROG061_UART_MONITOR_VA).unwrap();
    let (entry_u, start_imm_u, end_imm_u) = offsets.uncached_patch_vas(PROG061_UART_MONITOR_VA);
    let _ = cfg;
    let mut mock = MockPanamon1Transport::new();
    mock.bridge_active = true;
    mock.map_memory(
        (PROG061_UART_MONITOR_VA & 0x0FFF_FFFF) | AM34_SDRAM_UNCACHED_BASE,
        mon_payload,
    );
    mock.map_memory(AM34_FLASH_UNCACHED_BASE, nor);
    mock.enable_dynamic_nor_dumper(entry_u, start_imm_u, end_imm_u, offsets.block_size);

    let out_nvram = dir.join("nvram.bin");
    let cap = run_capture_nor_with_transport(
        mock,
        &CaptureNorArgs {
            port: "MOCK".to_string(),
            slice: "nvram".to_string(),
            start: None,
            len: None,
            out: out_nvram.clone(),
            json: None,
        },
    )
    .unwrap();
    assert_eq!(cap.total_len, AM34_NVRAM_SIZE as usize);
    assert_eq!(&fs::read(&out_nvram).unwrap()[..16], b"PANANVRAM_TEST_1");

    let mut dmock = MockPanamon1Transport::new();
    dmock.bridge_active = true;
    dmock.map_memory(0x8900_0000, vec![0u8; 0x1000]);
    dmock.map_memory(0x504F_8234, vec![0u8; 0x40]);
    dmock.map_memory(0x9050_0000, vec![0u8; 0x3000]);
    let mut sys = vec![0u8; 0x0C];
    sys[8..10].copy_from_slice(&0x8002u16.to_le_bytes());
    dmock.map_memory(0xC000_1000, sys);
    dmock.write_memory_bytes(DAVIS_STUB_DEFAULT_VA, &[]);
    let out_ec = dir.join("identify.bin");
    let rep = run_davis_probe_with_transport(
        dmock,
        &DavisProbeArgs {
            port: "MOCK".to_string(),
            drive: "master".to_string(),
            no_identify: true,
            out_ec: Some(out_ec.clone()),
            json: None,
        },
    )
    .unwrap();
    assert!(rep.identify_info.is_none());
    assert!(!out_ec.exists());
    let _ = DAVIS_IDENTIFY_BUF_DEFAULT_VA;
    let _ = DAVIS_IDENTIFY_SENTINEL;
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn wave2_hdd_format_meihdfs_inject_frm_and_flash_eh55_mock() {
    let dir = temp_dir("hdd_w2");
    let frm_path = dir.join("PANA__SD.FRM");
    let main_bytes = vec![0xA5u8; 2048];
    let drv_bytes = vec![0x5Au8; 1024];
    let (frm_bytes, _) = pack_frm(
        &[
            FrmPackModule::new("MAIN_EH55", main_bytes.clone()),
            FrmPackModule::new("DRV_S211", drv_bytes.clone()),
        ],
        FrmContainerMode::Sd48,
        None,
    )
    .unwrap();
    fs::write(&frm_path, frm_bytes).unwrap();

    let meihdfs_img = dir.join("disk_meihdfs.bin");
    {
        let f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&meihdfs_img)
            .unwrap();
        f.set_len(LBA_PARTITION_2_OFFSET + 4096).unwrap();
    }
    let target_str = meihdfs_img.to_str().unwrap();
    let confirm = format!("CONFIRM-{target_str}");
    let dry = run_format_meihdfs(target_str, 160, Some(&frm_path), &confirm, true, None).unwrap();
    assert!(dry.dry_run && dry.p1_bytes_written == 2048 && dry.p2_bytes_written == 1024);

    let real = run_format_meihdfs(target_str, 160, Some(&frm_path), &confirm, false, None).unwrap();
    assert_eq!(real.p1_bytes_written, 2048);
    let written = fs::read(&meihdfs_img).unwrap();
    assert_eq!(&written[6..8], b"SH");
    assert_eq!(
        &written[LBA_PARTITION_1_OFFSET as usize..LBA_PARTITION_1_OFFSET as usize + 2048],
        &main_bytes[..]
    );
    assert_eq!(
        &written[LBA_PARTITION_2_OFFSET as usize..LBA_PARTITION_2_OFFSET as usize + 1024],
        &drv_bytes[..]
    );

    let img_path = dir.join("eh55_primary.img");
    let mut primary = vec![0x3Cu8; 1024 * 1024];
    primary[..512].fill(0);
    primary[..16].copy_from_slice(&PANASONIC_HDD_MBR_FULL_SIGNATURE);
    fs::write(&img_path, &primary).unwrap();
    let mirror_p1_path = dir.join("eh55_p1_mirror.bin");
    fs::write(&mirror_p1_path, vec![0x7Du8; 64 * 1024]).unwrap();
    fs::create_dir_all(dir.join("flash_bkp")).unwrap();

    let mock_disk_path = dir.join("mock_250gb.bin");
    let disk_250gb: u64 = 250_000_000_000;
    let mut dev = FileBackedMockFlashDevice::create(&mock_disk_path, disk_250gb, 3).unwrap();
    let bad_args = FlashEh55Args {
        drive: r"\\.\PhysicalDrive0".to_string(),
        image: img_path.clone(),
        mirror_p1: Some(mirror_p1_path.clone()),
        backup_dir: dir.join("flash_bkp"),
        confirm_physical_drive: "PhysicalDrive0".to_string(),
        range: HddExpectedRangeArgs {
            expected_min_gb: None,
            expected_max_gb: None,
            allow_extended_4tb: false,
            json: None,
        },
    };
    assert!(run_flash_eh55_on_device(&mut dev, &bad_args).is_err());

    let mut nvme_dev =
        FileBackedMockFlashDevice::create(&mock_disk_path, disk_250gb, BUS_TYPE_NVME).unwrap();
    let ok_args = FlashEh55Args {
        drive: r"\\.\PhysicalDrive1".to_string(),
        image: img_path,
        mirror_p1: Some(mirror_p1_path),
        backup_dir: dir.join("flash_bkp"),
        confirm_physical_drive: "PhysicalDrive1".to_string(),
        range: HddExpectedRangeArgs {
            expected_min_gb: Some(240),
            expected_max_gb: Some(260),
            allow_extended_4tb: false,
            json: None,
        },
    };
    assert!(run_flash_eh55_on_device(&mut nvme_dev, &ok_args).is_err());

    let over_4tb = FlashEh55Args {
        range: HddExpectedRangeArgs {
            expected_min_gb: Some(240),
            expected_max_gb: Some(2500),
            allow_extended_4tb: false,
            json: None,
        },
        ..ok_args.clone()
    };
    assert!(run_flash_eh55_on_device(&mut dev, &over_4tb).is_err());

    let rep = run_flash_eh55_on_device(&mut dev, &ok_args).unwrap();
    assert!(rep.readback_verified);
    assert!(Path::new(&rep.backup_manifest_path).is_file());
    let _ = LBA_MIRROR_MBR_OFFSET;
    let _ = fs::remove_dir_all(&dir);
}
