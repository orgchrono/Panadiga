# Panadiga v0.1.0 — Hybrid Reverse Engineering, P7501 UART & U99 Recovery Suite

**Release Version:** `0.1.0`  
**Target Hardware:** Panasonic DMR-EH55 (DIGA Chassis / Main Board `VEP79138`)  
**Ecosystem Scope:** DMR-E, DMR-EH, DMR-ES, DMR-EX, DMR-BW (Gen 1 – Gen 5)  
**SoC:** Matsushita UniPhier `MN2DS0016` / `MN103S` — 32-bit RISC `AM34` / `MN10300`  
**License:** MIT

---

## 1. Technical Summary

Panadiga `0.1.0` is a cross-platform reverse engineering, forensic storage recovery, in-circuit probe, and unbricking suite for the Panasonic DIGA DVD/HDD recorder family.

This release unifies **three convergent engineering layers** validated directly against physical `VEP79138` hardware captures (`COM10`), factory binaries, and official Panasonic documentation:

| Layer | Methodology | Verified Artifacts |
| :--- | :--- | :--- |
| **Documentary** | `30` official Panasonic service manuals, schematics, and IC datasheets cross-referenced against PCB `VEP79138` | Canonical `P7501` (`B6B-ZR-3R4`) pinout, AM34 memory map, `MEIHDFS` geometry, `33` bilingual dossiers |
| **Mathematical** | Shannon entropy analysis (`H = 7.99958 bits/byte`), 64-bit ECB KPA codebook, `.FRM` TOC parsing, `XOR 0xFF` extent decoding | `PANA_DVD.FRM` (`48B` Feistel) & `PANA__SD.FRM` (`64B` plaintext) slots, `EEMUL` NVRAM sector cure |
| **Physical** | Reconstructed **P7501 Asynchronous UART (`115200 8N1`)**, discrete `SW_UART` / `XRESET` / `NSW5V` control, PIO timing samplers | Modular C11 + PIO probe firmware (`RP2040` / `RP2350` / `Unified`), `SC0` Live Bridge, `DIAGWATCH`, `PANARSP`, `DRVPANA`, GDB RSP server |

The platform operates across both stages of the hybrid DIGA architecture:

1. **Two-Stage Hybrid Boot Recovery:** Handles both onboard Spansion NOR Flash (**IPL + EEMUL NVRAM**) and internal HDD / SD / CD update stages (`BOOT.BIN`, `PROG061`, `MAIN3410PQ`, `DRV S211`, `DIGADIAG.FRM`, `PANA__SD.FRM`).
2. **Multi-Language Engineering Stack:** Native Rust workspace (5 crates), modular C11 + PIO probe firmware, Tauri 2.0 + React 18 TypeScript desktop & Android APK workbench, headless/MCP Ghidra Jython automation, and Python CI/build validation (`mypy --strict`).

---

## 2. Distributed Executables, Firmware & Payloads

