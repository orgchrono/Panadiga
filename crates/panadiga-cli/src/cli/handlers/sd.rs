#![deny(unsafe_code)]

//! Handlers for `panadiga sd targets` and `panadiga sd create`.

use anyhow::{Result, bail};
use panadiga_core::log_println;
use panadiga_core::re::unbrick_sd_kit::{
    SdCardCreatorRequest, create_sd_recovery_card_on_device, create_sd_recovery_image,
    inspect_sd_card_targets,
};

use super::hdd::flash::normalize_flash_confirm_token;
use super::json_io::emit_json_output;
use crate::cli::args::{SdActions, SdArgs, SdCreateArgs};

fn run_sd_create(args: SdCreateArgs) -> Result<()> {
    let has_proprietary_source = args.digadiag.is_some()
        || args.pana_dvd.is_some()
        || args.include_pana_sd
        || args.pana_sd.is_some();
    let request = SdCardCreatorRequest {
        device_path: args.device.clone(),
        confirm_token: args
            .confirm_token
            .as_deref()
            .map(normalize_flash_confirm_token),
        output_image_path: args.out.clone(),
        model: args.model,
        volume_mb: args.volume_mb,
        loader_profile: args.loader_profile.parse()?,
        digadiag_path: args.digadiag,
        pana_sd_path: args.pana_sd.clone(),
        pana_dvd_path: args.pana_dvd,
        include_pana_sd: args.include_pana_sd || args.pana_sd.is_some(),
        cleanroom_only: args.cleanroom || !has_proprietary_source,
    };

    let report = match (args.out.as_ref(), args.device.as_ref()) {
        (Some(out_path), None) => create_sd_recovery_image(&request, out_path)?,
        (None, Some(_)) => create_sd_recovery_card_on_device(&request, |prog| {
            log_println!("[SD/{:?}] {}", prog.stage, prog.detail);
        })?,
        (Some(_), Some(_)) => {
            bail!("Specify either --out <IMG_FILE> or --device <DEVICE>, not both")
        }
        (None, None) => bail!(
            "Missing destination: pass --out <IMG_FILE> for offline image creation or --device <DEVICE> --confirm-token CONFIRM-... to flash an SD card"
        ),
    };

    log_println!(
        "[OK] FAT16B (0x06, {} MiB) SD recovery {} ready -> {} ({} B, cleanroom={}, sha256={}, audit={})",
        report.volume_mb,
        report.target_kind,
        report.target,
        report.bytes_written,
        report.cleanroom_only,
        report.image_sha256,
        report.inspection.overall_status
    );
    log_println!(
        "     Mn103Vm Pre-Write Gate: passed={} | steps={} | ipl_untouched={} | artifact={}",
        report.emu_gate.passed,
        report.emu_gate.executed_steps,
        report.emu_gate.ipl_untouched,
        report.emu_gate.artifact_kind
    );
    if report.advisory.is_sdxc_warning {
        log_println!("[WARN] {}", report.advisory.detail);
    }
    for step in &report.next_steps {
        log_println!("  {step}");
    }
    emit_json_output(&report, args.json.as_deref())?;
    Ok(())
}

