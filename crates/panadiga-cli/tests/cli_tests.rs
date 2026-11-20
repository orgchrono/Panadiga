use clap::Parser;
use panadiga_cli::cli::{Cli, Commands};

#[test]
fn test_cli_args_parsing_matrix() {
    let cli = Cli::try_parse_from(["panadiga", "memory-map", "--format", "json"]).unwrap();
    match cli.command {
        Some(Commands::MemoryMap(args)) => {
            assert_eq!(args.format, "json");
            assert!(args.output.is_none());
        }
        _ => panic!("Expected MemoryMap command"),
    }

    let cli = Cli::try_parse_from([
        "panadiga",
        "gdb-config",
        "--port",
        "COM7",
        "--tcp-port",
        "4444",
    ])
    .unwrap();
    match cli.command {
        Some(Commands::GdbConfig(args)) => {
            assert_eq!(args.port, "COM7");
            assert_eq!(args.tcp_port, 4444);
        }
        _ => panic!("Expected GdbConfig command"),
    }

    assert!(matches!(
        Cli::try_parse_from(["panadiga", "pico", "list"])
            .unwrap()
            .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from(["panadiga", "pico", "flash", "--port", "COM3"])
            .unwrap()
            .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from(["panadiga", "pico", "status", "--port", "COM10"])
            .unwrap()
            .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from(["panadiga", "pico", "breakin", "--mode", "2"])
            .unwrap()
            .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "pico",
            "dump",
            "--port",
            "COM10",
            "--addr",
            "0x807F0000",
            "--words",
            "16"
        ])
        .unwrap()
        .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "pico",
            "gdb-bridge",
            "--port",
            "COM10",
            "--listen",
            "127.0.0.1:3333",
            "--target-mode",
            "panamon1"
        ])
        .unwrap()
        .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "pico",
            "bench-wizard",
            "--port",
            "COM10",
            "--profile",
            "prejig",
            "--wake-timeout-ms",
            "45000",
            "--non-interactive",
            "--vfd-cue",
            "2"
        ])
        .unwrap()
        .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from(["panadiga", "pico", "error-lookup", "0x83A7"])
            .unwrap()
            .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "gdb-bridge",
            "--port",
            "COM10",
            "--listen",
            "127.0.0.1:3333"
        ])
        .unwrap()
        .command,
        Some(Commands::GdbBridge { .. })
    ));

    assert!(matches!(
        Cli::try_parse_from(["panadiga", "hdd", "list-drives"])
            .unwrap()
            .command,
        Some(Commands::Hdd(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "hdd",
            "format-meihdfs",
            "--target",
            r"\\.\PhysicalDrive1",
            "--size-gb",
            "250",
            "--confirm",
            "CONFIRM-PhysicalDrive1",
        ])
        .unwrap()
        .command,
        Some(Commands::Hdd(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "hdd",
            "restore-mirrors",
            "--target",
            r"\\.\PhysicalDrive1",
            "--confirm",
            "CONFIRM-PhysicalDrive1",
        ])
        .unwrap()
        .command,
        Some(Commands::Hdd(_))
    ));

    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "hdd",
            "inspect-capacity",
            "EH55_MAIN_3410PQ.bin"
        ])
        .unwrap()
        .command,
        Some(Commands::Hdd(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "hdd",
            "inspect-capacity",
            "-i",
            "hdd_partition_firmware_0x02000000.bin",
            "--json",
            "cap.json",
        ])
        .unwrap()
        .command,
        Some(Commands::Hdd(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "hdd",
            "patch-capacity",
            "-i",
            "hdd_partition_firmware_0x02000000.bin",
            "--mode",
            "dynamic",
        ])
        .unwrap()
        .command,
        Some(Commands::Hdd(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "hdd",
            "patch-capacity",
            "-i",
            "hdd_partition_firmware_0x02000000.bin",
            "-o",
            "out.bin",
            "--mode",
            "cap",
            "--target-gb",
            "250",
        ])
        .unwrap()
        .command,
        Some(Commands::Hdd(_))
    ));

    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "frm",
            "pack",
            "--mode",
            "digadiag",
            "--input",
            "diag.bin",
            "--out",
            "DIGADIAG.FRM",
        ])
        .unwrap()
        .command,
        Some(Commands::Frm(_))
    ));

    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "frm",
            "unpack",
            "DIGADIAG.FRM",
            "--out",
            "diag_out.bin",
            "--mode",
            "plain",
        ])
        .unwrap()
        .command,
        Some(Commands::Frm(_))
    ));

    assert!(matches!(
        Cli::try_parse_from(["panadiga", "unbrick", "status"])
            .unwrap()
            .command,
        Some(Commands::Unbrick(_))
    ));
    assert!(matches!(
        Cli::try_parse_from(["panadiga", "unbrick", "prepare-sd"])
            .unwrap()
            .command,
        Some(Commands::Unbrick(_))
    ));
    assert!(matches!(
        Cli::try_parse_from(["panadiga", "unbrick", "verify-sd"])
            .unwrap()
            .command,
        Some(Commands::Unbrick(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "unbrick",
            "prepare-sd",
            "--outdir",
            "sd_out",
            "--loader-profile",
            "ver061",
            "--bgo-mode",
            "bypass",
            "--passthrough-mode",
            "av_passthrough",
        ])
        .unwrap()
        .command,
        Some(Commands::Unbrick(_))
    ));
    assert!(matches!(
        Cli::try_parse_from(["panadiga", "unbrick", "passthrough", "--rsp-plan"])
            .unwrap()
            .command,
        Some(Commands::Unbrick(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "unbrick",
            "nor-stub",
            "--wrap-digadiag",
            "--out",
            "DIGADIAG.FRM",
        ])
        .unwrap()
        .command,
        Some(Commands::Unbrick(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "unbrick",
            "matshita-emu",
            "boot",
            "--scenario",
            "unlock_disc_frm",
        ])
        .unwrap()
        .command,
        Some(Commands::Unbrick(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "nvram",
            "patch",
            "-i",
            "flash.bin",
            "--offset",
            "0x10",
            "--hex",
            "DEADBEEF",
        ])
        .unwrap()
        .command,
        Some(Commands::Nvram(_))
    ));

    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "re",
            "audit",
            "-i",
            "BOOT.BIN",
            "--base",
            "0x44000000"
        ])
        .unwrap()
        .command,
        Some(Commands::Re(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "re",
            "xrefs",
            "-i",
            "BOOT.BIN",
            "-t",
            "0x44007ba5"
        ])
        .unwrap()
        .command,
        Some(Commands::Re(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "re",
            "drv-cli",
            "-i",
            "DRV_decomp.bin",
            "--pass-unlocked"
        ])
        .unwrap()
        .command,
        Some(Commands::Re(_))
    ));
    assert!(matches!(
        Cli::try_parse_from(["panadiga", "pico", "decode-log", "-f", "boot.log"])
            .unwrap()
            .command,
        Some(Commands::Pico(_))
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "panadiga",
            "unbrick",
            "region-compat",
            "inspect-main",
            "-i",
            "MAIN.bin"
        ])
        .unwrap()
        .command,
        Some(Commands::Unbrick(_))
    ));

    let cli = Cli::try_parse_from(["panadiga", "-i"]).unwrap();
    assert!(cli.interactive);

    assert!(Cli::try_parse_from(["panadiga", "non-existent-subcommand"]).is_err());
}

