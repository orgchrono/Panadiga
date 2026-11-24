# Panadiga v0.2.0 — Relational DIGA Hardware Profiles SSOT, Express Rescue Workbench, Forensic Vault Pipeline & Silicon-Audited P7501 Probe Firmware

**Release Version:** `0.2.0`  
**Target Hardware:** Panasonic DIGA Recorder Family (`89` cataloged models — flagship `DMR-EH55` / `VEP79138`, `DMR-EH50/58/59/65/69`, `DMR-ES10/15/35V`, `DMR-EX/EZ`, `DMR-BW/BWT/XW/XS`)  
**SoCs Covered:** `12` SoCs across `5` core architectures (`MN10300/AM34`, `MN10300/AM33-2`, `ARM Cortex-A9`, `ARM64 Cortex-A53`, `SPARC v8 Domino`)  
**License:** MIT

---

## 1. What's New in `v0.2.0` (Highlights Since `v0.1.0`)

Panadiga `0.2.0` marks a major architectural milestone across all **11 Rust crates**, the **bare-metal C11 Raspberry Pi Pico (`RP2040` / `RP2350` / `Pico W / 2 W`) probe firmware**, the **Tauri 2 Desktop & Android Mobile Workbench**, and the **bilingual hardware archive**:

### 1.1 Relational Hardware Profiles SSOT (`profiles/` & `panadiga-profiles`)
- **New `panadiga-profiles` Crate & Multi-Language SSOT Generator (`panadiga ssot export-all`):**
  - Centralizes **`89` DIGA model manifests**, **`12` SoCs**, **`5` SoC cores**, **`31` mainboards**, **`18` parallel NOR flash chips**, **`6` I2C EEPROM chips**, **`4` NOR partition layouts** (incl. `loader_ver063_4mb_top_bgo35.json`), **`18` firmware trains**, **`3` media formats**, cataloged **`geometry_and_binary_traps.json`**, **`profiles/forensic_atlas/`**, and **`12` JSON Schema Draft 2020-12 definitions** (`profiles/schema/`).
  - Enforces a deterministic **`T0`–`T4` Capability Authorization Gate** (`panadiga profiles validate|show|authorize|export-schema`) respecting hardware WP/OTP limits, `256 KiB` IPL RO floors, and explicit offline CoW lab overrides.
- **Relational Hardware Profiles SSOT (`panadiga ssot export-all`):**
  - Projects one Single Source of Truth across **Rust**, **C11** (`p7501_ssot.{h,c}` incl. CS0/SDRAM/DIGADIAG/Lethal MMIO constants), **TypeScript** (`apps/desktop/src/models/generated/{ssot,profiles}.ts` incl. `SSOT_AM34_BUS_CONSTANTS`), **Python** (`scripts/profiles_types.py` + dynamic script discovery), and **Ghidra Jython** (`tools/ghidra/`).

### 1.2 Express Rescue Mode & Strict Forensic Vault Pipeline (Desktop & Android UI)
- **Guided Express Workbench (`ExpressView`):** Adds operator-friendly mission cards — **U99 Quick Rescue**, **SD Recovery Media**, **NOR Dump & CoW Patch**, and **Strict Vault Pipeline** — backed by `DigaModelSelect`, `RecorderDisplayGuide`, `PicoConnectionSafetySheet`, `StrictWorkflowStepper`, `VaultMediaAndGatePanel`, and EN/PT-BR localization (`locales/{en-US,pt-BR}/express.json`).
- **Segregated Forensic Vault Sessions & Auto-Fingerprinting (`panadiga session`, `crates/panadiga-core/src/{vault,ingest}/`):**
  - Enforces strict separation between immutable `raw/` chip/bus dumps and `frm/` factory containers with SHA-256 chain of custody and automatic binary fingerprinting.
- **Mn103Vm Pre-Write Verification Gate (`re/prewrite_gate/`, `re/verify_loop/`, `re/sig_locator/`):**
  - Executes recovery binaries inside the `MN103Vm` emulator before writing to SD, DVD, HDD, or NOR — verifying that the Bootloader IPL (`SA0..SA10`) remains untouched and lethal MMIO (`0xC000100C`) is blocked.