/// Dispatches `panadiga sd` subcommands.
pub fn handle_sd(args: SdArgs) -> Result<()> {
    match args.action {
        SdActions::Targets { json } => {
            let targets = inspect_sd_card_targets();
            for t in &targets {
                log_println!(
                    "{} [role={:?}, {:.2} GB, safe={}, class={}]",
                    t.device_path,
                    t.drive_role,
                    t.size_gb,
                    t.safe_sd_target,
                    t.advisory.card_class
                );
            }
            emit_json_output(&targets, json.as_deref())?;
            Ok(())
        }
        SdActions::Create(create) => run_sd_create(*create),
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::args::{Cli, Commands};
    use crate::cli::handlers::dispatch;
    use clap::Parser;
    use panadiga_core::constants::PANA_FEISTEL_ROUND_KEY_32B;
    use panadiga_core::crypto::{FrmContainerMode, FrmPackModule, pack_frm_with_key};
    use panadiga_core::re::bgo_unbrick::{
        OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ,
        ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PROG061_EXPECTED_SIZE,
        Prog061BgoPatchMode, patch_prog061_bgo_in_place,
    };
    use panadiga_core::re::inspect_sd_target;
    use std::fs;

    fn build_valid_prog061() -> Vec<u8> {
        let mut p = vec![0xFFu8; PROG061_EXPECTED_SIZE];
        p[0..4].copy_from_slice(&[0xDC, 0x14, 0x00, 0x00]);
        p[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
        p[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2].copy_from_slice(&ORIG_DEVINFO_BEQ);
        p[OFF_BGO_INIT_CALL..OFF_BGO_INIT_CALL + 7].copy_from_slice(&ORIG_A2BC_BGO_CALL);
        p[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20].copy_from_slice(&ORIG_A2CD_HOOK);
        patch_prog061_bgo_in_place(&mut p, Prog061BgoPatchMode::UartMonitor).unwrap();
        p
    }

    #[test]
    fn test_cli_sd_create_parses_and_writes_verified_32mb_image() {
        let dir = std::env::temp_dir().join(format!("panadiga_cli_sd_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let diag_path = dir.join("DIGADIAG.FRM");
        let pana_path = dir.join("PANA__SD.FRM");
        let out_img = dir.join("eh55_recovery_32mb.img");
        let json_out = dir.join("sd_report.json");

        let prog = build_valid_prog061();
        let pana_sd = pack_frm_with_key(
            &[FrmPackModule::new("PROG061", prog.clone())],
            FrmContainerMode::Sd48,
            Some(0x1800),
            &PANA_FEISTEL_ROUND_KEY_32B,
        )
        .unwrap()
        .0;
        fs::write(&diag_path, &prog).unwrap();
        fs::write(&pana_path, &pana_sd).unwrap();

        let cli = Cli::try_parse_from([
            "panadiga",
            "sd",
            "create",
            "--out",
            out_img.to_str().unwrap(),
            "--model",
            "DMR-EH55",
            "--volume-mb",
            "32",
            "--digadiag",
            diag_path.to_str().unwrap(),
            "--pana-sd",
            pana_path.to_str().unwrap(),
            "--json",
            json_out.to_str().unwrap(),
        ])
        .unwrap();

        dispatch(cli.command.unwrap()).unwrap();
        assert_eq!(fs::metadata(&out_img).unwrap().len(), 65_599 * 512);
        let inspected = inspect_sd_target(out_img.to_str().unwrap()).unwrap();
        assert_eq!(inspected.overall_status, "PASS");
        assert!(inspected.pana_sd_entry.is_some());
        assert!(json_out.is_file());

        let missing_dest = Cli::try_parse_from(["panadiga", "sd", "create"]).unwrap();
        let Some(Commands::Sd(sd_args)) = missing_dest.command else {
            panic!("Expected Sd command");
        };
        assert!(dispatch(Commands::Sd(sd_args)).is_err());

        let dev_cli = Cli::try_parse_from([
            "panadiga",
            "sd",
            "create",
            "--device",
            "/dev/sdb",
            "--confirm-token",
            "/dev/sdb",
            "--volume-mb",
            "64",
        ])
        .unwrap();
        assert!(matches!(dev_cli.command, Some(Commands::Sd(_))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cli_sd_create_cleanroom_only_passes_emu_gate() {
        let dir = std::env::temp_dir().join(format!("panadiga_cli_sd_cr_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let out_img = dir.join("eh55_cleanroom_32mb.img");
        let json_out = dir.join("sd_cleanroom_report.json");

        let cli = Cli::try_parse_from([
            "panadiga",
            "sd",
            "create",
            "--out",
            out_img.to_str().unwrap(),
            "--cleanroom",
            "--json",
            json_out.to_str().unwrap(),
        ])
        .unwrap();
        dispatch(cli.command.unwrap()).unwrap();

        let inspected = inspect_sd_target(out_img.to_str().unwrap()).unwrap();
        assert_eq!(inspected.overall_status, "PASS");
        let rep_json = fs::read_to_string(&json_out).unwrap();
        assert!(rep_json.contains("\"cleanroom_only\": true"));
        assert!(rep_json.contains("\"passed\": true"));
        let _ = fs::remove_dir_all(&dir);
    }
}
