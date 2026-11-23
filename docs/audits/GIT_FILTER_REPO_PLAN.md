# Git History Rewrite & Binary/Credential Scrubbing Plan (`git-filter-repo`)

> **STATUS: PROPOSED / UNEXECUTED — REQUIRES EXPLICIT USER APPROVAL BEFORE EXECUTION**
>
> **Notice:** Zero history rewriting was executed during Milestone M1. This document is strictly an approval-gated operational plan generated from a read-only object audit of base commit `a187e6990a01ec1481062dfcceff691f93321044` (`main`). Do **not** run `git filter-repo`, `git filter-branch`, `git rebase -i` over pre-`a187e69` history, or `git push --force` / `--force-with-lease` without explicit user sign-off, completed off-repo binary hosting, and CI test decoupling.

---

## 1. Scope & Motivation

The Panadiga Git pack (`a187e6990a01ec1481062dfcceff691f93321044`) currently stores **6,221 unique blobs** totaling **420,901,687 bytes (401.40 MiB uncompressed)** across **9,667 in-pack objects (225.51 MiB pack size)** (`git count-objects -vH`). Over **51.0%** of uncompressed blob history (**214,701,958 bytes / 204.76 MiB**) consists of two non-source binary sets:

1. **32 PDF service manuals and component datasheets** under `manuals_and_schematics/` (`151,177,031 B` / `144.17 MiB`).
2. **8 Panasonic firmware / NOR / NVRAM / HDD binary files** across **13 historical blob revisions** under `firmware_and_images/` (`63,524,927 B` / `60.58 MiB`).
3. **4 stray Windows cache `.db` blobs** (`988,568 B`) briefly committed in `d0740f6` and deleted in `bf66cc5`, plus **credential/keystore audit** (`0` keystore files in history; hardcoded dev/bench keystore strings `panadigabench` / `panadiga` in `scripts/build_lanes/android_verify.py:19-20` invoked by `.github/workflows/release.yml:440`).

Removing these binary blobs from Git history—once offloaded to GitHub Releases / an external verified asset store—will dramatically shrink clone sizes while preserving full SHA-256 verification and documentation manifests in Git.

---

## 2. Prerequisites & Local Tool Availability

Verified on workstation (`a187e69` baseline):

| Tool | Local Status | Required Version / Installation |
| :--- | :--- | :--- |
| `git` | **Installed** (`git version 2.55.0.windows.5`) | Git `>= 2.36` (satisfied) |
| `git-filter-repo` | **NOT INSTALLED** (`git filter-repo --version` exited `1`; `pip show` not found) | `python -m pip install git-filter-repo` (or download standalone `git-filter-repo` script and place on `PATH`) |
| `gitleaks` | **NOT INSTALLED** (`where.exe gitleaks` not found) | `winget install Gitleaks.Gitleaks` / `choco install gitleaks` / pinned GitHub Release binary verified by SHA-256 |

---

## 3. Exact Verified Target Inventory (`a187e69`)

### 3.1 Baseline Repository Refs & Object Counts

- `refs/heads/main`: `a187e6990a01ec1481062dfcceff691f93321044`
- `refs/remotes/origin/main`: `a187e6990a01ec1481062dfcceff691f93321044`
- `refs/tags/firmware-pack-v0.1.0`: `659106befb25f05edb2e1412bb41e99fa75a57f8`
- `refs/tags/v0.1.0`: `fd69e70063e5d62e9787f8084e9db5343e8390cc`
- `refs/tags/v0.2.0`: `a187e6990a01ec1481062dfcceff691f93321044`
- Unique blobs: `6,221` (`420,901,687 B` / `401.40 MiB` uncompressed; `225.51 MiB` pack)

### 3.2 Target Set A — 32 PDFs in `manuals_and_schematics/` (`151,177,031 B` / `144.17 MiB`, 32 blobs)

Each PDF has 1 unique historical blob:

