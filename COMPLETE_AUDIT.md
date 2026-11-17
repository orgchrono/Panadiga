# Consolidated Technical Audit — Panadiga Ecosystem (`DMR-EH55`)

> 🌐 **Language / Idioma:** **English** | [Português (Brasil) (`AUDITORIA_COMPLETA.md`)](./AUDITORIA_COMPLETA.md) · [Portal `docs/INDEX.md`](./docs/INDEX.md) · [Historical Audits (`docs/audits/`)](./docs/audits/README.md) · [Ghidra Pseudocode (`docs/pseudocode/`)](./docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) · [Bench Logs `COM10` (`docs/debug_logs/`)](./docs/debug_logs/README.md)

> **Live Document at Repository Root** · Pre-refactor historical audits from `2026-11-11` are preserved in [`docs/audits/`](./docs/audits/README.md).  
> **Scope:** Current modular architecture (`crates/`, `firmware/pico-p7501/`, `apps/desktop/`, `scripts/`), Pico `P7501` firmware upgrades, full Ghidra `??` (`Undefined`) root-cause diagnosis, pseudocode & bench log exposure, Python → Rust parity, and physical/cryptographic invariants.

---

## 1. Current Workspace Architecture Map

The repository operates as a unified Rust workspace ([`Cargo.toml`](./Cargo.toml), Edition `2024`) integrated with bare-metal C11/PIO firmware (`RP2040`/`RP2350`), a Tauri 2.0 Desktop/Android frontend (`React + TypeScript`), and Python automation for builds, validation, and IEEE/IEC CAD engineering drawings:

| Layer / Crate | Path | Primary Responsibility | Largest Current Source File |
| :--- | :--- | :--- | :--- |
| **Native Core** | [`crates/panadiga-core/`](./crates/panadiga-core/) | MEIHFS storage forensics, `EEMUL` NVRAM, Feistel/TOC/`EXTRHEAD` crypto, Ghidra generators, Pico SSOT, RE `mn10300`/`mn10300_traps`/`mn10300_emu`, `matshita_emu`, `log_decoder` | `atapi.rs` (`457 L`) |
| **Zero-Error RAG Engine** | [`crates/panadiga-rag/`](./crates/panadiga-rag/) | Recursive indexing of `docs/` (BM25 + vectors), `QueryGuard`, `FactCritic`, `HardwareSafetyValidator` (fail-closed IPL / overvoltage / `JTAG`) | `safety.rs` (`376 L`) |
| **MCP Server (`stdio`)** | [`crates/panadiga-mcp/`](./crates/panadiga-mcp/) | JSON-RPC 2.0 with **18 native tools** (`panadiga_*`) for AI agents and offline/bench auditing | `defs_re.rs` (`242 L`) |
| **Native CLI (`panadiga`)** | [`crates/panadiga-cli/`](./crates/panadiga-cli/) | Unified command surface (`nvram`, `hdd`, `frm`, `headers`, `scan`, `manuals`, `pico`, `ghidra`, `rag`, `memory-map`, `verify-artifact`, `gdb-bridge`, `mcp`, `unbrick`, `re`, `desktop`/`interactive`) | `cli_tests.rs` (`388 L`) |
| **Desktop / Tauri Backend** | [`apps/desktop/src-tauri/`](./apps/desktop/src-tauri/) | IPC commands (`crypto_manuals`, `engineering_docs`, `hardware`, `storage`, `frm_pack`, `unbrick`), serial/RSP transport & diagnostic parser | `hardware.rs` (`403 L`) |
| **React / TS Frontend** | [`apps/desktop/src/`](./apps/desktop/src/) | Strict MVVM views, i18n (`en-US`/`pt-BR`), interactive `docs/`/log/code viewer, `Unbrick Center`, `Pico Bench` | `mockUnbrickHandlers.ts` (`427 L`) / `usePicoTransport.ts` (`359 L` prod) |
| **Pico C11/PIO Firmware** | [`firmware/pico-p7501/`](./firmware/pico-p7501/) | `P7501` probing via `PIO0`/`PIO1`, GDB RSP, `DIAGWATCH`, `PANARSP`, `DRVPANA`, `SDSNIFF` + 4 host C11 test suites | `p7501_diag_watch_codec.c` (`316 L` prod) / `test_pana_logic.c` (`366 L` test) |
| **Scripts & Generators** | [`scripts/`](./scripts/) & [`tools/ghidra/`](./tools/ghidra/) | 9-Gate CI orchestrator (`validate.py`), multi-target builder (`build.py`), IEEE/IEC CAD drawings (`scripts/dev/`), Jython Ghidra (`tools/ghidra/`) | `tui.py` (`449 L`) / `ghidra_mn103_fix_decompilation.py` (`743 L`) |

