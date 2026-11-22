//! CLI handlers for universal Legacy/Blu-ray `.FRM` unpacking (`unpack-all`) and repacking (`repack`).

use std::path::Path;

use anyhow::{Context, Result, bail};
use panadiga_core::constants::{PANA_CRAMFS_MAGIC, PANA_CRAMFS_SIGNATURE, PANA_ROMFS_MAGIC};
use panadiga_core::crypto::{
    FrmKeyring, FrmRepackOptions, FrmTargetMode, FsExtractReport, RepackCompression,
    UniversalFrmContainer, UniversalFrmFormat, extract_partition_fs, repack_universal_frm,
    split_main_partitions, unpack_universal_frm,
};
use serde::Serialize;

use super::super::args::crypto::{FrmExtractFsArgs, FrmRepackArgs, FrmUnpackAllArgs};
use super::frm_pack::split_name_and_rest;
use super::json_io::{emit_json_output, write_cli_output_bytes};

/// Loads cascading keyring: built-in -> PANADIGA_KEY_FILE -> keys.local.ukf -> CLI `--key-file`.
pub fn load_frm_keyring(key_file: Option<&Path>) -> Result<FrmKeyring> {
    FrmKeyring::load_default_with_cli(key_file)
}

fn sanitize_filename_token(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "module".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Serialize)]
struct UnpackedModuleRecord {
    sub_index: usize,
    index: usize,
    name: String,
    stage_kind: String,
    encoding: String,
    compression_algo: Option<String>,
    build_name: Option<String>,
    dst_addr_hex: Option<String>,
    toc_sum_valid: bool,
    data_sum_valid: bool,
    size_bytes: usize,
    out_path: String,
    partition_files: Vec<String>,
}

#[derive(Serialize)]
struct UniversalUnpackReport {
    input_path: String,
    format: UniversalFrmFormat,
    aes_key_name: Option<String>,
    sub_containers: usize,
    modules: Vec<UnpackedModuleRecord>,
}

fn infer_stage_kind(name: &str) -> &'static str {
    let up = name.trim().to_ascii_uppercase();
    if up.starts_with("MAIN") || up.starts_with("SMAI") {
        "MAIN"
    } else if up.starts_with("PROG") || up.starts_with("DIAG") {
        "PROG"
    } else if up.starts_with("DRV") {
        "DRV"
    } else if up.starts_with("STB") {
        "STB"
    } else if up.starts_with("BOOT") || up.starts_with("IPL") {
        "IPL"
    } else {
        "AUX"
    }
}

fn write_unpacked_modules(
    container: &UniversalFrmContainer,
    out_dir: &Path,
    split_main: bool,
) -> Result<Vec<UnpackedModuleRecord>> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("Creating unpack output directory {}", out_dir.display()))?;
    let multi = container.sub_containers.len() > 1;
    let mut records = Vec::new();

    for sub in &container.sub_containers {
        let sub_dir = if multi {
            out_dir.join(format!("sub_{:02}", sub.sub_index))
        } else {
            out_dir.to_path_buf()
        };
        std::fs::create_dir_all(&sub_dir)?;

        for m in &sub.modules {
            let safe_name = sanitize_filename_token(&m.name);
            let mod_path = sub_dir.join(format!("{:02}_{safe_name}.bin", m.index));
            write_cli_output_bytes(&mod_path, &m.payload)?;

            let mut part_files = Vec::new();
            let up = m.name.trim().to_ascii_uppercase();
            if split_main
                && (up.starts_with("MAIN") || up.starts_with("SMAI"))
                && let Ok(slices) = split_main_partitions(&m.payload)
            {
                let part_dir = sub_dir.join(format!("{:02}_{safe_name}_partitions", m.index));
                std::fs::create_dir_all(&part_dir)?;
                for sl in slices {
                    let ppath = part_dir.join(format!("{}.bin", sanitize_filename_token(&sl.name)));
                    write_cli_output_bytes(&ppath, &sl.data)?;
                    part_files.push(ppath.to_string_lossy().into_owned());
                }
            }

            records.push(UnpackedModuleRecord {
                sub_index: sub.sub_index,
                index: m.index,
                name: m.name.clone(),
                stage_kind: infer_stage_kind(&m.name).to_string(),
                encoding: m.encoding.clone(),
                compression_algo: m.compression_algo.clone(),
                build_name: m.build_name.clone(),
                dst_addr_hex: m.dst_addr_hex.clone(),
                toc_sum_valid: m.toc_sum_valid,
                data_sum_valid: m.data_sum_valid,
                size_bytes: m.payload.len(),
                out_path: mod_path.to_string_lossy().into_owned(),
                partition_files: part_files,
            });
        }
    }

    Ok(records)
}