- `manuals_and_schematics/datasheets/matsushita_mn10300_series_c_debugger_users_manual.pdf` (`1,071,161 B`, blob `23636fffbed1`)
- `manuals_and_schematics/datasheets/matsushita_mn10300_series_cross_assembler_users_manual.pdf` (`772,116 B`, blob `aa4b0cb07f66`)
- `manuals_and_schematics/datasheets/microchip_24lc64_datasheet.pdf` (`602,424 B`, blob `e251a4ba6e5e`)
- `manuals_and_schematics/datasheets/rohm_br24l16_datasheet.pdf` (`693,706 B`, blob `dafa252aa93a`)
- `manuals_and_schematics/datasheets/rp2040_datasheet_raspberrypi.pdf` (`5,301,205 B`, blob `0dfae4ed1180`)
- `manuals_and_schematics/datasheets/rp2350_datasheet_raspberrypi.pdf` (`11,094,994 B`, blob `77073d132660`)
- `manuals_and_schematics/datasheets/spansion_am29lv640d_nor_flash_datasheet.pdf` (`1,011,821 B`, blob `909c64b3ac6d`)
- `manuals_and_schematics/datasheets/spansion_s29gl128n_nor_flash_datasheet.pdf` (`4,223,160 B`, blob `99c602e62b62`)
- `manuals_and_schematics/dmr-e500h/user/panasonic_dmr_e500h_user_manual.pdf` (`4,980,623 B`, blob `59e26a671094`)
- `manuals_and_schematics/dmr-e55/user/panasonic_dmr_e55_operating_instructions.pdf` (`3,293,547 B`, blob `43d86ebf5c54`)
- `manuals_and_schematics/dmr-e85h/user/panasonic_dmr_e85h_user_manual.pdf` (`2,731,428 B`, blob `793b5ee6605c`)
- `manuals_and_schematics/dmr-eh50/user/panasonic_dmr_eh50_user_manual.pdf` (`4,132,934 B`, blob `b639d414081e`)
- `manuals_and_schematics/dmr-eh55/firmware_update/panasonic_dmr_eh55_firmware_installation_guide.pdf` (`197,887 B`, blob `3ff8494cf6a1`)
- `manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_europe.pdf` (`5,638,504 B`, blob `4a3f753d23ff`)
- `manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol1_diagnostics.pdf` (`6,015,970 B`, blob `42d7da8be374`)
- `manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf` (`8,655,525 B`, blob `6d6c5c862ac0`)
- `manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ee_service_manual_east_europe.pdf` (`5,385,311 B`, blob `0b7f3639c6a6`)
- `manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf` (`6,177,046 B`, blob `b3b99ac7729b`)
- `manuals_and_schematics/dmr-eh55/user/panasonic_diga_dmr_eh55_user_manual.pdf` (`5,230,107 B`, blob `c28dd7f881d1`)
- `manuals_and_schematics/dmr-eh55/user/panasonic_dmr_eh55_australia_new_zealand_manual.pdf` (`4,337,730 B`, blob `5861358e51e0`)
- `manuals_and_schematics/dmr-eh55/user/panasonic_dmr_eh55_eh56_operating_instructions_europe.pdf` (`5,203,293 B`, blob `c2ce1da41841`)
- `manuals_and_schematics/dmr-eh55/user/panasonic_dmr_eh55_hdmi_hdavi_control_guide.pdf` (`366,150 B`, blob `afccdfcb1d98`)
- `manuals_and_schematics/dmr-eh55/user/panasonic_dmr_eh55_operating_instructions_global.pdf` (`5,287,663 B`, blob `54ad87e59679`)
- `manuals_and_schematics/dmr-eh55/user/panasonic_dmr_eh55_quick_setup_guide.pdf` (`2,772,445 B`, blob `1875fb1c7b18`)
- `manuals_and_schematics/dmr-eh55/user/panasonic_dmr_eh55_user_manual_oceania.pdf` (`4,819,044 B`, blob `ac3d27db25eb`)
- `manuals_and_schematics/dmr-eh55/user/panasonic_dmr_eh55s_operating_instructions_silver.pdf` (`5,062,896 B`, blob `11b1b3c33de5`)
- `manuals_and_schematics/dmr-eh56/user/panasonic_dmr_eh56_operating_instructions.pdf` (`5,136,699 B`, blob `289db2fcc9f5`)
- `manuals_and_schematics/dmr-eh65/user/panasonic_dmr_eh65_user_manual.pdf` (`4,851,492 B`, blob `1bc8729d64fd`)
- `manuals_and_schematics/dmr-eh80v/service/panasonic_dmr_eh80v_service_manual_europe.pdf` (`12,261,463 B`, blob `b9fe740c00a0`)
- `manuals_and_schematics/dmr-es10/service/panasonic_dmr_es10_service_manual_north_america.pdf` (`5,392,119 B`, blob `4cb1c105729b`)
- `manuals_and_schematics/dmr-es30v/user/panasonic_dmr_es30v_user_manual.pdf` (`7,997,852 B`, blob `c24c0f221e92`)
- `manuals_and_schematics/dmr-ex98v/service/panasonic_dmr_ex98v_service_manual_vol1.pdf` (`10,458,698 B`, blob `7260c58c73b2`)

