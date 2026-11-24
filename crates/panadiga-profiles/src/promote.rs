#![forbid(unsafe_code)]

//! Evidence-gated claim promotion (`panadiga profiles promote`):
//! prevents elevating any hardware/layout/train claim tier without valid on-disk SHA-256 + confined probe fixture
//! (and requires a bench capture log under `docs/debug_logs/` for `T4 bench_verified`).

use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::loader::{load_bundle_from_dir, model_storage_key};
use crate::types::{ClaimAxis, ClaimEvidence, EvidenceSource, EvidenceTier, ModelProfileManifest};

/// Request parameters for promoting a claim axis tier inside a model manifest.
#[derive(Debug, Clone)]
pub struct PromoteClaimRequest<'a> {
    pub workspace_root: &'a Path,
    pub profiles_dir: &'a Path,
    pub model_id: &'a str,
    pub axis: ClaimAxis,
    pub new_tier: EvidenceTier,
    pub evidence_sha256: &'a str,
    pub fixture_rel_path: &'a str,
    pub bench_log_rel_path: Option<&'a str>,
    pub notes: Option<&'a str>,
}

pub(crate) fn validate_sha256_hex(raw: &str) -> Result<String> {
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed.len() != 64 || !trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("evidence_sha256 must be exactly 64 hex characters (got '{raw}')");
    }
    Ok(trimmed)
}

/// Computes lowercase SHA-256 hex digest over real file bytes on disk.
pub fn compute_file_sha256_hex(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .with_context(|| format!("Reading '{}' for SHA-256 verification", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(hex::encode(digest))
}

/// Ensures `rel` is a non-empty relative path confined inside canonical `root`.
pub(crate) fn resolve_within_root(root: &Path, rel: &str, label: &str) -> Result<PathBuf> {
    let norm = rel.trim().replace('\\', "/");
    if norm.is_empty() {
        bail!("{label} path cannot be empty");
    }
    let rel_path = Path::new(&norm);
    if rel_path.is_absolute()
        || norm.starts_with('/')
        || rel_path.components().any(|c| {
            matches!(
                c,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
    {
        bail!("{label} '{rel}' must be a clean relative path without '..' or root prefixes");
    }
    let canon_root = fs::canonicalize(root)
        .with_context(|| format!("Canonicalizing workspace root '{}'", root.display()))?;
    let candidate = root.join(rel_path);
    let canon_target = fs::canonicalize(&candidate)
        .with_context(|| format!("Missing or inaccessible {label} '{}'", candidate.display()))?;
    if !canon_target.starts_with(&canon_root) {
        bail!(
            "{label} '{}' escapes workspace root '{}'",
            canon_target.display(),
            canon_root.display()
        );
    }
    Ok(canon_target)
}

fn claim_mut(m: &mut ModelProfileManifest, axis: ClaimAxis) -> &mut ClaimEvidence {
    match axis {
        ClaimAxis::Board => &mut m.claims.board,
        ClaimAxis::FlashChip => &mut m.claims.flash_chip,
        ClaimAxis::NorLayout => &mut m.claims.nor_layout,
        ClaimAxis::Train => &mut m.claims.train,
        ClaimAxis::Eeprom => &mut m.claims.eeprom,
    }
}

/// Validates on-disk proof SHA-256, updates the target model manifest JSON cleanly, and re-validates the bundle.
pub fn promote_model_claim(req: &PromoteClaimRequest<'_>) -> Result<ModelProfileManifest> {
    let sha = validate_sha256_hex(req.evidence_sha256)?;
    let fixture_abs =
        resolve_within_root(req.workspace_root, req.fixture_rel_path, "probe fixture")?;
    if !fixture_abs.is_file() {
        bail!(
            "Cannot promote '{}': missing probe fixture file '{}'",
            req.model_id,
            fixture_abs.display()
        );
    }
    let fixture_norm = req.fixture_rel_path.trim().replace('\\', "/");

    let bench_log_abs = if req.new_tier == EvidenceTier::BenchVerified {
        let log_rel = req.bench_log_rel_path.ok_or_else(|| {
            anyhow::anyhow!(
                "Promoting '{}' to T4 bench_verified requires --bench-log <docs/debug_logs/...>",
                req.model_id
            )
        })?;
        let norm_log = log_rel.trim().replace('\\', "/");
        if !norm_log.starts_with("docs/debug_logs/") {
            bail!("Bench verification log '{log_rel}' must live under docs/debug_logs/");
        }
        let log_abs = resolve_within_root(req.workspace_root, &norm_log, "bench capture log")?;
        if !log_abs.is_file() {
            bail!("Missing bench capture log '{}'", log_abs.display());
        }
        Some(log_abs)
    } else if let Some(log_rel) = req.bench_log_rel_path {
        let log_abs = resolve_within_root(req.workspace_root, log_rel, "bench capture log")?;
        if !log_abs.is_file() {
            bail!("Missing bench capture log '{}'", log_abs.display());
        }
        Some(log_abs)
    } else {
        None
    };

    let primary_artifact = bench_log_abs.as_deref().unwrap_or(&fixture_abs);
    let actual_sha = compute_file_sha256_hex(primary_artifact)?;
    if actual_sha != sha {
        bail!(
            "SHA-256 verification failed for '{}': declared {}, actual on disk {}",
            primary_artifact.display(),
            sha,
            actual_sha
        );
    }

    let model_key = model_storage_key(req.model_id);
    let model_path = req
        .profiles_dir
        .join("models")
        .join(format!("{model_key}.json"));
    if !model_path.is_file() {
        bail!(
            "Model manifest file '{}' not found for '{}'",
            model_path.display(),
            req.model_id
        );
    }

    let raw = fs::read_to_string(&model_path)
        .with_context(|| format!("Reading {}", model_path.display()))?;
    let mut manifest: ModelProfileManifest = serde_json::from_str(&raw)?;
    manifest.fixture_ref = Some(fixture_norm.clone());

    let target = claim_mut(&mut manifest, req.axis);
    if req.new_tier < target.tier {
        bail!(
            "Cannot demote {:?} on '{}' from {} to {} via promote",
            req.axis,
            req.model_id,
            target.tier.short_tag(),
            req.new_tier.short_tag()
        );
    }
    target.tier = req.new_tier;
    if !target
        .sources
        .iter()
        .any(|s| s.sha256.as_deref() == Some(sha.as_str()))
    {
        target.sources.push(EvidenceSource {
            kind: req
                .bench_log_rel_path
                .map_or_else(|| "probe_fixture".to_string(), |_| "bench_log".to_string()),
            reference: req
                .bench_log_rel_path
                .unwrap_or(&fixture_norm)
                .replace('\\', "/"),
            sha256: Some(sha),
            notes: req.notes.map(str::to_string),
        });
    }

    let mut out_json = serde_json::to_string_pretty(&manifest)?;
    out_json.push('\n');
    let backup_original = raw;
    fs::write(&model_path, &out_json)?;
    if let Err(e) = load_bundle_from_dir(req.profiles_dir) {
        let _ = fs::write(&model_path, backup_original);
        return Err(e);
    }
    Ok(manifest)
}