/// Handles `panadiga frm unpack-all`.
pub fn handle_frm_unpack_all(args: &FrmUnpackAllArgs) -> Result<()> {
    let keyring = load_frm_keyring(args.common.key_file.as_deref())?;
    let raw = std::fs::read(&args.file)
        .with_context(|| format!("Reading .FRM file {}", args.file.display()))?;
    let decoded = unpack_universal_frm(&raw, &keyring)?;
    let modules = write_unpacked_modules(&decoded, &args.common.out_dir, args.split_main)?;

    let report = UniversalUnpackReport {
        input_path: args.file.to_string_lossy().into_owned(),
        format: decoded.format,
        aes_key_name: decoded.aes_key_name.clone(),
        sub_containers: decoded.sub_containers.len(),
        modules,
    };
    emit_json_output(&report, args.common.json.as_deref())?;
    if args.common.json.is_none() {
        println!(
            "Unpacked {} ({:?}, aes={:?}, {} module(s)) -> {}",
            args.file.display(),
            report.format,
            report.aes_key_name,
            report.modules.len(),
            args.common.out_dir.display()
        );
    }
    Ok(())
}

fn load_patch_pairs(specs: &[String], flag_name: &str) -> Result<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::with_capacity(specs.len());
    for spec in specs {
        let Some((name, path_str)) = split_name_and_rest(spec) else {
            bail!("Invalid {flag_name} '{spec}': expected NAME=FILE_PATH");
        };
        let bytes = std::fs::read(path_str)
            .with_context(|| format!("Reading replacement '{name}' from {path_str}"))?;
        out.push((name.trim().to_string(), bytes));
    }
    Ok(out)
}

/// Handles `panadiga frm repack`.
pub fn handle_frm_repack(args: &FrmRepackArgs) -> Result<()> {
    let keyring = load_frm_keyring(args.key_file.as_deref())?;
    let raw = std::fs::read(&args.input)
        .with_context(|| format!("Reading input .FRM {}", args.input.display()))?;
    let target_mode: FrmTargetMode = args.target_mode.parse()?;
    let compression: RepackCompression = args.compression.parse()?;
    let patch_modules = load_patch_pairs(&args.patch_modules, "--patch-module")?;
    let patch_partitions = load_patch_pairs(&args.patch_partitions, "--patch-partition")?;

    let opts = FrmRepackOptions {
        target_mode,
        compression,
        drop_aes: args.drop_aes,
        patch_force: args.patch_force,
        patch_format_hdd: args.patch_format_hdd,
        patch_version_offset: args.patch_version_offset,
        patch_modules,
        patch_partitions,
        verify: args.report_opts.verify,
    };

    let (out_bytes, report) = repack_universal_frm(&raw, &keyring, &opts)?;
    write_cli_output_bytes(&args.out, &out_bytes)?;
    emit_json_output(&report, args.report_opts.json.as_deref())?;
    if args.report_opts.json.is_none() {
        println!(
            "Repacked {} ({:?} -> {:?}, {} B, verified={}) -> {}",
            args.input.display(),
            report.source_format,
            report.output_format,
            report.output_bytes,
            report.verified,
            args.out.display()
        );
    }
    Ok(())
}

#[derive(Serialize)]
struct ExtractFsPartitionRecord {
    partition_name: String,
    report: FsExtractReport,
}

#[derive(Serialize)]
struct UniversalExtractFsReport {
    input_path: String,
    partitions: Vec<ExtractFsPartitionRecord>,
    total_regular_files: usize,
    total_elfs: usize,
}