### Consolidation of Historical Audit Findings (`2026-11-11`)

The original audits ([`docs/audits/`](./docs/audits/README.md)) flagged monoliths (`> 1000 L` in Rust/TS/C), `Cargo.toml.bak`, `dwire32a_jtag_research` drift, missing family ID `0xE48BFF56` in the UF2 merger, and false-positive failure classification on legitimate boot logs (`BGO driver init failed[1]`, `-91006`). **All structural items have been fixed and closed:**
- No production Rust, C11, or TypeScript module exceeds `457` lines;
- Zero code clones (`threshold: 0` in [`.jscpd.json`](./.jscpd.json));
- Diagnostic classifier and timeouts (`apps/desktop/src-tauri/src/pico/diagnostics/`) hardened against normal Bootrom warnings and long capture windows;
- All critical C11 codecs (`p7501_diag_watch_codec.c`, `p7501_drv_pana_codec.c`, `p7501_pana_rsp_parse.c`, `p7501_sds_codec.c`, etc.) are covered by native host unit tests compiled with `-Wall -Wextra -Werror`.

---

## 2. Pico `P7501` Firmware (C11 / PIO) & Rebuilt UF2 Binaries

Embedded firmware images under [`crates/panadiga-core/src/pico/firmware_bins/`](./crates/panadiga-core/src/pico/firmware_bins/) were rebuilt for real hardware (`RP2040` and `RP2350`) via `python scripts/build.py --pico --no-cache` using **ARM GNU Toolchain (`14.2 rel1`)**:

| UF2 Artifact | Size | Verified SHA-256 |
| :--- | ---: | :--- |
| `panasonic_p7501_uart_rp2040.uf2` | `268,288 B` | `0f216f39a5eb863eee4a93b7a957bea872f467cc0c050c7d897a33a73d50e0d2` |
| `panasonic_p7501_uart_rp2350.uf2` | `254,464 B` | `c0cd3f9bd90401ea2699cfeeb2e8c067608c0db489ec4d724cf550ac74dab99d` |
| `panasonic_p7501_uart_unified.uf2` | `522,752 B` | `2d568f7f4c1d3b12c83721eef4a5666a59666ead5fe8ee7359915e45962aa405` |

### Active Probe Capabilities (`p7501_cli_help.c`)
- **`DIAGWATCH` Monitor & Unlock:** Supports simple actions (`AUTO`, `BYPASS`, `FORMAT`, `DUMP`, `QUIT`, `NONE`), two-step Upper-NOR governance (`NORUNLOCK:CONFIRM_UPPER_NOR` / `NORLOCK` dispatching `'U'` → `'W'`/`'E'` on `PANAMON1>`), and **parameterized actions** (`READ[:a[,n]]`, `WRITE:<a,v>`, `JUMP:<a>`, `ERASE:<a_nvram>`) with **fail-closed (`E05`) guards**:
  - `SA0..SA3` (`0x80000000..=0x8003FFFF` and `0x84000000..=0x8403FFFF`) is **permanently read-only (`immutable RO`, never writable → `E05`)**; `SA4..SA133` (`0x80040000..=0x807EFFFF` and `0x84040000..=0x847EFFFF`) stays **write-protected by default (`NorWritePolicy::NvramOnly` → `E05`)** until explicit `CONFIRM_UPPER_NOR` opt-in (`p7501_flash_is_protected_ipl_range` / `p7501_flash_is_nvram_microsector_addr`).
