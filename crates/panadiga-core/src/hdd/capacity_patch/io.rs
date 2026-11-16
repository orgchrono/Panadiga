//! Thin imperative filesystem shell for `MAIN` / HDD image capacity inspection & patching.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::constants::{LBA_PARTITION_1_OFFSET, LBA_PARTITION_2_OFFSET};
use crate::nvram::{calculate_hashes, resolve_input_path};

use super::codec::{
    LoadedMainSource, apply_capacity_patch_in_place, apply_optional_passthrough,
    inspect_main_buffer_capacity, inspect_unpacked_capacity, load_main_source,
    maybe_update_procedural_mbr_capacity, patch_and_repack_extrhead, verify_and_inspect_repacked,
};
use super::specs::{HddCapacityPatchMode, MainCapacityInspectionReport, MainCapacityPatchReport};

/// Inspects a file (`RAW_MAIN_BINARY`, `EXTRHEAD_CONTAINER`, or `FULL_HDD_IMAGE_P1`) on disk.
pub fn inspect_main_hdd_capacity(input_path: &Path) -> Result<MainCapacityInspectionReport> {
    let resolved = resolve_input_path(input_path)?;
    let raw = fs::read(&resolved)
        .with_context(|| format!("Failed reading firmware input: {}", resolved.display()))?;
    let label = resolved.to_string_lossy().into_owned();

    match load_main_source(raw)? {
        LoadedMainSource::RawMain {
            main_bytes,
            base_va,
        } => inspect_main_buffer_capacity(&main_bytes, base_va, "RAW_MAIN_BINARY", None, &label),
        LoadedMainSource::Extrhead { unpacked, .. } => {
            inspect_unpacked_capacity(&unpacked, "EXTRHEAD_CONTAINER", &label)
        }
        LoadedMainSource::FullHddP1 { unpacked, .. } => {
            inspect_unpacked_capacity(&unpacked, "FULL_HDD_IMAGE_P1", &label)
        }
    }
}

/// Computes the default output path (`<stem>_<suffix>.<ext>`) when the caller omits an explicit output file.
pub fn default_capacity_patched_output_path(
    input_path: &Path,
    mode_str: &str,
    target_gb: Option<u32>,
) -> PathBuf {
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("main_firmware");
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("bin");
    let suffix = match (mode_str.trim().to_ascii_lowercase().as_str(), target_gb) {
        ("factory" | "factory_200gb" | "default" | "restore", _) => "FACTORY_200GB".to_string(),
        (_, Some(gb)) => format!("PATCHED_{gb}GB"),
        _ => "PATCHED_DYNAMIC".to_string(),
    };
    parent.join(format!("{stem}_{suffix}.{ext}"))
}

type LoadedPatchResult = (
    String,
    MainCapacityInspectionReport,
    MainCapacityInspectionReport,
    u32,
    Option<String>,
    bool,
    Option<usize>,
    Vec<u8>,
);

fn apply_patch_to_loaded_source(
    loaded: LoadedMainSource,
    in_label: &str,
    out_label: &str,
    mode: HddCapacityPatchMode,
    target_gb: Option<u32>,
    passthrough_mode: Option<&str>,
) -> Result<LoadedPatchResult> {
    match loaded {
        LoadedMainSource::RawMain {
            mut main_bytes,
            base_va,
        } => {
            let before = inspect_main_buffer_capacity(
                &main_bytes,
                base_va,
                "RAW_MAIN_BINARY",
                None,
                in_label,
            )?;
            let applied = apply_capacity_patch_in_place(&mut main_bytes, mode, target_gb)?;
            let applied_pt = apply_optional_passthrough(&mut main_bytes, passthrough_mode)?;
            let after = inspect_main_buffer_capacity(
                &main_bytes,
                base_va,
                "RAW_MAIN_BINARY",
                None,
                out_label,
            )?;
            Ok((
                "RAW_MAIN_BINARY".to_string(),
                before,
                after,
                applied,
                applied_pt,
                false,
                None,
                main_bytes,
            ))
        }
        LoadedMainSource::Extrhead {
            mut unpacked,
            is_plain,
        } => {
            let (before, applied, applied_pt, repacked) = patch_and_repack_extrhead(
                &mut unpacked,
                is_plain,
                "EXTRHEAD_CONTAINER",
                in_label,
                mode,
                target_gb,
                passthrough_mode,
            )?;
            let repacked_sz = repacked.len();
            let after = verify_and_inspect_repacked(&repacked, "EXTRHEAD_CONTAINER", out_label)?;
            Ok((
                "EXTRHEAD_CONTAINER".to_string(),
                before,
                after,
                applied,
                applied_pt,
                false,
                Some(repacked_sz),
                repacked,
            ))
        }
        LoadedMainSource::FullHddP1 {
            mut image_bytes,
            p1_container_len,
            mut unpacked,
            is_plain,
        } => {
            let (before, applied, applied_pt, repacked) = patch_and_repack_extrhead(
                &mut unpacked,
                is_plain,
                "FULL_HDD_IMAGE_P1",
                in_label,
                mode,
                target_gb,
                passthrough_mode,
            )?;
            let repacked_sz = repacked.len();

            let p1_off = LBA_PARTITION_1_OFFSET as usize;
            let p2_off = LBA_PARTITION_2_OFFSET as usize;
            if p1_off + repacked_sz > p2_off {
                bail!(
                    "Repacked Partition 1 ({} bytes) exceeds 32 MB gap before Partition 2 ({} bytes)",
                    repacked_sz,
                    p2_off - p1_off
                );
            }
            if repacked_sz < p1_container_len {
                image_bytes[p1_off..p1_off + p1_container_len].fill(0);
            }
            image_bytes[p1_off..p1_off + repacked_sz].copy_from_slice(&repacked);

            let mbr_updated = if let Some(gb) = target_gb {
                maybe_update_procedural_mbr_capacity(
                    &mut image_bytes,
                    (gb as u64 * 1_000_000_000) / 512,
                )
            } else {
                maybe_update_procedural_mbr_capacity(&mut image_bytes, (applied as u64) * 4)
            };

            let after = verify_and_inspect_repacked(
                &image_bytes[p1_off..p1_off + repacked_sz],
                "FULL_HDD_IMAGE_P1",
                out_label,
            )?;
            Ok((
                "FULL_HDD_IMAGE_P1".to_string(),
                before,
                after,
                applied,
                applied_pt,
                mbr_updated,
                Some(repacked_sz),
                image_bytes,
            ))
        }
    }
}

