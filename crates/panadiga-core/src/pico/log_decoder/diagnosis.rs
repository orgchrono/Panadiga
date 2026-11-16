//! AM34 boot stage classification, FAT error mapping, and root-cause diagnostic notes.

/// Maps `/BOOT.BIN` FAT12/FAT16 negative return codes (`0xFFFFD8D2..0xFFFFD8E6`) to root causes.
pub fn describe_boot_fat_error_code(code: i32) -> &'static str {
    match code {
        -10010 => "FAT_ERR_FILE_NOT_FOUND (0x4400d16b: 8.3 entry missing in root dir)",
        -10011 => "FAT_ERR_NO_FREE_FD (0x4400d11e: fd table full or volume unmounted)",
        -10020 => {
            "FAT_ERR_INVALID_ARG (0x4400cfde/0x4400c9bd: invalid 8.3 filename or seek offset)"
        }
        -10025 => {
            "FAT_ERR_INVALID_FS_OR_IO (0x4400cb21/0x4400c696: bad MBR/BPB, FAT32 rejected, or SD sector read error)"
        }
        -10028 => "FAT_ERR_CORRUPT_CLUSTER (0x4400dfc8: FAT cluster entry < 2 or 0xFFF6/0xFFF7)",
        -10030 => "FAT_ERR_EOF (0x4400dfc8: read offset >= file_size)",
        _ => "FAT_ERR_UNKNOWN (/BOOT.BIN FAT16/FAT12 negative status)",
    }
}

pub(super) struct BootDiagnosisContext<'a> {
    pub(super) panamon_active: bool,
    pub(super) diag_started: bool,
    pub(super) sd_uncompress_failed: bool,
    pub(super) sd_diag_load_failed: Option<i32>,
    pub(super) sd_hw_init_ok: bool,
    pub(super) fat_mount_failed: Option<i32>,
    pub(super) fat_open_failed: Option<&'a (String, i32)>,
    pub(super) bgo_failed_code: Option<i32>,
    pub(super) sdcd_errors: &'a [i32],
    pub(super) odd_tray_failed: bool,
    pub(super) has_83a7: bool,
    pub(super) boot_mode: Option<&'a str>,
    pub(super) ddr_ok: bool,
}

pub(super) fn classify_boot_diagnosis(ctx: &BootDiagnosisContext<'_>) -> String {
    if ctx.panamon_active {
        return "PANAMON1 active on SC0 (ready for [B/R/W/D/F/J/E/Q])".to_string();
    }
    if ctx.diag_started {
        return "Stage-1 DIGADIAG.FRM decompressed & jumped (waiting PANAMON1> / factory UI)"
            .to_string();
    }
    if ctx.sd_uncompress_failed {
        let suffix = if ctx.bgo_failed_code.is_some() {
            " (masked by post-reset BGO)"
        } else {
            " (check container header/footer)"
        };
        return format!("Stage-1 DIGADIAG.FRM opened, EXTRHEAD/zlib unpack failed{suffix}");
    }
    if let Some(code) = ctx.sd_diag_load_failed {
        if let Some(merr) = ctx.fat_mount_failed {
            let desc = describe_boot_fat_error_code(merr);
            return format!(
                "Stage-1 SD HW OK (4-bit), FAT Mount failed[{merr}] ({desc}) before BGO reboot"
            );
        }
        if let Some((fname, oerr)) = ctx.fat_open_failed {
            let desc = describe_boot_fat_error_code(*oerr);
            return format!(
                "Stage-1 SD HW OK (4-bit), FAT Open({fname}) failed[{oerr}] ({desc}) before BGO reboot"
            );
        }
        if ctx.sd_hw_init_ok {
            let tail = if ctx.bgo_failed_code.is_some() {
                " before BGO reboot"
            } else {
                " (Mount+Open OK, DIGADIAG.FRM < 128KiB or cluster read fail)"
            };
            return format!(
                "Stage-1 SD HW OK (4-bit), FAILED at SD diag load failed[{code}] (FUN_4400a97c: Mount+Open OK, fat_read 0x20000 failed){tail}"
            );
        }
        return format!(
            "Stage-1 FAILED at SD diag load failed[{code}] (FUN_4400a97c: Mount+Open OK, fat_read 0x20000 failed)"
        );
    }
    if let Some(code) = ctx.bgo_failed_code {
        if ctx.has_83a7 {
            return format!(
                "BGO driver init failed[{code}] -> U99 hang (ODD 80040904 / 0x83A7 also observed)"
            );
        }
        if ctx.odd_tray_failed {
            return format!(
                "BGO driver init failed[{code}] -> U99 hang (ODD tray 80045300 also observed)"
            );
        }
        return format!("BGO driver init failed[{code}] -> U99 hang (load patched DIGADIAG.FRM)");
    }
    if ctx.sdcd_errors.contains(&-91006) {
        return "SDDIAG -91006 (4-bit ACMD6/ACMD13 SD_STATUS mismatch)".to_string();
    }
    if let Some(err) = ctx.sdcd_errors.first() {
        return format!("SDDIAG SD init error ({err})");
    }
    if ctx.has_83a7 {
        return "CDDIAG 80040904 / 0x83A7 (Laser APC < 0.5mW)".to_string();
    }
    if ctx.odd_tray_failed {
        return "ODD ** can not close tray ** (80045300 / no disc)".to_string();
    }
    if ctx.boot_mode == Some("serial_boot") {
        return "Serial boot mode entered (waiting SD/CD media)".to_string();
    }
    if ctx.ddr_ok {
        return "DDR CHECK: OK seen; boot in progress".to_string();
    }
    "No known Panasonic boot signatures observed".to_string()
}