| Platform | Asset(s) | Contents |
| :--- | :--- | :--- |
| **Windows (x86_64)** | `panadiga-windows-x86_64.zip` (+ `.sha256`) | `panadiga.exe`, `panadiga-desktop.exe`, RP2040/RP2350/Unified `.uf2`, `tools/ghidra/` & `.cmd` launchers |
| **Linux (x86_64)** | `panadiga-linux-x86_64.tar.gz`, `panadiga-linux-amd64.deb` (+ `.sha256`) | `panadiga`, `panadiga-desktop`, `tools/ghidra/` |
| **macOS (Apple Silicon ARM64)** | `panadiga-macos-aarch64.tar.gz` (+ `.sha256`) | `panadiga`, `panadiga-desktop`, `tools/ghidra/` |
| **Android (ARM64 APK)** | `panadiga-android-arm64.apk` (+ `.sha256`) | Native Tauri 2 mobile APK with USB-C OTG CDC-ACM & BLE Nordic UART Service (NUS) bridges |
| **Raspberry Pi Pico Probe** | `panasonic_p7501_uart_rp2040.uf2`, `panasonic_p7501_uart_rp2350.uf2`, `panasonic_p7501_uart_unified.uf2` (+ `.sha256`) | Dual-target C11 + PIO firmware (`RP2040`, `RP2350`, and RP2350-E10 compliant unified UF2) |
| **Verified Firmware & Recovery Pack** | [Release `firmware-pack-v0.1.0`](https://github.com/orgchrono/Panadiga/releases/tag/firmware-pack-v0.1.0) | Validated reference images (`DMR-EH55`, `DMR-EH58` `DVDRAM.7z`, `DMR-EH50`, `DMR-EX98V`, `DMR-EZ485V`, HDD `.bin.gz`/`.7z`, `sd_unbrick_kit`, `FIRMWARE_SHA256SUMS.txt`) |

### 2.1 Embedded RP2040 / RP2350 P7501 UART Probe Firmware

The compiled P7501 Async UART firmware is embedded directly in `panadiga` / `panadiga-desktop` and distributed standalone for drag-and-drop BOOTSEL flashing (`RPI-RP2` / `RP2350`):

- **Interfaces:** USB CDC (`115200`), PIO1 hardware UART samplers/TX (`p7501_uart.pio`, `p7501_uart_duplex.pio`, `p7501_logic_sampler.pio`, `p7501_sds_sampler.pio`), Android USB-C OTG, and optional **Bluetooth LE NUS** (`PANADIGA_BLE_ENABLED=1` for `pico_w` / `pico2_w`).
- **Diagnostics & Live Control:** `PING`, `STATUS`, `BANNER`, `WDT`, `LED`, `PINCHECK`, `BUSDIAG`, `PWRTRACE`, `BOOTTRACE`, `PIOLOGIC`, `SDSNIFF`, `JIGSWEEP`, `DUMP`, `BRIDGE` (`SC0` passthrough), `WAITTX`, `DIAGWATCH`, `PANARSP`, `DRVPANA`, and GDB RSP server (`127.0.0.1:3333`).

```bash
# List connected Pico serial probes or flash embedded UF2 automatically
panadiga pico list
panadiga pico flash
panadiga pico flash --port COM10
```

### 2.2 Surgical NVRAM U99 Sanitization & Copy-on-Write Guard

Derived directly from the Spansion Top-Boot flash sector invariant (`NVRAM_offset = flash_size - sector_size`):

| NOR Flash Size | Erase Block / Window | Device Example | NVRAM Offset |
| :---: | :---: | :--- | :--- |
| 2 MB | 64 KB | Spansion `S29AL016J` / `Am29LV160` | `0x001F0000` |
| 4 MB | 64 KB | Spansion `S29JL032H` / `Am29LV320` | `0x003F0000` |
| 8 MB | 64 KB | Spansion `S29JL064H` | `0x007F0000` (`SA134..SA141`, 8 × 8 KB) |
| 16 MB | 128 KB | Spansion `S29GL128P` / `S29GL128N` | `0x00FF0000` |

- **Sanitization:** Restores the writable EEPROM-emulated (`EEMUL`) window to virgin `0xFF`, backed up with MD5/SHA-256 verification.
- **IPL Protection (`E05`):** The IPL/Bootloader region (`0x80000000..0x807EFFFF` on 8 MB targets) is strictly **read-only** across the Rust core, C11 GDB RSP stub, ATAPI write injector, and Desktop/Mobile UI.

```bash
# Sanitize U99 NVRAM without altering the IPL bootloader (8 / 4 / 2 / 16 MB)
panadiga nvram clean flash_dump_8mb.bin flash_recovered.bin

# Guarded Copy-on-Write byte patch strictly inside the writable NVRAM sector
panadiga nvram patch --input flash_dump_8mb.bin --output flash_patched.bin --offset 0x0 --hex "FFFFFFFF"
```

### 2.3 MEIHDFS Storage Forensics, LBA48 Capacity Unlocker & Pre-MAIN Unbrick SD Kit

- **MEIHDFS Zero-Copy Forensics:** Validates `"SH"` magic superblock at LBA 0, bit-identical mirror at LBA 57,344 (28 MB), 10-block WAL journal (8 MB / LBA 16,384), `XOR 0xFF` inverted allocation descriptors (LBA 24..31), and firmware partitions at 32 MB (`LBA 65,536`) & 64 MB (`LBA 131,072`).
- **Dynamic Capacity Unlocker (`>200 GB` LBA48):** Removes the artificial 200 GB clamp in `MAIN` (`src/hdd_cmd.c`) on raw `MAIN`, `EXTRHEAD`, or Partition-1 HDD images.
- **Pre-MAIN SD Recovery Kit & Optical Emulator:** Builds/verifies `DIGADIAG.FRM` (128 KiB), `PANA__SD.FRM` (`PROG061` BGO recovery + `MAIN3410PQ` TBC Passthrough), `_dev_SERIAL` GDB RSP injection plans, and Matshita `R6-MV4` ATAPI `0x3B` / `0xED` emulation.

```bash
# Inspect MEIHDFS disk geometry and extract firmware partitions with SHA256SUMS
panadiga hdd inspect hdd_100.bin --density
panadiga hdd extract hdd_100.bin --outdir extracted_partitions

# Analyze PANA_DVD.FRM entropy, ECB codebooks, and cross-correlation with HDD
panadiga frm analyze firmware_and_images/PANA_DVD.FRM --hdd hdd_100.bin
panadiga frm dump-toc firmware_and_images/PANA_DVD.FRM

# Generate verified Pre-MAIN SD Recovery Kit
panadiga unbrick prepare-sd --outdir ./sd_card_eh55_kit
```

---

## 3. Verified P7501 Connector Pinout (`JST B6B-ZR-3R4` `TO JIGU`) & Cryptanalysis

### 3.1 P7501 Canonical Wiring on `VEP79138` (`115200 8N1`)

| Pin | Board Signal | Pico GPIO (Pico Pin) | Electrical Role & Non-Negotiable Constraint |
| :---: | :--- | :--- | :--- |
| **1** | `SW_UART` / `XFWE` | `GP26` / `ADC0` (Pin 31) | Sense (`~3.25V` Hi-Z / `~2.69V` PD); LOW forces serial boot (`< 10 mA`). **NEVER power Pico via Pin 1.** |
| **2** | `XRESET` | `GP5` (Pin 7) | **Strictly Open-Drain** (`Never push-pull +3.3V`). |
| **3** | `UARTM2P_T` (`Q7514`) | `GP4` (Pin 6) ← Pico TX (`115200 8N1`) | `3.3V LVTTL` (`10kΩ R7570` pull-up to `NSW5V`); conducts when `SW_UART = Hi-Z`. |
| **4** | `GND` | `GND` (Pin 38) | Mandatory `0.0 Ω` chassis ground reference verified prior to power-on. |
| **5** | `NSW5V` (`IC1506`) | `GP3` (Pin 5) | `~5.15V` upstream via `D7510` (`~3.29V` idle). **ALWAYS `Hi-Z Input` with NO pulls — NEVER drive push-pull.** |
| **6** | `UARTP2M_T` (`MAX TXD`) | `GP2` (Pin 4) → Pico RX (`115200 8N1`) | Active AM34 TX channel (`3.3V LVTTL`, `10kΩ R7572` pull-up to `NSW5V`). |

### 3.2 `PANA_DVD.FRM` Optical Container Cryptanalysis

- **Size:** `7,531,744` bytes | **Shannon Entropy:** `7.99958 bits/byte`
- **48-byte Feistel TOC Entries (`PANA_DVD.FRM`)** & **64-byte SD Plaintext TOC (`PANA__SD.FRM`)**: targets **Main AM34 OS**, **MN102 Optical Servo**, and **MN103S Front Panel**.
- **Verified 64-bit ECB Known-Plaintext Codebook:**
  - `DMR-EH55`: `FF×8 → 03 50 65 C8 7B B8 E0 90` (`3,812` hits) · `00×8 → E5 D1 F3 4A 51 8B 26 90` (`196` hits)
  - `DMR-EH50`: `FF×8 → 79 BB 2C 32 5A D4 FD 34` (`4,107` hits) · `00×8 → B3 A2 5A 0B 9D 86 4F 08` (`178` hits)

---

## 4. Ecosystem Architecture & Hard Invariants

| Layer | Implementation | Key Safeguards & Patterns |
| :--- | :--- | :--- |
| **Presentation (Desktop & Mobile)** | Tauri `2.0` · React `18.3.1` · TypeScript · Redux Toolkit | Strict MVVM enforced by `oxlint` (`no-restricted-imports`), immutable Redux slices + `createListenerMiddleware`, Vite `8` code-splitting (`< 150 kB`/chunk), 4-stage reactive `GuidedPipelineBar`, `DraggableBenchHud`, EN/PT-BR i18n, WCAG high-contrast/colorblind themes |
| **Core Engine, MCP & RAG** | Rust Workspace (5 crates) | `panadiga-core`, `panadiga-rag` (6-layer Zero-Error RAG + physical shields), `panadiga-mcp` (**18** JSON-RPC 2.0 stdio tools), `panadiga-cli`, `panadiga-desktop`; scoped `Mutex` closures & structured `PanadigaIpcError` |
| **Mobile Hardware Bridge** | Android APK (`aarch64-linux-android`) | Kotlin native USB-C OTG CDC-ACM & BLE NUS (`androidBridge.ts` / `bleService.ts`) + Rust HAL (`cargo-ndk`) |
| **Probe Firmware** | Modular C11 + PIO (`RP2040` / `RP2350`) | Non-overridable `4000ms` hardware `WDT`, isolated GDB RSP context, `NSW5V` Hi-Z invariant, Open-Drain `XRESET` |
| **RE & Automation** | Ghidra Jython + Python (`mypy --strict`) | AM34 SSOT memory map export, 7 MN103S decompiler trap fixers, 9-lane parallel quality validation |

### Non-Negotiable Safety Invariants
1. **Voltage Ceiling & `NSW5V` Protection:** Logic signals operate at `3.3V LVTTL`; Pin 5 (`NSW5V` / `GP3`) stays strictly **`Hi-Z Input` without pulls**.
2. **Zero Probe Back-Powering:** Never power the Raspberry Pi Pico from `P7501` Pin 1 (`< 10 mA` sense line); supply power exclusively via USB or `VSYS`.
3. **Read-Only IPL Lock (`E05`):** Any write targeting Bootloader/IPL (`0x80000000..0x807EFFFF`) is unconditionally blocked.
4. **Forensic Copy-on-Write:** Original `.bin` / `.FRM` images are never modified in place; all transformations emit new outputs with SHA-256 verification.

---

## 5. Quality Gates — 100% Verified

```text
✅ Gate 1   — Rust Workspace (`cargo fmt`, `clippy -D warnings`, `test`): 158/158 passed
✅ Gate 2   — Oxlint (`--deny-warnings`, 129 rules) + TypeScript:       0 warnings / 0 errors
✅ Gate 3   — Vitest Frontend MVVM / Redux / Hooks Suite:                89/89 passed (27 files)
✅ Gate 4   — jscpd Clone Detector (Threshold 0% across 603 files):      0 clones (0.00%)
✅ Gate 5   — ux-guard UI Completeness Audit:                            100/100 (0 issues)
✅ Gate 6   — Playwright E2E Chromium Headless Suite:                    15/15 passed (4 files)
✅ Gate 7   — Ghidra Native SSOT Jython Exporter:                        Idempotent export verified
✅ Gate 8   — C11 Pico Firmware Host Tests (`PICO_HOST_TEST=1`, -WX):    4/4 C11 test modules passed
✅ Gate 9   — Python (`py_compile`, `mypy --strict`, `pytest`):          23/23 passed
✅ Multi-CI — Windows x86_64 · Ubuntu x86_64 · macOS ARM64 · Android APK · Pico RP2040/RP2350 Dual UF2
```

---

## 6. Documentation & Legal

- **Master Bilingual Portal (33 EN/PT-BR Dossier Pairs):** [`docs/INDEX.md`](docs/INDEX.md)
- **Root Engineering Audits:** [`COMPLETE_AUDIT.md`](COMPLETE_AUDIT.md) · [`AUDITORIA_COMPLETA.md`](AUDITORIA_COMPLETA.md)
- **Service Manuals & Datasheets (`30` PDFs):** [`manuals_and_schematics/README.md`](manuals_and_schematics/README.md)
- **Ghidra Pseudocode & Live `COM10` Logs:** [`docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md`](docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) · [`docs/debug_logs/README.md`](docs/debug_logs/README.md)

All reverse engineering, cryptanalysis, and hardware documentation were conducted strictly for **interoperability, historical preservation, and repair of obsolete electronics**, in accordance with applicable statutory exceptions. Panasonic, Matsushita, DIGA, UniPhier, and Spansion are trademarks of their respective owners.

---

**License:** [MIT](LICENSE) · **Tree:** [`v0.1.0`](https://github.com/orgchrono/Panadiga/tree/v0.1.0)