- **Cleanroom Recovery Media Builders & Composed HDD Flash:**
  - **`panadiga sd create` (`fat16_builder`, `cleanroom_digadiag`):** Generates verified `32 MiB` FAT16B (`0x06`) SD recovery images (`DIGADIAG.FRM` / patched `PROG061`) with Mn103Vm pre-write verification.
  - **`panadiga dvd create` (`iso9660_builder`, `dvd_creator`):** Generates verified cleanroom `ISO 9660` (`PANADIGA_REC`) optical recovery images with `1x`/minimum DAO burn policy.
  - **`panadiga hdd flash-composed` (`composed_flash.rs`, `discovery_unix/lsblk.rs`, `raw_device.rs`):** Safe multi-OS raw disk discovery and composed MEIHDFS + firmware partition flashing with system-drive/NVMe lockouts.
  - **Expanded MCP Server (`panadiga-mcp`):** **26** JSON-RPC 2.0 stdio tools including `panadiga_profiles_authorize`, `panadiga_symbol_lookup`, BGO/DAVIS probes, region calibration, TBC lab, and vault workflows.
  - **BGO Partition-1 (`0xF101`) Compaction (`re/eh55_bgo`):** Compacts split/migrating Partition-1 sectors into a single authoritative sector with zero open `+0x14` migration links (`FUN_4402014f` rollback prevention) and updates the verified repaired `eh55_nor_8mb_ic5001_repaired.bin` baseline.

### 1.3 Silicon-Audited Bare-Metal C11 Pico `P7501` Probe Firmware (`GO` on `RP2040`, `RP2350` & `Pico W / 2 W`)
Audited adversarially across every silicon, electrical, DMA, QSPI, and protocol path — achieving **unanimous `GO`** across all targets (`PICO_STACK_SIZE = 0x1000`, `-Wframe-larger-than=512` with `0` warnings, `0` soft-float `_ftoa`/`_etoa` symbols):

- **Electrical & Silicon Hardening (`p7501_hw.c`, `p7501_pico_main.c`):**
  - `p7501_hw_init()` executes **first** in `main()` and clears RP2040/RP2350 reset-default pull-downs on `GP3 (NSW5V)` **before** `gpio_init(GP3)`.
  - Unconditional `p7501_nsw5v_falling_irq()` immediately reclaims `GPIO_FUNC_SIO` + `GPIO_IN` without pulls across `GP2`, `GP3`, `GP4`, `GP5`, `GP26`, **and auxiliary I2C/IR pins `GP6`/`GP7`** on target power loss.
  - **Boot Integrity & Contention Guards:** Boot-time `p7501_qspi_firmware_crc32()` self-check locks the bus into `SAFE_HIZ` + `LED_MODE_BUS_FAULT` on corruption; `GPIO_SLEW_RATE_SLOW` on `GP4`/`GP5`/`GP26`; `8 µs` pin-contention detector (`ECONTENTION`) before holding LOW drive on `GP4`/`GP26`.
  - Anti-backpowering (`EPWR`) guards in `I2C_SCAN`, `EEPROM`, `PINCHECK` (pure open-drain `GP5`), `BUSDIAG` (restores `GP4` `4 mA`), and `IDENTIFY`; passive `NSW5V` sensing + duplex TX in `M0KEEPALIVE` eliminating `GP26` pull-down glitches.
  - `stdio_set_translate_crlf(&stdio_usb, false)` + `putchar_raw()` eliminating `0x0A → 0x0D 0x0A` corruption in binary `PANADMP1` NOR dumps.
- **Unified Bus Security Policy (`p7501_bus_policy.c` + Rust `policy.rs`) & Adversarial Guards:**
  - Single gate `p7501_bus_check_access()` blocks AM34 lethal MMIO `0xC000100C..0xC000100F` (R/W/E), CS0 alias mirrors (`0x80000000..0x87FFFFFF`), permanent `SA0..SA10` IPL RO, `DMR-ES35V` `MAIN_XIP` RO, Full-RO profiles (`DMR-EZ485V`/`DMR-ES10`) regardless of flash-size overrides, full sector spans on erase, and `F14` wireless fail-closed auth.
  - Hardened `p7501_pana_rsp_guard.c` & `p7501_diag_watch_codec.c`: blocks >8-digit hex overflow attacks (`p7501_scan_hex_u32`), leading whitespace / `;` whitespace bypasses, chained `$M` commands, lowercase `w`/`e`/`f`/`j`, bare `W`/`E`/`J` without explicit hex address in `RAW:`/`HEX:`, `DIAGWATCH FORMAT` without unlock + `F14`, and enforces unconditional Upper NOR auto-relock + `F14` op consumption across `PANARSP`, `DIAGWATCH`, `GDB` (including partial writes), `JOB`, `EEPROM`, and `BLE`.