**Explicitly PRESERVED in `manuals_and_schematics/`:**
- `manuals_and_schematics/README.md` (3 historical revisions)
- `manuals_and_schematics/datasheets/SHA256SUMS.txt`

### 3.3 Target Set B — 8 Panasonic Binaries in `firmware_and_images/` (`63,524,927 B` / `60.58 MiB`, 13 historical blobs)

| # | Path | Historical Blobs & Sizes | Total Bytes | Current Disk SHA-256 |
| :---: | :--- | :--- | :---: | :--- |
| 1 | `firmware_and_images/eh55_ifodev2_m6930lb_2048b.bin` | 1 blob: `2,048 B` (`f3d352250966`) | `2,048` | `f4f22dd0407d9f0bebf4621e938f2ca02e3b6c5f23b21ae972c5185704a77676` |
| 2 | `firmware_and_images/eh55_loader_ver061_256k.bin` | 1 blob: `262,144 B` (`4ada81b8ac51`) | `262,144` | `9f5c448d8879106be5ca3da6d6ac649026d50fbc191251ae0f4a003efca63800` |
| 3 | `firmware_and_images/eh55_nor_8mb_ic5001_dump.bin` | 1 blob: `8,388,608 B` (`f4d64eb92014`) | `8,388,608` | `c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d` |
| 4 | `firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin` | **3 blobs** (`54ac7cf`, `dd8dece`, `55a7ccf`): `8,388,608 B` each (`dff9d14ff9f4`, `86b90506214a`, `f87629c6659a`) | `25,165,824` | `bd089ed1a062c48756c60407e6555463e8646cde094b0328b88a253e4f1d4121` |
| 5 | `firmware_and_images/eh55_nvram_64k_raw.bin` | 1 blob: `65,536 B` (`9a335abdbf2d`) | `65,536` | `803d1465f48fb32e17d3e478e02b2a9a7e707bbb675bb424b02c5c1df92f3643` |
| 6 | `firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin.gz` | 1 blob: `7,190,807 B` (`2e4b9f5ac680`) | `7,190,807` | `2aef5745ca7d83b4de22e7b2389a92dd36380e2a0ea3dcbaf0b3fbd428b4d2fa` |
| 7 | `firmware_and_images/sd_unbrick_kit/DIGADIAG.FRM` | **2 blobs** (`54ac7cf`, `475fd48`): `262,144 B` each (`21939fa6e016`, `6bb8589898a1`) | `524,288` | `305cc2eebc09bbaa3bb5c5d75c0395eb16831bed4b9f80eafd57b9945e6a762d` |
| 8 | `firmware_and_images/sd_unbrick_kit/PANA__SD.FRM` | **3 blobs** (`54ac7cf`, `475fd48`, `1587125`): `7,181,224 B` (`e190081eb8cb`), `7,181,224 B` (`897499a56b66`), `7,563,224 B` (`5a91dcabdb02`) | `21,925,672` | `2d3d62a75731f150762f269e3b650e83307da983d8d4bf222ec078b45298d9d0` |

