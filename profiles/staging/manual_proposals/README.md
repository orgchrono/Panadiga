# Staged Manual SSOT Claim Proposals (`profiles/staging/manual_proposals/`)

This directory holds evidence-backed manual promotion proposals (`<proposal_id>.json`) before they are audited and merged into canonical model manifests under `profiles/models/*.json`.

## Required Verification Workflow

1. Author a `ManualClaimProposal` JSON referencing confined relative paths inside the repository (`fixture_rel_path`, and `bench_log_rel_path` under `docs/debug_logs/` for `bench_verified` `T4`) plus its exact SHA-256 digest and optional `ledger_entry_id`.
2. Verify on-disk hashes and `docs/evidence/ledger.json` synchronization in dry-run mode:
   ```bash
   panadiga profiles review-proposal --proposal profiles/staging/manual_proposals/<id>.json --json -
   ```
3. Apply promotion only when `valid == true` and `sha256_verified == true`:
   ```bash
   panadiga profiles review-proposal --proposal profiles/staging/manual_proposals/<id>.json --apply --json -
   ```