/// Applies a dynamic or preset HDD capacity patch and writes the output artifact (`RAW`, `EXTRHEAD`, or full HDD image).
pub fn patch_main_hdd_capacity(
    input_path: &Path,
    output_path: &Path,
    mode_str: &str,
    target_gb: Option<u32>,
) -> Result<MainCapacityPatchReport> {
    patch_main_hdd_capacity_with_passthrough(input_path, output_path, mode_str, target_gb, None)
}

/// Applies a dynamic or preset HDD capacity patch plus an optional `MAIN` AV TBC passthrough patch.
pub fn patch_main_hdd_capacity_with_passthrough(
    input_path: &Path,
    output_path: &Path,
    mode_str: &str,
    target_gb: Option<u32>,
    passthrough_mode: Option<&str>,
) -> Result<MainCapacityPatchReport> {
    let mode: HddCapacityPatchMode = mode_str.parse()?;
    let resolved_in = resolve_input_path(input_path)?;
    let raw = fs::read(&resolved_in)
        .with_context(|| format!("Failed reading firmware input: {}", resolved_in.display()))?;
    let (_, original_sha256) = calculate_hashes(&raw);
    let in_label = resolved_in.to_string_lossy().into_owned();
    let out_label = output_path.to_string_lossy().into_owned();

    crate::re::bgo_unbrick::ensure_parent_dir(output_path)?;

    let (
        container_format,
        before,
        after,
        applied_blocks,
        applied_passthrough,
        mbr_updated,
        repacked_len,
        out_bytes,
    ) = apply_patch_to_loaded_source(
        load_main_source(raw)?,
        &in_label,
        &out_label,
        mode,
        target_gb,
        passthrough_mode,
    )?;

    let (_, patched_sha256) = calculate_hashes(&out_bytes);
    fs::write(output_path, &out_bytes).with_context(|| {
        format!(
            "Writing patched firmware output to {}",
            output_path.display()
        )
    })?;

    Ok(MainCapacityPatchReport {
        input_path: in_label,
        output_path: out_label,
        container_format,
        mode: mode_str.to_string(),
        target_gb,
        applied_blocks_2kb: applied_blocks,
        applied_blocks_2kb_hex: format!("0x{applied_blocks:08X}"),
        applied_sectors_512b: (applied_blocks as u64) * 4,
        mbr_superblocks_updated: mbr_updated,
        repacked_container_size: repacked_len,
        passthrough_mode: applied_passthrough,
        original_sha256,
        patched_sha256,
        inspection_before: before,
        inspection_after: after,
    })
}

#[cfg(test)]
mod tests {
    use super::super::codec::test_helpers::build_synthetic_hdd_cmd_main;
    use super::super::specs::MAIN_DEFAULT_BASE_VA;
    use super::*;
    use crate::constants::PANA_EXTRHEAD_DEFAULT_ADLER_STEP;
    use crate::crypto::{ExtrheadCryptMode, pack_extrhead};

    #[test]
    fn test_extrhead_container_roundtrip_capacity_patch() {
        let main = build_synthetic_hdd_cmd_main();
        let packed = pack_extrhead(
            &main,
            "MAIN",
            MAIN_DEFAULT_BASE_VA,
            "TEST3410PQ",
            ExtrheadCryptMode::Main,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
            0x0101,
        )
        .expect("Pack synthetic EXTRHEAD");

        let tmp = std::env::temp_dir().join("panadiga_cap_test");
        let _ = fs::create_dir_all(&tmp);
        let in_file = tmp.join("test_p1_in.bin");
        let out_file = tmp.join("test_p1_out.bin");
        fs::write(&in_file, &packed).expect("Write input");

        let report = patch_main_hdd_capacity(&in_file, &out_file, "dynamic", None)
            .expect("Patch EXTRHEAD container");
        assert_eq!(report.container_format, "EXTRHEAD_CONTAINER");
        assert!(report.inspection_after.is_dynamic_unclamped);

        let inspect_out = inspect_main_hdd_capacity(&out_file).expect("Inspect output EXTRHEAD");
        assert!(inspect_out.is_dynamic_unclamped);
        let _ = fs::remove_dir_all(&tmp);
    }
}