**Explicitly PRESERVED in `firmware_and_images/`:**
- All `README.md` files (`firmware_and_images/README.md`, `dmr-eh49_59_69/README.md`, `dmr-eh50/README.md`, `dmr-eh58/README.md`, `dmr-es35v/README.md`, `dmr-ex98v/README.md`, `dmr-ex_ez_series/README.md`, `dmr-ez485v/README.md`, `dmr-xw_bd_series/README.md`)
- `firmware_and_images/dmr-xw_bd_series/SHA256SUMS.txt`
- `firmware_and_images/sd_unbrick_kit/SHA256SUMS.txt`
- `firmware_and_images/sd_unbrick_kit/LEIA_ME_CARTAO_SD.txt`
- `firmware_and_images/sd_unbrick_kit/prepare_report.json`
- `firmware_and_images/Panasonic_DVD_TV.ir` (`4,639 B` IR capture text)

*(Note: local untracked directories `firmware_and_images/sd_unbrick_kit_sata_bridge/` and `firmware_and_images/sd_unbrick_kit_sata_bridge_stage2/` are gitignored and not in Git history.)*

### 3.4 Target Set C — Stray Historical Artifacts, Legacy UF2 Note & Credential Audit

1. **Spurious Windows Cache `.db` Blobs (`988,568 B`, added in `d0740f6`, deleted in `bf66cc5`):**
   - `%SystemDrive%/ProgramData/Microsoft/Windows/Caches/cversions.2.db` (`16,384 B`, blob `81f118bada9d`)
   - `%SystemDrive%/ProgramData/Microsoft/Windows/Caches/{30A716B8-E0D8-4F94-863E-6476B9717197}.2.ver0x0000000000000001.db` (`1,240 B`, blob `f254a998b8e6`)
   - `%SystemDrive%/ProgramData/Microsoft/Windows/Caches/{6AF0698E-D558-4F6E-9B3C-3716689AF493}.2.ver0x0000000000000001.db` (`308,896 B`, blob `7cf02322f159`)
   - `%SystemDrive%/ProgramData/Microsoft/Windows/Caches/{DDF571F2-BE98-426D-8288-1A9A39C3FDA2}.2.ver0x0000000000000001.db` (`662,048 B`, blob `d25b1a92142d`)
2. **Legacy `.uf2` Firmware Blobs Note (Separate Review Only — Do NOT Purge Blindly):**
   - History contains 88 `.uf2` blobs: 81 historical revisions across 3 files in `crates/panadiga-core/src/pico/firmware_bins/*.uf2` and 7 blobs in deleted legacy `src/pico/firmware_bins/*.uf2`.
   - The active `.uf2` binaries at `HEAD` in `crates/panadiga-core/src/pico/firmware_bins/` are compiled directly into Rust via `include_bytes!` and **must NOT be removed from `HEAD`**. Only legacy `src/pico/firmware_bins/*.uf2` (or squashed non-HEAD revisions via `--strip-blobs-bigger-than` only after separate review) could ever be pruned without breaking `cargo build`.
