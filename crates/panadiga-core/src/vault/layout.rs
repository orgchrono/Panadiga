#![deny(unsafe_code)]

//! Bucket segregation (`raw/` vs `frm/` vs `recompiled/`), SHA-256 immutability,
//! auto-classification ingest, and NOR `DRV` extraction helpers.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};
use crate::ingest::{ContainerFormat, fingerprint_binary};
use crate::nvram::calculate_hashes;
use crate::re::prewrite_gate::PreWriteGateReport;

use super::manifest::{epoch_secs_now, load_vault_session, save_vault_session};
use super::{StrictWorkflowStep, VaultArtifactRecord, VaultBucket};

const NOR_DRV_SEARCH_BASE: usize = 0x08_0000;
const NOR_DRV_SEARCH_END: usize = 0x20_0000;

/// Rejects path separators or `..` traversal in session IDs or artifact filenames.
pub fn validate_safe_segment(raw: &str, label: &str) -> Result<String> {
    let clean = raw.trim();
    if clean.is_empty()
        || clean == "."
        || clean == ".."
        || clean.contains(['/', '\\', ':', '\0'])
        || clean.contains("..")
    {
        bail!("Invalid {label} '{raw}': must be a single clean filename/id segment");
    }
    Ok(clean.to_string())
}

/// Returns `<session_dir>/<bucket>` and ensures the bucket directory exists.
pub fn session_bucket_dir(session_dir: &Path, bucket: VaultBucket) -> Result<PathBuf> {
    let dir = session_dir.join(bucket.dir_name());
    fs::create_dir_all(&dir)
        .with_context(|| format!("Creating vault bucket directory {}", dir.display()))?;
    Ok(dir)
}

fn validate_bucket_extension(filename: &str, bucket: VaultBucket) -> Result<()> {
    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match bucket {
        VaultBucket::Raw => {
            if ext == "frm" {
                bail!(
                    "Vault segregation violation: .FRM file '{filename}' belongs in frm/, NEVER raw/"
                );
            }
            if !matches!(ext.as_str(), "bin" | "img" | "nvram" | "dump") {
                bail!("Vault raw/ accepts .bin, .img, .nvram, or .dump (got '{filename}')");
            }
        }
        VaultBucket::Frm => {
            if ext != "frm" {
                bail!(
                    "Vault segregation violation: frm/ accepts ONLY .FRM files (got '{filename}')"
                );
            }
        }
        VaultBucket::Recompiled => {}
    }
    Ok(())
}

fn rewrite_bucket_sums_manifest(
    session_dir: &Path,
    bucket: VaultBucket,
    records: &[VaultArtifactRecord],
) -> Result<()> {
    let mut text = String::new();
    for rec in records {
        text.push_str(&format!(
            "{}  {}/{}\n",
            rec.sha256,
            bucket.dir_name(),
            rec.filename
        ));
    }
    let path = session_dir.join(bucket.sums_filename());
    fs::write(&path, text)
        .with_context(|| format!("Writing vault checksum manifest {}", path.display()))
}

/// Persists bytes into `raw/`, `frm/`, or `recompiled/` enforcing strict immutability and `Mn103Vm` gate on `recompiled/`.
pub fn ingest_vault_artifact(
    session_dir: &Path,
    source_bytes: &[u8],
    filename: &str,
    bucket: VaultBucket,
    origin: &str,
    emu_gate: Option<&PreWriteGateReport>,
) -> Result<VaultArtifactRecord> {
    let clean_name = validate_safe_segment(filename, "artifact filename")?;
    validate_bucket_extension(&clean_name, bucket)?;
    if source_bytes.is_empty() {
        bail!("Cannot ingest empty artifact '{clean_name}' into vault");
    }

    let emu_verified = match bucket {
        VaultBucket::Recompiled => {
            let gate = emu_gate.filter(|g| g.passed).with_context(|| {
                format!(
                    "FAIL-CLOSED: Cannot write '{clean_name}' to recompiled/ without passed Mn103Vm PreWriteGateReport"
                )
            })?;
            let report_path = session_bucket_dir(session_dir, bucket)?.join("emu_gate_report.json");
            let report_json = serde_json::to_string_pretty(gate)?;
            fs::write(&report_path, report_json)?;
            true
        }
        VaultBucket::Raw | VaultBucket::Frm => emu_gate.is_some_and(|g| g.passed),
    };

    let bucket_dir = session_bucket_dir(session_dir, bucket)?;
    let dst = bucket_dir.join(&clean_name);
    let (_, sha256) = calculate_hashes(source_bytes);

    if dst.is_file() && matches!(bucket, VaultBucket::Raw | VaultBucket::Frm) {
        let existing = fs::read(&dst)?;
        let (_, existing_sha) = calculate_hashes(&existing);
        if existing_sha != sha256 {
            bail!(
                "Immutable vault artifact conflict at {}: existing SHA-256 {} differs from {}",
                dst.display(),
                existing_sha,
                sha256
            );
        }
    } else {
        fs::write(&dst, source_bytes)
            .with_context(|| format!("Writing vault artifact {}", dst.display()))?;
    }

    let rel = format!("{}/{}", bucket.dir_name(), clean_name);
    let record = VaultArtifactRecord {
        id: format!("{}:{clean_name}", bucket.dir_name()),
        bucket,
        filename: clean_name,
        relative_path: rel,
        size_bytes: source_bytes.len(),
        sha256,
        origin: origin.trim().to_string(),
        created_at_epoch_s: epoch_secs_now(),
        emu_verified,
    };

    let mut manifest = load_vault_session(session_dir)?;
    let list = match bucket {
        VaultBucket::Raw => &mut manifest.raw_artifacts,
        VaultBucket::Frm => &mut manifest.frm_artifacts,
        VaultBucket::Recompiled => {
            if let Some(gate) = emu_gate {
                manifest.last_emu_gate = Some(gate.clone());
            }
            &mut manifest.recompiled_artifacts
        }
    };
    list.retain(|r| r.filename != record.filename);
    list.push(record.clone());
    rewrite_bucket_sums_manifest(session_dir, bucket, list)?;

    if matches!(bucket, VaultBucket::Raw | VaultBucket::Frm)
        && !manifest
            .completed_steps
            .contains(&StrictWorkflowStep::CaptureAndBackup)
    {
        manifest
            .completed_steps
            .push(StrictWorkflowStep::CaptureAndBackup);
        if manifest.current_step < StrictWorkflowStep::OrganizeAndRecompile {
            manifest.current_step = StrictWorkflowStep::OrganizeAndRecompile;
        }
    }
    save_vault_session(session_dir, &manifest)?;
    Ok(record)
}

