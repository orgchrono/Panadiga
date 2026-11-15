use super::*;
use crate::crypto::FrmContainerMode;

#[test]
fn matshita_boot_sequence_covers_healthy_unlock_and_bench_fault_modes() {
    let mut healthy = MatshitaOddEmulator::new(MatshitaBootScenario::HealthyWithDisc);
    let rep_ok = healthy.run_boot_sequence().expect("healthy boot");
    assert!(rep_ok.startup_succeeded);
    assert!(rep_ok.inquiry_id.contains("MATSHITA"));
    assert_eq!(rep_ok.final_boot_code_hex, "0x00000000");

    let mut unlock = MatshitaOddEmulator::new(MatshitaBootScenario::UnlockDiscFrm);
    let rep_unlock = unlock.run_boot_sequence().expect("unlock disc boot");
    assert!(rep_unlock.startup_succeeded);
    assert!(rep_unlock.unlock_media_verified);
    assert_eq!(rep_unlock.jump_target_va_hex.as_deref(), Some("0x50400000"));

    let mut tray_err = MatshitaOddEmulator::new(MatshitaBootScenario::TrayCannotClose80045300);
    let rep_tray = tray_err.run_boot_sequence().expect("tray error trace");
    assert!(!rep_tray.startup_succeeded);
    assert_eq!(rep_tray.final_boot_code_hex, "0x80045300");

    let mut apc_err = MatshitaOddEmulator::new(MatshitaBootScenario::LaserApcFailure80040904);
    let rep_apc = apc_err.run_boot_sequence().expect("apc error trace");
    assert!(!rep_apc.startup_succeeded);
    assert_eq!(rep_apc.final_boot_code_hex, "0x80040904");
}

#[test]
fn atapi_write_injection_and_serial_pass_unlock_enforce_hardware_safety() {
    let mut emu = MatshitaOddEmulator::new(MatshitaBootScenario::HealthyWithDisc);
    let payload = b"PANADIGA_UNLOCK_STUB_0123456789ABCDEF";

    // ATAPI 0x3B WRITE BUFFER -> 0x3C READ BUFFER roundtrip
    let inj_3b = emu
        .inject_write_buffer_and_verify(0x100, 0x05, false, payload)
        .expect("0x3B injection");
    assert_eq!(inj_3b.cdb0_or_cmd, "0x3B");
    assert_eq!(inj_3b.target_va_or_lba, 0x5040_0100);

    // ATAPI 0xED (FUN_400209f3) sub=0x91 prep + 0x93 commit -> 0x3C readback
    let inj_ed = emu
        .inject_write_buffer_and_verify(0, 0, true, b"VENDOR_ED_OK")
        .expect("0xED injection");
    assert_eq!(inj_ed.cdb0_or_cmd, "0xED");

    // Factory Serial CLI: WB blocked until PASS 0x4d415453 0x48495441
    assert!(
        emu.execute_drv_serial_command("WB 0x50400010 0xAA")
            .is_err()
    );
    let pass_res = emu
        .execute_drv_serial_command("PASS 0x4d415453 0x48495441")
        .expect("PASS unlock");
    assert_eq!(pass_res.authorized_flag_cd8, 0x01);
    assert_eq!(pass_res.s211_flag_d78, 0x02);

    // After PASS, guarded RAM/buffer WW succeeds
    let ww_res = emu
        .execute_drv_serial_command("WW 0x50400010 0xCAFEBABE")
        .expect("WW after PASS");
    assert_eq!(ww_res.status, "WRITTEN");

    // Hardware safety invariant: IPL NOR must ALWAYS fail even after PASS unlock
    assert!(
        emu.execute_drv_serial_command("WB 0x80000010 0xFF")
            .is_err()
    );
    assert!(
        emu.execute_drv_serial_command("WW 0x84000000 0x12345678")
            .is_err()
    );
}

#[test]
fn builds_and_verifies_cd_and_sd_unlock_frm_containers() {
    let (_, dvd_sum) =
        build_sample_unlock_frm(FrmContainerMode::Dvd, b"DVD_UNLOCK_IMAGE").expect("dvd frm");
    assert!(dvd_sum.verified);
    assert_eq!(dvd_sum.container_kind, "PANA_DVD.FRM");
    assert_eq!(dvd_sum.bootloader_jump_va_hex, "0x50400000");

    let (_, sd_sum) =
        build_sample_unlock_frm(FrmContainerMode::Sd, b"SD_UNLOCK_IMAGE").expect("sd frm");
    assert!(sd_sum.verified);
    assert_eq!(sd_sum.bootloader_jump_va_hex, "0x50400000");
}