3. **Keystores, Secrets & Credentials Audit:**
   - **Blob scan (`git rev-list --objects --all`)**: **0** files matching `*.jks`, `*.keystore`, `*.pem`, `*.key`, `*.p12`, `*.pfx`, `*.ukf`, `*.crt`, `*.cer`, `*.der`, `.env*`, `*secret*`, `*keystore*`, `*credential*`, or `*password*` exist in Git history.
   - Local `apps/desktop/src-tauri/gen/android/keystore/panadiga-bench.keystore` is untracked and gitignored.
   - **Hardcoded bench keystore strings in source (`scripts/build_lanes/android_verify.py:19-20`, commit `cebd217`, invoked by `.github/workflows/release.yml:440`)**:
     - `BENCH_KEY_ALIAS = "panadigabench"`
     - `BENCH_KEY_PASS = "panadiga"`
     - These are dev/bench-only placeholders (to be replaced in Wave P0 via `--dev-bench` + `env:ANDROID_KS_PASS`), never to be reused for production releases.
     - If full history redaction of those strings is desired alongside binary purging, use **strictly scoped regexes** in `replace-text.txt` (never a bare `panadiga==>` rule, which would corrupt crate names, URLs, and identifiers across the entire repo):
       ```text
       regex:BENCH_KEY_PASS = "panadiga"==>BENCH_KEY_PASS = "REDACTED_BENCH_PASS"
       regex:BENCH_KEY_ALIAS = "panadigabench"==>BENCH_KEY_ALIAS = "REDACTED_BENCH_ALIAS"
       ```

---

## 4. Pre-Execution Blockers (Mandatory Before Filtering)

Do **not** execute `git filter-repo` until all 5 blockers below are completed:

1. **Offload & Pin External Assets:** Upload the 32 PDFs and 8 Panasonic binaries to GitHub Releases (`firmware-pack-v0.1.0` / `manuals-pack-v1.0.0`) or a verified asset mirror, retaining matching SHA-256 checksums (`SHA256SUMS.txt`) and a deterministic fetch/hydrate script for developers/CI.
2. **Update `.gitignore` for Offloaded Paths:** Remove the `!firmware_and_images/*.bin`, `!firmware_and_images/*.bin.gz`, `!firmware_and_images/sd_unbrick_kit/DIGADIAG.FRM`, and `!firmware_and_images/sd_unbrick_kit/PANA__SD.FRM` allowlist lines (keeping `!firmware_and_images/**/README.md` and metadata manifests), and ignore `manuals_and_schematics/**/*.pdf`.
3. **Update `docs/evidence/ledger.json` & `scripts/tests/test_docs_parity.py`:** Currently `docs/evidence/ledger.json` marks 5 of the 8 binaries `"required": true` (`DIGADIAG.FRM`, `PANA__SD.FRM`, `eh55_nor_8mb_ic5001_dump.bin`, `eh55_nor_8mb_ic5001_repaired.bin`, `eh55_ifodev2_m6930lb_2048b.bin`), and `test_docs_parity.py:279-288` fails if `"required": true` files are missing or gitignored. Update those entries (`required: false` or optional-asset verification when hydrated).
4. **Update `crates/panadiga-core/tests/firmware_scan_tests.rs:96-108` (`test_manuals_search_pdf`):** Currently asserts `!res.is_empty()` whenever `manuals_and_schematics/` exists; update it to skip/pass cleanly when no `*.pdf` files are present locally.
5. **Update `crates/panadiga-cli/tests/cli_e2e_parity.rs:96-107`:** Currently runs `unbrick verify-sd ... firmware_and_images/sd_unbrick_kit` expecting `DIGADIAG.FRM` and `PANA__SD.FRM` on disk; generate synthetic test fixtures in a temp dir or gate on local asset presence.

---

## 5. Step-by-Step Safe Execution Procedure (Approval-Gated)

Once the user approves and Section 4 blockers are resolved:

### Step 1: Pre-Rewrite Baseline, SHA-256 Manifest & Secret Scan
```bash
git show-ref > ../pre-filter-refs.txt
git count-objects -vH > ../pre-filter-objects.txt
gitleaks detect --source . --log-opts="--all" --report-format json --report-path ../gitleaks-pre-audit.json
```
Verify and back up all 32 PDFs and 8 Panasonic binaries outside the repo with their SHA-256 hashes (Section 3.3).