- **`_dev_SERIAL` Debugger (`PANARSP`):** Supports `REGS` (12 MN103S registers decoded from 96 little-endian hex chars in `p7501_rsp_decode_mn103_regs`), `MEM`, `WMEM`, `DUMPNOR`, `CALL`, `NORUNLOCK:CONFIRM_UPPER_NOR` / `NORLOCK`, and JEDEC SRAM stub commands at `0x51800000` (`JEDEC_ERASE:<addr>`, `JEDEC_PROG:<addr>,<hex>` → `PANAJEDEC_OK` / `PANAJEDEC_TO`).
- **Optical Drive Factory Console (`DRVPANA`):** Executes `Matshita R6-MV4` front-end commands with optional automatic `PASS 0x4d415453 0x48495441` unlock (or `NOPASS`).
- **Timing & SD Analyzers (`PIO0`/`PIO1`):** Passive `SDSNIFF` on `GP10..GP15` (CRC16/lane), `JIGSWEEP`, `PIOLOGIC`, `BOOTTRACE`, `PWRTRACE`, `WAITTX`, `BAUDSCAN`, and `HEXCAP`.

---

## 3. Ghidra MN103S Disassembly, `??` Diagnosis & Exposed Artifacts

All evidence extracted strictly **read-only** from `C:\ghidra\projects\Panadiga_MN103S` is committed directly in the repository:

- **Full C Pseudocode & Disassembly:** [`docs/pseudocode/EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`](./docs/pseudocode/EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) (`488,538 B`, `13,677` lines, `SHA-256: 050e29aed745db3b...`) and [`docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md`](./docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) + raw dumps [`docs/pseudocode/raw/`](./docs/pseudocode/raw/).
- **Exhaustive `??` (`Undefined`) Audit Report:** [`docs/pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md`](./docs/pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md) proves that `??` gaps are NOT dump-read failures, but rather:
  1. **Untyped literal pools / tables & strings** (`41,508 B` in `/BOOT.BIN`, `417,487 B` in `/DRV_decomp.bin`, `8,168,656 B` in `/EH55_MAIN_3410PQ.bin`);
  2. **Valid code islands with `0xCF`/`0xF8 0xFE` prologues lacking direct xrefs** behind `<8sIII>` dispatch tables and ATAPI CDB switch tables (`93` islands / `22,910 B` in `BOOT.BIN`, `1,706` islands / `4,004,135 B` in `MAIN`);
  3. **Trap 4 (mid-instruction jump targets)** and **`AM33` opcodes missing from stock Sleigh** (`0xF9 0xEA <reg>` `MOVHU (Rm0+), Rn2` — `3,935` `0xF9` occurrences in `MAIN` — plus `0xFF 0xF0 0xFC`, `0xF9 0xFA`, and literal boundaries such as `0x4400493B` `dd 0e fe ff ff`).
- **Real Bench Logs (`COM10` / `P7501`):** [`docs/debug_logs/`](./docs/debug_logs/README.md) consolidates `DOSSIE_COMPLETO_EH55_UNBRICK_E_FIRMWARE.txt` and all **12 unique `.log` captures** (deduplicated from disk), decodable via `panadiga pico decode-log`.

---

## 4. Python → Rust Parity Audit (`panadiga re`, `decode-log` & Full Matrix)

The binary, cryptographic, forensic, and analytical core has been consolidated in Rust; only external build/validation orchestration, JVM Jython, and CAD drawing remain in Python. Full details are documented in the bilingual dossiers:
- 🇺🇸 [`docs/en-US/PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md`](./docs/en-US/PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md)
- 🇧🇷 [`docs/pt-BR/UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md`](./docs/pt-BR/UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md)

