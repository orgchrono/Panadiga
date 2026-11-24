> 🌐 **Language / Idioma:** **English (US)** · [Versão em Português (`pt-BR`)](../pt-BR/GUIA_ANTI_ARMADILHAS_GEOMETRIA_BGO_E_ATLAS_FORENSE_SSOT.md) · [Documentation Index](../INDEX.md)

# Anti-Trap NOR/BGO Geometry (`4 MiB` vs `8 MiB`), Fail-Closed SSOT & `100%` Ghidra Forensic Atlas Guide

> `[PROVEN ON REAL HARDWARE]` · `[EVD: Panadiga_MN103S_ZeroUndef + profiles/ + validate_bgo.rs]`

This dossier documents the engineering lessons behind physical **NOR Flash / BGO FTL** geometry traps (`4 MiB` vs `8 MiB`), describes the new fail-closed Rust and Python validators added to the declarative `profiles/` SSOT, and specifies the auto-updating **Forensic Atlas** generated from `100%`-disassembled Ghidra MN103S firmware captures.

---

## 1. Critical Trap Post-Mortem: `4 MiB` vs `8 MiB` BGO Geometry

When retargeting factory updater payloads (`PROG061` / `MAIN` / `/BOOT.BIN`) onto real **DMR-EH55 hardware (`IC5001 Spansion S29JL064H`, `8 MiB`)**, assuming a uniform BGO geometry causes off-by-window bugs across Flash sectors and NVRAM:

| Parameter | Real `Loader ver0.61` (`EH55`) | Stock `PROG061` (`0x50436E30`) | `/BOOT.BIN ver0.63` (`0x44036520`) |
| :--- | :--- | :--- | :--- |
| **Target Flash** | `S29JL064H` (`8 MiB`, `SA0..SA141`) | `4 MiB` Top-Boot donor | `S29JL032H` (`4 MiB`, Top-Boot `SA0..SA70`) |
| **BGO Base (`word[12]`)** | **`0x84130000`** (`SA26`) | `0x84200000` | `0x84200000` |
| **BGO Ceiling (`word[13]`)** | **`0x84400000`** (exclusive) | `0x843F0000` + Bank 2 `8×8K` | `0x843F0000` |
| **Physical Sectors (`word[31]`)** | **`45` (`0x2D`) × `64 KiB`** (`2,880 KiB`) | `31 × 64 KiB` + `8 × 8 KiB` (`35` logical) | `29` (`0x1D`) |
| **`EEMUL` NVRAM** | `SA134..SA141` (`0x847F0000..0x847FFFFF`) | Upper top-boot | Top-boot boundary (`0x3F0000`) |

### Additional Cataloged Traps ([`profiles/traps/geometry_and_binary_traps.json`](../../profiles/traps/geometry_and_binary_traps.json))
1. **`TRAP_TOP_BOOT_NON_UNIFORM_SECTORS`:** `S29JL032H` (`63×64K + 8×8K`) vs `S29JL064H` (`8×8K + 126×64K + 8×8K`) require exact sector tables from `derive_sector_table(flash)`.
2. **`TRAP_ES35V_MAIN_XIP_VS_EH55_BGO`:** On `DMR-ES35V`, `0x130000..0x40FFFF` holds executable **`MAIN_XIP`** code, NOT BGO (`0x700000..0x7EFFFF`).
3. **`TRAP_EZ485V_4MB_SPARC_VENDOR_NVRAM`:** `DMR-EZ485V` (`4 MiB` MX29LV320 bottom-boot SPARC/VxWorks) places `vendor_nvram CLsiPMABS` at `0x360000..0x3FFFFF`.
4. **`TRAP_DIGADIAG_256K_RAW_VS_128K_EXTRHEAD` & `TRAP_PANA_SD_TOC_SD48_VS_SD64`:** `Loader ver0.61` requires raw `DIGADIAG.FRM` (`262,144 B`) + `PANA__SD.FRM sd48` (`0x1800`), whereas `ver0.63` uses `EXTRHEAD` (`131,072 B`) + `sd64` (`0x2000`).
5. **`TRAP_EH58_P7501_5V8_HDMI`:** On `VEP79183` boards (`EH58/EH59/EH68`), `P7501` carries `+5.8V_AV` HDMI — NEVER attach a `3.3V` UART probe.
6. **`TRAP_MN103S_SLEIGH_10_CLASSES` & `TRAP_PROG061_BGO_DESTRUCTIVE_CALLERS`:** Audit `SUB fb 9c`, `NOT f2 3x`, `BTST mn_todo()`, and block unattended calls to BGO flag mutators (`0x50409DF2` / `0x50409E65`).