### Step 2: Create Isolated Bare Mirror Backup & Working Mirror
Never run `git filter-repo` inside your active working tree (`c:\Users\danie\Documents\Panadiga`):
```bash
cd ..
git clone --mirror https://github.com/orgchrono/Panadiga.git Panadiga-backup-mirror.git
git clone --mirror Panadiga-backup-mirror.git Panadiga-filter-work.git
cd Panadiga-filter-work.git
```

### Step 3: Non-Mutating Object Analysis
```bash
git filter-repo --analyze
```
Inspect `.git/filter-repo/analysis/path-all-sizes.txt` and `.git/filter-repo/analysis/blob-shas-and-paths.txt` to confirm no unexpected large files or credentials were missed.

### Step 4: Prepare Explicit `filter-paths.txt` & Run `--dry-run`
Create `../filter-paths.txt` containing the exact 32 PDF paths (Section 3.2), 8 binary paths (Section 3.3), and 4 `%SystemDrive%/ProgramData/Microsoft/Windows/Caches/*.db` paths (Section 3.4), and inspect the dry-run report:
```bash
git filter-repo --dry-run --paths-from-file ../filter-paths.txt --invert-paths
```

### Step 5: Execute Live Filter in Isolated Working Mirror
```bash
git filter-repo --paths-from-file ../filter-paths.txt --invert-paths
# Optional (only if approved to scrub historical dev/bench strings in android_verify.py):
# git filter-repo --replace-text ../replace-text.txt
git reflog expire --expire=now --all
git gc --prune=now --aggressive
git count-objects -vH
```

### Step 6: Post-Rewrite Verification
1. Confirm zero targeted paths remain anywhere in rewritten history:
   ```bash
   git log --all --stat -- manuals_and_schematics/*.pdf firmware_and_images/*.bin firmware_and_images/*.FRM firmware_and_images/*.bin.gz
   ```
2. Run `gitleaks detect --source . --log-opts="--all"` on the filtered mirror.
3. Clone `../Panadiga-filter-work.git` into a fresh working copy, restore/hydrate local untracked binaries if needed for offline bench testing, verify SHA-256 checksums of all preserved files, and run `python scripts/validate.py` (all Quality Gates must pass).

---

## 6. Remote Rollout, Force-Push Warning & Team Re-Clone Protocol

- **Destructive History Impact:** Executing `git filter-repo` alters **every commit SHA** across all branches and invalidates/rewrites all **3 existing tags** (`firmware-pack-v0.1.0`, `v0.1.0`, `v0.2.0`).
- **Coordinated Rollout:**
  1. Freeze pushes and close/merge open PRs.
  2. Temporarily adjust GitHub branch protection if force-pushes to `main` are blocked.
  3. Push rewritten history from the verified mirror:
     ```bash
     git remote add origin https://github.com/orgchrono/Panadiga.git
     git push --force-with-lease origin main
     git push --force origin --tags
     ```
  4. Re-verify/re-attach assets on GitHub Releases `firmware-pack-v0.1.0`, `v0.1.0`, and `v0.2.0`, and contact GitHub Support if orphaned cached commit SHAs must be garbage-collected on GitHub's servers.
- **Mandatory Collaborator / Workstation Re-Clone Instructions:**
  - **NEVER** run `git pull` or `git merge` from an old clone onto the rewritten `main` (that re-introduces the purged 204.76 MiB history).
  - Either perform a fresh `git clone https://github.com/orgchrono/Panadiga.git`, or back up any local untracked assets and reset cleanly:
    ```bash
    git fetch origin
    git checkout main
    git reset --hard origin/main
    git reflog expire --expire=now --all
    git gc --prune=now --aggressive
    ```