- **Dual-Core SRAM, RP2350 `.bss` Reduction (`-57.5 KiB`) & Zero-Gadget On-Probe Capabilities (`67` Studio Commands):**
  - Core 1 edge-sampler loop and `p7501_spsc_push()` are **100% SRAM-resident (`__not_in_flash_func`)**, removing the 50 ms multicore lockout stall during `BOOTSEL` polling and making QSPI lockout fail-closed (50 ms timeout).
  - Bounded `GDB_SECTOR_CACHE_SIZE = 512 B` (SSOT) reduces `pico2` `.bss` to **`72,900 B`**, eliminates GDB `populate_cache()` WDT timeouts, moves `tx_resp_buf[2048]` off the Core 0 stack, caps `$m` read length (`E02`), and replaces silent `0xFFFFFFFF` reads with checked block reads (`E01`).
  - **Nibble-LUT CRC32, `VAULT VERIFY <1..3>`, `IR VERIFY <12hex>` & `DUMPNOR ff_blocks`:** Keep all UI, audio cues, and IR Commander control on smartphone (`USB-C OTG` / `BLE`) and Desktop with **zero external probe gadgets**, fitting comfortably inside `P0_FW_RO` (`256 KiB` — `RP2040` `160,208 B` / `RP2350` `157,552 B`).
  - Added native on-probe `PANAMON1>` PIO1 target-bus driver (`p7501_target_bus_panamon.c`), live `$m` `0→1` pre-read validation for `BGO_BITCLEAR`, `min_post_hdr_bytes` + incremental `DUMPNOR_RESUME` + streaming blank-block counter (`ff_blocks`), `PWROSC` `8192`-sample arena + clean `ADC_FCS` teardown, `EEPROM WRITE` page-boundary splitting, `SELFTEST LOOPBACK`, chronological `BLACKBOX DUMP`, and **100% `@P7501_RES` error-code parity across all CLI handlers**.
- **Atomic Ping-Pong A/B CoW `P2_VAULT` & RP2350 `P6_NOR_MIRROR`:**
  - True Copy-on-Write A/B banking across Bank A (`0x100000`) and Bank B (`0x140000`) with sibling-slot preservation and XIP CRC32 verification before manifest commit, plus `P6_NOR_MIRROR` (`2 MiB` extra QSPI partition on `4 MiB` RP2350).

### 1.4 Hardened Pico ↔ Host Boundary (Desktop Tauri Rust, React/TS BLE & Android Kotlin)
- **Serial Transport (`transport.rs`, `constants.rs`):** Bounded `2 MiB` raw byte buffer, strict newline + `@P7501_RES` / prompt framing (never truncating structured `@P7501_`/`@PROG`/`@EVT` captures on intermediate lines or 350 ms idle gaps), and deterministic `CDC0/CLI` vs `CDC1/Mirror` numeric COM ranking.
- **Forensic Pre-Erase Guarantees (`backup.rs`, `executor.rs`, `nvram_clean.rs`):** `ClearBuffer::Input` before every sequential command, full double-read SHA-256 verification of NVRAM + `SA134..SA141` micro-sectors, exact size & dead-bus pattern checks (`SRAM` `16,384 B`, `IPL` `4,096 B`), exclusive `fs::create_dir` backup folders (`first_contact_<ms>_<sha8>`), and `File::sync_all()` (`fsync`) + disk readback SHA-256 verification across all 3 backup files.
- **BLE NUS & Android OTG/BLE (`bleService.ts`, `AndroidPicoBridge.kt`):** Eliminated fake timeout `${command}: OK` fabrication, added `<= 20 B` BLE 4.2 ATT_MTU GATT chunking with per-chunk write status checks, streaming UTF-8 decoding, and strict anchored single-token regex framing (`^[A-Z0-9_]+:\s+(?:OK|...)`) immune to target boot lines like `"DDR CHECK: OK"`.
- **SSOT Diagnostic Parser (`findingsRules.ts`, `lineParsers.ts`, `index.ts`):** Consumes `SSOT_DIAG_SIGNATURE_RULES` directly, precompiles dump regexes, and matches Rust legacy fallback parity.