---

## 2. Fail-Closed SSOT Validators & Reference JSON Maps (`profiles/` + `panadiga-profiles`)

1. **`4 MiB` Top-Boot Reference Map ([`loader_ver063_4mb_top_bgo35.json`](../../profiles/nor_layouts/loader_ver063_4mb_top_bgo35.json)):** Pairs `loader_ver063_bootbin` with `spansion_s29jl032h_4mb_top` (`4 MiB`), keeping IPL RO at `0x0000_0000..=0x0003_FFFF`, `Bgo` Bank 1 at `0x0020_0000..=0x003E_FFFF`, and Top-Boot at `0x003F_0000..=0x003F_FFFF`.
2. **Declarative Trap Catalog ([`geometry_and_binary_traps.json`](../../profiles/traps/geometry_and_binary_traps.json)):** Encodes each verified trap with `wrong_assumption`, `verified_ground_truth`, `affected_entities`, `automated_guard`, and checked-in `evidence_refs`.
3. **Compile-Time & CLI Validators ([`validate_bgo.rs`](../../crates/panadiga-profiles/src/validate_bgo.rs)):**
   - Cross-checks `_4MB_`/`_8MB_`/`_16MB_` and base `0x...` in `bgo_retarget_spec` / `digadiag_spec.bgo_geometry_target` against `flash.total_bytes`, `bgo_table_words[12..13, 15, 31]`, and each bound `nor_layout` BGO span.
   - Fails compile-time build (`crates/panadiga-core/build.rs`) and `panadiga profiles validate` on any `4 MiB` vs `8 MiB` pairing error, mismatched sector count (`35` vs `45`), or wrong `DIGADIAG.FRM` byte size.
4. **Python Pre-Flight Verifier (`scripts/verify_geometry_traps.py`):** Audits all traps/layouts and performs read-only binary NOR size/fingerprint checks (`--nor-dump <DUMP.bin> --layout <ID>`).

---

## 3. Auto-Updating `100%` Ghidra Forensic Atlas & New Ecosystem Skills (`29 Skills`)

- **Deterministic Forensic Atlas (`scripts/generate_firmware_atlas.py`):** Aggregates `53/53` zero-undef coverage from [`fleet_verify.json`](../audits/zero_undef/fleet_verify.json), curated functions/callers/callees/hex constants/bitwise patterns from `docs/pseudocode/raw/doc_*.txt`, `profiles/symbols/` dictionaries, and SSOT headers/footers (`EXTRHEAD`/`EXTRFOOT`/`MainListHeader`/`MEIHFS`/`FAT16B`/`ISO9660`/`EEMUL`) into [`firmware_forensic_atlas.json`](../../profiles/forensic_atlas/firmware_forensic_atlas.json) and [`FORENSIC_ATLAS_INDEX.md`](../../.agents/skills/panadiga-firmware-forensic-atlas/references/FORENSIC_ATLAS_INDEX.md).
- **Dedicated Agent Skills (`.agents/skills/`):**
  - [`panadiga-device-onboarding`](../../.agents/skills/panadiga-device-onboarding/SKILL.md) — Authoring, `T0..T4` onboarding, and fail-closed validation of `profiles/` JSON SSOT.
  - [`panadiga-anti-trap-geometry-verification`](../../.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md) — Mandatory `4 MiB` vs `8 MiB` & cross-board geometry guards.
  - [`panadiga-firmware-forensic-atlas`](../../.agents/skills/panadiga-firmware-forensic-atlas/SKILL.md) — Auto-updating Ghidra `100%`-disassembled forensic atlas (`--write` / `--check`).
