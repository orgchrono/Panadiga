> 🌐 **Idioma / Language:** **English (US)** | [Português (BR)](../pt-BR/WHATS_NEXT_ONBOARDING_NOVOS_APARELHOS.md)
> 🧭 **Portal:** [Documentation Index](../INDEX.md) | [DIGA Hardware Profiles Catalog](DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# What's Next: End-to-End Onboarding Pipeline for New DIGA Devices (Bench → JSON SSOT → Pull Request)

> **Purpose:** This guide standardizes the complete *end-to-end* workflow to onboard any additional **Panasonic DIGA** recorder/player into **Panadiga** using a **100% declarative JSON SSOT architecture** — preventing product-hardcoded logic, maintaining strict parity across Rust/CLI/Tauri/Python/Ghidra, and enforcing bench silicon safety invariants.

---

## Overview of the 7 Onboarding Stages (`T0` → `T4`)

| Stage | Primary Objective | Canonical Tools / Scripts | Exit Gate |
| :--- | :--- | :--- | :--- |
| **1. Bench Wiring & `P7501` Safety** | Verify 3.3V LVTTL logic levels, keep `GP3`/`NSW5V` `Hi-Z`, and `XRESET` Open-Drain | Pico probe + `panadiga pico status` / `bench` | Clean capture in `docs/debug_logs/` with zero 5V overdrive |
| **2. NOR Capture (`2..16 MiB`)** | Read-only capture of IC5001 via `P7501` UART or external TSOP-48 programmer | `panadiga unbrick nor-stub`, `decode-nor-dump`, `panadiga profiles discover-nor`, `scripts/re/nor_intake_analyzer.py` | Immutable SHA-256 dump + draft `nor_layout` JSON |
| **3. `.FRM` Unpack, `panamon` & `rebuild-from-dump`** | Pin exact `DIGADIAG` loader family (`ver0.61`, `ver0.63`, ...), unpack `DRV`/`PROG`/`MAIN`, compose `PANA__SD.FRM` | `scripts/re/frm_train_harvester.py`, `scripts/re/pana_frm.py`, `panadiga unbrick extract-frm`, `panadiga unbrick rebuild-from-dump` | Verified `EXTRHEAD` hashes, `MAIN` build IDs & train metadata |
| **4. PDF Manual Hydration (`T1`)** | Extract PCB codes (`VEP`/`REP`), SoCs (`MN2DS...`), Flash (`S29...`/`MX29...`) & EEPROMs | `scripts/re/manual_spec_extractor.py --apply` | `boards/`, `socs/`, `flash_chips/`, `eeprom_chips/` promoted to `T1` |
| **5. Ghidra MN103S & Plain-Text Bundles** | Resolve the 7 MN103S traps, eliminate `UNDEF`, locate BGO/FTL, TBC, `DAVIS` & HDD VAs | `tools/ghidra/ghidra_zero_undef_pipeline.py`, `tools/ghidra/ghidra_export_pseudocode_bundle.py`, `panadiga ghidra export-all` | Domain plain-text bundles in `docs/pseudocode/` + `ghidra_ssot_maps.json` |
| **6. Declarative SSOT JSON Modeling** | Express BGO/NOR/HDD/TBC rules by geometry/train ID (`BGO_*`, `blobs/*.bin.gz`), never by product name in Rust/TS | `profiles/*.json`, `panadiga ssot export-all`, `panadiga profiles promote-evidence`, `panadiga profiles completeness` | `panadiga profiles validate` passes with `0` errors at `T2/T3/T4` |
| **7. Full Validation & Pull Request** | Verify CLI + Playwright E2E parity + 0% `jscpd` duplication prior to PR | `python scripts/validate.py` (`9/9` gates) | All 9 Quality Gates green + [`CONTRIBUTING.md`](../../CONTRIBUTING.md) Section 8 checklist |

---

## Step-by-Step Bench Execution Guide

### Stage 1 — Mandatory Electrical Checklist (`P7501` / `IC5001` / `CH1`)

Before connecting test leads or powering the unit:
1. Check the main board service schematic (`VEP79138`, `VEP79106A`, etc.) to confirm **`P7501` (`TO JIGU`)** pinout and **3.3V LVTTL** signaling.
2. On the Pico probe (`RP2040` / `RP2350`), verify **`GP3` (`NSW5V` / Pin 5)** stays **strict `Hi-Z Input` without pulls**, **`GP5` (`XRESET` / Pin 2)** runs strictly **Open-Drain**, and the Pico is **NEVER powered from `P7501`** (power externally via USB or `VSYS`).
3. On HDD boot failures (`UPD NG` / `MAIN` / `FINISH` or `0xE8000000` on `DAVIS CH1`), inspect **`P1503 Pin 4 (DR5V = +5.1V)`**, `P1503 Pin 1 (DR12V = +12.3V)`, and FFC ribbon contact polarity (`Type A` vs `Type B` on SATA adapters) before concluding firmware incompatibility.

### Stage 2 — Physical NOR Capture & Automated Triage

```bash
# 1. Build a read-only UART NOR dumper stub (or enter PANAMON1> via PANA__SD.FRM)
panadiga unbrick nor-stub --start 0x80000000 --len 0x800000 --out tmp/nor_stub.bin --wrap-digadiag

# 2. Store raw UART captures in docs/debug_logs/ and decode the immutable binary
panadiga unbrick decode-nor-dump -i docs/debug_logs/new_capture.log -o tmp/dmr_new_8mb.bin

# 3. Auto-discover Loader banners, EXTRHEAD sections, BGO sectors (0xF101/0xF2xx) & EEMUL markers
panadiga profiles discover-nor --input tmp/dmr_new_8mb.bin --model-id DMR-NEW --out-dir tmp/draft_profiles --json tmp/nor_report.json
# Offline Python triage equivalent:
python -m scripts.re.nor_intake_analyzer --input tmp/dmr_new_8mb.bin --model-id DMR-NEW --out-dir tmp/draft_profiles
```

> **Forensic Invariant:** Never mutate original dumps in-place. Record original SHA-256 hashes inside a vault session (`panadiga session init` + `ingest`) and archive verified logs under `docs/debug_logs/` (indexed in `docs/debug_logs/README.md`).

### Stage 3 — `DIGADIAG` Loader Versioning, `panamon` & `rebuild-from-dump`

Each DIGA hardware train requires a matching `Loader` container layout and `.FRM` TOC width:
- **`Loader ver0.61` (`06/03/06`, classic `SDDIAG` — e.g. `DMR-EH55`)**: loads **`256 KiB` raw decrypted `DIGADIAG.FRM`** (`PROG` payload stripped of `EXTRHEAD`) and pairs with **`48-byte` TOC `PANA__SD.FRM` (`sd48`)**. `PROG061` executes at **`0x50400000`** (`50400000` jump into `PANAMON1>`).
- **`Loader ver0.63+` (`bootbin` / later trains)**: loads **`128 KiB EXTRHEAD`-wrapped `DIGADIAG.FRM`** paired with **`64-byte` TOC `PANA__SD.FRM` (`sd64`)**.

Assemble verified SD recovery media directly from factory updates or raw NOR + HDD dumps:

```bash
# Harvest all official PANA_DVD.FRM containers and hydrate firmware_trains JSON metadata:
python -m scripts.re.frm_train_harvester --unpack-missing --apply

# Rebuild complete DIGADIAG.FRM + PANA__SD.FRM kit directly from NOR dump + Partition 1 MAIN:
panadiga unbrick rebuild-from-dump \
  --model DMR-EH55 \
  --nor-dump tmp/dmr_new_8mb.bin \
  --hdd-main tmp/partition_1_am34_os.bin \
  --target-gb 250 \
  --out-dir tmp/sd_kit_rebuild \
  --json tmp/rebuild_report.json
```

When swapping `MAIN` across sub-revisions causes `UPD NG` (`<deviceinfo>` / `DEV.BIN` checks in `panamon` / `MAIN`), inspect compatibility with `panadiga unbrick region-compat` or pass matching `--hdd-main` and `--frm` images. For clean-room SDRAM `panamon2` / diskless TBC bring-up experiments, consult [`firmware/panadiga-blackbox-fw/README.md`](../../firmware/panadiga-blackbox-fw/README.md).

### Stage 4 — Declarative Service Manual Hydration (`T1`)

Archive official PDFs under `manuals_and_schematics/` (updating `manuals_and_schematics/README.md`) and run:

```bash
python -m scripts.re.manual_spec_extractor --apply
```

The extractor populates `soc_ref`, `flash_ref`, `service_replacement_part`, and `p7501_header_present` while obeying `T0 → T1` non-destructive guards (entities at `T2+` are never overwritten).

### Stage 5 — Ghidra MN103S Disassembly, Plain-Text Bundles & `ghidra_ssot_maps.json`

1. Review [`MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md`](MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md) to eliminate compiler/disassembly traps (PC-relative `cur_insn`, `setlb/lene`, jump tables, and 64-bit division helpers).
2. Run [`tools/ghidra/ghidra_zero_undef_pipeline.py`](../../tools/ghidra/ghidra_zero_undef_pipeline.py) and [`tools/ghidra/ghidra_export_pseudocode_bundle.py`](../../tools/ghidra/ghidra_export_pseudocode_bundle.py) to export domain-grouped plain-text bundles (`boot_core`, `bgo_nfrom`, `sd_fat`, `main_hdd_tbc`, `davis_ide`) alongside SSOT map stubs for [`tools/ghidra/ghidra_ssot_maps.json`](../../tools/ghidra/ghidra_ssot_maps.json).
3. Regenerate native Rust-backed Ghidra helpers cleanly:

```bash
panadiga ghidra export-all --dir tools/ghidra
```

### Stage 6 — Product-Agnostic JSON SSOT Rule & `T0..T4` Capability Gate

**STRICT PROHIBITION ON PRODUCT-HARDCODED LOGIC:**
- Never branch on model strings (`match model { "DMR-EH55" => ... }`) or introduce product-named structs/constants (`EH55_TRAIN_ID`) inside `crates/panadiga-core`, `apps/desktop/src`, or `firmware/pico-p7501`.
- Encode every BGO sector geometry, NVRAM offset, `nor_layout`, compressed driver blob (`profiles/blobs/*.bin.gz`), `firmware_train`, and capacity/TBC patch rule declaratively in `profiles/**/*.json` under geometry/train IDs (e.g., `BGO_45SEC_8MB_0X84130000`, `BGO_35SEC_4MB_0X84200000`).
- After updating any SSOT entity, regenerate cross-language projections and promote evidence tiers:

```bash
panadiga profiles export-schema --out-dir profiles/schema
panadiga ssot export-all
panadiga profiles promote-evidence --entity-type model --id DMR-NEW --tier T3 --sha256 <SHA256> --fixture <PATH>
panadiga profiles completeness
panadiga profiles validate
```

### Stage 7 — Mandatory 9 Quality Gates & Pull Request

Run `python scripts/validate.py` (`npm run validate`) to confirm `0` `jscpd` clones, `100/100` `uxlint`, clean `mypy --strict`, all Rust/Vitest/Playwright/C11 suites passing, and 1:1 bilingual parity (`test_docs_parity.py`) before opening your PR per **Section 8 of [`CONTRIBUTING.md`](../../CONTRIBUTING.md)**.
