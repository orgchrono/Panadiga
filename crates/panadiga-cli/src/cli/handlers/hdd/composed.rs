#![deny(unsafe_code)]

//! Handlers for streaming MEIHFS HDD flashing by composition (`panadiga hdd flash-composed`).

use anyhow::Result;
use panadiga_core::hdd::writer::{
    BlockFlashDevice, ComposedHddFlashReport, ComposedHddFlashRequest, WindowsPhysicalFlashDevice,
    execute_composed_meihfs_hdd_flash,
};
use panadiga_core::log_println;

use super::super::json_io::emit_json_output;
use super::flash::{normalize_flash_confirm_token, resolve_expected_gb_range};
use crate::cli::args::hdd::FlashComposedArgs;

/// Converts CLI `FlashComposedArgs` into a verified [`ComposedHddFlashRequest`].
pub(crate) fn build_composed_flash_request(
    args: &FlashComposedArgs,
) -> Result<ComposedHddFlashRequest> {
    let expected_gb_range = resolve_expected_gb_range(
        args.range.expected_min_gb,
        args.range.expected_max_gb,
        args.range.allow_extended_4tb,
    )?;
    Ok(ComposedHddFlashRequest {
        device_path: args.drive.clone(),
        confirmation_token: normalize_flash_confirm_token(&args.confirm_physical_drive),
        backup_dir: args.backup_dir.clone(),
        expected_gb_range,
        detected_os_disks: None,
        main_source_path: args.main_source.clone(),
        drv_source_path: args.drv_source.clone(),
        apply_dynamic_capacity_unlock: args.unlock_capacity,
        target_capacity_gb: args.target_gb,
        apply_av_passthrough: args.av_passthrough,
        custom_guid: None,
    })
}

/// Executes streaming MEIHFS composition flash onto any [`BlockFlashDevice`].
pub fn run_flash_composed_on_device<D: BlockFlashDevice>(
    device: &mut D,
    args: &FlashComposedArgs,
) -> Result<ComposedHddFlashReport> {
    let req = build_composed_flash_request(args)?;
    let report = execute_composed_meihfs_hdd_flash(device, &req, |p| {
        log_println!(
            "[HDD-COMPOSED {:?}] {}/{} B — {}",
            p.stage,
            p.bytes_done,
            p.bytes_total,
            p.detail
        );
    })?;
    log_println!(
        "[OK] Composed flash {} | sectors={} | P1={} B | P2={} B | gate={} ({} steps) | readback={}",
        report.device_path,
        report.total_sectors,
        report.p1_main_bytes_written,
        report.p2_drv_bytes_written,
        report.emu_gate.passed,
        report.emu_gate.executed_steps,
        report.readback_verified
    );
    emit_json_output(&report, args.range.json.as_deref())?;
    Ok(report)
}

pub(crate) fn handle_flash_composed(args: FlashComposedArgs) -> Result<()> {
    let _ = build_composed_flash_request(&args)?;
    let mut device = WindowsPhysicalFlashDevice::open(&args.drive, true)?;
    let _ = run_flash_composed_on_device(&mut device, &args)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::{Cli, Commands, HddActions};
    use clap::Parser;
    use panadiga_core::hdd::writer::FileBackedMockFlashDevice;
    use std::fs;

    #[test]
    fn test_cli_hdd_flash_composed_parses_and_passes_emu_gate_on_mock_disk() {
        let dir =
            std::env::temp_dir().join(format!("panadiga_cli_composed_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let backup_dir = dir.join("backups");
        fs::create_dir_all(&backup_dir).unwrap();
        let disk_path = dir.join("mock_disk.img");
        let json_out = dir.join("composed_report.json");
        let mut dev =
            FileBackedMockFlashDevice::create(&disk_path, 5 * 1024 * 1024 * 1024u64, 7).unwrap();

        let cli = Cli::try_parse_from([
            "panadiga",
            "hdd",
            "flash-composed",
            "--drive",
            r"\\.\PhysicalDrive4",
            "--backup-dir",
            backup_dir.to_str().unwrap(),
            "--confirm-physical-drive",
            "PhysicalDrive4",
            "--expected-min-gb",
            "5",
            "--expected-max-gb",
            "260",
            "--json",
            json_out.to_str().unwrap(),
        ])
        .unwrap();

        let Some(Commands::Hdd(hdd_args)) = cli.command else {
            panic!("Expected Hdd command");
        };
        let HddActions::FlashComposed(fc_args) = hdd_args.action else {
            panic!("Expected FlashComposed action");
        };
        let mut req = build_composed_flash_request(&fc_args).unwrap();
        req.detected_os_disks = Some(vec![r"\\.\PhysicalDrive0".to_string()]);
        let rep = execute_composed_meihfs_hdd_flash(&mut dev, &req, |_| {}).unwrap();
        assert!(rep.emu_gate.passed);
        assert!(rep.readback_verified);
        let _ = fs::remove_dir_all(&dir);
    }
}
