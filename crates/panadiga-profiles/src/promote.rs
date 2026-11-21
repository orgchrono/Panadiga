#![deny(unsafe_code)]

//! Evidence-gated claim promotion (`panadiga profiles promote`):
//! prevents elevating any hardware/layout/train claim tier without valid SHA-256 + probe fixture
//! (and requires a bench capture log under `docs/debug_logs/` for `T4 bench_verified`).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

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

fn validate_sha256_hex(raw: &str) -> Result<String> {
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed.len() != 64 || !trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("evidence_sha256 must be exactly 64 hex characters (got '{raw}')");
    }
    Ok(trimmed)
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

/// Validates proof artifacts, updates the target model manifest JSON cleanly, and re-validates the bundle.
pub fn promote_model_claim(req: &PromoteClaimRequest<'_>) -> Result<ModelProfileManifest> {
    let sha = validate_sha256_hex(req.evidence_sha256)?;
    let fixture_abs = req.workspace_root.join(req.fixture_rel_path);
    if !fixture_abs.is_file() {
        bail!(
            "Cannot promote '{}': missing probe fixture file '{}'",
            req.model_id,
            fixture_abs.display()
        );
    }
    let fixture_norm = req.fixture_rel_path.replace('\\', "/");

    if req.new_tier == EvidenceTier::BenchVerified {
        let log_rel = req.bench_log_rel_path.ok_or_else(|| {
            anyhow::anyhow!(
                "Promoting '{}' to T4 bench_verified requires --bench-log <docs/debug_logs/...>",
                req.model_id
            )
        })?;
        let norm_log = log_rel.replace('\\', "/");
        if !norm_log.starts_with("docs/debug_logs/") {
            bail!("Bench verification log '{log_rel}' must live under docs/debug_logs/");
        }
        let log_abs = req.workspace_root.join(log_rel);
        if !log_abs.is_file() {
            bail!("Missing bench capture log '{}'", log_abs.display());
        }
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
