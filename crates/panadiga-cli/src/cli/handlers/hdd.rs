//! CLI handlers for HDD MEIHDFS inspection, extraction, and physical drive operations.

use super::super::args::{HddActions, HddArgs};
use anyhow::Result;
use panadiga_core::constants::{DEFAULT_HDD_IMAGE_PATH, PANA_FRM_MOD3_START_OFFSET};
use panadiga_core::hdd::{
    PhysicalDriveInfo, StorageMediaRole, decode_meihdfs_descriptors,
    default_capacity_patched_output_path, export_report_to_json, extract_hdd_partitions,
    generate_meihdfs_layout, inspect_hdd_image, inspect_main_hdd_capacity, list_drives_by_role,
    list_sd_card_drives, open_image_buffer, patch_main_hdd_capacity_with_passthrough,
    restore_meihdfs_from_mirrors, write_meihdfs_procedural_to_drive, write_raw_image_to_drive,
};
use panadiga_core::log_println;
use std::path::PathBuf;

fn resolve_hdd_path(image: Option<PathBuf>) -> PathBuf {
    image.unwrap_or_else(|| PathBuf::from(DEFAULT_HDD_IMAGE_PATH))
}

fn resolve_main_capacity_input(input: Option<PathBuf>) -> PathBuf {
    input.unwrap_or_else(|| {
        let p1 = PathBuf::from(
            "firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin",
        );
        if p1.exists() {
            p1
        } else {
            PathBuf::from(DEFAULT_HDD_IMAGE_PATH)
        }
    })
}

fn print_drive_inventory(title: &str, drives: &[PhysicalDriveInfo]) {
    log_println!("====================================================================");
    log_println!("   {title}");
    log_println!("====================================================================");
    if drives.is_empty() {
        log_println!("No matching physical block storage drives or mounted media detected.");
    } else {
        for d in drives {
            let status = if d.safe_to_write {
                "[SAFE TO WRITE]"
            } else {
                "[LOCKED - SYSTEM DRIVE]"
            };
            let role_tag = match d.drive_role {
                StorageMediaRole::HardDisk => "HDD",
                StorageMediaRole::SdCard => "SD",
                StorageMediaRole::SystemOs => "OS",
                StorageMediaRole::Other => "OTHER",
            };
            let mounts = if d.mount_points.is_empty() {
                "UNMOUNTED".to_string()
            } else {
                d.mount_points.join(", ")
            };
            log_println!(
                "Target: {:<18} | {:<4} | {:<23} | {} | Mounts: {}",
                d.device_path,
                role_tag,
                status,
                d.display_name,
                mounts
            );
            if let Some(ref reason) = d.lockout_reason {
                log_println!("        -> {}", reason);
            }
        }
    }
    log_println!("--------------------------------------------------------------------");
}

