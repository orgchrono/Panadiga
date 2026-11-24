#![deny(unsafe_code)]

//! Hardened `rebuild-from-dump` engine with physical NOR dump integrity inspection
//! and `IPL`/`DRV`/`MAIN` version compatibility gating (R4).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{NOR_MIN_VALID_DUMP_BYTES, NOR_MIRROR_MIN_BLOCK_BYTES, SD_STATUS_INVALID};
use crate::nvram::calculate_hashes;
use crate::ssot::HardwareProfileRegistry;
use crate::ssot::fingerprint::contains_bytes;
use crate::vault::{
    VaultBucket, extract_nor_drv_into_vault, init_vault_session, session_bucket_dir,
};

use super::super::bgo_unbrick::{Prog061BgoPatchMode, enforce_train_evidence_tier};
use super::super::main_passthrough::{MainPassthroughMode, inspect_main_firmware_identity};
use super::super::unbrick_workbench::load_main_binary_and_meta;
use super::{
    SdKitAuditProfile, SdLoaderProfile, SdMediaVerificationReport, SdUnbrickKitReport,
    SdUnbrickKitRequest, prepare_sd_unbrick_kit_with_tier_policy,
    verify_sd_unbrick_media_with_profile,
};

/// Integrity report produced before carving or mixing modules from a physical NOR flash dump.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NorDumpIntegrityReport {
    pub size_bytes: usize,
    pub sha256: String,
    pub byte_swapped_16bit: bool,
    pub mirrored_block_size: Option<usize>,
    pub is_all_ff: bool,
    pub loader_banner: Option<String>,
    pub loader_version: Option<String>,
    pub loader_date: Option<String>,
    pub issues: Vec<String>,
    pub ok: bool,
}

/// Request contract for building a hardened SD unbrick kit from an immutable NOR dump and optional HDD `MAIN`.
#[derive(Debug, Clone)]
pub struct RebuildFromDumpRequest {
    pub model_id: String,
    pub nor_dump_path: PathBuf,
    pub hdd_main_path: Option<PathBuf>,
    pub frm_path: Option<PathBuf>,
    pub prog061_path: Option<PathBuf>,
    pub target_gb: Option<u32>,
    pub out_dir: PathBuf,
    pub allow_unverified_mix: bool,
    pub allow_unverified_profile: bool,
}

/// Complete report returned by [`rebuild_sd_kit_from_dump`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildFromDumpReport {
    pub model_id: String,
    pub resolved_loader_profile: String,
    pub nor_dump_path: String,
    pub nor_dump_sha256: String,
    pub nor_integrity: NorDumpIntegrityReport,
    pub carved_drv_from_nor: bool,
    pub compatibility_warnings: Vec<String>,
    pub kit: SdUnbrickKitReport,
    pub verification: SdMediaVerificationReport,
}

fn next_nul_cstr(slice: &[u8]) -> Option<(&str, &[u8])> {
    let nul_pos = slice.iter().position(|&b| b == 0)?;
    let s = std::str::from_utf8(&slice[..nul_pos]).ok()?.trim();
    Some((s, &slice[nul_pos + 1..]))
}

/// Scans an IPL slice for Panasonic's `"Loader ver..."` banner, version, and bracketed date
/// (supporting both literal banners like `Loader ver0.99 [07/01/02]` and ROM printf format
/// tables like `Loader ver%s [%s]\n\00.61\006/03/06\0` found in DMR-EH55 SA10).
#[must_use]
pub fn scan_loader_banner(ipl: &[u8]) -> (Option<String>, Option<String>, Option<String>) {
    let mut pos = 0usize;
    let mut fallback: Option<(Option<String>, Option<String>, Option<String>)> = None;
    while let Some(rel) = ipl[pos..].windows(10).position(|w| w == b"Loader ver") {
        let hit = pos + rel;
        let tail = &ipl[hit..ipl.len().min(hit + 64)];
        let end = tail
            .iter()
            .position(|&b| !(0x20..0x7F).contains(&b))
            .unwrap_or(tail.len());
        let raw_banner = String::from_utf8_lossy(&tail[..end]).trim().to_string();
        let ver_tok = raw_banner
            .strip_prefix("Loader ver")
            .and_then(|s| s.split_whitespace().next())
            .map(str::to_string);
        let inline_date = raw_banner.find('[').and_then(|lo| {
            raw_banner[lo + 1..]
                .find(']')
                .map(|hi| raw_banner[lo + 1..lo + 1 + hi].to_string())
        });

        if let Some(ref v) = ver_tok
            && !v.starts_with('%')
        {
            return (Some(raw_banner), Some(v.clone()), inline_date);
        }

        if raw_banner.contains("%s")
            && let Some(nul_off) = tail.iter().position(|&b| b == 0)
            && let Some((ver_str, rem)) = next_nul_cstr(&tail[nul_off + 1..])
            && ver_str.starts_with(|c: char| c.is_ascii_digit())
        {
            let date_str = next_nul_cstr(rem)
                .map(|(d, _)| d)
                .filter(|d| d.contains('/'))
                .map(str::to_string);
            let banner = match &date_str {
                Some(d) => format!("Loader ver{ver_str} [{d}]"),
                None => format!("Loader ver{ver_str}"),
            };
            return (Some(banner), Some(ver_str.to_string()), date_str);
        }

        if fallback.is_none() {
            fallback = Some((Some(raw_banner), ver_tok, inline_date));
        }
        pos = hit + 10;
    }
    fallback.unwrap_or((None, None, None))
}