fn looks_like_raw_fs(bytes: &[u8]) -> bool {
    (bytes.len() >= 32 && &bytes[0..8] == PANA_ROMFS_MAGIC)
        || (bytes.len() >= 76
            && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == PANA_CRAMFS_MAGIC
            && &bytes[16..32] == PANA_CRAMFS_SIGNATURE)
}

/// Handles `panadiga frm extract-fs`.
pub fn handle_frm_extract_fs(args: &FrmExtractFsArgs) -> Result<()> {
    let raw = std::fs::read(&args.input)
        .with_context(|| format!("Reading input {}", args.input.display()))?;
    let mut partitions = Vec::new();
    let mut total_regular_files = 0usize;
    let mut total_elfs = 0usize;

    if looks_like_raw_fs(&raw) {
        let part_name = args
            .input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("partition")
            .to_string();
        let rep = extract_partition_fs(&raw, Some(&args.common.out_dir), args.elf_only)?;
        total_regular_files += rep.regular_files;
        total_elfs += rep.elf_count;
        partitions.push(ExtractFsPartitionRecord {
            partition_name: part_name,
            report: rep,
        });
    } else {
        let keyring = load_frm_keyring(args.common.key_file.as_deref())?;
        let decoded = unpack_universal_frm(&raw, &keyring)?;
        for sub in &decoded.sub_containers {
            for m in &sub.modules {
                if let Ok(slices) = split_main_partitions(&m.payload) {
                    for sl in &slices {
                        if looks_like_raw_fs(&sl.data) {
                            let part_dir = args.common.out_dir.join(&sl.name);
                            let rep =
                                extract_partition_fs(&sl.data, Some(&part_dir), args.elf_only)?;
                            total_regular_files += rep.regular_files;
                            total_elfs += rep.elf_count;
                            partitions.push(ExtractFsPartitionRecord {
                                partition_name: sl.name.clone(),
                                report: rep,
                            });
                        }
                    }
                }
            }
        }
    }

    let summary = UniversalExtractFsReport {
        input_path: args.input.to_string_lossy().into_owned(),
        partitions,
        total_regular_files,
        total_elfs,
    };
    emit_json_output(&summary, args.common.json.as_deref())?;
    if args.common.json.is_none() {
        println!(
            "Extracted {} filesystem partition(s) ({} files, {} ELFs) -> {}",
            summary.partitions.len(),
            summary.total_regular_files,
            summary.total_elfs,
            args.common.out_dir.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::crypto::FrmOutKeyJsonArgs;
    use panadiga_core::crypto::{FrmContainerMode, FrmPackModule, pack_frm};

    #[test]
    fn cli_unpack_all_and_repack_smoke_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "panadiga_cli_universal_frm_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let src_frm = dir.join("PANA_TEST.FRM");
        let unpack_dir = dir.join("unpacked");
        let repacked_frm = dir.join("PANA_SD.FRM");
        let report_json = dir.join("report.json");

        let (bytes, _) = pack_frm(
            &[
                FrmPackModule::new("PROG061 ", vec![0x11u8; 64]),
                FrmPackModule::new("DRV S211", vec![0x22u8; 64]),
            ],
            FrmContainerMode::Dvd,
            None,
        )
        .unwrap();
        std::fs::write(&src_frm, &bytes).unwrap();

        handle_frm_unpack_all(&FrmUnpackAllArgs {
            file: src_frm.clone(),
            split_main: false,
            common: FrmOutKeyJsonArgs {
                out_dir: unpack_dir.clone(),
                key_file: None,
                json: Some(report_json.clone()),
            },
        })
        .unwrap();
        assert!(unpack_dir.join("00_PROG061.bin").is_file());
        assert!(unpack_dir.join("01_DRV_S211.bin").is_file());
        assert!(report_json.is_file());

        handle_frm_repack(&FrmRepackArgs {
            input: src_frm,
            out: repacked_frm.clone(),
            key_file: None,
            target_mode: "sd".to_string(),
            compression: "preserve".to_string(),
            drop_aes: false,
            patch_force: true,
            patch_format_hdd: true,
            patch_version_offset: None,
            patch_modules: Vec::new(),
            patch_partitions: Vec::new(),
            report_opts: crate::cli::args::crypto::FrmVerifyJsonArgs {
                verify: true,
                json: None,
            },
        })
        .unwrap();
        assert!(repacked_frm.is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