### 1.5 Bilingual Hardware Archive, Datasheets & Bench Evidence
- Curated **`43` bilingual dossiers & `28` curated** `COM10` bench captures, adding the **DIGA Lineup Hardware Profiles Catalog & Audit** ([EN](docs/en-US/DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md) · [PT-BR](docs/pt-BR/CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md)), **What's Next: New Device Onboarding Guide** ([EN](docs/en-US/WHATS_NEXT_NEW_DEVICE_ONBOARDING.md) · [PT-BR](docs/pt-BR/WHATS_NEXT_ONBOARDING_NOVOS_APARELHOS.md)), **Anti-Trap BGO Geometry & SSOT Forensic Atlas Guide** ([EN](docs/en-US/ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md) · [PT-BR](docs/pt-BR/GUIA_ANTI_ARMADILHAS_GEOMETRIA_BGO_E_ATLAS_FORENSE_SSOT.md)), **Hardware Guide (`StarTech IDE2SAT2` & `Type B` FFC)** ([EN](docs/en-US/HARDWARE_GUIDE_STARTECH_IDE2SAT2_AND_TYPE_B_FFC_CABLE.md) · [PT-BR](docs/pt-BR/GUIA_HARDWARE_STARTECH_IDE2SAT2_E_CABO_FFC_TYPE_B.md)), and **Validation Gates `0..4` & `8 MiB` BGO Retarget Research** ([EN](docs/en-US/RESEARCH_VALIDATION_GATES_SATA_STAGE2_AND_8MB_BGO_RETARGET.md) · [PT-BR](docs/pt-BR/PESQUISA_PORTOES_VALIDACAO_SATA_STAGE2_E_RETARGET_BGO_8MB.md)), official `Microchip 24LC64` & `ROHM BR24L16` EEPROM datasheets (`manuals_and_schematics/datasheets/` + `SHA256SUMS.txt`), `DMR-XW / BD` series catalog (`firmware_and_images/dmr-xw_bd_series/`), and `IDE2SAT2` chassis clearance reference photography (`docs/assets/images/re_dmr_eh55/`).

---

## 2. Distributed Executables, Firmware & Payloads

