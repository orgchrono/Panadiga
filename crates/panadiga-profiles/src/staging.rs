#![forbid(unsafe_code)]

//! Staging & review pipeline for manual SSOT claim promotion proposals under
//! `profiles/staging/manual_proposals/`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::promote::{
    PromoteClaimRequest, compute_file_sha256_hex, promote_model_claim, resolve_within_root,
    validate_sha256_hex,
};
use crate::types::{ClaimAxis, EvidenceTier};

/// Relative directory inside `profiles/` holding queued manual claim promotion proposals.
pub const MANUAL_PROPOSALS_REL_DIR: &str = "staging/manual_proposals";

/// JSON proposal queued in `profiles/staging/manual_proposals/<proposal_id>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManualClaimProposal {
    pub proposal_id: String,
    pub model_id: String,
    pub axis: ClaimAxis,
    pub proposed_tier: EvidenceTier,
    pub fixture_rel_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bench_log_rel_path: Option<String>,
    pub declared_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_entry_id: Option<String>,
}

/// Deterministic review output produced by `panadiga profiles review-proposal`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProposalReviewReport {
    pub proposal_id: String,
    pub model_id: String,
    pub valid: bool,
    pub computed_sha256: Option<String>,
    pub declared_sha256: String,
    pub sha256_verified: bool,
    pub ledger_synced: bool,
    pub issues: Vec<String>,
    pub promoted: bool,
}

/// Lists sorted `.json` proposal manifests in `<profiles_dir>/staging/manual_proposals`.
pub fn list_manual_proposals(profiles_dir: &Path) -> Result<Vec<PathBuf>> {
    let dir = profiles_dir.join("staging").join("manual_proposals");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .with_context(|| format!("Reading manual proposals dir '{}'", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect();
    paths.sort();
    Ok(paths)
}

fn check_ledger_sync(
    ledger_path: Option<&Path>,
    actual_sha: &str,
    entry_id: Option<&str>,
    issues: &mut Vec<String>,
) -> bool {
    let Some(path) = ledger_path else {
        return false;
    };
    if !path.is_file() {
        issues.push(format!("Ledger file not found: '{}'", path.display()));
        return false;
    }
    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            issues.push(format!("Failed reading ledger '{}': {err}", path.display()));
            return false;
        }
    };
    let doc: Value = match serde_json::from_str(&raw) {
        Ok(parsed) => parsed,
        Err(err) => {
            issues.push(format!("Invalid ledger JSON '{}': {err}", path.display()));
            return false;
        }
    };
    let Some(entries) = doc.get("entries").and_then(Value::as_array) else {
        issues.push("Ledger JSON missing 'entries' array".to_string());
        return false;
    };
    let matched = entries.iter().any(|entry| {
        if let Some(req_id) = entry_id {
            let matches_id = entry.get("id").and_then(Value::as_str) == Some(req_id);
            if !matches_id {
                return false;
            }
        }
        entry
            .get("sha256_assertions")
            .and_then(Value::as_array)
            .is_some_and(|assertions| {
                assertions.iter().any(|a| {
                    a.get("sha256")
                        .and_then(Value::as_str)
                        .is_some_and(|s| s.eq_ignore_ascii_case(actual_sha))
                })
            })
    });
    if !matched {
        issues.push(format!(
            "SHA-256 '{}' not registered in ledger '{}'",
            actual_sha,
            path.display()
        ));
    }
    matched
}

/// Validates a staged `ManualClaimProposal` against real on-disk hashes and optional ledger,
/// promoting the target model profile when `apply == true` and all checks pass.
pub fn review_manual_proposal(
    workspace_root: &Path,
    profiles_dir: &Path,
    proposal_path: &Path,
    apply: bool,
    ledger_path: Option<&Path>,
) -> Result<ProposalReviewReport> {
    let raw = fs::read_to_string(proposal_path)
        .with_context(|| format!("Reading proposal '{}'", proposal_path.display()))?;
    let proposal: ManualClaimProposal = serde_json::from_str(&raw)
        .with_context(|| format!("Parsing proposal '{}'", proposal_path.display()))?;

    let mut issues = Vec::new();
    let declared_sha256 = match validate_sha256_hex(&proposal.declared_sha256) {
        Ok(normalized) => normalized,
        Err(err) => {
            issues.push(err.to_string());
            proposal.declared_sha256.trim().to_ascii_lowercase()
        }
    };

    let fixture_abs =
        match resolve_within_root(workspace_root, &proposal.fixture_rel_path, "probe fixture") {
            Ok(path) if path.is_file() => Some(path),
            Ok(path) => {
                issues.push(format!("Missing probe fixture file '{}'", path.display()));
                None
            }
            Err(err) => {
                issues.push(err.to_string());
                None
            }
        };

    let bench_log_abs = if let Some(log_rel) = proposal.bench_log_rel_path.as_deref() {
        let norm_log = log_rel.trim().replace('\\', "/");
        if proposal.proposed_tier == EvidenceTier::BenchVerified
            && !norm_log.starts_with("docs/debug_logs/")
        {
            issues.push(format!(
                "Bench verification log '{log_rel}' must live under docs/debug_logs/"
            ));
        }
        match resolve_within_root(workspace_root, &norm_log, "bench capture log") {
            Ok(path) if path.is_file() => Some(path),
            Ok(path) => {
                issues.push(format!("Missing bench capture log '{}'", path.display()));
                None
            }
            Err(err) => {
                issues.push(err.to_string());
                None
            }
        }
    } else {
        if proposal.proposed_tier == EvidenceTier::BenchVerified {
            issues.push(
                "Proposed T4 bench_verified claim requires bench_log_rel_path under docs/debug_logs/"
                    .to_string(),
            );
        }
        None
    };

    let primary_path = bench_log_abs.as_deref().or(fixture_abs.as_deref());
    let computed_sha256 = match primary_path {
        Some(path) => match compute_file_sha256_hex(path) {
            Ok(digest) => Some(digest),
            Err(err) => {
                issues.push(err.to_string());
                None
            }
        },
        None => None,
    };

    let sha256_verified = match computed_sha256.as_deref() {
        Some(actual) if issues.is_empty() && actual == declared_sha256 => true,
        Some(actual) => {
            if actual != declared_sha256 {
                issues.push(format!(
                    "SHA-256 mismatch: declared {}, computed on disk {}",
                    declared_sha256, actual
                ));
            }
            false
        }
        None => false,
    };

    let ledger_synced = match computed_sha256.as_deref() {
        Some(actual) => check_ledger_sync(
            ledger_path,
            actual,
            proposal.ledger_entry_id.as_deref(),
            &mut issues,
        ),
        None => false,
    };

    let valid = issues.is_empty() && sha256_verified;
    let mut promoted = false;
    if apply && valid {
        let req = PromoteClaimRequest {
            workspace_root,
            profiles_dir,
            model_id: &proposal.model_id,
            axis: proposal.axis,
            new_tier: proposal.proposed_tier,
            evidence_sha256: &declared_sha256,
            fixture_rel_path: &proposal.fixture_rel_path,
            bench_log_rel_path: proposal.bench_log_rel_path.as_deref(),
            notes: proposal.notes.as_deref(),
        };
        promote_model_claim(&req)?;
        promoted = true;
    }

    Ok(ProposalReviewReport {
        proposal_id: proposal.proposal_id,
        model_id: proposal.model_id,
        valid,
        computed_sha256,
        declared_sha256,
        sha256_verified,
        ledger_synced,
        issues,
        promoted,
    })
}
