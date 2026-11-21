#![deny(unsafe_code)]

//! Handlers for `panadiga dvd targets` and `panadiga dvd create` (ISO 9660 Level 1 @ 1x).

use anyhow::{Result, bail};
use panadiga_core::log_println;
use panadiga_core::re::unbrick_sd_kit::{
    DvdCreatorRequest, create_dvd_recovery_disc, create_dvd_recovery_iso, inspect_optical_burners,
};

use super::hdd::flash::normalize_flash_confirm_token;
use super::json_io::emit_json_output;
use crate::cli::args::{DvdActions, DvdArgs, DvdCreateArgs};

fn run_dvd_create(args: DvdCreateArgs) -> Result<()> {
    let has_prop = args.digadiag.is_some();
    let req = DvdCreatorRequest {
        burner_id: args.burner.clone(),
        confirm_token: args
            .confirm_token
            .as_deref()
            .map(normalize_flash_confirm_token),
        output_iso_path: args.out.clone(),
        model: args.model,
        volume_label: args.volume_label,
        loader_profile: args.loader_profile.parse()?,
        digadiag_path: args.digadiag,
        pana_dvd_path: args.pana_dvd,
        cleanroom_only: args.cleanroom || !has_prop,
        dry_run: args.dry_run,
    };

    let report = match (args.out.as_ref(), args.burner.as_ref()) {
        (Some(out_iso), None) => create_dvd_recovery_iso(&req, out_iso)?,
        (_, Some(_)) => create_dvd_recovery_disc(&req, |prog| {
            log_println!("[DVD {:?} {}%] {}", prog.stage, prog.percent, prog.detail);
        })?,
        (None, None) => bail!(
            "Missing destination: pass --out <ISO_FILE> or --burner <ID> --confirm-token CONFIRM-..."
        ),
    };

    log_println!(
        "[OK] ISO 9660 ({}) {} -> {} ({} B, policy={}, gate={}, audit={})",
        report.volume_label,
        report.target_kind,
        report.target,
        report.bytes_written,
        report.write_speed_policy,
        report.emu_gate.passed,
        report.inspection.overall_status
    );
    emit_json_output(&report, args.json.as_deref())?;
    Ok(())
}

/// Dispatches `panadiga dvd` subcommands.
pub fn handle_dvd(args: DvdArgs) -> Result<()> {
    match args.action {
        DvdActions::Targets { json } => {
            let burners = inspect_optical_burners();
            for b in &burners {
                log_println!(
                    "{} [{}] min_speed={} ({})",
                    b.display_name,
                    b.device_id,
                    b.min_write_speed_label,
                    b.os_backend
                );
            }
            emit_json_output(&burners, json.as_deref())?;
            Ok(())
        }
        DvdActions::Create(create) => run_dvd_create(*create),
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::args::Cli;
    use crate::cli::handlers::dispatch;
    use clap::Parser;
    use panadiga_core::re::inspect_iso9660_image;
    use std::fs;

    #[test]
    fn cli_dvd_create_builds_verified_cleanroom_iso9660_image() {
        let dir = std::env::temp_dir().join(format!("panadiga_cli_dvd_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let out_iso = dir.join("eh55_cleanroom.iso");
        let json_out = dir.join("dvd_report.json");

        let cli = Cli::try_parse_from([
            "panadiga",
            "dvd",
            "create",
            "--out",
            out_iso.to_str().unwrap(),
            "--cleanroom",
            "--json",
            json_out.to_str().unwrap(),
        ])
        .unwrap();
        dispatch(cli.command.unwrap()).unwrap();

        let iso_bytes = fs::read(&out_iso).unwrap();
        let inspected = inspect_iso9660_image(&iso_bytes).unwrap();
        assert_eq!(inspected.overall_status, "PASS");
        assert!(inspected.valid_pvd);
        assert_eq!(inspected.files[0].name_8_3, "DIGADIAG.FRM");
        let rep_json = fs::read_to_string(&json_out).unwrap();
        assert!(rep_json.contains("1x_or_drive_minimum_dao"));
        let _ = fs::remove_dir_all(&dir);
    }
}