| Platform | Asset(s) | Contents |
| :--- | :--- | :--- |
| **Windows (x86_64)** | `panadiga-windows-x86_64.zip` (+ `.sha256`) | `panadiga.exe`, `panadiga-desktop.exe`, Unified `panasonic_p7501_uart_unified.uf2`, `tools/ghidra/` & `.cmd` launchers |
| **Linux (x86_64)** | `panadiga-linux-x86_64.tar.gz`, `panadiga-linux-amd64.deb` (+ `.sha256`) | `panadiga`, `panadiga-desktop`, `panasonic_p7501_uart_unified.uf2`, `tools/ghidra/`, Freedesktop desktop launcher & Hicolor icons |
| **macOS (Apple Silicon ARM64)** | `panadiga-macos-aarch64.tar.gz` (+ `.sha256`) | `panadiga`, `panadiga-desktop`, `panasonic_p7501_uart_unified.uf2`, `tools/ghidra/` |
| **Android (ARM64 APK)** | `panadiga-android-arm64.apk` (+ `.sha256`) | Native Tauri 2 mobile APK with USB-C OTG CDC-ACM, BLE Nordic UART Service (NUS, `<= 20 B` GATT chunking), and **DIGA IR Commander** |
| **Raspberry Pi Pico Probe** | `panasonic_p7501_uart_unified.uf2` (`SHA-256: 1af8da3e15ca65abc78e1a4fb4d3281e99d34cc5c968186584243de717b14077`) | Single size-optimized (`-Os --gc-sections`, `636,416 B` / `1,243` blocks) C11 + PIO container serving both `RP2040` (`320,512 B` UF2 / `160,208 B` ELF) and `RP2350` (`315,904 B` UF2 / `157,552 B` ELF), sliced per target via `extract_uf2_family_slice` |
| **Checksums** | `SHA256SUMS.txt` | Master unified SHA-256 manifest covering every released binary artifact |
| **Verified Firmware & Recovery Pack** | [Release `firmware-pack-v0.1.0`](https://github.com/orgchrono/Panadiga/releases/tag/firmware-pack-v0.1.0) | Companion archive of verified factory/repaired NOR, HDD, EEPROM & plaintext AM34 images + `FIRMWARE_SHA256SUMS.txt` |

---

## 3. Non-Negotiable Physical & Forensic Safety Invariants

1. **Voltage Ceiling & `NSW5V` Protection (`P7501`):** Logic signals operate at `3.3V LVTTL`; Pin 5 (`NSW5V` / `GP3`) stays strictly **`Hi-Z Input` without pulls** (cleared before `gpio_init`); Pin 2 (`XRESET` / `GP5`) is **strictly Open-Drain**; `p7501_nsw5v_falling_irq()` isolates `GP2/GP3/GP4/GP5/GP26/GP6/GP7` unconditionally on power drop.
2. **Zero Probe Back-Powering:** Never power the Raspberry Pi Pico from `P7501` Pin 1 (`< 10 mA` sense line); supply power exclusively via USB or `VSYS`.
3. **3-Zone NOR Protection (`E05`) + `0xC000100C` + `F14`:** Bootloader/IPL (`SA0..SA10`), `DMR-ES35V` `MAIN_XIP`, Full-RO profiles, CS0 alias mirrors, and lethal MMIO `0xC000100C..0xC000100F` are blocked; Upper NOR requires explicit unlock (`CONFIRM_UPPER_NOR`) with auto-relock on exit; wireless origins (`BLE_NUS`/`WIFI_TCP`) are fail-closed (`F14`).
4. **Forensic Copy-on-Write:** Original `.bin` / `.FRM` images are never modified in place; live backups require double-read SHA-256 + `fsync` + disk readback verification before any erase.

---

## 4. Quality Gates — Automated Enforcement (`scripts/validate.py` — 9/9 PASS)

```text
✅ Gate 1   — Rust Workspace (11 crates: cargo fmt, clippy --all-targets -D warnings, workspace tests)
✅ Gate 2   — Oxlint (0 warnings/errors across 386 files, 224 rules) + TypeScript Strict (tsc --noEmit)
✅ Gate 3   — Vitest Frontend MVVM / Redux / Express / BLE / Parser Suite (128 tests across 39 files)
✅ Gate 4   — jscpd Clone Detector (0 clones / 0.00% duplication across 1,030+ files)
✅ Gate 5   — ux-guard UI/UX Completeness & State Integrity Audit (100/100, 0 issues across 386 files)
✅ Gate 6   — Playwright E2E Chromium Headless Suite (27 tests across 9 specs, incl. Express Rescue Mode)
✅ Gate 7   — Multi-Language SSOT + Ghidra Jython Exporter (Idempotent export verification)
✅ Gate 8   — C11 Pico Firmware Host Tests (48 sources, /std:c11 /W4 /WX, 38 test suites/4 modules) + ARM RP2040 & RP2350 Builds
✅ Gate 9   — Python (dynamic py_compile, mypy --strict scripts, pytest scripts/tests — 58 tests)
✅ Multi-CI — Windows x86_64 · Ubuntu x86_64 (.tar.gz + .deb) · macOS ARM64 · Android ARM64 APK · Pico RP2040/RP2350/Unified UF2
```

---

## 5. Documentation & Legal

- **Master Bilingual Portal (43 EN/PT-BR Dossier Pairs):** [`docs/INDEX.md`](docs/INDEX.md)
- **DIGA Lineup Hardware Profiles Catalog & Audit:** [`docs/en-US/DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md`](docs/en-US/DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md) ([PT-BR](docs/pt-BR/CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md))
- **Anti-Trap BGO Geometry & SSOT Forensic Atlas Guide:** [`docs/en-US/ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md`](docs/en-US/ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md) ([PT-BR](docs/pt-BR/GUIA_ANTI_ARMADILHAS_GEOMETRIA_BGO_E_ATLAS_FORENSE_SSOT.md))
- **Root Engineering Audits:** [`COMPLETE_AUDIT.md`](COMPLETE_AUDIT.md) · [`AUDITORIA_COMPLETA.md`](AUDITORIA_COMPLETA.md)
- **Service Manuals & Datasheets (`32+` PDFs):** [`manuals_and_schematics/README.md`](manuals_and_schematics/README.md)

All reverse engineering, cryptanalysis, and hardware documentation were conducted strictly for **interoperability, historical preservation, and repair of obsolete electronics**, in accordance with applicable statutory exceptions. Panasonic, Matsushita, DIGA, UniPhier, and Spansion are trademarks of their respective owners.

---

**License:** [MIT](LICENSE) · **Tree:** [`v0.2.0`](https://github.com/orgchrono/Panadiga/tree/v0.2.0) · **Full Changelog:** [`v0.1.0...v0.2.0`](https://github.com/orgchrono/Panadiga/compare/v0.1.0...v0.2.0)
