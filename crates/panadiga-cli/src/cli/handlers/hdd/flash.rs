#![deny(unsafe_code)]

//! Handlers for MEIHDFS Stage-2 FRM partition injection and verified EH55 disk flashing.

use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use panadiga_core::constants::{
    LBA_PARTITION_1_OFFSET, LBA_PARTITION_2_OFFSET, LBA_SECTOR_SIZE_BYTES,
};
use panadiga_core::crypto::frm_pack::unpack_dvd_frm_to_modules;
use panadiga_core::crypto::toc::parse_sd_frm_toc_with_layout;
use panadiga_core::hdd::capacity_patch::specs::blocks_2kb_for_gb_bounded;
use panadiga_core::hdd::writer::{
    BlockFlashDevice, Eh55HddFlashReport, Eh55HddFlashRequest, WindowsPhysicalFlashDevice,
    execute_eh55_hdd_flash, open_verified_block_device, verify_safety_lockout,
    write_meihdfs_procedural_to_drive,
};
use panadiga_core::hdd::{generate_meihdfs_layout, open_image_buffer};
use panadiga_core::log_println;
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::json_io::emit_json_output;
use crate::cli::args::hdd::FlashEh55Args;

#[derive(Debug, Clone)]
pub(crate) struct Stage2Partitions {
    pub p1_name: String,
    pub p1_aligned: Vec<u8>,
    pub p1_sha256: String,
    pub p2_name: Option<String>,
    pub p2_aligned: Option<Vec<u8>>,
    pub p2_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormatMeihdfsSummary {
    pub target: String,
    pub size_gb: u64,
    pub total_sectors: u64,
    pub dry_run: bool,
    pub mbr_bytes_written: u64,
    pub p1_module: Option<String>,
    pub p1_bytes_written: usize,
    pub p1_sha256: Option<String>,
    pub p2_module: Option<String>,
    pub p2_bytes_written: usize,
    pub p2_sha256: Option<String>,
    pub message: String,
}

fn sha256_hex_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn align_to_sector(mut payload: Vec<u8>) -> Vec<u8> {
    let rem = payload.len() % LBA_SECTOR_SIZE_BYTES;
    if rem != 0 {
        payload.resize(payload.len() + (LBA_SECTOR_SIZE_BYTES - rem), 0);
    }
    payload
}

pub(crate) fn extract_frm_stage2_modules(frm_bytes: &[u8]) -> Result<Stage2Partitions> {
    let named: Vec<(String, Vec<u8>)> = match unpack_dvd_frm_to_modules(frm_bytes) {
        Ok(mods) => mods.into_iter().map(|m| (m.name, m.payload)).collect(),
        Err(_) => {
            let (layout, entries) = parse_sd_frm_toc_with_layout(frm_bytes);
            if layout.is_none() || entries.is_empty() {
                bail!("Input .FRM does not contain a valid Feistel PANA_DVD or SD TOC");
            }
            let mut out = Vec::new();
            for entry in entries {
                let start = entry.module_offset as usize;
                let end = start.saturating_add(entry.module_size as usize);
                if entry.module_size > 0 && end <= frm_bytes.len() && entry.list_sum_valid {
                    out.push((entry.module_name, frm_bytes[start..end].to_vec()));
                }
            }
            out
        }
    };

    let (p1_name, p1_raw) = named
        .iter()
        .find(|(n, _)| n.trim().to_ascii_uppercase().starts_with("MAIN"))
        .cloned()
        .context("No MAIN module found in .FRM container for Partition 1 (0x02000000)")?;
    let p1_aligned = align_to_sector(p1_raw);
    let max_p1 = (LBA_PARTITION_2_OFFSET - LBA_PARTITION_1_OFFSET) as usize;
    if p1_aligned.len() > max_p1 {
        bail!(
            "Partition 1 MAIN payload ({} B) exceeds 32 MiB ceiling ({} B)",
            p1_aligned.len(),
            max_p1
        );
    }
    let p1_sha256 = sha256_hex_bytes(&p1_aligned);

    let p2_pair = named
        .iter()
        .find(|(n, _)| n.trim().to_ascii_uppercase().starts_with("DRV"))
        .map(|(n, b)| {
            let aligned = align_to_sector(b.clone());
            let sha = sha256_hex_bytes(&aligned);
            (n.clone(), aligned, sha)
        });

    Ok(Stage2Partitions {
        p1_name,
        p1_aligned,
        p1_sha256,
        p2_name: p2_pair.as_ref().map(|x| x.0.clone()),
        p2_aligned: p2_pair.as_ref().map(|x| x.1.clone()),
        p2_sha256: p2_pair.map(|x| x.2),
    })
}

pub fn run_format_meihdfs(
    target: &str,
    size_gb: u64,
    inject_frm: Option<&Path>,
    confirm: &str,
    dry_run: bool,
    json: Option<&Path>,
) -> Result<FormatMeihdfsSummary> {
    verify_safety_lockout(target, confirm)?;
    let total_sectors = size_gb * 1_000_000_000 / 512;
    let layout = generate_meihdfs_layout(total_sectors, None)?;
    let stage2 = if let Some(frm_path) = inject_frm {
        let frm_buf = open_image_buffer(frm_path)?;
        Some(extract_frm_stage2_modules(&frm_buf)?)
    } else {
        None
    };

    let mbr_written = if dry_run {
        0
    } else {
        let rep = write_meihdfs_procedural_to_drive(target, &layout, confirm)?;
        log_println!("[OK] {}", rep.message);
        if let Some(ref s2) = stage2 {
            let mut dev = open_verified_block_device(target, confirm)?;
            dev.seek(SeekFrom::Start(LBA_PARTITION_1_OFFSET))?;
            dev.write_all(&s2.p1_aligned)?;
            if let Some(ref p2) = s2.p2_aligned {
                dev.seek(SeekFrom::Start(LBA_PARTITION_2_OFFSET))?;
                dev.write_all(p2)?;
            }
            dev.flush()?;
        }
        rep.bytes_written
    };

    let summary = FormatMeihdfsSummary {
        target: target.to_string(),
        size_gb,
        total_sectors,
        dry_run,
        mbr_bytes_written: mbr_written,
        p1_module: stage2.as_ref().map(|s| s.p1_name.clone()),
        p1_bytes_written: stage2.as_ref().map_or(0, |s| s.p1_aligned.len()),
        p1_sha256: stage2.as_ref().map(|s| s.p1_sha256.clone()),
        p2_module: stage2.as_ref().and_then(|s| s.p2_name.clone()),
        p2_bytes_written: stage2
            .as_ref()
            .and_then(|s| s.p2_aligned.as_ref())
            .map_or(0, Vec::len),
        p2_sha256: stage2.as_ref().and_then(|s| s.p2_sha256.clone()),
        message: if dry_run {
            format!("Dry-run validated MEIHDFS layout for {target} ({size_gb} GB)")
        } else {
            format!("Formatted MEIHDFS on {target} ({size_gb} GB)")
        },
    };
    emit_json_output(&summary, json)?;
    Ok(summary)
}

pub(crate) fn normalize_flash_confirm_token(raw_confirm: &str) -> String {
    let trimmed = raw_confirm.trim();
    if trimmed.is_empty() {
        String::new()
    } else if trimmed.starts_with("CONFIRM-") {
        trimmed.to_string()
    } else {
        format!("CONFIRM-{trimmed}")
    }
}

pub(crate) fn resolve_expected_gb_range(
    expected_min_gb: Option<u64>,
    expected_max_gb: Option<u64>,
    allow_extended_4tb: bool,
) -> Result<Option<(u32, u32)>> {
    match (expected_min_gb, expected_max_gb) {
        (None, None) => Ok(None),
        (min_opt, max_opt) => {
            let min_u64 = min_opt.unwrap_or(5);
            let max_u64 = max_opt.unwrap_or(260);
            let min_u32 = u32::try_from(min_u64).context("expected_min_gb overflow")?;
            let max_u32 = u32::try_from(max_u64).context("expected_max_gb overflow")?;
            blocks_2kb_for_gb_bounded(max_u32, allow_extended_4tb)?;
            Ok(Some((min_u32, max_u32)))
        }
    }
}

pub(crate) fn build_eh55_flash_request(args: &FlashEh55Args) -> Result<Eh55HddFlashRequest> {
    let expected_gb_range = resolve_expected_gb_range(
        args.range.expected_min_gb,
        args.range.expected_max_gb,
        args.range.allow_extended_4tb,
    )?;
    Ok(Eh55HddFlashRequest {
        device_path: args.drive.clone(),
        confirmation_token: normalize_flash_confirm_token(&args.confirm_physical_drive),
        image_path: args.image.clone(),
        mirror_p1_path: args.mirror_p1.clone(),
        extract_p1_mirror_from_image: args.mirror_p1.is_none(),
        backup_dir: args.backup_dir.clone(),
        expected_gb_range,
        detected_os_disks: None,
    })
}

pub fn run_flash_eh55_on_device<D: BlockFlashDevice>(
    device: &mut D,
    args: &FlashEh55Args,
) -> Result<Eh55HddFlashReport> {
    let req = build_eh55_flash_request(args)?;
    let report = execute_eh55_hdd_flash(device, &req, |p| {
        log_println!(
            "[HDD-FLASH {:?}] {}/{} B — {}",
            p.stage,
            p.bytes_done,
            p.bytes_total,
            p.detail
        );
    })?;
    log_println!(
        "[OK] Flashed {} ({} B primary sha={}, mirror_lba={}) | readback_verified={}",
        report.device_path,
        report.primary_bytes_written,
        report.primary_sha256,
        report.mirror_main_lba,
        report.readback_verified
    );
    emit_json_output(&report, args.range.json.as_deref())?;
    Ok(report)
}

pub(crate) fn handle_flash_eh55(args: FlashEh55Args) -> Result<()> {
    let _ = build_eh55_flash_request(&args)?;
    let mut device = WindowsPhysicalFlashDevice::open(&args.drive, true)?;
    let _ = run_flash_eh55_on_device(&mut device, &args)?;
    Ok(())
}