/// Audits a physical NOR flash dump for truncation, all-`0xFF` erasure, 16-bit byte-swapping,
/// power-of-2 floating-address-pin mirroring, and IPL `Loader ver*` presence.
#[must_use]
pub fn inspect_nor_dump_integrity(nor_bytes: &[u8]) -> NorDumpIntegrityReport {
    let size_bytes = nor_bytes.len();
    let (_, sha256) = calculate_hashes(nor_bytes);
    let mut issues = Vec::new();

    if size_bytes < NOR_MIN_VALID_DUMP_BYTES || !size_bytes.is_power_of_two() {
        issues.push(format!(
            "NOR dump size {size_bytes} B is invalid (must be power-of-2 >= {NOR_MIN_VALID_DUMP_BYTES} B)"
        ));
    }

    let is_all_ff = !nor_bytes.is_empty() && nor_bytes.iter().all(|&b| b == 0xFF);
    if is_all_ff {
        issues.push(
            "NOR dump is completely erased (100% 0xFF); check programmer/probe read contact"
                .to_string(),
        );
    }

    let has_normal_magic =
        contains_bytes(nor_bytes, b"EXTRHEAD") || contains_bytes(nor_bytes, b"Loader ver");
    let byte_swapped_16bit = !has_normal_magic
        && (contains_bytes(nor_bytes, b"XERTDAEH") || contains_bytes(nor_bytes, b"oLdarev er"));
    if byte_swapped_16bit {
        issues.push(
            "Detected 16-bit bus byte-swapped signatures ('XERTDAEH'/'oLdarev er'); swap 16-bit words before processing"
                .to_string(),
        );
    }

    let mut mirrored_block_size = None;
    if !is_all_ff && size_bytes >= NOR_MIRROR_MIN_BLOCK_BYTES * 2 && size_bytes.is_power_of_two() {
        let mut blk = NOR_MIRROR_MIN_BLOCK_BYTES;
        while blk <= size_bytes / 2 {
            let mut chunks = nor_bytes.chunks_exact(blk);
            if let Some(first) = chunks.next()
                && chunks.clone().all(|c| c == first)
            {
                mirrored_block_size = Some(blk);
                break;
            }
            blk *= 2;
        }
        if mirrored_block_size.is_none() {
            let half = size_bytes / 2;
            if nor_bytes[..half] == nor_bytes[half..] {
                mirrored_block_size = Some(half);
            }
        }
        if let Some(m) = mirrored_block_size {
            issues.push(format!(
                "Detected power-of-2 floating-address-pin mirroring across 0x{m:X}-byte blocks"
            ));
        }
    }

    let ipl_len = size_bytes.min(0x4_0000);
    let (loader_banner, loader_version, loader_date) = scan_loader_banner(&nor_bytes[..ipl_len]);

    let ok = issues.is_empty();
    NorDumpIntegrityReport {
        size_bytes,
        sha256,
        byte_swapped_16bit,
        mirrored_block_size,
        is_all_ff,
        loader_banner,
        loader_version,
        loader_date,
        issues,
        ok,
    }
}

fn gate_unverified_mix(
    allow_unverified_mix: bool,
    warnings: &mut Vec<String>,
    detail: String,
) -> Result<()> {
    if allow_unverified_mix {
        warnings.push(detail);
        Ok(())
    } else {
        bail!(
            "REBUILD COMPATIBILITY BLOCKED (unverified mix): {detail}; pass --allow-unverified-mix to override"
        );
    }
}