Recent Rust deliveries:
- **`panadiga re {audit, xrefs, drv-cli, drv-errors}`** ([`crates/panadiga-core/src/re/`](./crates/panadiga-core/src/re/) & [`crates/panadiga-cli/src/cli/handlers/re.rs`](./crates/panadiga-cli/src/cli/handlers/re.rs)): native CLI parity for the 7 Traps + `AM33` Sleigh gap audit, MN103S xrefs, `<8sIII>` dispatch table with `PASS` guard, and the `0x34` Shift-JIS error dictionary.
- **`panadiga pico decode-log`** ([`crates/panadiga-core/src/pico/log_decoder/`](./crates/panadiga-core/src/pico/log_decoder/)): offline decoding of UTF-16LE (BOM or BOM-less) / UTF-8 captures, boot modes, `SD = 0x...[LBA]`, `SDCD` errors (`-91006`, `-90002`, `-90001`, `0`), SCSI/ATAPI `SENSE[...]` packets (`0x83A7`), and automated diagnostics.

---

## 5. Physical Safety Invariants & Quality Verification

1. **Electrical Safety (`VEP79138` / `P7501`):** `GP3` (`NSW5V`) ALWAYS in **Hi-Z Input**; logic ceiling `<= 3.35 V`; `GP5` (`XRESET`) strictly open-drain (`0 V` / `Hi-Z`); `SW_UART` (`GP26`) held in `ADC0 Hi-Z` outside controlled `JIG:<ms>` windows.
2. **Two-Layer NOR Write Protection (`IC5001` Spansion `S29JL064H`):** Layer 1 IPL (`SA0..SA3`, `0x80000000..=0x8003FFFF` and `0x84000000..=0x8403FFFF`) is **permanently read-only (`immutable RO`, never writable → `E05`)** on the Pico probe; Layer 2 BGO/App Upper NOR (`SA4..SA133`, `0x80040000..=0x807EFFFF` and `0x84040000..=0x847EFFFF`) remains write-protected by default (`NvramOnly` → `E05`), unlockable only via explicit `CONFIRM_UPPER_NOR` opt-in / CoW `panadiga nvram patch-upper-nor` (and guarded by RAG `HardwareSafetyValidator`).
3. **Ghidra Database Integrity:** `C:\ghidra\projects\Panadiga_MN103S` is never modified by tests, CLI runs, or CI checks.
4. **9 Quality Gates Pipeline ([`scripts/validate.py`](./scripts/validate.py)):** Rust (`fmt`/`check`/`test`), TypeScript (`tsc --noEmit`), Vitest, `jscpd` (`0%` clones), `uxlint`, Playwright E2E, Ghidra Jython export (`panadiga ghidra export-all`), C11 host tests (`-Wall -Wextra -Werror` / `/W4 /WX`), and Python (`py_compile`, `mypy --strict`, `pytest`).

---

## 6. Verified Engineering Updates (Schematics, IR SSOT, Android APK & Two-Layer NOR)