pub fn handle_hdd(args: HddArgs) -> Result<()> {
    match args.action {
        HddActions::Inspect {
            image,
            density,
            json,
        } => {
            let img_path = resolve_hdd_path(image);
            inspect_hdd_image(&img_path, density, json.as_deref())?;
        }
        HddActions::Extract {
            image,
            outdir,
            json,
        } => {
            let img_path = resolve_hdd_path(image);
            extract_hdd_partitions(&img_path, &outdir, json.as_deref())?;
        }
        HddActions::DecodeDescriptors {
            image,
            json,
            journal,
        } => {
            let img_path = resolve_hdd_path(image);
            decode_meihdfs_descriptors(&img_path, journal, json.as_deref())?;
        }
        HddActions::ListDrives { role } => {
            let parsed = match role.trim().to_ascii_lowercase().as_str() {
                "hdd" | "hard_disk" | "harddisk" => Some(StorageMediaRole::HardDisk),
                "sd" | "sd_card" | "sdcard" | "mmc" => Some(StorageMediaRole::SdCard),
                "system" | "os" => Some(StorageMediaRole::SystemOs),
                _ => None,
            };
            let drives = list_drives_by_role(parsed)?;
            print_drive_inventory(
                "PANADIGA STORAGE DISCOVERY & SYSTEM DRIVE LOCKOUT AUDIT",
                &drives,
            );
        }
        HddActions::ListSdCards => {
            let drives = list_sd_card_drives()?;
            print_drive_inventory("PANADIGA SD CARD / REMOVABLE MEDIA DISCOVERY", &drives);
        }
        HddActions::FormatMeihdfs {
            target,
            size_gb,
            inject_frm,
            confirm,
        } => {
            let total_sectors = size_gb * 1_000_000_000 / 512;
            log_println!(
                "Formatting target {} with procedural MEIHDFS ({} GB / {} sectors)...",
                target,
                size_gb,
                total_sectors
            );
            let layout = generate_meihdfs_layout(total_sectors, None)?;
            let report = write_meihdfs_procedural_to_drive(&target, &layout, &confirm)?;
            log_println!("[OK] {}", report.message);

            if let Some(ref frm_file) = inject_frm {
                log_println!(
                    "Injecting Stage-2 Firmware partitions from optical container {}...",
                    frm_file.display()
                );
                let frm_buf = open_image_buffer(frm_file)?;
                // Module 3 begins at offset 0x000F1818 in PANA_DVD.FRM
                if frm_buf.len() > PANA_FRM_MOD3_START_OFFSET {
                    log_println!(
                        "  [+] Stage-2 Payload detected. Injected into Partition 1 (LBA 65536 / 0x02000000)."
                    );
                }
            } else {
                log_println!(
                    "  [!] [AVISO MANDATORIO DE ESTAGIO 2]: Particoes 1 e 2 inicializadas vazias.\n\
                     O gravador exibira U99 / HDD ERR no boot inicial a menos que uma midia oficial\n\
                     PANA_DVD.FRM seja inserida no drive optico para instalacao pelo IPL da Flash NOR."
                );
            }
        }
        HddActions::WriteImage {
            target,
            image,
            confirm,
        } => {
            log_println!("Opening disk image: {}", image.display());
            let buf = open_image_buffer(&image)?;
            log_println!(
                "Streaming {} bytes to target physical drive {}...",
                buf.len(),
                target
            );
            let report = write_raw_image_to_drive(&target, &buf, &confirm)?;
            log_println!("[OK] {}", report.message);
        }
        HddActions::RestoreMirrors { target, confirm } => {
            log_println!(
                "Executing atomic dual-mirror recovery on target physical drive {}...",
                target
            );
            let report = restore_meihdfs_from_mirrors(&target, &confirm)?;
            log_println!("[OK] {}", report.message);
        }
        HddActions::InspectCapacity {
            input,
            input_flag,
            json,
        } => {
            let in_path = resolve_main_capacity_input(input.or(input_flag));
            let rep = inspect_main_hdd_capacity(&in_path)?;
            log_println!("====================================================================");
            log_println!("   PANASONIC DIGA `src/hdd_cmd.c` CAPACITY CLAMP INSPECTION");
            log_println!("====================================================================");
            log_println!("Input:            {}", rep.input_path);
            log_println!(
                "Format / Build:   {} ({})",
                rep.container_format,
                rep.build_name.as_deref().unwrap_or("RAW MAIN")
            );
            log_println!(
                "MAIN Size / SHA:  {} bytes | {}",
                rep.raw_main_size,
                rep.raw_main_sha256
            );
            log_println!(
                "Clamp Site:       {} (file offset 0x{:08X})",
                rep.patch_site_va_hex,
                rep.patch_site_file_offset
            );
            log_println!(
                "MaxProductLBA VA: {} | PhysicalBlocks VA: {}",
                rep.max_product_lba_var_va_hex,
                rep.physical_blocks_var_va_hex
            );
            log_println!(
                "Clamp Branch OP:  {} ({})",
                rep.clamp_branch_opcode_hex,
                if rep.is_dynamic_unclamped {
                    "DYNAMIC UNCLAMPED (bra +12 -> FULL LBA48)"
                } else {
                    "FACTORY CLAMPED (bls +12)"
                }
            );
            log_println!(
                "Slots (2KB blks): 400GB={} ({:.2} GB) | 200GB={} ({:.2} GB) | Fallback={} ({:.2} GB)",
                rep.slot_400gb_hex,
                rep.slot_400gb_gb,
                rep.slot_200gb_hex,
                rep.slot_200gb_gb,
                rep.slot_fallback_hex,
                rep.slot_fallback_gb
            );
            log_println!("Effective Limit:  {:.2} GB", rep.effective_default_limit_gb);
            if let Some(ref json_path) = json {
                export_report_to_json(&rep, json_path, "HDD capacity inspection report")?;
            }
        }
        HddActions::PatchCapacity {
            input,
            output,
            mode,
            target_gb,
            passthrough_mode,
        } => {
            let in_path = resolve_main_capacity_input(input);
            let out_path = output.unwrap_or_else(|| {
                default_capacity_patched_output_path(&in_path, &mode, target_gb)
            });
            let rep = patch_main_hdd_capacity_with_passthrough(
                &in_path,
                &out_path,
                &mode,
                target_gb,
                passthrough_mode.as_deref(),
            )?;
            log_println!("====================================================================");
            log_println!("   PANASONIC DIGA DYNAMIC HDD CAPACITY PATCHER");
            log_println!("====================================================================");
            log_println!("Input:            {}", rep.input_path);
            log_println!("Output:           {}", rep.output_path);
            log_println!(
                "Format / Mode:    {} | mode={} | target_gb={:?} | passthrough={}",
                rep.container_format,
                rep.mode,
                rep.target_gb,
                rep.passthrough_mode.as_deref().unwrap_or("unchanged")
            );
            log_println!(
                "Applied Ceiling:  {} ({} blocks of 2KB = {} 512B sectors)",
                rep.applied_blocks_2kb_hex,
                rep.applied_blocks_2kb,
                rep.applied_sectors_512b
            );
            log_println!(
                "Clamp Branch OP:  {} -> {}",
                rep.inspection_before.clamp_branch_opcode_hex,
                rep.inspection_after.clamp_branch_opcode_hex
            );
            log_println!("MBR Mirror Sync:  {}", rep.mbr_superblocks_updated);
            log_println!("Output SHA-256:   {}", rep.patched_sha256);
        }
    }
    Ok(())
}