/// Hardened R4 SD unbrick kit builder consuming an immutable NOR dump and optional HDD `MAIN` partition.
pub fn rebuild_sd_kit_from_dump(req: &RebuildFromDumpRequest) -> Result<RebuildFromDumpReport> {
    let nor_bytes = fs::read(&req.nor_dump_path)
        .with_context(|| format!("Reading NOR dump {}", req.nor_dump_path.display()))?;
    let nor_integrity = inspect_nor_dump_integrity(&nor_bytes);
    if !nor_integrity.ok {
        bail!(
            "Corrupt/unusable NOR dump '{}': {}",
            req.nor_dump_path.display(),
            nor_integrity.issues.join("; ")
        );
    }

    let reg = HardwareProfileRegistry::load_with_overlay(None)?;
    let resolved = reg.resolve_model(&req.model_id)?;
    let profile_key = resolved
        .firmware_train
        .as_ref()
        .map_or("ver061", |t| t.loader_profile_key.as_str());
    let _ = enforce_train_evidence_tier(profile_key, req.allow_unverified_profile)?;
    let loader_profile: SdLoaderProfile = profile_key.parse().with_context(|| {
        format!(
            "Unsupported resolved loader_profile_key '{profile_key}' for model '{}'",
            resolved.manifest.model_id
        )
    })?;

    let mut compatibility_warnings = Vec::new();
    let expected_loader_ver = resolved
        .firmware_train
        .as_ref()
        .map(|t| t.loader_version.as_str());
    match (expected_loader_ver, nor_integrity.loader_version.as_deref()) {
        (Some(exp), Some(found)) if exp != found => gate_unverified_mix(
            req.allow_unverified_mix,
            &mut compatibility_warnings,
            format!(
                "NOR IPL Loader version '{found}' does not match model '{}' SSOT train '{exp}'",
                resolved.manifest.model_id
            ),
        )?,
        (Some(exp), None) => gate_unverified_mix(
            req.allow_unverified_mix,
            &mut compatibility_warnings,
            format!(
                "NOR IPL lacks 'Loader ver{exp}' banner required by model '{}'",
                resolved.manifest.model_id
            ),
        )?,
        _ => {}
    }

    if let Some(ref main_path) = req.hdd_main_path {
        let raw_main =
            fs::read(main_path).with_context(|| format!("Reading MAIN {}", main_path.display()))?;
        let (main_bin, build_name, _, _) = load_main_binary_and_meta(&raw_main)?;
        let ident = inspect_main_firmware_identity(&main_bin, Some(&build_name));
        if ident.build_id == "UNKNOWN" {
            gate_unverified_mix(
                req.allow_unverified_mix,
                &mut compatibility_warnings,
                format!(
                    "HDD MAIN '{}' has unrecognized build_id ({} B)",
                    main_path.display(),
                    ident.size_bytes
                ),
            )?;
        }
    }

    struct TempDirCleanup(PathBuf);
    impl Drop for TempDirCleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    static REBUILD_STAGE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let stage_seq = REBUILD_STAGE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staging_root = std::env::temp_dir().join(format!(
        "panadiga_rebuild_stage_{}_{}",
        std::process::id(),
        stage_seq
    ));
    let _ = fs::remove_dir_all(&staging_root);
    let _staging_guard = TempDirCleanup(staging_root.clone());
    let carved_drv_path = if req.frm_path.is_none() {
        let manifest = init_vault_session(
            &staging_root,
            &resolved.manifest.model_id,
            loader_profile,
            Some("nor_carve"),
        )?;
        let sess_path = Path::new(&manifest.session_dir);
        match extract_nor_drv_into_vault(sess_path, &nor_bytes) {
            Ok(rec) => Some(session_bucket_dir(sess_path, VaultBucket::Raw)?.join(&rec.filename)),
            Err(err) => {
                gate_unverified_mix(
                    req.allow_unverified_mix,
                    &mut compatibility_warnings,
                    format!(
                        "Failed carving checksum-verified DRV from NOR dump without --frm fallback: {err}"
                    ),
                )?;
                None
            }
        }
    } else {
        None
    };

    let sd_req = SdUnbrickKitRequest {
        output_dir: req.out_dir.clone(),
        pana_dvd_path: req.frm_path.clone(),
        prog061_path: req.prog061_path.clone(),
        main_path: req.hdd_main_path.clone(),
        drv_path: carved_drv_path.clone(),
        bgo_mode: Prog061BgoPatchMode::UartMonitor,
        main_passthrough_mode: MainPassthroughMode::AvPassthrough,
        loader_profile,
        include_pana_sd: true,
        unlock_hdd_dynamic: req.target_gb.is_none(),
        hdd_target_gb: req.target_gb,
        write_extracted_sources: false,
        allow_unsafe_auto_format: false,
        expected_main_build: None,
        allow_target_mismatch: false,
        sata_bridge_reset_fix: false,
        sata_bridge_stage2_boot: false,
        main_bgo_retarget: false,
        portao4_self_unpack: false,
    };

    let kit_result = prepare_sd_unbrick_kit_with_tier_policy(&sd_req, req.allow_unverified_profile);
    let _ = fs::remove_dir_all(&staging_root);
    let kit = kit_result?;

    let verification = verify_sd_unbrick_media_with_profile(
        Path::new(&kit.output_dir),
        SdKitAuditProfile::HardenedRecovery,
    );
    if verification.overall_status == SD_STATUS_INVALID
        || verification.factory_unpatched_prog061_detected
    {
        bail!(
            "Rebuilt SD kit verification failed: status={} unpatched_prog={}",
            verification.overall_status,
            verification.factory_unpatched_prog061_detected
        );
    }

    let nor_sha256 = nor_integrity.sha256.clone();
    Ok(RebuildFromDumpReport {
        model_id: resolved.manifest.model_id,
        resolved_loader_profile: profile_key.to_string(),
        nor_dump_path: req.nor_dump_path.to_string_lossy().into_owned(),
        nor_dump_sha256: nor_sha256,
        nor_integrity,
        carved_drv_from_nor: carved_drv_path.is_some(),
        compatibility_warnings,
        kit,
        verification,
    })
}