pub(super) fn build_boot_stages(
    ctx: &BootDiagnosisContext<'_>,
    saw_serial_boot: bool,
    saw_normal_mode: bool,
) -> Vec<String> {
    let mut boot_stages = Vec::new();
    if ctx.ddr_ok {
        boot_stages.push("DDR_OK".to_string());
    }
    if saw_serial_boot {
        boot_stages.push("SERIAL_BOOT".to_string());
    }
    if ctx.sd_hw_init_ok {
        boot_stages.push("SD_HW_OK".to_string());
    }
    for code in ctx.sdcd_errors {
        if *code != 0 {
            boot_stages.push(format!("SD_ERR_{code}"));
        }
    }
    if ctx.sd_diag_load_failed.is_some() {
        boot_stages.push("SD_DIAG_LOAD_FAIL".to_string());
    }
    if ctx.sd_uncompress_failed {
        boot_stages.push("SD_UNCOMPRESS_FAIL".to_string());
    }
    if ctx.diag_started {
        boot_stages.push("DIAG_EXEC".to_string());
    }
    if ctx.panamon_active {
        boot_stages.push("PANAMON_ACTIVE".to_string());
    }
    if saw_normal_mode {
        boot_stages.push("NORMAL_MODE".to_string());
    }
    if ctx.bgo_failed_code.is_some() {
        boot_stages.push("BGO_FAIL".to_string());
    }
    if ctx.odd_tray_failed {
        boot_stages.push("ODD_NO_TRAY_80045300".to_string());
    }
    if ctx.has_83a7 {
        boot_stages.push("ODD_APC_80040904".to_string());
    }
    boot_stages
}

pub(super) fn build_diagnostic_notes(
    ctx: &BootDiagnosisContext<'_>,
    vdd_volts: Option<f32>,
) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(code) = ctx.bgo_failed_code {
        notes.push(format!(
            "BGO driver init failed[{code}] detected (CS1 NOR BGO corruption -> U99 loop unless bypassed via JIGBOOT/DIGADIAG.FRM)."
        ));
    }
    if ctx.sdcd_errors.contains(&-91006) {
        notes.push(
            "SDCD -91006 (ACMD6/ACMD13 status failure @ 0x44027F79): replace SD card or use standard SLC <=2GB card."
                .to_string(),
        );
    }
    if let Some(code) = ctx.sd_diag_load_failed {
        if let Some(merr) = ctx.fat_mount_failed {
            let desc = describe_boot_fat_error_code(merr);
            notes.push(format!("FAT Mount failed[{merr}]: {desc}"));
        } else if let Some((fname, oerr)) = ctx.fat_open_failed {
            let desc = describe_boot_fat_error_code(*oerr);
            notes.push(format!("FAT Open({fname}) failed[{oerr}]: {desc}"));
        } else if ctx.sd_hw_init_ok {
            notes.push(format!(
                "SD diag load failed[{code}] (FUN_4400a97c) without prior FAT Mount/Open error: FUN_4400c696 (fat_mount) and FUN_4400c879 (fat_open DIGADIAG.FRM) succeeded (FUN_4400b3d9 prints unconditionally on mount/open error), so FUN_4400c922 fat_read(0x93fc0000, 0x20000) returned != 0x20000 (DIGADIAG.FRM < 131072 B or SD cluster/multi-block read failure in FUN_4400dfc8 -> FUN_4400ca59). Check MBR/FAT16 cluster count <= 0xFFF5 and root 8.3 DIGADIAG.FRM."
            ));
        }
    }
    if ctx.odd_tray_failed {
        notes.push(
            "ODD tray close failure (** can not close tray ** / 80045300): check 4-pin tray motor/sensor cable."
                .to_string(),
        );
    }
    if let Some(v) = vdd_volts
        && v <= 0.10
    {
        notes.push(format!(
            "VDD={v:.2}V <= 0.10V (sticky-LOW / target rail off or unpowered P7501)."
        ));
    }
    notes
}
