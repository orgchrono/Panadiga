# Panadiga v0.2.0 — Live Hardware-Proven EH55 Recovery, Relational SSOT, Silicon-Audited PicoFW (P7501) Probe & Zero-Trust Workbench

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Workspace](https://img.shields.io/badge/Rust-Edition%202024%20(11%20Crates)-orange.svg)](Cargo.toml)
[![PicoFW Probe](https://img.shields.io/badge/PicoFW-RP2040%20%2F%20RP2350%20%2F%20Pico%20W-green.svg)](firmware/picofw/)
[![CI & Security](https://img.shields.io/badge/Quality%20Gates-9%2F9%20Pass-brightgreen.svg)](scripts/validate.py)

| Overview | Verified Scope |
| :--- | :--- |
| **Release Version** | `v0.2.0` |
| **Hardware Catalog** | `89` Panasonic DIGA models · `12` SoCs · `5` core architectures (`MN10300/AM34`, `MN10300/AM33-2`, `ARM Cortex-A9`, `ARM64 Cortex-A53`, `SPARC v8 Domino`) |
| **Core Stack** | **11 Rust crates** (Edition 2024) · **C11 Bare-Metal** (`PicoFW` `RP2040`/`RP2350`/Pico W/2 W + `DigaOS` AM34) · **Tauri 2.12.7 + React 19.3.1 + Tailwind CSS 4.3.5 + Vite 8.3.6** · **Python 3.10+ (`mypy --strict`)** |
| **Evidence Ledger** | **`36` audited claims** (`EV-20261119-001` .. `EV-20261124-003`) with SHA-256 assertions in [`docs/evidence/ledger.json`](docs/evidence/ledger.json) & [`docs/STATUS.md`](docs/STATUS.md) |
| **MCP Integration** | Native Rust stdio JSON-RPC 2.0 server exposing **26** JSON-RPC 2.0 stdio tools |

---

## 1. Major Highlights & Breakthroughs Since `v0.1.0`

### 1.1 Live Hardware Proofs (`N4_HARDWARE` on Real `DMR-EH55` / `VEP79138`)
Real-hardware bench validation (`COM10`, `StarTech IDE2SAT2` + `Type B` FFC adapter, `SAMSUNG HM251JI` `488,397,168` LBA48) elevated core recovery and diagnostics milestones to **`N4_HARDWARE`**:
- **Full `8 MiB` `S1..S14` + `S13A/S13B` BGO Retarget Proven Live (`EV-20261123-030` & `EV-20261124-002`):**
  - Bootstrapped both `MAIN3410PQ` and `MAIN3060NP` cleanly from SATA HDD mirrors (`AVOUT_V_Mute: onoff=0`, `sys_rsv init success`, `setgui end. tsk=106`, **`0` BGO/`U99` errors**, `CLEAN_MAIN_OK` verdict).
- **NOR Autoboot & Hardened Recovery Pipelines:**
  - Added NOR autoboot test suites and hardening across [`nor_autoboot_tests.rs`](crates/panadiga-core/src/re/sata_bridge_stage2/tests/nor_autoboot_tests.rs), [`norfrom_install.rs`](crates/panadiga-core/src/re/eh55_bgo/norfrom_install.rs), [`sata_bridge_stage2`](crates/panadiga-core/src/re/sata_bridge_stage2/mod.rs), standalone TBC calibration ([`tbc_appliance`](crates/panadiga-core/src/re/tbc_appliance/mod.rs)), and [`unbrick_sd_kit`](crates/panadiga-core/src/re/unbrick_sd_kit/mod.rs).
- **Round 4 & Round 5 Live Pico + `DAVIS CH1` Proofs (`EV-20261124-003` & [`com10_round5_live_unbrick_sd_trace_20261124.log`](docs/evidence/com10_round5_live_unbrick_sd_trace_20261124.log)):**
  - Verified bare-metal self-tests (`SURG ASM`/`CRYPTO`/`DECOMP`/`HDD_PLAN`/`TRAP`/`PAYLOAD VERIFY ALL`), live `PANAMON1>` CFI & `fast_bgo_stream`, live `DAVIS CH1` `0xEC IDENTIFY DEVICE` capture (`512 B`), and clean `BRIDGE` exit confirmed via direct `ISOLATE` + `STATUS` (`VDD=3.25V`, `NSW5V=1`, `0` WDT recoveries).

### 1.2 Relational Hardware Profiles SSOT (`profiles/` & `panadiga-profiles`)
- **Single Source of Truth (`panadiga ssot export-all`):**
  - Centralizes **`89` model manifests**, **`12` SoCs**, **`5` cores**, **`31` mainboards**, **`18` parallel NOR chips**, **`6` I2C EEPROMs**, **`4` NOR layouts**, **`18` firmware trains**, **`3` media formats**, [`geometry_and_binary_traps.json`](profiles/traps/geometry_and_binary_traps.json), `profiles/forensic_atlas/`, and **`12` JSON Schema Draft 2020-12** definitions (`profiles/schema/`).
  - Enforces a deterministic **`T0`–`T4` Capability Gate** (`panadiga profiles validate|show|authorize|export-schema`) respecting hardware WP/OTP ceilings and `256 KiB` IPL RO floors.
  - Projects one canonical contract across **Rust**, **C11** (`picofw_ssot.h`, `digaos_ssot.h`), **TypeScript** (`apps/desktop/src/models/generated/{ssot,profiles,digaos}.ts`), **Python** (`scripts/profiles_types.py`), and **Ghidra Jython** (`tools/ghidra/`).

### 1.3 Embedded Firmware Ecosystem (`PicoFW` & `DigaOS`)
- **Silicon-Audited `PicoFW` Probe for `P7501` (`RP2040`, `RP2350` & `Pico W / 2 W`):**
  - **Electrical & Pin Safety:** `picofw_hw_init()` clears reset pull-downs on `GP3 (NSW5V)` before `gpio_init(GP3)`; unconditional falling-edge IRQ isolates `GP2/GP3/GP4/GP5/GP26/GP6/GP7` on target power drop; boot-time QSPI CRC32 self-check (`SAFE_HIZ` fail-closed); `8 µs` contention detection (`ECONTENTION`) and anti-backpowering guards (`EPWR`).
  - **Unified Security & Protocol Hardening:** Finite-state probe machine (`picofw_probe_fsm.{c,h}`), TX/bus guards (`picofw_bus_tx_guard.c`, `picofw_bus_policy.c`), golden wire contract (`tests/golden/picofw_wire_contract.json`), RSP/Watch abuse guards (`picofw_pana_rsp_guard.c`), unconditional Upper NOR auto-relock + `F14` wireless fail-closed gate, `BOOTTRACE` & `PiE-OS` X-Ray codecs (`picofw_boottrace_codec.c`, `picofw_pieos_xray.c`), SRAM-resident Core 1 sampler, bounded `512 B` GDB sector cache, zero external probe gadgets (`P0_FW_RO = 256 KiB`), atomic ping-pong A/B `P2_VAULT`, and RP2350 `P6_NOR_MIRROR`.
- **Bare-Metal DIGA SoC Operating System (`DigaOS`):**
  - Unified bare-metal SoC OS in [`firmware/digaos/`](firmware/digaos/README.md) (`digaos_ssot.h` sharing `resolve_eh55_verified_nor_context()`, `nor_bgo_ro.c`, `sdcd_sd.c`, `tbus_ic7501.c`, `uart_sc0.c`, `digaos_main.c`, `test_digaos_host.c`, absorbing TBC/SECAM baseline & `SW_GUARDS` from the retired bare-metal prototype).
- **Visual SSOT Studio & Hot Bare-Metal Silicon Explorer (`SsotStudioView`, `SiliconExplorerView`):**
  - Added real Tauri 2 IPC-backed **Visual SSOT Schema & DigaOS Customizer Studio** (`ssot_studio.rs`) and **Hot Bare-Metal Silicon Explorer** (`silicon_explorer.rs`, `ndb>` parser/session, `pico silicon-replay`, `pico silicon-snapshot`) with `OLD -> NEW` hot-diff telemetry and zero production demo mode.

### 1.4 Express Rescue Workbench, Forensic Vault & Modernized Stack (Desktop + Android)
- **Express Rescue & Strict Vault (`ExpressView`):** Guided missions for **U99 Quick Rescue**, **SD Recovery Media**, **NOR Dump & CoW Patch**, and **Strict Vault Pipeline**, backed by immutable `raw/` vs `frm/` segregation, `ops.jsonl` append-only journal (`panadiga-vault`), WORM ingest, real SHA-256 staging/promotion, `Mn103Vm` pre-write gate, cleanroom `panadiga sd create` / `dvd create`, and safe multi-OS `panadiga hdd flash-composed`.
- **Hardened Host Boundary & BLE/Android:** Bounded `2 MiB` serial ring framing, double-read SHA-256 + `fsync` backups before erase, anchored BLE NUS chunking (`<= 20 B` MTU), **DIGA IR Commander** isolated strictly to Android APK builds, and Tauri 2 capability/CSP/ephemeral-nonce/shell-free launch hardening.
- **Upgraded Toolchain & Dependencies (`15/15` dependency bumps consolidated on `main`):**
  - **UI & Bundler:** React `19.3.1`, Tailwind CSS `4.3.5` (`@tailwindcss/postcss`), PostCSS `8.5.33`, Framer Motion `14.14.0`, Vite `8.3.6`.
  - **Rust & Tauri:** Rust Edition 2024 (`1.89+` workspace MSRV), Tauri `2.12.7` / `tauri-build` `2.7.6`, RustCrypto `sha2` `0.11` & `md-5` `0.11`, `regex` `1.13.2`.
  - **CI & Supply Chain:** Node 24-compatible GitHub Actions (`upload-artifact` v7, `download-artifact` v8, `setup-node` v7, `setup-gradle` v6, `action-gh-release` v3), `cargo-deny`, `cargo-audit` (`0` vulnerabilities), and pinned SHA-256 `gitleaks` (`0` leaks with line-scoped [`.gitleaks.toml`](.gitleaks.toml)).

### 1.5 Bilingual Engineering Archive, Datasheets & Audits
- Curated **`44` bilingual dossiers & `28` curated** `COM10` bench captures indexed in [`docs/INDEX.md`](docs/INDEX.md) and [`docs/debug_logs/README.md`](docs/debug_logs/README.md), plus **32 service manuals & datasheets** cataloged in [`manuals_and_schematics/README.md`](manuals_and_schematics/README.md) (`Microchip 24LC64`, `ROHM BR24L16`), **26** JSON-RPC 2.0 stdio tools in `panadiga-mcp` (with root-jail, HITL policy & injection-safe RAG citations in `panadiga-rag`), and the `StarTech IDE2SAT2` chassis clearance reference archive.

---

## 2. Release Assets & Checksum Verification

| Platform | Asset(s) | Contents |
| :--- | :--- | :--- |
| **Windows (x86_64)** | `panadiga-windows-x86_64.zip` (+ `.sha256`) | `panadiga.exe`, `panadiga-desktop.exe`, `picofw_unified.uf2`, `tools/ghidra/` & `.cmd` launchers |
| **Linux (x86_64)** | `panadiga-linux-x86_64.tar.gz`, `panadiga-linux-amd64.deb` (+ `.sha256`) | `panadiga`, `panadiga-desktop`, `picofw_unified.uf2`, `tools/ghidra/`, desktop integration & icons |
| **macOS (Apple Silicon ARM64)** | `panadiga-macos-aarch64.tar.gz` (+ `.sha256`) | `panadiga`, `panadiga-desktop`, `picofw_unified.uf2`, `tools/ghidra/` |
| **Android (ARM64 APK)** | `panadiga-android-arm64.apk` (+ `.sha256`) | Signed Tauri 2 APK with USB-C OTG CDC-ACM, BLE Nordic UART Service, and **DIGA IR Commander** |
| **Raspberry Pi Pico Probe** | `picofw_unified.uf2` (+ `.sha256`) | Dual-family unified C11 + PIO UF2 supporting both `RP2040` and `RP2350` (auto-sliced by `INFO_UF2.TXT`; verify digest via `picofw_unified.uf2.sha256`) |
| **Master Manifest** | `SHA256SUMS.txt` | Authoritative POSIX SHA-256 manifest covering every released binary asset |
| **Companion Firmware Pack** | [Release `firmware-pack-v0.1.0`](https://github.com/orgchrono/Panadiga/releases/tag/firmware-pack-v0.1.0) | Verified factory/repaired NOR, HDD, EEPROM & plaintext AM34 images + `FIRMWARE_SHA256SUMS.txt` |

```bash
# Linux / macOS — verify all downloaded artifacts against SHA256SUMS.txt
sha256sum -c SHA256SUMS.txt --ignore-missing
```

```powershell
# Windows PowerShell — verify individual artifact digest
Get-FileHash .\panadiga-windows-x86_64.zip -Algorithm SHA256
Get-Content .\panadiga-windows-x86_64.zip.sha256
```

---

## 3. Non-Negotiable Physical & Forensic Safety Invariants

1. **Voltage Ceiling & `NSW5V` Protection (`P7501`):** Logic signals operate at `3.3V LVTTL`; Pin 5 (`NSW5V` / `GP3`) stays strictly **`Hi-Z Input` without pulls** (cleared before `gpio_init`); Pin 2 (`XRESET` / `GP5`) is **strictly Open-Drain**; `picofw_nsw5v_falling_irq()` isolates `GP2/GP3/GP4/GP5/GP26/GP6/GP7` unconditionally on target power drop (`STANDBY_GATE_STOP`).
2. **Zero Probe Back-Powering:** Never power the Raspberry Pi Pico from `P7501` Pin 1 (`< 10 mA` sense line); supply power exclusively via USB or `VSYS`.
3. **3-Zone NOR Protection (`E05`) + `0xC000100C` + `F14`:** Bootloader/IPL (`SA0..SA10`), `DMR-ES35V` `MAIN_XIP`, Full-RO profiles, CS0 alias mirrors, and lethal MMIO `0xC000100C..0xC000100F` are blocked; Upper NOR writes require explicit unlock (`CONFIRM_UPPER_NOR`) with auto-relock on exit; wireless origins (`BLE_NUS`/`WIFI_TCP`) are fail-closed (`F14`).
4. **Forensic Copy-on-Write:** Original `.bin` / `.FRM` images are never modified in place; live erases require double-read SHA-256 + `fsync` + disk readback verification.

---

## 4. Quality Gates & Cross-Platform CI (`9/9` Local + `7/7` GitHub Actions)

```text
✅ Gate 1   — Rust Workspace (11 crates: cargo fmt, clippy --all-targets -D warnings, tests, proptest/insta)
✅ Gate 2   — Oxlint (0 warnings/errors across 390 files, 224 rules) + TypeScript Strict (tsc --noEmit)
✅ Gate 3   — Vitest Frontend MVVM / Redux / Express / BLE / Parser Suite (133 tests across 40 files)
✅ Gate 4   — jscpd Clone Detector (0 clones / 0.00% duplication across 1,245 files)
✅ Gate 5   — ux-guard UI/UX Completeness & State Integrity Audit (100/100, 0 issues)
✅ Gate 6   — Playwright E2E Chromium Headless Suite (33 tests across 9 specs)
✅ Gate 7   — Multi-Language SSOT + Ghidra Jython Exporter (Idempotent export verification)
✅ Gate 8   — C11 PicoFW & DigaOS Host Tests + ARM RP2040 / RP2350 Dual-Target Builds
✅ Gate 9   — Python (py_compile, mypy --strict scripts, pytest scripts/tests — 75 tests incl. SSOT/FP & docs parity)
✅ Multi-CI — Windows x86_64 · Ubuntu x86_64 (.tar.gz + .deb) · macOS ARM64 · Android ARM64 APK · Pico UF2 · Supply-Chain (cargo-deny / cargo-audit / gitleaks)
```

---

## 5. Documentation & Legal

- **Master Bilingual Portal (44 EN/PT-BR Dossier Pairs):** [`docs/INDEX.md`](docs/INDEX.md) · [`docs/STATUS.md`](docs/STATUS.md)
- **DIGA Lineup Hardware Profiles Catalog & Audit:** [`docs/en-US/DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md`](docs/en-US/DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md) ([PT-BR](docs/pt-BR/CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md))
- **Anti-Trap BGO Geometry & SSOT Forensic Atlas Guide:** [`docs/en-US/ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md`](docs/en-US/ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md) ([PT-BR](docs/pt-BR/GUIA_ANTI_ARMADILHAS_GEOMETRIA_BGO_E_ATLAS_FORENSE_SSOT.md))
- **Root Engineering Audits:** [`COMPLETE_AUDIT.md`](COMPLETE_AUDIT.md) · [`AUDITORIA_COMPLETA.md`](AUDITORIA_COMPLETA.md)
- **Service Manuals & Datasheets (`32` PDFs):** [`manuals_and_schematics/README.md`](manuals_and_schematics/README.md)

All reverse engineering, cryptanalysis, and hardware documentation were conducted strictly for **interoperability, historical preservation, and repair of obsolete electronics**, in accordance with applicable statutory exceptions. Panasonic, Matsushita, DIGA, UniPhier, and Spansion are trademarks of their respective owners.

---

**License:** [MIT](LICENSE) · **Tree:** [`v0.2.0`](https://github.com/orgchrono/Panadiga/tree/v0.2.0) · **Full Changelog:** [`v0.1.0...v0.2.0`](https://github.com/orgchrono/Panadiga/compare/v0.1.0...v0.2.0)