* **Standby Physics (`IC7501` / `TBUS` / `TW7501`), `P7501` vs. `TW7501`, `IC3001` & Exact `<2s` `U99` VFD Paths:** [`STANDBY_IC7501_TBUS_TW7501_AND_BOOT_STATES_ARCHITECTURE.md`](./docs/en-US/STANDBY_IC7501_TBUS_TW7501_AND_BOOT_STATES_ARCHITECTURE.md) and [`GLOBAL_COMMUNITY_COMPENDIUM_DIGA_FIRMWARE_AND_HARDWARE.md`](./docs/en-US/GLOBAL_COMMUNITY_COMPENDIUM_DIGA_FIRMWARE_AND_HARDWARE.md) (§§3–3.5) separate `TW7501` (`VWJ0795=5`, `C7584` `0.047F` / `R7623` `180Ω` / `D7509` / `IC7505` / `IC7502` `XRESET`) from `P7501` Pin 1 (`SW_UART`), clarify `IC3001` (`C1AB00002379` A/V matrix, `EEPROM_W_C = NC(OPEN)`), correct the empirical VFD sequences (`PLEASE WAIT (~2s) -> U99` or direct `U99` on AC/auto-retry vs. `[POWER]` off + energized `TW7501` short + `[POWER]` hold $\rightarrow$ stable `0:00` Standby $\rightarrow$ `HELLO (~2s) -> U99` on wake before `FUN_44005cd0()`), and document Pico `PREJIG WAKE` capture.
* **Declarative DIGA IR Service Macros (SSOT):** [`PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md`](./docs/en-US/PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md) (§5) catalogs the **14 verified macros** in [`ir_macros_ssot.json`](./crates/panadiga-core/src/ir/ir_macros_ssot.json) (`[0][0]`..`[9][9]`), unit-tested exclusions in [`tests.rs`](./crates/panadiga-core/src/ir/tests.rs) (`[0][0]` = `Release Items` vs `[5][1]` = VFD; `[8][0]/[8][1]` = EU SCART vs `[1][1]/[1][2]`; `MULTI_REGION` `OBC 0x04 = REWIND`), and `Panasonic_DVD_TV.ir` mappings (`AUDIO [A] = 0x33` ≠ `[RED] = 0x141`).
* **Unified Android APK (`dist/releases/panadiga-android-arm64.apk`) & Desktop Isolation:** [`ANDROID_APK_AND_BLUETOOTH_ARCHITECTURE.md`](./docs/en-US/ANDROID_APK_AND_BLUETOOTH_ARCHITECTURE.md) documents the unified React 18 + Tailwind + Radix + shadcn UI ([`IrCommanderView.tsx`](./apps/desktop/src/views/IrCommanderView.tsx), Zero Jetpack Compose), `panasonic_ir_catalog.dat.gz` (`1,763` codesets / `14,744` models / `80,047` commands), minimal Kotlin I/O shims ([`AndroidIrBridge.kt`](./apps/desktop/src-tauri/gen/android/app/src/main/java/com/panasonic/diga/engineering/ir/AndroidIrBridge.kt), [`AndroidIrExecutor.kt`](./apps/desktop/src-tauri/gen/android/app/src/main/java/com/panasonic/diga/engineering/ir/AndroidIrExecutor.kt) with `sec_ir` + Magisk `su` cascade, [`AndroidPicoBridge.kt`](./apps/desktop/src-tauri/gen/android/app/src/main/java/com/panasonic/diga/engineering/AndroidPicoBridge.kt)), Exynos `SM-T805M`/`GT-N8000` timing in [`adapter.rs`](./crates/panadiga-core/src/ir/adapter.rs), Pico `IRTX` fallback, and 3-layer Desktop Zero-Leakage isolation (`android-ir`, `build_invoke_handler!`, `__PANADIGA_ANDROID_APK__`).
* **Two-Layer NOR Write Policy, JEDEC SRAM Stubs & `Mn103Vm`:** [`THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md`](./docs/en-US/THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md) (§§3.4–3.5) and [`PICO_P7501_FIRMWARE_GUIDE.md`](./docs/en-US/PICO_P7501_FIRMWARE_GUIDE.md) (§§6, 8–9) cover `NorWritePolicy` ([`policy.rs`](./crates/panadiga-core/src/pico/rsp_codec/policy.rs)), `CONFIRM_UPPER_NOR` opt-in unlock across GDB RSP / `PANAMON1` / `PANARSP` / `DIAGWATCH` (`0x51800000` stub [`p7501_pana_jedec.c`](./firmware/pico-p7501/uart/p7501_pana_jedec.c) → `PANAJEDEC_OK`/`PANAJEDEC_TO`), offline CoW `panadiga nvram patch-upper-nor`, Spansion FSM [`spansion_nor.rs`](./crates/panadiga-core/src/re/mn10300_emu/spansion_nor.rs), and real-binary integration in [`checks_real_nor.rs`](./crates/panadiga-core/src/re/emu_patch_suite/checks_real_nor.rs).