/// Automatically classifies a local `.bin` vs `.FRM` source via [`fingerprint_binary`] and archives a copy into the vault.
pub fn auto_classify_and_ingest_file(
    session_dir: &Path,
    source_path: &Path,
    bucket_override: Option<VaultBucket>,
    origin: &str,
    emu_gate: Option<&PreWriteGateReport>,
) -> Result<VaultArtifactRecord> {
    let bytes = fs::read(source_path)
        .with_context(|| format!("Reading source file {}", source_path.display()))?;
    let name = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .context("Invalid source filename")?;

    let ext_is_frm = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("frm"));

    let detected_bucket = bucket_override.unwrap_or_else(|| {
        if ext_is_frm {
            return VaultBucket::Frm;
        }
        let fp = fingerprint_binary(&bytes, name);
        match fp.container {
            ContainerFormat::Sd48B
            | ContainerFormat::Sd64B
            | ContainerFormat::Dvd48B
            | ContainerFormat::Dvd64B => VaultBucket::Frm,
            _ => VaultBucket::Raw,
        }
    });

    ingest_vault_artifact(session_dir, &bytes, name, detected_bucket, origin, emu_gate)
}

/// Scans an 8 MiB NOR capture for an embedded `EXTRHEAD "DRV "` container (plaintext or Feistel-wrapped)
/// and archives it into `raw/drv_from_nor.bin`.
pub fn extract_nor_drv_into_vault(
    session_dir: &Path,
    nor_bytes: &[u8],
) -> Result<VaultArtifactRecord> {
    let limit = nor_bytes.len().min(NOR_DRV_SEARCH_END);
    if limit <= NOR_DRV_SEARCH_BASE + 64 {
        bail!(
            "NOR image too small ({} B) to scan for embedded DRV container",
            nor_bytes.len()
        );
    }
    let mut off = NOR_DRV_SEARCH_BASE;
    while off + 0x48 <= limit {
        let head = &nor_bytes[off..off + 8];
        if !head.iter().all(|&b| b == 0xFF) {
            let mode = if head == b"EXTRHEAD" {
                ExtrheadCryptMode::Plain
            } else {
                ExtrheadCryptMode::Drv
            };
            let slice = &nor_bytes[off..limit];
            if let Ok(unpacked) = unpack_extrhead(slice, mode)
                && unpacked.header.tag.trim().eq_ignore_ascii_case("DRV")
                && unpacked.checksum_valid
            {
                let footer_span = if unpacked.header.footer_offset >= 0x40 {
                    unpacked.header.footer_offset as usize + 8
                } else {
                    0x40 + unpacked.header.src_len as usize + 8
                };
                let total_len = ((footer_span + 7) & !7).min(slice.len());
                return ingest_vault_artifact(
                    session_dir,
                    &slice[..total_len],
                    "drv_from_nor.bin",
                    VaultBucket::Raw,
                    &format!("extracted_nor@0x{off:06X}"),
                    None,
                );
            }
        }
        off += 0x1000;
    }
    bail!("No EXTRHEAD DRV container found inside NOR 0x080000..0x200000")
}
