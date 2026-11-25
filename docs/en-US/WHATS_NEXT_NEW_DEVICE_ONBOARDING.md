> 🌐 **Idioma / Language:** **English (US)** | [Português (BR)](../pt-BR/WHATS_NEXT_ONBOARDING_NOVOS_APARELHOS.md)
> 🧭 **Portal:** [Documentation Index](../INDEX.md) | [DIGA Hardware Profiles Catalog](DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# What's Next: End-to-End Onboarding Pipeline for New DIGA Devices (Bench → JSON SSOT → Pull Request)

> **Purpose:** This guide standardizes the complete *end-to-end* workflow to integrate any additional **Panasonic DIGA** recorder into **Panadiga** using a **100% declarative JSON SSOT architecture** — keying hardware physically via the **`(model, region, board_rev, flash_jedec_id)`** tuple, preventing product-hardcoded logic, and enforcing SMPS/logic electrical safety prior to any flash or disk write.

---

## Overview of the 7 Onboarding Stages (`T0` → `T4`)

| Stage | Primary Objective | Canonical Tools / Scripts | Exit Gate |
| :--- | :--- | :--- | :--- |
| **1. SMPS & `P7501` Electrical Safety** | AC/USB galvanic isolation, capacitor discharge, verify `3.3V`/`1.8V`/`5V` logic rails, `GP3`/`NSW5V` `Hi-Z`, `XRESET` Open-Drain | Multimeter + Isolation Transformer / `ADuM3160` USB Isolator + `panadiga pico status` | Verified log in `docs/debug_logs/` with safe ground reference |
| **2. NOR Capture (`2..16 MiB`) & Intake** | Read-only IC5001 dump, detect 16-bit byte-swapping, floating-address-pin mirroring, and `EEMUL` generations | `panadiga unbrick nor-stub`, `decode-nor-dump`, `panadiga profiles discover-nor`, `scripts/re/nor_intake_analyzer.py` | Integral SHA-256 dump (no swap/mirror) + draft `nor_layout` |
| **3. `.FRM`, `panamon` & `rebuild-from-dump`** | Pin `Loader` / `trust_model`, unpack `DRV`/`PROG`/`MAIN`/Blu-ray `fmaX`, block unverified mixes, and compose `PANA__SD.FRM` | `scripts/re/fleet_decryptor.py`, `scripts/re/frm_train_harvester.py --frm`, `panadiga frm unpack-all`, `panadiga unbrick rebuild-from-dump` | Verified `EXTRHEAD`/`Adler32` checksums, `MAIN` build IDs & `trust_model` |
| **4. PDF Quarantine Proposals (`T1`)** | Extract PCB (`VEP`/`REP`), SoC, Flash, and EEPROM tokens with page citations (**never mutate** `profiles/*.json` directly) | `scripts/re/manual_spec_extractor.py --apply` → `profiles/staging/manual_proposals/` | Human-reviewed proposal files with cited pages before `T1` merge |
| **5. Ghidra Fleet & Reachable Coverage** | Resolve MN103S compiler traps / SPARC / ARM / AArch64 across decoded images while keeping data pools typed as data | `scripts/re/ghidra_fleet_orchestrator.py`, `tools/ghidra/ghidra_zero_undef_pipeline.py`, `panadiga ghidra export-all` | Plain-text bundles in `docs/pseudocode/` + `fleet_verify.json` |
| **6. Declarative SSOT, Allowlist & `< T3` Gate** | Model `(model, region, board_rev, flash_jedec_id)` tuple and positive BGO allowlist; block `< T3` profiles at runtime | `profiles/*.json`, `panadiga ssot export-all`, `panadiga profiles promote-evidence`, `panadiga profiles completeness` | `panadiga profiles validate` clean + mathematical BGO allowlist proof |
| **7. `Mn103Vm` Scope, Tests & Pull Request** | Validate ISA control flow in `Mn103Vm` (never mistaking it for silicon IRQ/WDT/CFI proof), Playwright E2E, IPC contracts & `0%` `jscpd` | `python scripts/validate.py` (`9/9` gates) | All 9 Quality Gates green + [`CONTRIBUTING.md`](../../CONTRIBUTING.md) Section 8 checklist |

---

## Step-by-Step Bench Execution Guide

### Stage 1 — Mandatory Electrical Checklist (SMPS / `P7501` / `IC5001` / `CH1`)

> [!CAUTION]
> **SMPS Floating-Chassis & Ground-Loop Shock Hazard:** Consumer Panasonic switch-mode power supplies couple chassis/secondary reference to mains through **Y-capacitors (~60–110 VAC open-circuit leakage to protective earth)**. Connecting a grounded PC USB-UART directly without galvanic isolation creates a ground loop capable of damaging your USB host, destroying the `AM34` SoC, or shocking the operator.

Before connecting probes, test leads, or USB cables:
1. **Galvanic Isolation & Capacitor Discharge:**
   - Power the DUT through an **AC/AC isolation transformer** *or* place a **USB galvanic isolator** (`ADuM3160` / `ADuM4160`) inline between the host computer and the Raspberry Pi Pico.
   - With AC mains disconnected, verify via DC multimeter that primary bulk (**~170–340 VDC**) and secondary filter capacitors have fully discharged prior to soldering or clipping TSOP-48 leads.
2. **Logic Rail Verification:**
   - Check the service schematic (`VEP79138`, `VEP79106A`, etc.) and measure actual rail voltage (`3.3V`, `1.8V`, or `5V`) on **`P7501` (`TO JIGU`)** and Flash VCC pins with a multimeter before connecting any Pico GPIO. Never assume `3.3V` on an unfamiliar chassis.
3. **Pico Probe (`RP2040` / `RP2350`) Pin Invariants:**
   - Confirm **`GP3` (`NSW5V` / Pin 5)** stays **strict `Hi-Z Input` without pulls**, **`GP5` (`XRESET` / Pin 2)** runs strictly **Open-Drain**, and the Pico is **NEVER powered from `P7501`** (power externally via isolated USB or `VSYS`).
4. **HDD Bus Inspection (`CH1`):**
   - On HDD boot failures (`UPD NG` / `MAIN` / `FINISH` or `0xE8000000` on `DAVIS CH1`), inspect `P1503 Pin 4 (DR5V = +5.1V)`, `P1503 Pin 1 (DR12V = +12.3V)`, and FFC ribbon contact polarity (`Type A` vs `Type B`) before diagnosing firmware errors.

### Stage 2 — Physical NOR Capture & Automated Integrity Triage

```bash
# 1. Build a read-only UART NOR dumper stub (or enter PANAMON1> via PANA__SD.FRM)
panadiga unbrick nor-stub --start 0x80000000 --len 0x800000 --out tmp/nor_stub.bin --wrap-digadiag

# 2. Store raw UART captures in docs/debug_logs/ and decode the immutable binary
panadiga unbrick decode-nor-dump -i docs/debug_logs/new_capture.log -o tmp/dmr_new_8mb.bin

# 3. Auto-discover layout and verify dump integrity (flags 16-bit byte-swap, pin mirroring, all-0xFF):
panadiga profiles discover-nor --input tmp/dmr_new_8mb.bin --model-id DMR-NEW --out-dir tmp/draft_profiles --json tmp/nor_report.json
# Offline Python triage with stale-EXTRHEAD filtering and multi-generation EEMUL tracking:
python -m scripts.re.nor_intake_analyzer --offline-only --input tmp/dmr_new_8mb.bin --model-id DMR-NEW --out-dir tmp/draft_profiles
```

> **Forensic & Hardware-Identity Invariant:** Never mutate original dumps in-place. Always read the physical CFI/autoselect JEDEC ID and record the full **`(model, region, board_rev, flash_jedec_id)`** hardware identity tuple rather than assuming a bare `--model` name maps 1:1 to physical flash sector geometry.

### Stage 3 — `DIGADIAG` Versioning, `trust_model` & `rebuild-from-dump`

Each firmware train declares `digadiag_spec.trust_model` (`checksum32_only`, `encrypted_toc`, `rsa_signed`, `unknown`):
- **`Loader ver0.61` (`06/03/06`, classic `SDDIAG` — `checksum32_only`)**: loads **`256 KiB` raw `DIGADIAG.FRM`** paired with **`48-byte` TOC `PANA__SD.FRM` (`sd48`)** on `<= 2 GiB` FAT16 SDSC. Trains verified at `>= T3` are enabled by default; `< T3` trains (such as `T2` `ver063`) require `--allow-unverified-profile`.
- **`Loader ver0.63+` (`bootbin` — `encrypted_toc`)**: loads **`128 KiB EXTRHEAD`-wrapped `DIGADIAG.FRM`** paired with **`64-byte` TOC `PANA__SD.FRM` (`sd64`)**.

```bash
# Rebuild hardened DIGADIAG.FRM + PANA__SD.FRM directly from NOR dump + Partition 1 MAIN:
panadiga unbrick rebuild-from-dump \
  --model DMR-EH55 \
  --nor-dump tmp/dmr_new_8mb.bin \
  --hdd-main tmp/partition_1_am34_os.bin \
  --target-gb 250 \
  --out-dir tmp/sd_kit_rebuild \
  --json tmp/rebuild_report.json
```

`rebuild-from-dump` fails closed on corrupt/byte-swapped/mirrored NOR dumps, IPL loader version mismatches, bad `DRV` checksums, or unrecognized `MAIN` builds (`--allow-unverified-mix` overrides cross-build compatibility checks only). For experimental SDRAM bring-up harnesses, see [`firmware/digaos/README.md`](../../firmware/digaos/README.md) (explicitly marked as an open-source hardware-interoperability prototype, **NOT LEGALLY CLEAN-ROOM**).

### Stage 4 — Quarantined PDF Manual Proposals (`T1`)

Archive service PDFs under `manuals_and_schematics/` and run:

```bash
python -m scripts.re.manual_spec_extractor --apply
```

To prevent OCR typos and multi-region alternate BOM lists from poisoning SSOT JSONs, `manual_spec_extractor.py` **NEVER writes directly to `profiles/boards/` or `profiles/eeprom_chips/`**. Instead it writes review-required proposal files to **`profiles/staging/manual_proposals/*.proposal.json`** with exact 1-based **`page_citations`** for human verification before merging into `profiles/`.

### Stage 5 — Ghidra MN103S Reachable-Code Coverage

1. Follow [`MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md`](MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md) to resolve Matsushita compiler traps (`cur_insn`, `setlb/lene`, jump tables, 64-bit math helpers).
2. Measure disassembly coverage strictly across **bounded executable code regions and reachable functions** from known reset/ISR vectors — **never chase blind `0 ??` (*Zero-Undef*) metrics** across literal pools, jump tables, video TBC coefficient tables, or `0xFF` flash padding, which turns data tables into hallucinated MMIO xrefs.
3. Batch-decrypt and deduplicate new `.FRM`/`.zip`/`.7z`/`.exe` packages via [`scripts/re/fleet_decryptor.py`](../../scripts/re/fleet_decryptor.py), triage train metadata with `python -m scripts.re.frm_train_harvester --frm <PATH> --unpack-missing`, orchestrate SSOT-aware Ghidra analysis via [`scripts/re/ghidra_fleet_orchestrator.py`](../../scripts/re/ghidra_fleet_orchestrator.py), export bundles with [`tools/ghidra/ghidra_export_pseudocode_bundle.py`](../../tools/ghidra/ghidra_export_pseudocode_bundle.py), and regenerate Rust helpers via `panadiga ghidra export-all --dir tools/ghidra` (see [`scripts/re/README.md`](../../scripts/re/README.md)).

### Stage 6 — Declarative SSOT Tuple, Positive BGO Allowlist & `< T3` Runtime Gate

- **No Product-Hardcoded Logic:** Express all BGO sector geometries, NVRAM layouts, compressed blobs, and patch specs declaratively under `(model, region, board_rev, flash_jedec_id)` and geometry IDs in `profiles/**/*.json`.
- **Positive Mathematical Allowlist:** Every BGO retarget/write proof verifies `[start..=end] ⊆ nor_layout.regions[kind == bgo]`, exact physical sector alignment on `flash_ref`, and empty intersection (`∩ = ∅`) with `ipl`, `extrhead`, `nor_fs`, and `eemul`.
- **Runtime `< T3` Gate:** Destructive generators block trains below `T3` unless `--allow-unverified-profile` is passed.

```bash
panadiga profiles export-schema --out-dir profiles/schema
panadiga ssot export-all
panadiga profiles promote-evidence --entity-type model --id DMR-NEW --tier T3 --sha256 <SHA256> --fixture <PATH>
panadiga profiles completeness
panadiga profiles validate
```

### Stage 7 — `Mn103Vm` Scope Honesty, Mandatory 9 Quality Gates & Pull Request

Remember that **`Mn103Vm` validates MN103S ISA control flow and synthetic bounds only** — it does **NOT** prove real-silicon bus wait-states, cache coherency, Spansion CFI toggle-bit timing, atomic `CLI` IRQ masking, or `IC7501` watchdog survival. Always confirm on isolated bench hardware before promoting to `T4`, and run `python scripts/validate.py` (`9/9` gates, `0` `jscpd` clones, `100/100` `uxlint`, `mypy --strict`, Rust/Vitest/Playwright/C11 suites, and bilingual parity `test_docs_parity.py`) per **Section 8 of [`CONTRIBUTING.md`](../../CONTRIBUTING.md)**.

---

## Future Research & Next Gates After `EV-20261124-004`

With `S11` verified on real silicon and `BGO` (`p1_sa=41`, 45/45 healthy) + `SDCD_CFG4` (`0x000040B8` `4BIT`) captured live in [`com10_round5_live_unbrick_sd_trace_20261124.log`](../evidence/com10_round5_live_unbrick_sd_trace_20261124.log), the immediate engineering gates are:

1. **Opt-In `NorAutoboot` `SA11..SA14` (`--nor-autoboot-capsule`, SD-Less `Nor-FROM` Boot):** `NOR_MAP` and `BGO_INSPECT NORFROM` proved `SA11 0x84040000 = ERASED_FF` (`norfrom_used=0`), explaining why `MAIN` still relies on the inserted SD card for pre-FVU SATA HDD autoboot launch. When explicitly authorized by the operator, program `SA11..SA14` (keeping `SA0..SA10` `E05` RO) via `--sata-hdd-autoboot --nor-autoboot-capsule` + `--confirm-upper-nor` after a full SHA-256-verified NOR backup.
2. **Live `DigaOS` SDRAM Bring-Up (`M1 -> M3`):** Package `firmware/digaos` (`digaos_ssot.h`, `digaos_shell_poll_once()`, `sd`/`bgo`/`ssot` commands, `DIGAOS>`) as a raw `262,144 B` `DIGADIAG.FRM` image at `0x5040_0000` and validate live interactive shell + `IC7501` `255s` keepalive without touching NOR.
3. **Full `Upper EEMUL` (`SA71..SA133` `"1284"`) Parser:** Decode the `7` active and `11` superseded records observed by `NOR_MAP` for fine-grained NV configuration auditing beyond `SA134`.
4. **`PANAEUSB.FRM` & NAND/eMMC `Schema v2.1` Expansion:** Extend USB installer container unpacking and non-NOR flash SSOT schemas across `T1 -> T4` hardware profiles.
