use super::*;

#[test]
fn decodes_utf16le_bom_and_bootcap_sense() {
    let sample = "***** Loader ver0.61 [06/03/06] *****\r\n\
                  ***** normal mode *****\r\n\
                  BGO driver init failed[1].\r\n\
                  [RTSD] === DRIVE ACCESS ERROR === ( 80045300 )\r\n\
                  *** SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00]\r\n\
                  VDD: 3.24V (OK)\r\n";
    let mut utf16_bytes = vec![0xFFu8, 0xFEu8];
    for unit in sample.encode_utf16() {
        utf16_bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let text = decode_captured_log_bytes(&utf16_bytes);
    let report = parse_bench_log_text(&text);

    assert_eq!(report.boot_mode.as_deref(), Some("normal"));
    assert_eq!(report.bgo_failed_code, Some(1));
    assert_eq!(report.drive_error_codes, vec!["80045300".to_string()]);
    assert_eq!(report.sense_packets.len(), 1);
    assert_eq!(
        report.sense_packets[0].mecha_error_code.as_deref(),
        Some("0x83A7")
    );
    assert_eq!(report.vdd_volts, Some(3.24));
}

#[test]
fn detects_sd_bus_ok_with_missing_fat16_digadiag() {
    let sample = "DDR CHECK: OK\r\n\
                  ***** Loader ver0.61 [06/03/06] *****\r\n\
                  ***** serial boot mode *****\r\n\
                  capacity:3bb000[LBA](2002780160Byte) \t transfer speed:25000[Kbps]\r\n\
                  RCA:2 \t Bus Width:4(current) 4(support)\r\n\
                  SD = 0x3BB000[LBA]\r\n\
                  ... SD init end\r\n\
                  SD diag load failed[0].\r\n\
                  ***** normal mode *****\r\n\
                  BGO driver init failed[1].\r\n\
                  ** can not close tray **\r\n";
    let report = parse_bench_log_text(sample);
    assert_eq!(report.sd_capacity_lba, Some(0x003B_B000));
    assert_eq!(report.sd_capacity_bytes, Some(2_002_780_160));
    assert_eq!(report.sd_bus_width, Some(4));
    assert_eq!(report.sd_transfer_kbps, Some(25_000));
    assert!(report.sd_hw_init_ok);
    assert_eq!(report.sd_diag_load_failed, Some(0));
    assert!(report.odd_tray_failed);
    assert_eq!(report.bgo_failed_code, Some(1));
    assert!(report.boot_stages.contains(&"SD_HW_OK".to_string()));
    assert!(
        report
            .boot_stages
            .contains(&"SD_DIAG_LOAD_FAIL".to_string())
    );
    assert!(report.diagnosis.contains("SD diag load failed[0]"));
    assert!(report.notes.iter().any(|n| n.contains("FAT16")));
}

#[test]
fn decodes_sdcd_error_variants_without_brackets() {
    let sample = "[SDCD]SetBusWidth:SD Status ERROR -91006\r\n\
                  [SDCD] Set SD Bus Width fail ERROR:-91006\r\n\
                  SD init failed[-90002].\r\n";
    let report = parse_bench_log_text(sample);
    assert_eq!(report.sdcd_errors, vec![-91006, -90002]);
    assert!(report.notes.iter().any(|n| n.contains("-91006")));
}

#[test]
fn rejoins_interleaved_diagwatch_target_lines() {
    let sample = concat!(
        "DDR CHECK: OK\r\n",
        "***** Loader ver0.61 [06/03/06] *****\r\n",
        "***** serial boot mode\r\n",
        "[DIAGWATCH +26ms] serial boot mode -> SW_UART released Hi-Z (Q7514 TX ready)\r\n",
        " *****\r\n",
        "SD = 0x3BB000[LBA]\r\n",
        "... SD init end\r\n",
        "[DIAGWATCH +1453ms] STAGE: SD hardware 4-bit init OK (... SD init end)\r\n",
        "\r\n",
        "SD diag load failed\r\n",
        "[DIAGWATCH +1504ms] ALERT: SD diag load failed[0] (FUN_4400a97c)\r\n",
        "[0].\r\n",
        "***** normal mode *****\r\n",
        "BGO driver init failed\r\n",
        "[DIAGWATCH +3599ms] ALERT: BGO driver init failure detected\r\n",
        "[1].\r\n",
        "** can not close tray\r\n",
        "[DIAGWATCH +6786ms] ALERT: ODD tray close error detected\r\n",
        " **\r\n",
    );
    let report = parse_bench_log_text(sample);
    assert_eq!(report.boot_mode.as_deref(), Some("normal"));
    assert!(report.sd_hw_init_ok);
    assert_eq!(report.sd_diag_load_failed, Some(0));
    assert_eq!(report.bgo_failed_code, Some(1));
    assert!(report.odd_tray_failed);
    assert!(report.sdcd_errors.is_empty());
    assert!(report.diagnosis.contains("SD diag load failed[0]"));
    assert!(report.diagnosis.contains("fat_read 0x20000 failed"));
}

#[test]
fn describes_boot_fat_error_codes_and_silent_fat_read() {
    assert!(describe_boot_fat_error_code(-10010).contains("FAT_ERR_FILE_NOT_FOUND"));
    assert!(describe_boot_fat_error_code(-10011).contains("FAT_ERR_NO_FREE_FD"));
    assert!(describe_boot_fat_error_code(-10020).contains("FAT_ERR_INVALID_ARG"));
    assert!(describe_boot_fat_error_code(-10025).contains("FAT_ERR_INVALID_FS_OR_IO"));
    assert!(describe_boot_fat_error_code(-10028).contains("FAT_ERR_CORRUPT_CLUSTER"));
    assert!(describe_boot_fat_error_code(-10030).contains("FAT_ERR_EOF"));

    let mount_fail = "SD = 0x3BB000[LBA]\r\n... SD init end\r\nFAT Mount failed[-10025].\r\nSD diag load failed[0].\r\n";
    let rep_m = parse_bench_log_text(mount_fail);
    assert_eq!(rep_m.fat_mount_failed, Some(-10025));
    assert!(rep_m.diagnosis.contains("FAT_ERR_INVALID_FS_OR_IO"));
}
