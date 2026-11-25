<p align="center">
  <strong>English</strong> | <a href="README.pt-BR.md">Português (Brasil)</a> | <a href="CONTRIBUTING.md">Contributing & Architecture Rules</a>
</p>

# Panadiga — Hardware Diagnostic, Firmware & Reverse Engineering Suite for Panasonic DIGA Recorders

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![SoC](<https://img.shields.io/badge/SoC-Matsushita%20UniPhier%20%28AM34%2FMN103S%29-blue.svg>)](#1-executive-project-overview)
[![Flash](<https://img.shields.io/badge/NOR%20Flash-Spansion%20S29JL%20%2F%20Am29LV%20%282MB--16MB%29-green.svg>)](#61-ssot-hardware-bus-regions--verified-firmware-virtual-addresses)
[![Storage](<https://img.shields.io/badge/Storage-MEIHDFS%20%28IDE%2FPATA%2FSATA%29-yellow.svg>)](#7-forensic-cryptanalysis--storage-architecture-deep-dive)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

> **Master Technical Systems & Hardware Engineering Reference**
> **Classification:** Forensic Engineering / Unified Recovery & Tooling Platform
> **Primary Target Device:** Digital DVD/HDD Video Recorder **Panasonic DMR-EH55** (DIGA Chassis / Main Board `VEP79138`)
> **Ecosystem Compatibility:** Panasonic DIGA Lineup (**DMR-E, DMR-EH, DMR-ES, DMR-EX, DMR-BW Series**)
> **Core Architecture:** Matsushita `AM34` / `MN103S` (32-bit RISC `MN10300` / UniPhier)
> **Flash Memory Range:** 2 MB (16 Mbit) to 16 MB (128 Mbit) TSOP-48 / TSOP-56 Parallel NOR Flash
> **Storage Subsystem:** PATA/IDE and SATA Hard Disk Drives with proprietary **MEIHDFS** partition tables
> **Hardware Interface:** Asynchronous **P7501 UART (`115200 8N1`) + Discrete `SW_UART` / `XRESET` / `NSW5V` Sense** (RP2040/RP2350 C11 + PIO + Autonomous `PANARSP` Engine)

---

## 1. Executive Project Overview

The **Panasonic DIGA** series represents the pinnacle of Japanese consumer electronics engineering from the 2001–2008 era. Designed by **Matsushita Electric Industrial Co., Ltd.** (now Panasonic Corporation), these devices integrate high-performance audio/video processing built upon the proprietary **UniPhier** System-on-Chip (SoC) family, executing the **Matsushita AM34** (MN10300 architecture) instruction set.

While the primary laboratory focus of this project is the widely deployed **DMR-EH55**, the engineering discoveries, algorithms, and tools within Panadiga span the **entire Panasonic DIGA recorder ecosystem**:

1. **Two-Stage Hybrid Bootloader Architecture:** The onboard Spansion NOR Flash (`IC5001` on DMR-EH55 `VEP79138`, 2 MB to 16 MB depending on generation) stores only the Initial Program Loader (*IPL*). The primary operating system, OSD graphics acceleration drivers, optical drive servo microcodes, and codecs reside entirely in reserved sectors within the internal hard drive and optical update containers.
2. **Proprietary MEIHDFS Storage System:** The internal hard drive does not use standard PC MBR partition tables or FAT/ext filesystems. The storage layout is structured around **MEIHDFS** (*Matsushita Electric Industrial HDD File System*), featuring an ASCII `"SH"` magic superblock at LBA 0, a redundant security mirror at LBA 57,344 (28 MB), a 10-block transactional journal, and encrypted firmware partitions.
3. **Cryptography & Intellectual Property Protection:** The optical distribution container (`PANA_DVD.FRM`) and the executable HDD partitions are protected with proprietary 64-bit symmetric block ciphers operating in ECB mode, tied to CPRM (*Content Protection for Recordable Media*) and DivX DRM engines.
4. **The Critical `U99` Error Vulnerability:** Channel presets, HDD volume mount states, and operating parameters are saved into the final 64 KB sector of the Flash memory, which emulates EEPROM wear-leveling (`EEMUL`). Cell degradation, unexpected power loss during disk write operations, or corrupted status words trigger startup validation panics. Consequently, the Matsushita kernel halts permanently with the fatal code **`U99`** on the front VFD vacuum fluorescent display, ignoring remote commands and deadlocking the optical tray mechanism.

This repository establishes the **definitive forensic reverse engineering, physical unbricking, and laboratory diagnosis ecosystem** for the Panasonic DIGA lineup. It provides the high-performance Rust toolchain **`panadiga`**, a zero-latency desktop and mobile Android (`.apk`) bench GUI, C11/PIO firmware for **Raspberry Pi Pico (RP2040) and Pico 2 (RP2350)** with **Bluetooth Low Energy (BLE)** and USB-C OTG support, **32 cataloged factory service manuals and silicon datasheets**, **44 paired bilingual (`en-US` / `pt-BR`) engineering dossiers**, full **Ghidra MN103S disassembly/pseudocode** (`docs/pseudocode/`), and **28 deduplicated live `COM10` / bench logs** (`docs/debug_logs/`).

---

## 2. End-to-End System Architecture

```mermaid
%%{init: {'theme':'base','themeCSS':'.edgeLabel span, .edgeLabel div, .edgeLabel p { white-space: nowrap !important; }','themeVariables':{'fontFamily':'ui-monospace, SFMono-Regular, Menlo, monospace','fontSize':'18px','background':'#161b22','primaryColor':'#21262d','primaryTextColor':'#e6edf3','primaryBorderColor':'#484f58','lineColor':'#8b949e','secondaryColor':'#21262d','tertiaryColor':'#21262d','edgeLabelBackground':'#161b22','titleColor':'#e6edf3'},'flowchart':{'htmlLabels':true,'useMaxWidth':false,'wrappingWidth':4000,'nodeSpacing':70,'rankSpacing':110,'curve':'basis','padding':30}}}%%
flowchart TD
    USER(["Forensic Engineer · Operator"])

    subgraph SOFTWARE_SUITE["1 · Panadiga Software"]
        direction TB
        GUI["Desktop · Android GUI · Tauri 2.0 · React 19 · ExpressView · IR Commander"]
        CLI["panadiga Core · Rust Edition 2024 (11 crates + profiles)"]
        GUI <--> |"IPC"| CLI

        P7501_HAL["P7501 UART HAL 115200 8N1 - PIO"]
        GDB_SRV["GDB RSP :3333"]
        MEIHDFS["MEIHDFS Parser Storage Forensics"]
        CRYPTO["Cryptanalysis ECB - KPA"]
        NVRAM["NVRAM Sanitizer 64 / 128 KB"]
        RAG["RAG Zero-Error 6 shields"]

        CLI <--> P7501_HAL
        CLI <--> GDB_SRV
        CLI <--> MEIHDFS
        CLI <--> CRYPTO
        CLI <--> NVRAM
        CLI <--> RAG
    end

    subgraph LAB_BENCH["2 · Zero-Gadget Probe & Mobile/Desktop Host"]
        BLE(["BLE NUS · Smartphone IR / UI / Audio"])
        USB(["USB-C OTG · Desktop / Android"])
        PICO{{"Raspberry Pi Pico · RP2040 · RP2350 · P0_FW_RO 256 KiB"}}
        BLE <--> PICO
        USB <--> PICO
    end

    subgraph TARGET_HARDWARE["3 · Panasonic DIGA"]
        direction LR
        P7501{{"P7501 · JST B6B-ZR-3R4 · SW_UART·XRESET·UARTM2P_T·GND·NSW5V·UARTP2M_T"}}
        AM34{{"Matsushita AM34 · MN103S · VEP79138"}}
        FLASH[("NOR Flash 2–16 MB · IC5001 · EEMUL")]
        HDD[("PATA · SATA HDD · LBA 0 'SH'")]
        P7501 <--> AM34
        AM34 <--> FLASH
        AM34 <--> HDD
    end

    USER --> |"operates"| GUI
    GUI <--> |"uses"| BLE
    GUI <--> |"uses"| USB
    BLE <--> |"radio"| PICO
    USB <--> |"cable"| PICO
    PICO <--> |"115200 8N1"| P7501

    classDef lab       fill:#21262d,stroke:#58a6ff,color:#e6edf3,stroke-width:2px
    classDef transport fill:#21262d,stroke:#d29922,color:#e6edf3,stroke-width:2px
    classDef probe     fill:#21262d,stroke:#f0883e,color:#e6edf3,stroke-width:2px
    classDef sw        fill:#21262d,stroke:#3fb950,color:#e6edf3,stroke-width:2px
    classDef swc       fill:#21262d,stroke:#3fb950,color:#e6edf3,stroke-width:2px,font-size:14px
    classDef tgt       fill:#21262d,stroke:#f85149,color:#e6edf3,stroke-width:2px

    class USER lab
    class GUI,CLI sw
    class P7501_HAL,GDB_SRV,MEIHDFS,CRYPTO,NVRAM,RAG swc
    class BLE,USB transport
    class PICO probe
    class P7501,AM34,FLASH,HDD tgt

    style SOFTWARE_SUITE fill:transparent,stroke-width:0px
    style LAB_BENCH fill:transparent,stroke-width:0px
    style TARGET_HARDWARE fill:transparent,stroke-width:0px
```

---

## 3. Repository File Structure & Semantic Mapping

```text
Panadiga/
├── package.json                                      <-- Unified Entrypoint (`npm start`, `npm run build:desktop`, `npm run validate`)
├── Cargo.toml                                        <-- Master Rust Workspace Specification (2024 Edition, 11 crates)
├── COMPLETE_AUDIT.md / AUDITORIA_COMPLETA.md         <-- Live Root Technical Audit (EN / PT-BR)
├── profiles/                                         <-- Data-Driven Hardware SSOT (89 DIGA models, 12 JSON Schemas Draft 2020-12)
├── crates/                                           <-- Modular Multi-Crate Rust Architecture (acyclic layers)
│   ├── panadiga-constants/                           <-- Physical/Flash/MMIO/BGO SSOT Constants & Trap/Geometry Rules
│   ├── panadiga-ir/                                  <-- Compiled Hardware IR & Cross-Language Contract Hashes (`SSOT_CONTRACT_HASH`)
│   ├── panadiga-crypto/                              <-- `.FRM` Containers, 64-bit ECB/Feistel, `keys.ukf`, Okumura LZSS & SHA-256
│   ├── panadiga-bench/                               <-- Unidirectional Coalesced/Batched Bench Stream (`BenchSnapshot`, `BenchEvent`)
│   ├── panadiga-vault/                               <-- Content-Addressed Forensic Vault, Atomic `manifest.json` & Append-Only Journal
│   ├── panadiga-profiles/                            <-- Relational Chip/Board Registry, `traps/`, `forensic_atlas/` & `T0..T4` Gate
│   ├── panadiga-core/                                <-- Deterministic Core: NVRAM, MEIHDFS + `hdd/capacity_patch/` (`5 GB..2 TB`), RE (`prewrite_gate/`, `sig_locator/`, `verify_loop/`, `mn10300_emu/`, `eh55_bgo/`, `davis_probe/`, `region_compat/`, `tbc_appliance/`, `bgo_unbrick/`, `unbrick_sd_kit/`, `prog061_uart_monitor/`, `symbol_index/`), `pico/panamon1/`, Hardware SSOT
│   ├── panadiga-cli/                                 <-- `panadiga` / `panadiga-cli` Binary & Interactive TUI Workbench
│   ├── panadiga-rag/                                 <-- Zero-Error 6-Layer RAG Engine + `HardwareSafetyValidator` + `FactCritic`
│   └── panadiga-mcp/                                 <-- JSON-RPC 2.0 MCP Stdio Server Exposing All 26 Native Tools (18 Core + 8 Wave-3/Symbol Modules)
├── apps/desktop/                                     <-- Tauri 2.0 Desktop & Android GUI (`panadiga-desktop` + React 19 + Vite MVVM)
│   ├── src-tauri/                                    <-- Tauri 2.0 Native Rust Commands, Android HAL Shims, Docs/Log IPC & GDB RSP Bridge
│   ├── src/                                          <-- Strict MVVM (`views/ExpressView.tsx`, `views/IrCommanderView.tsx`, `components/express/`, `models/generated/`)
│   ├── e2e/                                          <-- 9 Playwright E2E Browser Test Specs (27 tests)
│   ├── package.json                                  <-- Node.js Dependencies, Vitest, uxlint & Playwright Scripts
│   └── playwright.config.ts                          <-- End-to-End Browser Testing Configuration
├── docs/                                             <-- Official Technical Documentation Repository
│   ├── INDEX.md                                      <-- Master Documentation Index & Gateway Portal
│   ├── translation_pairs.json                        <-- Authoritative 44-Pair Bilingual Mapping (`pt-BR` <-> `en-US` + Root)
│   ├── en-US/                                        <-- Canonical English Technical Dossiers (44 pairs)
│   ├── pt-BR/                                        <-- Native Portuguese Technical Dossiers (44 pairs)
│   ├── audits/                                       <-- Preserved Pre-Refactor Historical Audit Snapshots (`2026-11-11`)
│   ├── pseudocode/                                   <-- Full MN103S Pseudocode, `??` Audit Report & Raw Ghidra Dumps (`raw/*.txt`)
│   ├── debug_logs/                                   <-- 22 Deduplicated `COM10` / `P7501` / `PhysicalDrive1` Captures & `DOSSIE_COMPLETO_...txt`
│   └── assets/ / images/                             <-- High-Resolution Schematics, 241 Visual Catalog PNGs & Pinouts
├── manuals_and_schematics/                           <-- Technical Library (32 Manuals, Schematics & Datasheets in PDF)
├── tools/                                            <-- Lab Utilities & Ghidra AM34/MN103S Scripts (Generated from Rust SSOT)
├── scripts/                                          <-- Cross-Platform Build & 9-Gate Validation Suite (`build.py`, `validate.py`, `bench/`)
└── firmware/
    ├── picofw/                                       <-- RP2040/RP2350 C11 Zero-Gadget P7501 Probe Firmware (`P0_FW_RO = 256 KiB`)
    │   ├── cli/                                      <-- Interactive USB-CDC / BLE `PICO>` Parser & Help (`IR`, `VAULT`, `DUMPNOR`)
    │   ├── common/                                   <-- Pure C11 Hex, SPSC Ring, WDT, Nibble-LUT CRC32 & `ff_blocks` Helpers
    │   ├── diag/                                     <-- `PIOLOGIC` 4-Ch Sampler, `SDSNIFF`, `JIGSWEEP`, `IRTX`/`IRRX`/`IR VERIFY`
    │   ├── flash/                                    <-- QSPI Vault A/B (`VAULT VERIFY <1..3>`), Spansion NOR CFI & `E05` Bus Policy
    │   ├── gdb/                                      <-- Deterministic GDB RSP Protocol Codec & Server
    │   ├── generated/                                <-- Generated `picofw_ssot.h` / `picofw_ssot_tables.c` from Rust SSOT
    │   ├── hal/                                      <-- Open-Drain `XRESET`, `SW_UART` (GP26), `NSW5V` (GP3 Hi-Z IRQ) & LED HAL
    │   ├── tests/                                    <-- Host C11 Unit Test Suite (`PICO_HOST_TEST=1`)
    │   ├── uart/                                     <-- PIO1 UART Engine, `DIAGWATCH`, `DRVPANA`, `PANARSP` & `DUMPNOR` `ff_blocks`
    │   ├── picofw_uart_duplex.pio / picofw_logic_sampler.pio / picofw_sds_sampler.pio <-- PIO State Machines
    │   ├── picofw_pico_main.c                        <-- Entrypoint & Main Diagnostic Loop
    │   └── CMakeLists.txt                            <-- Dual-Target Bare-Metal RP2040/RP2350 CMake Pipeline
    └── digaos/                                       <-- Bare-Metal `DigaOS` (`DIGAOS>`, SDRAM Stage-1.5/2, `-DPANADIGA_DIGAOS_HOST_TEST=1`)
```

---

### 3.1. Unified Cross-Platform Commands (`package.json` & `scripts/`)

| Command                                       | Platform                | Mode                     | Description                                                                                                                                                                   |
| :-------------------------------------------- | :---------------------- | :----------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`npm start`**                       | Windows / Linux / macOS | Fast Launch / Auto-Build | Launches the Desktop GUI directly from `dist/` or `target/release/` (auto-builds via `scripts/build.py --desktop` if missing).                                          |
| **`npm run dev`**                     | Windows / Linux / macOS | Vite Dev Server          | Starts the frontend live-reload development server in `apps/desktop`.                                                                                                       |
| **`npm run cli -- <args>`**           | Windows / Linux / macOS | Native CLI Dispatch      | Forwards arguments to `panadiga` (`dist/`, `target/release/`, or `cargo run -p panadiga --`).                                                                         |
| **`npm run build:desktop`**           | Windows / Linux / macOS | Desktop Release          | Builds React bundle, exports Ghidra SSOT scripts, compiles Rust release binary, copies UF2s & `tools/`, and packages `.zip`/`.tar.gz` + `SHA256SUMS.txt`.             |
| **`npm run build:android`**           | Windows / Linux / macOS | Android APK Release      | Configures Android SDK/NDK, compiles AArch64 APK via Tauri Mobile, and safely restores `Cargo.toml`.                                                                        |
| **`npm run build:all`**               | Windows / Linux / macOS | Dual Target + Pico       | Compiles Pico UF2 firmwares (if ARM GCC present), Desktop Release, and Android APK.                                                                                           |
| **`npm run validate`** (`npm test`) | Windows / Linux / macOS | 9 Quality Gates          | Executes `python scripts/validate.py` covering Rust, TypeScript, Vitest, `jscpd`, `uxlint`, Playwright, Ghidra SSOT, C11 host tests, `mypy --strict`, and `pytest`. |

---

## 4. Complete Native Rust CLI Guide (`panadiga`)

The native binary `panadiga` functions as both the Tauri graphical desktop client and a high-performance command-line utility.

### 4.1. Release Compilation

```bash
# Build the native binary with maximum optimizations
cargo build --release -p panadiga

# Directly launch GUI or CLI via unified npm scripts (Windows / Linux / macOS):
npm start
npm run cli -- --help
```

### 4.2. Command 1: Multi-Geometry NVRAM Sanitation (`nvram clean`)

Wipes the corrupted wear-leveling state (`EEMUL`) within the NOR Flash without modifying the bootloader (*IPL*). Automatically detects dump size (2 MB, 4 MB, 8 MB, 16 MB) and calculates the exact 64 KB sector at `len - 64 KB`.

```bash
# 8 MB Dump (DMR-EH50 / EH55 / EH56 / EH60 / EH65 / EH80V):
panadiga nvram clean --input flash_dump_8mb.bin --output flash_recovered.bin

# 4 MB Dump (DMR-E85H / E95H / ES20):
panadiga nvram clean --input flash_dump_4mb.bin --output flash_recovered.bin

# 2 MB Dump (DMR-E50 / E55 / ES10):
panadiga nvram clean --input flash_dump_2mb.bin --output flash_recovered.bin

# 16 MB Dump (DMR-EX75 / EX85 / EX98V):
panadiga nvram clean --input flash_dump_16mb.bin --output flash_recovered.bin
```

* **Sanitation Byte:** Erased state `0xFF` (canonical virgin state of NOR flash sectors).
* **Silicon Invariant:** Never touches offsets `0x00000000` to `len - 65537`, preserving the factory IPL.

### 4.3. Command 2: Structural Hard Drive Inspection (`hdd inspect`)

Parses the MEIHDFS partition layout, locates the ASCII `"SH"` superblock, verifies the 28 MB backup mirror, and outputs partition boundaries.

```bash
panadiga hdd inspect --input hdd_image.bin
```

### 4.4. Command 3: Surgical Partition Extraction (`hdd extract`)

Extracts raw firmware partitions from the MEIHDFS drive image for reverse engineering in Ghidra or IDA Pro.

```bash
panadiga hdd extract --input hdd_image.bin --output-dir partitions/
```

### 4.5. Command 4: Optical Container Cryptanalysis (`frm analyze`)

Validates the optical update container `PANA_DVD.FRM`, checking block entropy, TOC records, and electronic codebook (ECB) leakage patterns.

```bash
panadiga frm analyze --input firmware_and_images/PANA_DVD.FRM
```

### 4.6. Command 5: MEIHDFS Bit-Inverted Descriptor Decoding (`hdd decode-descriptors`)

Decodes the bit-inverted (One's Complement / XOR 0xFF) partition descriptors found in the MEIHDFS superblock.

```bash
panadiga hdd decode-descriptors --input hdd_100.bin
```

### 4.7. Command 6: Container Deconstruction & TOC Extraction (`frm dump-toc`)

Splits `PANA_DVD.FRM` (48-byte Feistel `"Firm list"` TOC) or `PANA__SD.FRM` (`48`-byte `sd48` or `64`-byte `sd64` plaintext TOC) into its empirical firmware slots: `[0] DRV S211`, `[1] PROG061 `, and `[2] MAIN3060NP` (`3410PQ`).

```bash
panadiga frm dump-toc --input firmware_and_images/PANA_DVD.FRM
```

### 4.8. Command 7: 64-Bit ECB Codebook Audit (`frm codebook`)

Extracts high-frequency 64-bit blocks across the optical container and HDD firmware partitions to establish cryptographic equivalence.

```bash
panadiga frm codebook --frm firmware_and_images/PANA_DVD.FRM --hdd firmware_and_images/hdd_100.bin
```

### 4.9. Command 8: Critical Firmware Headers Inspection (`headers`)

```bash
panadiga headers --input firmware_and_images/hdd_100.bin
```

### 4.10. Command 9: Forensic String & Keyword Scanner (`scan`)

```bash
panadiga scan --input firmware_and_images/PANA_DVD.FRM --terms U99,FIRMWARE,PANASONIC
```

### 4.11. Command 10: Service Manuals Indexer & Fast Search (`manuals`)

Searches the 32 official Panasonic service manuals, schematics, and silicon datasheets using fast regex matching across subdirectories.

```bash
panadiga manuals --query "P7501"
panadiga manuals --query "U99" --filter "dmr-es10"
panadiga manuals --query "MN103" --filter "datasheets"
```

### 4.12. Command 11: Zero-Error RAG Engine Query (`rag query`)

Queries the local retrieval-augmented generation engine, enforcing the 6 physical invariant shields.

```bash
panadiga rag query --question "Qual o pino de terra canônico do conector P7501?"
panadiga rag query --question "What is the canonical ground pin of P7501?" --json
```

### 4.13. Command 12: Pre-MAIN Unbrick Center, SD Kit & TBC Passthrough (`unbrick`)

Unified CLI orchestration for solderless recovery: workspace inspection, Feistel module extraction (`PANA_DVD.FRM`), SD Recovery Kit generation/verification under **`--loader-profile ver061`** (default for real DMR-EH55: `256 KiB` raw `DIGADIAG.FRM` + `48B` `sd48` `PANA__SD.FRM` in `firmware_and_images/sd_unbrick_kit/`) or `ver063` (`128 KiB` `EXTRHEAD` + `sd64`), `MAIN3410PQ` TBC Passthrough `E-E` patching, `_dev_SERIAL` GDB RSP frame generation, and `NOR_DMP` stub compilation.

```bash
panadiga unbrick status
panadiga unbrick prepare-sd --loader-profile ver061 --bgo-mode uart_monitor --passthrough-mode av_passthrough
panadiga unbrick verify-sd firmware_and_images/sd_unbrick_kit
panadiga unbrick passthrough --rsp-plan --callback-va 0x50401F3C
```

---

## 5. Electrical Specification & `P7501` Async UART Interface (`115200 8N1`)

### 5.1. Canonical Pinout, Bench Wiring & Universal RP2040 / RP2350 Connection

On the digital/main board `VEP79138` (Panasonic DMR-EH55 schematic `TO JIGU`), the service header **`P7501 [B6B-ZR-3R4]`** is an **asynchronous 115200 8N1 UART + discrete boot/reset control interface**, physically positioned on the top edge between the 10-conductor flat flex cable (`FP50003`) on the left and the 8-conductor flat flex cable (`FP50004`) on the right.

> [!CAUTION]
> **ELECTRICAL SAFETY INVARIANTS — `GND` CONTINUITY & `NSW5V` (`GP3`) Hi-Z PROTECTION:**
>
> * **Canonical Ground:** Per the official Panasonic Service Manual (`P7501`), **`GND` IS STRICTLY PIN 4**. Always measure **$0.0\,\Omega$ continuity** between Pin 4 and bare chassis ground with a multimeter before connecting any bench wire.
> * **`NSW5V` (`Pin 5`, `X_SW_5R2V` -> `GP3`):** Driven from `IC1506` (`~5.15V` upstream) through `D7510` (resting ~`3.29V` on bench) to bias pull-up resistors `R7570`/`R7572` (`10 kΩ`) and `IC7501` VDD. **`GP3` MUST ALWAYS remain `Hi-Z Input` with NO internal pulls — NEVER drive push-pull LOW or HIGH!**
> * **No Back-Powering:** NEVER power the Raspberry Pi Pico from `P7501`; keep USB/VSYS as the sole supply.
> * **`Q7510`/`Q7514` Interlock:** Driving `SW_UART` (Pin 1) LOW activates `XFWE` serial boot mode and cuts off `Q7514`, disconnecting `UARTM2P_T` (Pin 3) from AM34 RX. Keep `SW_UART` in `Hi-Z` (~`3.25V`) when transmitting UART bytes from Pico `GP4` into AM34!

<p align="center">
  <img src="docs/en-US/images/panasonic_dmr_eh55_pcb_p7501_orientation.png" alt="Physical Topography and P7501 Orientation on PCB VEP79138" width="720" />
  <br/>
  <em>Figure 5.1: Physical topography of PCB VEP79138 showing connector P7501 between flat cables FP50003 (10-pin) and FP50004 (8-pin).</em>
</p>

```text
       CANONICAL MECHANICAL ORIENTATION OF CONNECTOR P7501 [B6B-ZR-3R4] (PANASONIC SCHEMATIC)
       Rear wall facing board interior (top) | Locking key window facing front edge (bottom)
   
       10-Pin Flat Cable (FP50003)                                         8-Pin Flat Cable (FP50004)
       [On the Left]                                                                     [On the Right]
            ┌──────────────────────────────────────────────────────────────────────────────────┐
      Pin 6 │ ■ UARTP2M_T [Square Pad] MAX TXD: AM34 TX -> Pico RX GP2 (115200 8N1, R7572 10k) │
      Pin 5 │ ● NSW5V     [Round Pad]  X_SW_5R2V via D7510 (~5.15V up / ~3.29V) GP3 Hi-Z ONLY! │
      Pin 4 │ ● GND       [Round Pad]  Canonical Chassis Ground (0.0Ω Continuous!)              │
      Pin 3 │ ● UARTM2P_T [Round Pad]  MAX RXD: Pico TX GP4 -> AM34 RX (via Q7514, R7570 10k)  │
      Pin 2 │ ● XRESET    [Round Pad]  Master Reset Control (Strict Open-Drain GP5)            │
      Pin 1 │ ● SW_UART   [Round Pad]  Serial Boot / Q7510 Interlock (GP26 / ADC0, ~3.25V)     │
            └──────────────────────────────────────────────────────────────────────────────────┘
```

<p align="center">
  <img src="docs/assets/images/schematic_p7501_panasonic_official.png" alt="Official Panasonic DMR-EH55 P7501 Schematic" width="460" />
  <br/>
  <em>Figure 5.2: Excerpt from official Panasonic DMR-EH55 schematic (VEP79138) detailing header P7501 [B6B-ZR-3R4] "TO JIGU".</em>
</p>

> [!NOTE]
> **RP2040 / RP2350 Hardware Compatibility:** Any board powered by an **RP2040** (Pico, Pico W) or **RP2350** (Pico 2) is 100% compatible. Standard `115200 8N1` UART framing (`p7501_uart_duplex.pio`) and deterministic `XRESET`/`SW_UART` open-drain timing are executed directly by the RP2040/RP2350 hardware.

| P7501 Pin (Canonical) |      Matsushita Signal      |     RP2040 / Pico Pin     |                 Conditioning / Board Network                 |           Signal Type           | Bench Function & Safety Directive                                                                                                                      |
| :-------------------: | :-------------------------: | :------------------------: | :-----------------------------------------------------------: | :------------------------------: | :----------------------------------------------------------------------------------------------------------------------------------------------------- |
|    **Pin 1**    |     `SW_UART` `(D)`     |  `GP26 / ADC0` (Pin 31)  | $1.0\text{ k}\Omega$ ADC series resistor; Q7510/R7562/R7559 |    Analog Sense / Open-Drain    | Reads `~3.25V` Hi-Z (`~2.69V` Pull-Down via `Q7510`). Driving LOW forces `XFWE` serial boot and disables `Q7514` RX path (`I < 10 mA`).    |
|    **Pin 2**    |         `XRESET`         |      `GP5` (Pin 7)      |           Optional $100\,\Omega$ series resistor           |   **Strict Open-Drain**   | Master system reset. **Drain to 0V (`LOW`) to assert reset; release to `Hi-Z` (~20 µs rise) to run. Never drive 3.3V Push-Pull!**           |
|    **Pin 3**    | `UARTM2P_T` (`MAX RXD`) |      `GP4` (Pin 6)      | Board `R7570 10 kΩ` pull-up to `NSW5V` + `Q7514` gate | **UART TX (Pico -> AM34)** | Asynchronous `115200 8N1` command input to AM34. Requires `SW_UART` in `Hi-Z` so `Q7514` conducts. **Do NOT add external 3.3V pull-up.** |
|    **Pin 4**    |      **`GND`**      | **Pin 38 (`GND`)** |              **Direct low-impedance bond**              |      **0V Reference**      | **Mandatory equipotential bond: verify $0.0\,\Omega$ continuity to bare metal recorder chassis!**                                              |
|    **Pin 5**    |  `NSW5V` (`X_SW_5R2V`)  |      `GP3` (Pin 5)      |     High-impedance sense via board `IC1506` / `D7510`     |   **Static Sense ONLY**   | Upstream ~5.15V via `D7510` (`~3.29V` resting). **MUST remain `Hi-Z Input` with NO pulls — NEVER configure as push-pull output!**         |
|    **Pin 6**    | `UARTP2M_T` (`MAX TXD`) |      `GP2` (Pin 4)      |    Board `R7572 10 kΩ`; optional $100\,\Omega$ series    | **UART RX (AM34 -> Pico)** | Asynchronous `115200 8N1` console output from AM34 (`<00>DDR CHECK: OK`, `CD diag boot mode`, `X(...)`). Square PCB pad.                       |

<p align="center">
  <img src="docs/en-US/images/p7501_pico_probe_assembly_schematic.png" alt="Standardized Electrical Schematic of P7501 UART Pico Probe" width="760" />
  <br/>
  <em>Figure 5.3: Standardized electrical schematic (IEEE Std 315 / IEC 60617) for building the P7501 async UART bench probe linking Raspberry Pi Pico to header P7501.</em>
</p>

---

### 5.2. Critical Engineering Rules & Damage Prevention

1. **Rule 1 (`NSW5V` Strict `Hi-Z` Protection):** `P7501` Pin 5 (`NSW5V`) is NOT a synchronous clock (`SCLK`). It links through `D7510` to the `~5.15V` auxiliary rail (`IC1506`). Configure `GP3` strictly as `Hi-Z Input` with pulls disabled; never sink or source current into Pin 5.
2. **Rule 2 (Open-Drain Emulation on `XRESET`):** Driving `XRESET` with active 3.3V push-pull logic contends against the onboard reset supervisor. Always sink to `0V` to assert reset and float to `Hi-Z` to release execution.
3. **Rule 3 (Mandatory Ground Verification):** Ensure $0.0\,\Omega$ continuity between Pin 4 and chassis ground before mating any other signals.
4. **Rule 4 (No External Pull-Up on `UARTM2P_T` & No Desoldering Required):** `VEP79138` already pulls `UARTM2P_T` via `R7570` (`10 kΩ`) to `NSW5V`; connect a 1.5mm pitch JST ZR female socket (`B6B-ZR-3R4`) cleanly without removing the TSOP-48 Flash (`IC5001`).
5. **Rule 5 (`SW_UART` / `Q7514` Interlock Awareness):** Pulse `SW_UART` low together with `XRESET` only to latch `XFWE` serial/JIG boot mode, then release `SW_UART` to `Hi-Z` before sending `115200 8N1` commands over `GP4` (`UARTM2P_T`).

---

### 5.3. Asynchronous `P7501` UART (`115200 8N1`) & Boot Mode Sequencing

```text
UART Frame (GP2 RX <- UARTP2M_T | GP4 TX -> UARTM2P_T @ 115200 baud, 8N1):
IDLE (HIGH) ──\ START (0) /─ B0 ─ B1 ─ B2 ─ B3 ─ B4 ─ B5 ─ B6 ─ B7 ─\ STOP (1) ──  (8.68 µs/bit)

Verified DMR-EH55 Bench Boot Sequence (CDDIAG Trap via P7501):
  1. XRESET (GP5) = LOW (10 ms Open-Drain) -> Release Hi-Z (~20 µs rise)
  2. SW_UART (GP26) = LOW during reset window for JIG / CDDIAG sampling -> Release Hi-Z for Q7514 TX
  3. UARTP2M_T (GP2) emits <00>DDR CHECK: OK -> CD diag boot mode -> X(80020401)59..X(80040904)0
```

---

## 6. Memory Map of Matsushita AM34 (`MN103S` / `MN2DS0009` on `VEP79138`)

### 6.1. SSOT Hardware Bus Regions & Verified Firmware Virtual Addresses

| Address Range (Hex)             | Region / Binary                                 | Access / Purpose                                                                                                                                                                                                                                                                                                                 |
| :------------------------------ | :---------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `0x00000000..0x0003FFFF`      | On-Chip Internal SRAM (`256 KB`)              | Exception vectors & RAM loader aperture (`0x00004000`, `4 KB`)                                                                                                                                                                                                                                                               |
| `0x10000000..0x1000FFFF`      | Core MMIO Bus (`64 KB`)                       | Core timers & interrupt registers                                                                                                                                                                                                                                                                                                |
| `0x40000000..0x43FFFFFF`      | System SDRAM (`64 MB`)                        | Low-aperture cached SDRAM / `DRV_S211` `0x40000000` / `BOOT.BIN` at `0x44000000`                                                                                                                                                                                                                                         |
| `0x50400000..0x5043FFFF`      | `PROG061` FVU Updater (`262,144 B`)         | Stage-2 updater (`eh55_frm_PROG061_BOOT.bin`) & `DIGADIAG.FRM` jump target                                                                                                                                                                                                                                                   |
| `0x515B2E00` / `0x515B2E40` | `_dev_SERIAL` State & Callback                | UART0 RSP protocol state table (`0x515B2E60`: `0..4`) & serial callback pointer (`0x515B2E40`)                                                                                                                                                                                                                             |
| `0x5316AE8C` (`+0x64`)      | ISR Context Frame Pointer                       | Active ISR register save context frame (`PC` saved at offset `0x64`)                                                                                                                                                                                                                                                         |
| `0x57000000..0x57C664EB`      | `MAIN` OS (`EH55_MAIN_3410PQ.bin`)          | Main AM34 OS kernel (`13,003,052 B`), AV codecs, Matsushita internal `"src/hdd_cmd.c"`, `_dev_SERIAL` (`0x574F0775`)                                                                                                                                                                                                     |
| `0x80000000..0x807FFFFF`      | Spansion `S29JL064H` NOR (`8 MB`, CS0)      | Cached NOR window (`142` sectors `SA0..SA141`): IPL `Loader ver0.61` (`SA0..SA10`), `DRV` (`0x80080000`), BGO `[nfrom]` (`0x80130000..0x803FFFFF` on EH55 `ver0.61`, `SA26..SA70`), Upper EEMUL (`SA71..SA141`, `0x80400000..0x807FFFFF`, `SA134` @ `0x807F0000` header + `SA135..SA141` `0xFF`) |
| `0x84000000..0x847FFFFF`      | Uncached NOR Flash & BGO Window                 | Read-only dump base (`PANADMP1` -> `0x84000000`), EH55 `ver0.61` BGO (`0x84130000..0x843FFFFF`, `45×64 KB` @ `0x440337A4` vs `ver0.63`/`PROG061` `0x84200000..0x843FFFFF`), & Upper EEMUL (`0x84400000..0x847FFFFF`)                                                                                        |
| `0x90400000..0x9043FFFF`      | Uncached `PROG061` SDRAM Mirror               | Uncached alias of `0x50400000`, used for AM34 D-cache/I-cache coherence in `PANADMP1` (`0x9043950E` -> `J 904394FA`) & RAM stubs (`0x90439B40`)                                                                                                                                                                        |
| `0x89000500` / `0x89000D00` | `DAVIS` IDE Controllers (`ch=0` / `ch=1`) | Primary Master (`ch=0, dev=0`) = ODD ATAPI (`DRV S211`); Secondary **MASTER** (`ch=1, dev=0`) = HDD (`StarTech IDE2SAT2` `Marvell 88SA8052` in **MASTER**)                                                                                                                                                 |
| `0xD4002008..0xD400200C`      | `P7501` UART0 (`115200 8N1`)                | `SC0TXB = 0xD4002008`, `SC0RXB = 0xD4002009`, `SC0STR = 0xD400200C` (TX busy `0xA0`, RX ready `0x10`)                                                                                                                                                                                                                  |

> [!NOTE]
> **`DMR-EH55` Unbrick Stub Aperture (`PANADMP1`):** The read-only MN103S NOR Flash dumper stub assembled by `panadiga_nor_dumper_stub` defaults to `0x51800000` in free SDRAM and reads strictly from the uncached `0x84000000` window (`8,388,608` bytes in `4096`-byte checksummed blocks) over UART0 `P7501`.

---

## 7. Forensic Cryptanalysis & Storage Architecture Deep-Dive

### 7.1. Electronic Codebook (ECB) Leakage

Statistical analysis of both `PANA_DVD.FRM` and raw HDD dumps reveals that Matsushita implemented 64-bit symmetric block encryption without Cipher Block Chaining (CBC). Consecutive identical 8-byte blocks in plaintext yield identical 8-byte ciphertext blocks, exposing memory tables and structures.

### 7.2. 64-Bit ECB Codebook & Known-Plaintext Attack (KPA)

| Ciphertext Block (Hex) |      Plaintext Meaning      | FRM Occurrences | HDD Occurrences | Forensic Significance                        |
| :--------------------: | :-------------------------: | :-------------: | :-------------: | :------------------------------------------- |
|  `035065C87BB8E090`  | `FF FF FF FF FF FF FF FF` |      3,812      |      1,280      | Virgin NOR Flash erased cells / zero-padding |
|  `E5D1F34A518B2690`  | `00 00 00 00 00 00 00 00` |       196       |        3        | Null padding / Zero blocks                   |
|  `4D3C5DBB0C357777`  |   Partition Table Header   |      1,280      |        0        | Internal Module Descriptor Frame             |
|  `EFDC1CDA13881BAB`  |   Processor Family Header   |        3        |        0        | TOC: Matsushita UniPhier SoC Family Marker   |
|  `74A78ACD789B84FF`  |      CPU Core Selector      |        2        |        0        | TOC: AM34 / MN103S Core Identifier           |
|  `3B7E7EDBB6C57A53`  |      DSP Core Selector      |        1        |        0        | TOC: MN102 Optical Drive Servo DSP           |

### 7.3. Optical Container Structure (`PANA_DVD.FRM`)

The update disc packaging decomposes cleanly into three discrete modules preceded by a 16-byte container header (`492F0560CEDE508CE3C08AFD49B64A9A`) and a 144-byte TOC (three 48-byte records):

1. **Slot `[0]` (`DRV S211`, offset `0x001800`, `720,896 B` -> `1,441,792 B` @ `0x90202000`):** Optical drive servo & ATAPI controller firmware.
2. **Slot `[1]` (`PROG061 `, offset `0x0B1800`, `262,144 B` raw @ `0x50400000`):** Stage-2 SD/DVD Upgrader & `BGO` / `NVRAM U99` initializer (`SDDIAG`).
3. **Slot `[2]` (`MAIN3060NP`, offset `0x0F1800`, `6,542,560 B` -> `13,427,852 B` @ `0x57000000`):** MN103S Main OS kernel, OSD, MEIHDFS HDD driver, and TBC controller (HDD Partition 1 `0x02000000` unpacks to `13,003,052 B` `EH55_MAIN_3410PQ.bin`).

### 7.4. Irrefutable Cryptographic Link to the Internal HDD

The presence of identical cipher blocks `E5D1F34A518B2690` (zeros) and `035065C87BB8E090` (erased flash) in both the optical `.FRM` container and the hard drive partitions proves conclusively that:

1. The firmware stored on the hard drive is encrypted using the **same algorithm and key** as the optical update media.
2. The internal HDD cannot be cloned or repaired with arbitrary unencrypted data; it requires the correct Panasonic partition structure and decryption keys.

### 7.5. Bit-Inverted (One's Complement / XOR 0xFF) MEIHDFS Architecture

The LBA 0 superblock stores partition geometry inverted with bitwise NOT (`XOR 0xFF`):

* ASCII magic `"SH"` appears in raw disk hex as `0xAE 0xB7` (`~0x51 0x48`).
* LBA offset counters and cluster sizes are similarly inverted, preventing standard partition managers from identifying or corrupting the volume.

### 7.6. Plaintext Firmware Architecture (`DIGADIAG.FRM` vs `PANA_DVD.FRM` vs `PANA__SD.FRM`), HDD LBA48 & Autonomous Pico `PANARSP`

* **Firmware Update Containers (`Loader ver0.61` vs `/BOOT.BIN` `ver0.63` & `PROG061`):**
  * **`DIGADIAG.FRM`** (`SDDIAG`): Supports dual bootloader formats — real DMR-EH55 **`Loader ver0.61 [06/03/06]`** loads **`262,144` raw uncompressed bytes (`0x40000`)** directly to `0x90400000` (`JMP 0x50400000`, default `--loader-profile ver061`), whereas `/BOOT.BIN` (`ver0.63`) reads `131,072` B (`0x20000`) `EXTRHEAD` (`"FVU "`/`"DIAG"`, `dst_addr = 0x50400000`, strided Adler-32 + `zlib` + `EXTRFOOT`) into `0x93FC0000`. `panadiga_bgo_unbrick` also supports `"uart_monitor"` (`PANAMON1>[B/R/W/D/F/J/E/U/Q]` at `0x50439494`, MN103S compiler `CALL ..., [], 0` ABI + `46 B` `PUTC_D3_BYTES` `SC0STR & 0xA0`/`0x00040000`).
  * **`PANA_DVD.FRM`** (`FUN_4400A6D0`): `16` B global header + Feistel-encrypted `3 x 48` B `"Firm list"` TOC (`FUN_4400a666`, 8-round 64-bit ECB, S-Box `0x44035D84`, `list_sum` at `+0x2E`, `fvu_checksum_u32` at `+0x24`) + 3 `EXTRHEAD` modules (`PROG061` -> `0x50400000`; `DRV` S211 first `0x400` B encrypted -> ATAPI ODD; `MAIN` first/last `0x400` B encrypted -> HDD Partition 1 `LBA 65536` / `0x02000000`).
  * **`PANA__SD.FRM`**: Plaintext TOC (`0x30` = `48` B `sd48` entries in `PROG061` / real EH55 `ver0.61`, `0x1800` B TOC, `list_sum_u16` at `+0x2E`; or `0x40` = `64` B `sd64` entries in `/BOOT.BIN` `ver0.63`, `0x2000` B TOC, `list_sum_u16` at `+0x3E`).
* **PATA Topology & LBA48 Bridge:** ODD is Primary Master (`ch=0, dev=0`, `0x89000500`) and HDD is Secondary **MASTER** (`ch=1, dev=0`, `0x89000D00`). SATA drives require a **Marvell `88SA8052`** bridge (`StarTech IDE2SAT2`) jumpered strictly in **`MASTER`** plus dynamic LBA48 patching (`crates/panadiga-core/src/hdd/capacity_patch/mod.rs`).
* **Autonomous Pico `PANARSP`, `DIAGWATCH` & NOR Geometry (`P7501` `115200 8N1`):**
  * **`PANARSP`:** Supports `WMEM`, `WRITE32`, `PASSTHROUGH`, `DUMPNOR [<addr_hex> [<len_hex>]]` (`PANADMP1..PANADONE` 8 MB read-only NOR stream from `0x84000000` via `$M` -> `0x51800000` + callback `0x515B2E40`), and `CALL` with automatic `0E 04 02` (`DEV_SERIAL_HANDSHAKE_BYTES`) + `0x15` handshake targeting `0x515B2E60`, plus the 3 `MAIN` AV TBC `E-E` passthrough frames (`0x5771ABFB` `#7a`, `0x574F7B8C` `#40`, `0x574F7C5C` `#63`).
  * **`DIAGWATCH` & Flash Aperture:** `DIAGWATCH [ms] [NORMAL|JIG[:ms]|NOPULL] [AUTO|BYPASS|FORMAT|DUMP|NONE] [RELEASE|HOLD]` monitors `SC0` with millisecond timestamps, releases `SW_UART` to Hi-Z on `serial boot mode` (when `JIG`, preserving `P50` LOW sampling) or `DDR CHECK: OK` (when `NORMAL`), classifies `-91006`/`83A7`/`BGO` failures, and dispatches `B`/`F`/`D` on `PANAMON1>`. `FLASH_SIZE` and `INJECT_LOADER [UNCACHED] CONFIRM` expose uncached `0x84000000`, the real EH55 `Loader ver0.61` `BGO` partition (`0x84130000..0x843FFFFF`, `45 x 64 KB` sectors `SA26..SA70` vs `ver0.63`/`PROG061` `0x84200000..0x843FFFFF`), and Upper EEMUL (`SA71..SA141`, `0x84400000..0x847FFFFF`).

### 7.7. Bench-Captured `IC5001` 8 MiB Gold Dump & Surgical `U99` BGO Recovery

* **Gold Dumps (`firmware_and_images/`, read-only `PANADMP1` capture over `COM10` via `0x9043950E` -> `J 904394FA`):**
  * `eh55_nor_8mb_ic5001_dump.bin`: `8,388,608 B` (`SHA-256: c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`)
  * `eh55_loader_ver061_256k.bin`: `262,144 B` (`0x84000000..0x8403FFFF`, `SHA-256: 9f5c448d8879106be5ca3da6d6ac649026d50fbc191251ae0f4a003efca63800`)
  * `eh55_nvram_64k_raw.bin`: `65,536 B` (`0x847F0000..0x847FFFFF`, `SHA-256: 803d1465f48fb32e17d3e478e02b2a9a7e707bbb675bb424b02c5c1df92f3643`)
* **FTL BGO Geometry (`0x440337A4`) & ~700 ms Spansion Erase:** On real EH55 `Loader ver0.61`, BGO spans `45` sectors (`SA26..SA70`, `1` P1 `0xF101`, `40` P2 `0xF201..0xF228`, `4` spares `0xFFFF`, max PID `0x0F24`, control `0x0F01`, dir `0x0F02..0x0F04`, FAT `0x0F07..0x0F15` + backup `0x0F16..0x0F24`), separated from Upper EEMUL `"1284"` (`SA71..SA141`, `SA134` @ `0x847F0000` active header `0ff00000002c`, `SA135..SA141` `0xFF`). Real 64 KiB sector erase takes ~`700 ms`, outlasting bounded `0x00100000` DQ7 loops and requiring `0xFF` readback polling.
* **Surgical `U99` Cure & `BOOTTRACE 15000 NOJIG` Proof:** Formatting `SA70` (`0x843F0000`) + `1,629` pure `1->0` byte writes across 13 spares (`SA42..SA66`, rebuilding 13 `0xF2xx` sectors, `ifodev2` blk 2 `0x0055` @ `0x84250800`, and FAT `0x0F0A`/`0x0F19` @ `0x8427A200`/`0x8427AE00`) + `6` pure `1->0` writes at `SA41+0x14` (`0x84220014..0x84220023` -> `SA29`) eliminated `BGO driver init failed[1].` and booted live on `COM10` through `***** normal mode *****` -> `*** RTSC Initialize ***`.

---

## 8. Practical U99 Resolution Guide (3 Engineering Routes)

```text
                   +---------------------------------------------+
                   |       PANASONIC DMR-EH55 U99 ERROR          |
                   |       (Permanent VFD Display Panic)         |
                   +----------------------+----------------------+
                                          |
                +-------------------------+-------------------------+
                |                         |                         |
                v                         v                         v
     [ROUTE A: HARDWARE LAB]   [ROUTE B: IN-CIRCUIT UART]  [ROUTE C: FORENSIC CLONE]
     Desolder TSOP-48 Flash    Connect RP2040 to P7501     Direct raw clone of HDD
     Program via T48 / SVOD4   115200 8N1 / CDDIAG Trap    Zero sector 0x7F0000
     Direct Clean 0x7F0000     Live NVRAM Capture / GDB    Preserve 100 MB System Area
```

### Route A: External Flash Programmer (TSOP-48 via T48 / SVOD4)

* Desolder `IC5001` (Spansion S29JL064H70TFI00).
* Read full 8 MB dump and save backup.
* Use `panadiga nvram clean --input dump.bin --output fixed.bin`.
* Flash `fixed.bin` back to chip and resolder.

### Route B: In-Circuit via `P7501` Async UART, GDB `PANARSP` & SD Recovery Kit (`DIGADIAG.FRM` / `PANA__SD.FRM`)

* **SD Card Pre-MAIN Unbrick (SDSC `<= 2 GiB` FAT16):** Generate the **`--loader-profile ver061`** kit in `firmware_and_images/sd_unbrick_kit/` (`256 KiB` raw `DIGADIAG.FRM` for real DMR-EH55 `Loader ver0.61` + `48B` `sd48` `PANA__SD.FRM`; use `ver063` only for `/BOOT.BIN` `128 KiB EXTRHEAD`) via **`panadiga unbrick prepare-sd --loader-profile ver061 --bgo-mode uart_monitor`** (or Desktop **`Unbrick Center`**) to bypass `U99` / run `PANAMON1>[B/R/W/D/F/J/E/U/Q]` and persist `MAIN3410PQ` TBC Passthrough + dynamic HDD sizing.
* **UART / GDB `P7501` Injection:** Connect the 6-wire JST ZR bench probe from Pico to header `P7501` (confirm Pin 4 `GND` = $0.0\,\Omega$ and keep `GP3` / `NSW5V` strictly `Hi-Z`), then run `panadiga pico uart-probe` / `panadiga gdb-bridge` for live `115200 8N1` telemetry and `PANARSP` commands.

### Route C: Hard Drive Cloning & Sector Sanitization (Preserving MEIHDFS)

* When U99 is caused by corrupted HDD system state, connect drive to bench PC.
* Clone raw 100 MB system area: `panadiga hdd extract --input /dev/sdb --output-dir backup/`.
* Write clean partition descriptors and remount in recorder.

---

## 9. Comprehensive Panasonic DIGA Lineup Compatibility & Architectural Comparison Matrix

The following matrix formally specifies the architecture, silicon variants, Flash geometries, NVRAM locations, and unbricking compatibility across the 5 generations of the Panasonic DIGA lineup:

| Device Lineup & Models                                                                                          | Generation / Era                             | SoC Processor & Architecture                                             | NOR Flash Size & Chip                                            | NVRAM / EEMUL Offset Range              | Storage System & Interface                     | Service / Debug Header (`P7501` & Lineup)                                                                               | U99 Root Cause & Panadiga Unbrick Method                                                                                       |
| :-------------------------------------------------------------------------------------------------------------- | :------------------------------------------- | :----------------------------------------------------------------------- | :--------------------------------------------------------------- | :-------------------------------------- | :--------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------ | :----------------------------------------------------------------------------------------------------------------------------- |
| **DMR-E20, DMR-E30, DMR-E50, DMR-E55, DMR-E60**                                                           | **Gen 1** (2001–2003)                 | Matsushita MN10300 (16/32-bit AM32 / AM33)                               | **2 MB** (16 Mbit) TSOP-48 (Am29LV160)                     | `0x001F0000..0x001FFFFF` (Last 64 KB) | Optical DVD-RAM / DVD-R (Direct UDF 1.5)       | 5-pin / 6-pin factory UART / service header (3.3V)                                                                        | Flash wear-leveling corruption. Fixed via `panadiga nvram clean` (auto-detected 2 MB).                                       |
| **DMR-E80H, DMR-E85H, DMR-E95H, DMR-E100H, DMR-E500H, DMR-ES20**                                          | **Gen 2** (2003–2004)                 | Matsushita MN103S / Early UniPhier                                       | **4 MB** (32 Mbit) TSOP-48 (Am29LV320)                     | `0x003F0000..0x003FFFFF` (Last 64 KB) | PATA/IDE HDD (40–160 GB) + MEIHDFS-V1         | 6-pin service UART header (3.3V LVTTL)                                                                                    | Pioneer generation of the `U99` panic. Fixed via `panadiga nvram clean` (auto-detected 4 MB).                              |
| **DMR-EH50, DMR-EH52, DMR-EH55, DMR-EH56, DMR-EH60, DMR-EH65, DMR-EH75V, DMR-EH80V, DMR-ES10, DMR-ES30V** | **Gen 3** (2005–2007) [Core Platform] | Matsushita `AM34` / `MN103S` (`VEP79138` Main / Syscon `IC7501`) | **8 MB** (64 Mbit) TSOP-48 (`IC5001` Spansion S29JL064H) | `0x007F0000..0x007FFFFF` (Last 64 KB) | PATA/IDE HDD (80–250 GB) + MEIHDFS-V2         | Canonical **`P7501`** (JST B6B-ZR-3R4: `SW_UART`, `XRESET`, `UARTM2P_T`, `GND`, `NSW5V`, `UARTP2M_T`) | Classical `U99` panic due to EEMUL ring desync. Fixed via `panadiga nvram clean` or in-circuit UART telemetry (`P7501`). |
| **DMR-EX75, DMR-EX85, DMR-EX95V, DMR-EX98V, DMR-EH67, DMR-EH68**                                          | **Gen 4** (2006–2008)                 | Matsushita `MN103S` / `AM34` Gen 2                                   | **16 MB** (128 Mbit) TSOP-56 (Spansion S29GL128N)          | `0x00FF0000..0x00FFFFFF` (Last 64 KB) | SATA / PATA HDD (160–320 GB) + MEIHDFS-V2+    | 6-pin JST ZR `115200 8N1` UART / service header                                                                         | Digital DVB-T channel table desync triggering U99. Fixed via `panadiga nvram clean` (auto-detected 16 MB).                   |
| **DMR-BW500, DMR-BR500, DMR-BW700, DMR-BW800, DMR-BWT700**                                                | **Gen 5** (2008–2011)                 | Matsushita UniPhier Dual-Core (`MN103S` / ARM hybrid)                  | **32 MB -- 64 MB** NOR/NAND Flash                          | Dynamic allocation per volume           | SATA 3.5" HDD (250 GB -- 1 TB) + BD-RE Optical | Service UART / JTAG debug header                                                                                          | Blu-ray generation. MEIHDFS-V3 with AES encryption. Partition extraction via `panadiga hdd extract`.                         |

### 9.1. Why Panadiga Operates Universally Across Generations ("Why It Works")

1. **Mathematical Invariant of Flash Sizing (`len - 64 KB`):** Matsushita firmware engineers strictly allocated the circular EEPROM emulation ring buffer (`EEMUL`) in the highest 64 KB physical sector of the primary NOR Flash across all generations. Instead of hardcoding static offsets, `panadiga nvram clean` dynamically evaluates `offset = len - 65536`. Thus, whether an engineer feeds a 2 MB dump (`0x1F0000`), 4 MB dump (`0x3F0000`), 8 MB dump (`0x7F0000`), or 16 MB dump (`0xFF0000`), the tool safely targets the exact wear-leveling sector without touching IPL code.
2. **Service UART & Discrete Control Invariance (`115200 8N1`):** Across `VEP79xxx` service headers (`P7501`), console telemetry runs over asynchronous `115200 8N1` UART paired with open-drain reset (`XRESET`) and high-impedance rail sensing (`NSW5V`). The `p7501_uart_duplex.pio` + C11 HAL interacts deterministically with these headers while enforcing zero push-pull contention.
3. **Storage Superblock Invariance (MEIHDFS-V1 through V2+):**
   The ASCII `"SH"` magic byte sequence and 28 MB backup mirror geometry were preserved from early HDD models (DMR-E85H) through mature digital units (DMR-EH55, DMR-EX98V).

---

## 10. Technical Documentation Library (32 Manuals, Schematics & Datasheets)

All official documentation is cataloged and hierarchically structured in `manuals_and_schematics/`:

### A. Panasonic DIGA Service Manuals & Technical Schematics

* `dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf` (177 pages, VEP79138 schematics, P7501 pinout)
* `dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol1_diagnostics.pdf` (97 pages, diagnostics, U99 error flow)
* `dmr-eh55/service/panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf` (97 pages, North American NTSC chassis)
* `dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_europe.pdf` (92 pages, European PAL chassis)
* `dmr-eh55/service/panasonic_dmr_eh55ee_service_manual_east_europe.pdf` (91 pages, Eastern European SECAM chassis)
* `dmr-eh80v/service/panasonic_dmr_eh80v_service_manual_europe.pdf` (12.2 MB, Triple combo VHS+DVD+HDD service schematics)
* `dmr-es10/service/panasonic_dmr_es10_service_manual_north_america.pdf` (5.3 MB, DVD-RAM service manual, U99 error code analysis)
* `dmr-ex98v/service/panasonic_dmr_ex98v_service_manual_vol1.pdf` (10.4 MB, DVB-T/HDMI combo service manual Vol 1)
* `dmr-eh55/firmware_update/panasonic_dmr_eh55_firmware_installation_guide.pdf` (8 pages, factory firmware CD-R update guide)

### B. Panasonic DIGA User & Operational Instructions

* `dmr-eh55/user/panasonic_dmr_eh55_operating_instructions_global.pdf` (89 pages)
* `dmr-eh55/user/panasonic_dmr_eh55s_operating_instructions_silver.pdf` (89 pages)
* `dmr-eh55/user/panasonic_dmr_eh55_eh56_operating_instructions_europe.pdf` (84 pages)
* `dmr-eh55/user/panasonic_diga_dmr_eh55_user_manual.pdf` (84 pages)
* `dmr-eh55/user/panasonic_dmr_eh55_user_manual_oceania.pdf` (76 pages)
* `dmr-eh55/user/panasonic_dmr_eh55_australia_new_zealand_manual.pdf` (76 pages)
* `dmr-eh55/user/panasonic_dmr_eh55_hdmi_hdavi_control_guide.pdf` (6 pages)
* `dmr-eh55/user/panasonic_dmr_eh55_quick_setup_guide.pdf` (2 pages)
* `dmr-eh56/user/panasonic_dmr_eh56_operating_instructions.pdf` (89 pages)
* `dmr-eh50/user/panasonic_dmr_eh50_user_manual.pdf` (4.1 MB)
* `dmr-eh65/user/panasonic_dmr_eh65_user_manual.pdf` (4.8 MB)
* `dmr-e85h/user/panasonic_dmr_e85h_user_manual.pdf` (2.7 MB)
* `dmr-e55/user/panasonic_dmr_e55_operating_instructions.pdf` (3.2 MB)
* `dmr-e500h/user/panasonic_dmr_e500h_user_manual.pdf` (4.9 MB)
* `dmr-es30v/user/panasonic_dmr_es30v_user_manual.pdf` (7.9 MB)

### C. Silicon Datasheets & Architecture Reference Manuals (`datasheets/`)

* `datasheets/matsushita_mn10300_series_cross_assembler_users_manual.pdf` (Instruction set, addressing modes, register conventions)
* `datasheets/matsushita_mn10300_series_c_debugger_users_manual.pdf` (C-level debugging and On-Chip Debug OCD protocol)
* `datasheets/rp2040_datasheet_raspberrypi.pdf` (5.3 MB, RP2040 microcontroller datasheet, PIO state machines)
* `datasheets/rp2350_datasheet_raspberrypi.pdf` (11.1 MB, RP2350 microcontroller datasheet, Pico 2 architecture)
* `datasheets/spansion_am29lv640d_nor_flash_datasheet.pdf` (1.0 MB, Parallel NOR Flash 64 Mbit TSOP-48 specifications)
* `datasheets/spansion_s29gl128n_nor_flash_datasheet.pdf` (4.2 MB, MirrorBit NOR Flash 128 Mbit TSOP-56 specifications)
* `datasheets/rohm_br24l16_datasheet.pdf` (I²C EEPROM 16 Kbit timer/standby reference, `0x50..0x57`)
* `datasheets/microchip_24lc64_datasheet.pdf` (I²C EEPROM 64 Kbit optical drive calibration reference)

---

## 11. Direct Links to Audits, Technical Dossiers, Pseudocode & Logs (`docs/`)

The repository maintains **44 synchronized bilingual (`en-US` / `pt-BR`) engineering dossiers** (indexed in [`docs/INDEX.md`](docs/INDEX.md) and [`docs/translation_pairs.json`](docs/translation_pairs.json)), root technical audits ([`COMPLETE_AUDIT.md`](COMPLETE_AUDIT.md) / [`AUDITORIA_COMPLETA.md`](AUDITORIA_COMPLETA.md)), historical audit snapshots ([`docs/audits/`](docs/audits/README.md)), full MN103S Ghidra pseudocode/disassembly ([`docs/pseudocode/`](docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) & [`docs/pseudocode/raw/`](docs/pseudocode/raw/)), and 28 deduplicated live `COM10` / bench logs ([`docs/debug_logs/`](docs/debug_logs/README.md)):

* 📋 **[Consolidated Root Technical Audit](COMPLETE_AUDIT.md)** ([Português: `AUDITORIA_COMPLETA.md`](AUDITORIA_COMPLETA.md))
  *Live repository audit (`crates/`, `firmware/picofw/`, `firmware/digaos/`, `scripts/`), rebuilt RP2040/RP2350/Unified UF2 hashes, Ghidra `??` root-cause summary, Python-to-Rust parity, and 9 Quality Gates.*
* 🏛️ **[Systems Architecture &amp; Engineering Blueprint](docs/en-US/SYSTEM_ARCHITECTURE_AND_ENGINEERING_BLUEPRINT.md)** ([Portuguese Mirror](docs/pt-BR/ARQUITETURA_E_BLUEPRINT_DE_ENGENHARIA.md))
  *Master specification: modular decomposition, Pure FP domain, strict MVVM, GDB ring buffer, and silicon invariants.*
* 📱 **[Android APK &amp; Bluetooth Architecture](docs/en-US/ANDROID_APK_AND_BLUETOOTH_ARCHITECTURE.md)** ([Portuguese Mirror](docs/pt-BR/ARQUITETURA_MOBILE_ANDROID_E_BLUETOOTH.md))
  *Android APK build, Native Rust HAL (BLE NUS / USB-C OTG), and P7501 power safety rules.*
* 🧪 **[Testing Architecture &amp; Compliance Verification](docs/en-US/TESTING_ARCHITECTURE_AND_COMPLIANCE.md)** ([Portuguese Mirror](docs/pt-BR/ARQUITETURA_DE_TESTES_E_CONFORMIDADE.md))
  *5-layer test pyramid (Pure FP, GDB RSP, MVVM ADTs, CLI SSOT, E2E), traceability matrix, and CI/CD status.*
* ⚡ **[Bench Wiring Blueprint &amp; Electrical Schematic](docs/en-US/BLUEPRINT_BENCH_WIRING_AND_SCHEMATIC.md)** ([Portuguese Mirror](docs/pt-BR/BLUEPRINT_ESQUEMA_ELETRICO_BANCADA.md))
  *Complete wiring harness, P7501 connector pinout, RP2040 hookup, prototype vs production resistors.*
* 💾 **[MEIHDFS Storage &amp; Partition Forensics](docs/en-US/STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md)** ([Portuguese Mirror](docs/pt-BR/ARQUITETURA_STORAGE_MEIHDFS_PARTICOES.md))
  *Panasonic MEIHDFS, LBA 0 "SH" MBR, 28 MB mirror, 8 MB journal, and firmware partitions.*
* 🔐 **[Dossier on Cryptography &amp; PANA_DVD.FRM Container](docs/en-US/DOSSIER_CRYPTOGRAPHY_AND_FIRMWARE_PANA_DVD.md)** ([Portuguese Mirror](docs/pt-BR/DOSSIE_CRIPTOGRAFIA_E_FIRMWARE_PANA_DVD.md))
  *Optical container PANA_DVD.FRM, CPRM/C2 ciphers, 64-bit ECB codebooks, and entropy analysis.*
* 🔬 **[Ghidra &amp; GDB Integration Strategy](docs/en-US/GHIDRA_GDB_INTEGRATION_STRATEGY.md)** ([Portuguese Mirror](docs/pt-BR/ESTRATEGIA_INTEGRACAO_GHIDRA_GDB.md))
  *AM34/MN103S processor architecture, Ghidra SLED module, and GDB Remote Serial Protocol (:3333).*
* 🛠️ **[Bench Probe Assembly &amp; Photographic Guide](docs/en-US/BENCH_PROBE_ASSEMBLY_AND_PHOTOS_GUIDE.md)** ([Portuguese Mirror](docs/pt-BR/GUIA_MONTAGEM_E_FOTOS_PROBE_BANCADA.md))
  *Step-by-step soldering, JST ZR 1.5mm female socket mating, Pin 4 GND continuity verification.*
* 📋 **[Technical Inventory &amp; Operations Manual](docs/en-US/TECHNICAL_INVENTORY_AND_OPERATIONS_MANUAL.md)** ([Portuguese Mirror](docs/pt-BR/INVENTARIO_TECNICO_E_GUIA_DE_OPERACAO.md))
  *File inventory, cryptographic SHA-256 manifests, and step-by-step U99 unbrick runbook.*
* 📊 **[Final Audit &amp; Modularization Report](docs/en-US/FINAL_AUDIT_AND_MODULARIZATION_REPORT.md)** ([Portuguese Mirror](docs/pt-BR/RELATORIO_AUDITORIA_E_MODULARIZACAO_FINAL.md))
  *Historical `2026-11-07` modularization baseline, cross-stack integration (Rust/C/Python/TypeScript), and 9 quality gates foundation.*
* 📡 **[Logging &amp; Telemetry Subsystem Specification](docs/en-US/LOGGING_AND_TELEMETRY_SUBSYSTEM.md)** ([Portuguese Mirror](docs/pt-BR/SUBSISTEMA_LOGGING_E_TELEMETRIA.md))
  *Structured JSON logging, thread-safe sinks, and real-time serial terminal integration.*
* 💡 **[Semiconductor Theory &amp; Practical U99 Unbricking](docs/en-US/THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md)** ([Portuguese Mirror](docs/pt-BR/TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md))
  *Semiconductor physics of NOR Flash, EEMUL circular wear-leveling, and 64 KB NVRAM sanitization.*
* 🛡️ **[Zero-Error RAG Engine &amp; Physical Safety Invariants](docs/en-US/ZERO_ERROR_RAG_ENGINE_AND_PHYSICAL_SAFETY.md)** ([Portuguese Mirror](docs/pt-BR/ZERO_ERROR_RAG_ENGINE_E_SEGURANCA_FISICA.md))
  *6-layer defense system (QueryGuard, CRAG, GroundedSynthesis, FactCritic, InvariantShield).*
* 🔬 **[Deep Reverse Engineering Dossier: MN103S Bootloader, Optical Drive &amp; Live Telemetry](docs/en-US/REVERSE_ENGINEERING_BOOTLOADER_AND_OPTICAL_DRIVE_MN103S.md)** ([Portuguese Mirror](docs/pt-BR/DOSSIE_ENGENHARIA_REVERSA_BOOTLOADER_E_DRIVE_MN103S.md))
  *Live `DMR-EH55` bench telemetry (`P7501`/`CDDIAG`), MN103S disassembly of `BOOT.BIN` & `DRV_decomp.bin`, 74 ATAPI opcodes, 62 factory CLI commands, and the full 431-entry Shift-JIS error dictionary (`0x83A7`).*
* 🧩 **[Exploded Architecture, Pico Injection &amp; AV Passthrough](docs/en-US/EXPLODED_VIEW_PICO_INJECTION_ARCHITECTURE_AND_BYPASS_PASSTHROUGH.md)** ([Portuguese Mirror](docs/pt-BR/VISTA_EXPLODIDA_ARQUITETURA_INJECAO_PICO_E_BYPASS_PASSTHROUGH.md))
  *`DIGADIAG.FRM` vs `PANA_DVD.FRM` vs `PANA__SD.FRM` container specs, autonomous Pico `PANARSP` (`WMEM`/`WRITE32`/`PASSTHROUGH`/`CALL`), `P7501` handshake (`0x515B2E60`), and the 3 `MAIN` AV TBC `E-E` passthrough sites (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`).*
* 🛠️ **[`DIGADIAG.FRM` vs `PANA__SD.FRM`, SD `-91006` Gate &amp; Standalone TBC Feasibility](docs/en-US/DIGADIAG_PANA_SD_AND_STANDALONE_TBC.md)** ([Portuguese Mirror](docs/pt-BR/DIGADIAG_PANA_SD_E_TBC_AUTONOMO.md))
  *Stage-1 `/BOOT.BIN` (`SDDIAG`) vs Stage-2 `PROG061` container consumption, `-91006` `ACMD6` (`0x44027F79`) hardware proof, Digital PCB (`MN2DS0009` / `P59001`) Main PCB dependencies (`IC3001`/`IC7501`/`P7402`), and unified `panadiga unbrick` + Unbrick Center guide.*
* 💿 **[Matshita `R6-MV4` Emulator &amp; ATAPI Injection](docs/en-US/MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md)** ([Portuguese Mirror](docs/pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md)) · 🪤 **[MN103S Seven Traps &amp; Ghidra Resolution](docs/en-US/MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md)** ([Portuguese Mirror](docs/pt-BR/ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md)) · ✍️ **[`DIGADIAG` Jump `0x50400000` &amp; Guarded R/W](docs/en-US/DIGADIAG_JUMP_50400000_AND_GUARDED_RW.md)** ([Portuguese Mirror](docs/pt-BR/DIGADIAG_SALTO_50400000_E_ESCRITA_PROTEGIDA.md))
* 🔬 **[Forensic Bootloader Dossier: `SDDIAG`, `Loader ver0.61` (`COM10`) &amp; MN103S ABI](docs/en-US/FORENSIC_DOSSIER_BOOTLOADER_SDDIAG_EH55.md)** ([Portuguese Mirror](docs/pt-BR/DOSSIE_FORENSE_BOOTLOADER_SDDIAG_EH55.md))
  *Real hardware `Loader ver0.61` (`262,144 B` raw `DIGADIAG.FRM` + `sd48` `PANA__SD.FRM`) vs `/BOOT.BIN` `ver0.63`, live `COM10` `669 B`/`641 B` captures, 11 Ghidra Java FAT/SD scripts, SLEIGH Trap #8 (`fb 9c`), and `PANAMON1` compiler `CALL ..., [], 0` + `PUTC_D3_BYTES` fixes.*
* ⚡ **[Pico `P7501` C11 Upgrades, Ghidra `??` Root Causes &amp; Python-to-Rust Parity](docs/en-US/PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md)** ([Portuguese Mirror](docs/pt-BR/UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md))
  *Parameterized `DIAGWATCH` / `PANARSP REGS` / `DRVPANA`, rebuilt RP2040/RP2350/Unified UF2 hashes, exhaustive MN103S `??` breakdown across 5 Ghidra binaries (`docs/pseudocode/`), live bench captures (`docs/debug_logs/`), root audit (`COMPLETE_AUDIT.md`), historical audits (`docs/audits/`), and `panadiga re` + `panadiga pico decode-log` parity.*
* ⚡ **[Standby (`IC7501`), `TBUS`, `TW7501` &amp; Empirical Boot States](docs/en-US/STANDBY_IC7501_TBUS_TW7501_AND_BOOT_STATES_ARCHITECTURE.md)** ([Portuguese Mirror](docs/pt-BR/ARQUITETURA_STANDBY_IC7501_TBUS_TW7501_E_MODOS_BOOT.md))
  *Schematic power domains (`C7584`, `R7623`, `IC7502`, `IC7505`, `P59001`), `FUN_44005cd0` `255s` handshake, true `PLEASE WAIT -> U99` vs `HELLO -> U99` VFD states, `PREJIG WAKE` probe capture & 457-entry `error-lookup` / `bench-wizard` triage.*
* 🗂️ **Full Bilingual Directory (`44` EN + `44` PT-BR pairs + `audits/` + `pseudocode/` + `debug_logs/`):** See [**`docs/INDEX.md`**](docs/INDEX.md).
* 🏆 **Gold Standard for Forensic RE Architecture & Point-by-Point Gap Audit (`7` Dimensions, `P0..P3`):** [**`GOLD_STANDARD_FORENSIC_RE_ARCHITECTURE_AND_AUDIT.md`**](docs/en-US/GOLD_STANDARD_FORENSIC_RE_ARCHITECTURE_AND_AUDIT.md) ([PT-BR](docs/pt-BR/PADRAO_OURO_ARQUITETURA_FORENSE_RE_E_AUDITORIA.md))
* 📦 **Hardware Profiles v2 Audit (`89` Models, `T0` -> `T4`):** [**`DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md`**](docs/en-US/DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md) ([PT-BR](docs/pt-BR/CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md))
* 🔬 **EH55 Unbrick Breakthrough (BGO Cycle 2, Direct `PANAMON1>`, `DAVIS`/`CH1` & `PAL-M` Region):** [**`EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md`**](docs/en-US/EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md) ([PT-BR](docs/pt-BR/HISTORICO_UNBRICK_EH55_BGO_CH1_DAVIS_E_REGIAO.md))
* 🚀 **What's Next: End-to-End New Device Onboarding (`T0` -> `T4` & PR Checklist):** [**`WHATS_NEXT_NEW_DEVICE_ONBOARDING.md`**](docs/en-US/WHATS_NEXT_NEW_DEVICE_ONBOARDING.md) ([PT-BR](docs/pt-BR/WHATS_NEXT_ONBOARDING_NOVOS_APARELHOS.md))
* 🧭 **Anti-Trap BGO Geometry Verification & 100%-Disassembled Ghidra Forensic Atlas (`SSOT`):** [**`ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md`**](docs/en-US/ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md) ([PT-BR](docs/pt-BR/GUIA_ANTI_ARMADILHAS_GEOMETRIA_BGO_E_ATLAS_FORENSE_SSOT.md))
* 🔌 **Hardware Guide (`StarTech IDE2SAT2`, `P58001` & `Type B` FFC Case Fitment):** [**`HARDWARE_GUIDE_STARTECH_IDE2SAT2_AND_TYPE_B_FFC_CABLE.md`**](docs/en-US/HARDWARE_GUIDE_STARTECH_IDE2SAT2_AND_TYPE_B_FFC_CABLE.md) ([PT-BR](docs/pt-BR/GUIA_HARDWARE_STARTECH_IDE2SAT2_E_CABO_FFC_TYPE_B.md))
* 🛡️ **Technical Research (Validation Gates `0..4`, ~10h Root-Cause Audit & `8 MiB` BGO Retarget):** [**`RESEARCH_VALIDATION_GATES_SATA_STAGE2_AND_8MB_BGO_RETARGET.md`**](docs/en-US/RESEARCH_VALIDATION_GATES_SATA_STAGE2_AND_8MB_BGO_RETARGET.md) ([PT-BR](docs/pt-BR/PESQUISA_PORTOES_VALIDACAO_SATA_STAGE2_E_RETARGET_BGO_8MB.md))

---

## 12. Recommended Laboratory Equipment & Zero-Gadget Probe Architecture

* **Microcontroller Probe (Zero External Gadgets):** Raspberry Pi Pico (RP2040) or Raspberry Pi Pico 2 / Pico 2W (RP2350) running `~157–160 KiB` inside `P0_FW_RO` (`256 KiB`). No external OLED display, buzzer, or push-button hardware gadgets are attached to the probe — your Android smartphone (via **USB-C OTG** or **BLE NUS**) and Desktop Tauri 2 Workbench (`ExpressView` + `IrCommanderView`) provide the screen, audio cues, and IR Commander (`IRTX`/`IRRX`/`IR VERIFY`).
* **Mating Connector:** JST ZR female 6-pin socket, 1.5mm pitch (`B6B-ZR-3R4` compatible).
* **Passive Protection:** Optional $100\,\Omega$ $0.25\text{W}$ series current-limiting resistors (`GP2`, `GP4`, `GP5`), one $1.0\text{ k}\Omega$ ADC series isolation resistor (`GP26` / `SW_UART`). No external pull-up required (`VEP79138` integrates `R7570` / `R7572` `10 kΩ`).
* **Instrumentation:** Digital Multimeter (continuity testing), 2-channel 100 MHz Oscilloscope or 24 MHz USB Logic Analyzer.
* **Storage Forensics:** PATA/IDE to USB 3.0 bridge adapter with independent 12V/5V Molex power supply.

---

## 13. Total Quality, Test Pyramid & 9 Quality Gates

The Panadiga engineering ecosystem enforces a strict **9 Quality Gates** anti-regression pipeline ([`scripts/validate.py`](scripts/validate.py)):

```text
       Layer 5: End-to-End E2E Tests (Playwright Chromium — 27 tests / 9 specs)
       Layer 4: Desktop UI Unit & Component Tests (Vitest + React Testing Library — 128 tests / 39 files)
       Layer 3: C11 Pico Firmware Host Tests (GCC/MSVC Host Harness + ARM Check — 38 tests / 4 modules)
       Layer 2: Python Scripts & RE Pytest Verification (32 Pytest tests / 6 files + mypy --strict 46 files)
       Layer 1: Native Rust Workspace Tests (Cargo Test across 11 crates: constants/ir/crypto/bench/vault/profiles/core/cli/desktop/mcp/rag)
```

### 13.1. The 9 Active Quality Gates

1. **Rust Cargo Check & Test Gate:** Verifies formatting (`cargo fmt`), strict clippy (`-D warnings`), and workspace unit and integration tests across all 11 crates (`panadiga-constants`, `panadiga-ir`, `panadiga-crypto`, `panadiga-bench`, `panadiga-vault`, `panadiga-profiles`, `panadiga-core`, `panadiga-cli`, `panadiga-desktop`, `panadiga-mcp`, and `panadiga-rag`).
2. **TypeScript Strict Type-Check Gate:** `oxlint --deny-warnings` + `tsc --noEmit` verifies strict frontend types with zero `any` leaks.
3. **Frontend Vitest Unit Suite:** Covers ViewModels, Express Rescue pipelines, binary/doc link resolvers, React components, and algebraic state transitions across **128** tests in **39** files under `apps/desktop/src/**/*.test.{ts,tsx}`.
4. **Copy/Paste Detector (`jscpd`):** Enforces 0.00% code duplication threshold.
5. **UI/UX Completeness Linter (`uxlint`):** Audits **386** React files with score `100/100` (`0` issues) for loading, empty, error, accessibility, and form validation coverage.
6. **Playwright E2E Suite:** **27** browser-level functional tests across **9** specs (`calibration-and-tbc`, `express-mode`, `frm-packager`, `hdd-safe-flash`, `ir-commander`, `navigation`, `nvram`, `pico-bench`, `pico-bgo-davis`) running against headless Chromium.
7. **Ghidra Script Generator Gate:** Validates automatic export of memory maps and symbols from Rust SSOT (`tools/ghidra/*.py`, paired with `tools/ghidra/java/`).
8. **C11 Firmware Host Test Gate:** Compiles and executes **38** bare-metal Pico firmware unit test suites/functions across **4** modules (`test_gdb_rsp_host.c`, `test_pana_diag_drv.c`, `test_pana_logic.c`, `test_uart_codec.c`) with `-Wall -Wextra -Werror` / `/W4 /WX`, plus dual-target RP2040/RP2350 ARM Release compile verification.
9. **Python & Pytest Gate:** Verifies strict typing (`mypy --strict` across **46** source files) and **32** automated tests across **6** files in `scripts/tests/`.

### 13.2. Executing the Full Validation Suite

```bash
# Universal across Windows, Linux, and macOS:
npm run validate
# or directly via Python 3:
python scripts/validate.py
```

---

## 14. Artificial Intelligence Governance, MCP Server & Structured Logging

### 14.1. Dedicated Specialist Subagents (`.agents/agents/`)

The project configures 6 specialized subagents for autonomous engineering assistance:

* **`panadiga-architect`:** Cross-stack orchestration across Rust, C11, TypeScript, Python, and bench tooling.
* **`panadiga-hardware-embedded`:** Schematics, PCB layout, voltages, RP2040/RP2350 PIO C11 firmware, `P7501` Async UART (`115200 8N1`), and `NSW5V` Hi-Z signal integrity.
* **`panadiga-firmware-re`:** Matsushita AM34/MN103S firmware reverse engineering, Ghidra scripts, CPRM/C2 cryptanalysis, and entropy analysis.
* **`panadiga-storage-forensics`:** MEIHFS partition geometry, MBR `"SH"` repair, and filesystem forensics.
* **`panadiga-desktop-ui`:** Tauri 2.0 desktop/mobile frontend, React 18, TypeScript, Tailwind CSS, Vitest, and Playwright E2E.
* **`panadiga-qa-auditor`:** Strict 9 Quality Gates compliance, invariant verification, zero duplication (`jscpd`), and FMEA safety.

### 14.2. Native Rust Model Context Protocol (MCP) Server (`panadiga mcp`)

A standardized JSON-RPC 2.0 MCP stdio server (`panadiga mcp`) exposes **26 native tools** for AI agents (18 core tools + 8 Wave-3/Symbol `panamon1`/`eh55_bgo`/`davis_probe`/`tv_standard`/`tbc_appliance`/`eh55_flash`/`profiles`/`panadiga_symbol_lookup` modules):

* `panadiga_memory_map`: Returns the official physical memory map of the Matsushita AM34 / MN103S (`VEP79138`) architecture.
* `panadiga_safety_check`: Validates bench voltages (`<= 3.3V` LVTTL / `3.35V` ceiling) and memory write ranges.
* `panadiga_inspect_hdd`: Inspects partition boundaries and validates the MEIHFS `"SH"` MBR and mirror sectors.
* `panadiga_clean_nvram`: Analyzes or sanitizes the 64 KB NVRAM/EEMUL region (`0x807F0000..0x807FFFFF`) on 8 MB Flash dumps.
* `panadiga_search_manuals`: Searches the official technical manuals and schematics catalog under `manuals_and_schematics/`.
* `panadiga_rag_query`: Queries the native Rust 6-layer Zero-Error RAG engine with factual grounding.
* `panadiga_inspect_frm`: Inspects and decodes `.FRM` firmware containers (`DIGADIAG.FRM`, `PANA_DVD.FRM`, `PANA__SD.FRM`).
* `panadiga_read_hex`: Reads raw hex slices with ASCII dumps from firmware images or disk dumps.
* `panadiga_pico_catalog`: Lists Pico `P7501` firmware profiles, UF2 targets, and bench commands.
* `panadiga_ghidra_symbols`: Queries exported Ghidra symbols and disassembly annotations across `BOOT`, `MAIN`, and `DRV`.
* `panadiga_search_code`: Searches across the Panadiga Rust/C11/TypeScript workspace.
* `panadiga_search_visuals`: Searches hardware board renders, oscilloscope captures, and schematic diagrams.
* `panadiga_project_status`: Reports live workspace inventory, crate status, and bench readiness.
* `panadiga_hdd_capacity`: Inspects or dynamically patches LBA48 capacity descriptors on extracted HDD firmware/images.
* `panadiga_nor_dumper_stub`: Generates/validates the MN103S NOR flash dumper stub for 8 MB `P7501` capture.
* `panadiga_bgo_unbrick`: Inspects or generates `DIGADIAG.FRM` BGO unbrick patches targeting `PROG061` (`0x84200000..0x843FFFFF` / EH55 `Loader ver0.61` real BGO window `0x84130000..0x843FFFFF`).
* `panadiga_main_passthrough`: Inspects, patches, or generates `$M` GDB RSP plans for `MAIN` AV TBC `E-E` passthrough sites (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`).
* `panadiga_sd_unbrick_kit`: Unified unbrick diagnostics, Feistel `.FRM` extraction, full SD Kit generation (`DIGADIAG.FRM` + `PANA__SD.FRM` + `SHA256SUMS.txt`), and post-write verification.

### 14.3. Unified Logging Subsystem (`crates/panadiga-core/src/logger.rs`)

All components log exclusively through structured sinks:

* **Zero console.log/println:** No raw or unstructured logging is permitted in production code.
* **Log Levels:** `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR` with microsecond timestamps and subsystem tags.
* **Desktop GUI Streaming:** Real-time log events stream directly into the UI terminal panel.

---

## 15. Engineering Compliance & License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for complete terms.

All reverse engineering, cryptanalysis, and hardware documentation in this project were conducted strictly for interoperability, historical preservation, and repair of obsolete electronics in accordance with international reverse engineering exceptions.