#[test]
fn test_re_audit_and_pico_decode_log_handlers_roundtrip() {
    let dir = std::env::temp_dir().join("panadiga_cli_re_and_log_test");
    std::fs::create_dir_all(&dir).unwrap();

    let bin_path = dir.join("mini_boot.bin");
    let mut img = vec![0xCB_u8; 0x200];
    img[0x10..0x16].copy_from_slice(&[0xF9, 0x3A, 0x78, 0xFF, 0xF0, 0xFC]);
    img[0x40..0x43].copy_from_slice(&[0xF9, 0xEA, 0xC8]);
    std::fs::write(&bin_path, &img).unwrap();

    let re_json = dir.join("re_audit.json");
    panadiga_cli::cli::handlers::dispatch(
        Cli::try_parse_from([
            "panadiga",
            "re",
            "audit",
            "-i",
            bin_path.to_str().unwrap(),
            "--base",
            "0x44000000",
            "--json",
            re_json.to_str().unwrap(),
        ])
        .unwrap()
        .command
        .unwrap(),
    )
    .unwrap();
    let re_text = std::fs::read_to_string(&re_json).unwrap();
    assert!(re_text.contains("am33_f9_ea_sites"));
    assert!(re_text.contains("am33_ff_pre_rets_sites"));

    let log_path = dir.join("capture.log");
    std::fs::write(
        &log_path,
        "***** Loader ver0.61 [06/03/06] *****\r\n\
         ***** normal mode *****\r\n\
         BGO driver init failed[1].\r\n\
         [RTSD] === DRIVE ACCESS ERROR === ( 80045300 )\r\n\
         VDD: 3.24V (OK)\r\n",
    )
    .unwrap();
    let log_json = dir.join("capture.json");
    panadiga_cli::cli::handlers::dispatch(
        Cli::try_parse_from([
            "panadiga",
            "pico",
            "decode-log",
            "-f",
            log_path.to_str().unwrap(),
            "--json",
            log_json.to_str().unwrap(),
        ])
        .unwrap()
        .command
        .unwrap(),
    )
    .unwrap();
    let log_out = std::fs::read_to_string(&log_json).unwrap();
    assert!(log_out.contains("80045300"));
    assert!(log_out.contains("bgo_failed_code"));

    let _ = std::fs::remove_dir_all(&dir);
}
