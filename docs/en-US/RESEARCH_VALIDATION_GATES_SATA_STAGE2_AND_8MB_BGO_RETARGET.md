> 🌐 **Language / Idioma:** **English (US)** · [Versão em Português (`pt-BR`)](../pt-BR/PESQUISA_PORTOES_VALIDACAO_SATA_STAGE2_E_RETARGET_BGO_8MB.md) · [Documentation Index](../INDEX.md)

# Technical Research: Validation Gates `0..4`, ~10h Root-Cause Investigation, `MAIN3060NP` `8 MiB` BGO Retarget & SSOT/RAG Hardening

> `[PROVEN ON REAL HARDWARE]` · `[EVD: EV-20261122-019..EV-20261123-029 + Mn103Vm + P7501 COM10]`

> **Canonical Naming Note (Deprecated $\rightarrow$ Current — Legacy Shims Removed):** Historical validation campaign nicknames in this report have been replaced across code, firmware, SSOT, and CLI/MCP (legacy aliases removed): **`Portão 4` $\rightarrow$ `NorAutoboot` / `NORFROM`** (`--nor-autoboot-capsule`, `BGO_INSPECT NORFROM`, tag `NAUT` / `0x5455414E` at `+0xFC`), **`Stage-1.5` $\rightarrow$ `DIAG_PROG_RAM`** (`0x50400000`, `digadiag_ram_tbc`), and **`Stage-2` $\rightarrow$ `HddAutoboot` / `MAIN_OS_RAM`** (`--sata-hdd-autoboot`, tag `[HDD_AUTOBOOT]`, key `hdd_autoboot=`, `0x57000000`).

This technical paper documents the systematic ~10-hour engineering investigation that achieved cold SATA HDD boot behind a `StarTech IDE2SAT2` bridge on a real **Panasonic DMR-EH55LB-S** (`8 MiB` Spansion `S29JL064H` NOR, `Loader ver0.61`, `VEP79138` board) with **zero physical writes to NOR `SA0..SA14`**. It details **Validation Gates `0..4`**, the full **`MAIN3060NP` `4 MiB -> 8 MiB` BGO retarget anatomy**, fail-closed regression hardening, **`MAIN3410PQ`** (`M6:PAL:DVD.HDD` European build with switchable `NTSC`/`PAL` + shared `AIOF` `PAL_M` driver symbols) support, and the **`Mn103Vm` + `NorAutoboot` `P4ZU` silicon proof (`Nor-FROM` dry-run)**.

---

## 1. ~10-Hour Investigation Chronology & Root-Cause Isolation

Bringing a `U99`-blocked DMR-EH55LB-S (`M6930LB`, `PAL-M`) equipped with a `250 GB` `SAMSUNG HM251JI` SATA HDD behind `StarTech IDE2SAT2` (`Marvell 88SA8052`, `MASTER`) up to full `PiE-OS` (`MAIN`) execution required peeling back three overlapping failure layers without risking the immutable boot sectors (`SA0..SA10`):

| Failure Layer | Observed Bench Symptom (`COM10` `P7501`) | Isolated Root Cause | Implemented Fix & Hardening |
| :--- | :--- | :--- | :--- |
| **Layer A — SATA Bridge Reset Race** | Stock `MAIN` lost drive readiness behind `StarTech IDE2SAT2` during ATA reset sequences. | `Marvell 88SA8052` takes **`~32 ms`** to assert `BSY` and **`~555 ms`** to assert `DRDY`, whereas stock firmware sleeps only **`~2 ms`**. | Deterministic `1000 ms` `SataBridgeResetFix` patch in [`sata_bridge_fix.rs`](../../crates/panadiga-core/src/re/sata_bridge_fix.rs) (`MAIN3060NP` & `MAIN3410PQ`). |
| **Layer B — Stage-2 Cold SATA Boot** | Gate 2 (SD boot) succeeded, yet the first Gate 3 cold HDD boot aborted inside `MAIN3060NP` initialization. | **`MAIN3060NP`** was compiled for **`4 MiB` Top-Boot** (`0x84200000..0x84400000`, `35` sectors, `P1=3`, `P2=28`, `max_did=0x0A1A`) and hit **`src/nfd_api.c:228`** (`[nfrom] BGO driver init failed[1].`) on the real **`8 MiB` `S29JL064H`** (`0x84130000..0x84400000`, `45` sectors, `P1=1`, `P2=40`, `max_did=0x0F24`). | Complete `4 MiB -> 8 MiB` binary retarget in [`main_bgo_retarget/mod.rs`](../../crates/panadiga-core/src/re/main_bgo_retarget/mod.rs) + deterministic **`Mn103Vm`** proof before hardware touch. |
| **Layer C — Packaging Regression Guard** | `compose` previously allowed `--sata-bridge-stage2-boot` to emit `PANA__SD.FRM` carrying un-retargeted `4 MiB` `MAIN3060NP` against an `8 MiB` target. | Missing fail-closed check binding target NOR capacity (`8 MiB`) to the embedded `MAIN` BGO geometry. | Mandatory **`--main-bgo-retarget`**, new **`CHK_ID_MAIN_BGO_GEOMETRY`** in `verify-sd --audit-profile hardened`, and declarative traps in `profiles/traps/`. |

---

## 2. The 5 Validation Gates (`Gate 0 .. Gate 4`)

| Gate | Scope & Containment Boundary | Proof / Passing Criteria | Status (`2026-11-23`) |
| :--- | :--- | :--- | :--- |
| **Gate 0 — Static Hardening, `Mn103Vm` & SSOT** | No SD image reaches bench testing without `verify-sd --audit-profile hardened` (`CHK_ID_MAIN_BGO_GEOMETRY`, `0` fails) and `Mn103Vm` emulation proof. | [`main_bgo_retarget/mod.rs`](../../crates/panadiga-core/src/re/main_bgo_retarget/mod.rs), [`validate_bgo.rs`](../../crates/panadiga-profiles/src/validate_bgo.rs), and [`geometry_and_binary_traps.json`](../../profiles/traps/geometry_and_binary_traps.json). | **PASSED** |
| **Gate 1 — Electrical & ATA Integrity (`P58001` / `P1503`)** | Verified `1:1` continuity before AC power, `IDE2SAT2` jumper set to **`MASTER`**, absence of **`0x00002004`** mirrored cable fault. | Documented in [`HARDWARE_GUIDE_STARTECH_IDE2SAT2_AND_TYPE_B_FFC_CABLE.md`](HARDWARE_GUIDE_STARTECH_IDE2SAT2_AND_TYPE_B_FFC_CABLE.md). | **PASSED (REAL HW)** |
| **Gate 2 — SD Boot with Live SATA Bridge (No Flash/HDD Writes)** | Proves real `Loader ver0.61` enumerates `SAMSUNG HM251JI` behind `StarTech IDE2SAT2` without touching `SA0..SA14` or writing MEIHFS mirrors. | [`boottrace_sata_bridge_gate2_20261122.log`](../evidence/boottrace_sata_bridge_gate2_20261122.log), [`boottrace_sata_bridge_nosd_only_20261122.log`](../evidence/boottrace_sata_bridge_nosd_only_20261122.log), [`boottrace_sata_bridge_nosd_warm_20261122.log`](../evidence/boottrace_sata_bridge_nosd_warm_20261122.log). | **PASSED (REAL HW)** |
| **Gate 3 — Stage-2 in MEIHFS Mirrors + Cold SATA HDD Boot** | **`FVU)`** writes Stage-2 + retargeted `MAIN3060NP` (`8 MiB` BGO) into both HDD mirrors; Boot 2 cold-boots **`PiE-OS 3060NP`** straight from the SATA drive. | **Boot 1:** [`boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log) (`check_sum = 0x32836a32`, `Vefify OK` × 2)<br>**Boot 2:** [`boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log) (`39,652 B` `PiE-OS 3060NP`). | **PASSED LIVE (`2026-11-23`)** |
| **Gate 4 — No-SD Boot via `Loader ver0.61` `Nor-FROM` (`SA11..SA14`)** | Load Stage-2 directly from **`SA11..SA14` (`0x84040000..0x8407FFFF`)** when `"fvu.exe"` exists in BGO, keeping **`SA0..SA10` strictly RO**. | Full `Mn103Vm` + `SpansionNorFlash` proof in [`sata_hdd_autoboot/mod.rs`](../../crates/panadiga-core/src/re/sata_hdd_autoboot/mod.rs). **Zero physical NOR writes performed.** | **PROVEN IN EMULATOR (`Mn103Vm`)** |

---

## 3. Gate 3: From `src/nfd_api.c:228` Failure to Live Hardware Pass (`2026-11-23`)

### 3.1 Initial Gate 3 failure (`4 MiB` vs `8 MiB`) on real bench hardware
During the initial Gate 3 attempt ([`boottrace_sata_bridge_gate3_boot1_20261122.log`](../evidence/boottrace_sata_bridge_gate3_boot1_20261122.log) & [`boottrace_sata_bridge_gate3_boot2_20261122.log`](../evidence/boottrace_sata_bridge_gate3_boot2_20261122.log)), `MAIN3060NP` had not yet received `--main-bgo-retarget`. Executing on the real `8 MiB` `S29JL064H` chassis caused immediate abort at:
- **`[nfrom] BGO driver init failed[1].` (`src/nfd_api.c:228`)**

### 3.2 Complete `MAIN3060NP` (`4 MiB -> 8 MiB` BGO) retarget anatomy
In [`main_bgo_retarget/mod.rs`](../../crates/panadiga-core/src/re/main_bgo_retarget/mod.rs), `MAIN3060NP` (`13,427,852 B`) is cleanly retargeted with exact pre/post byte checks:
- **Reserved BSS tail expansion (`NFD`/`BGO` control structs):** expanded from **`0x1CC4`** to **`0x26C4`** bytes above `_end` (**`0x531B7774 -> 0x531BC000`**, active relocated window **`0x531B8000..0x531BC000`**).
- **Code cave & trampolines:** **`165 B`** code cave at **`VA 0x57000320`**, reached via **`12` trampolines**, patching **`7` `NFD_API` routines/call sites** and **`48` `nfd_ctrl` struct offsets**.
- **FTL / geometry parameters:** BGO base shifted from **`0x84200000`** (`35` sectors, `P1=3`, `P2=28`, `max_did=0x0A1A`) to **`0x84130000`** (`45` physical sectors `SA26..SA70`, `P1=1`, `P2=40`, `max_did=0x0F24`), including table/constant **`0x2202`**.
- **Pre-hardware `Mn103Vm` verification:** deterministic emulation executes **`NFD_Read(17/22/12)`** cleanly with zero `nfd_api.c:228` failure.

### 3.3 Live Gate 3 Pass (`2026-11-23`)
Rebuilding `firmware_and_images/sd_unbrick_kit_sata_bridge_stage2` with `--loader-profile ver061 --bgo-mode uart_monitor --passthrough-mode factory_original --target-gb 250 --sata-bridge-reset-fix --sata-bridge-stage2-boot --main-bgo-retarget --expected-main-build 3060NP` and passing `verify-sd --audit-profile hardened` (`main_bgo_geometry=BGO_45SEC_8MB_0X84130000`, `0` fails) yielded full bench execution:

1. **Boot 1 (`FVU)` — Mirror flash onto `SAMSUNG HM251JI`):**
   - `WAITTX 25000 NORMAL "FVU)..." 4646 30000` via `COM10` → [`boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log).
   - Observed result: **`triggered=YES`**, **`size=0x645028`**, **`check_sum = 0x32836a32`**, **`Vefify OK` × 2** at **`0x00010000-0x00013228`** and **`0x1D1A5970-0x1D1A8B98`**.
2. **Boot 2 (`BOOTTRACE 60000 NORMAL` — Direct cold SATA boot):**
   - Full capture: [`boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log) (`55,917 B`).
   - Stage-2 decompression & handoff: **`HEADER:81132B55, DECOMP:81132B55`**, **`[STAGE2] Boot MAIN @ 0x57000000`**.
   - Main OS telemetry: **`39,652` bytes** of full **`PiE-OS / 3060NP`**, `[nfrom] DeviceInfo(EEPROM : 8Kbit)`, `M6930LB`, **`SAMSUNG HM251JI`**, `AVOUT_V_Mute: onoff=0`, `SYS_CNTL_SetSysState[EE]`, `FactoryMode = Start`, `Media Confirm = OK`, **zero `BGO driver init failed`**, **zero `U99`**.
   - **Front-panel state observed:** `HDD` LED solid, `DVD` LED lit then turned off, `SD` LED solid, VFD displaying **`TM LI`** — the normal factory-mode state under `WakeupReason:[FACT(18)]` & `FactoryMode = Start` when the HDD still has an empty HDFS superblock (`Hdd illegal (HDD ID mismatched)`).

---

### 3.4 Why **Boot 1** (`FVU` Installer) and **Boot 2** (`[HDD_AUTOBOOT]` Runtime) Are Separate

A frequent question when operating the SATA bridge recovery kit (`StarTech IDE2SAT2`) is why initial deployment is split into **two boots**:

| Phase | Architectural Role | What the SD Card Does | When It Happens |
| :--- | :--- | :--- | :--- |
| **Boot 1 — One-Time Flash (`Fvu ver0.61` $\rightarrow$ `SD version up mode.`)** | The main OS (`MAIN`, `~13 MiB` unpacked) **does not fit inside the `8 MiB` NOR** and does not execute in-place from SD — it resides inside the **two MEIHFS firmware partitions on the HDD** (`0x00010000` and outer mirror `0x1D1A5970`). | Loads the `FVU` updater into SDRAM, reads the `MAIN` payload inside `PANA__SD.FRM` (`File ID[0xC8] seek 0xF1800`), and flashes/verifies (`Vefify OK` × 2) both HDD mirrors using the `1000 ms` SATA bridge reset wait. | **Once only**, whenever installing or switching `MAIN` builds/patches (or explicitly triggered via UART `'F'` or `FORCEFVU.TXT`). |
| **Boot 2+ — Normal Everyday Startup (`[HDD_AUTOBOOT]`)** | Because factory IPL **`Loader ver0.61` (`SA0..SA10` RO)** only waits **`2 ms`** after ATA `RESET-` — while the **`Marvell 88SA8052`** inside `StarTech IDE2SAT2` needs **`~555 ms`** to assert `DRDY` — cold NOR boot fails with `HDD_MAX_LBA get failed[0xE8000000]` and falls back to SD. | **`PROG061`** inside `PANA__SD.FRM` runs the pre-FVU **`[HDD_AUTOBOOT]`** hook (`0x504050D3 -> 0x50439600`) with **`1000 ms`** SATA bridge wait **without modifying HDD mirrors**: reads `MAIN` straight from `0x00010000` (or `max_lba - 0x20000`), decompresses to `0x57000000`, and boots the recorder. | **Every normal power-on** once Boot 1 has populated the HDD (**1 single boot** per power cycle). |

> [!NOTE]
> Kits built **without** `--sata-hdd-autoboot` always fall straight into `SD version up mode.` (useful when forcing a clean Boot 1 flash). Once rebuilt with **`--sata-hdd-autoboot`**, a **single SD card** handles both roles permanently: it boots `MAIN` straight from the HDD on every normal power-on (**Boot 2+**) and only enters `FVU` if the operator holds `'F'` on UART or puts `FORCEFVU.TXT` on the SD root.

---

## 4. `MAIN3410PQ` (`M6:PAL:DVD.HDD` with `NTSC`/`PAL` Switch) Extension & SSOT/RAG Hardening

### 4.1 `MAIN3410PQ` (`13,003,052 B`) support
Extracted from the European reference HDD firmware partition (`hdd_partition_firmware_0x02000000.bin` `6,173,264 B` → `eh55_main3410pq_unpacked.bin` `13,003,052 B`, SHA-256 `c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1` — European `M6:PAL:DVD.HDD` build supporting runtime `PAL/NTSC` switching plus shared `AIOF` `PAL_M` driver symbols):
1. **Native `8 MiB` BGO geometry:** `MAIN3410PQ` already ships natively with **`0x84130000`** at **`VA 0x5756EFD4`** (verified via `verify_main3410pq_native_8mb_bgo`), integrated into `compose` and `verify-sd`.
2. **`1000 ms` SATA bridge reset fix:** following an exact `-0x78E2` shift in `RTSD`, **`VA 0x5710A9D0`** (`OFF 0x0010A9D0`, original `[2C D0 07 89 DD A7 C9 FF FF 00 00]`) is patched to `CALL 0x574DF538` (`rtsd_sleep_10ms`), cross-checked against all 4 sibling callers (`0x5710A452`, `0x5710B086`, `0x5710B5AF`, `0x5710C7E5`) and proven by `mn103vm_proves_main3410pq_and_main3060np_sata_reset_wait_1000ms` in [`sata_bridge_fix.rs`](../../crates/panadiga-core/src/re/sata_bridge_fix.rs).

### 4.2 Anti-regression hardening promoted to SSOT, Symbols & RAG
- **Declarative Trap Catalog ([`profiles/traps/geometry_and_binary_traps.json`](../../profiles/traps/geometry_and_binary_traps.json)):** now holds **`11` cataloged traps**, promoting `TRAP_MAIN3060NP_4MB_BGO_ON_8MB_EH55`, `TRAP_SATA_BRIDGE_2MS_POST_RESET_RACE`, and `TRAP_NORFROM_FVU_BGO_MARKER_VER061`.
- **Curated Symbol Dictionaries (`profiles/symbols/loader_ver061_toc48_main57/`):** [`LOADER_VER061.json`](../../profiles/symbols/loader_ver061_toc48_main57/LOADER_VER061.json), [`MAIN3060NP.json`](../../profiles/symbols/loader_ver061_toc48_main57/MAIN3060NP.json), and [`MAIN3410PQ.json`](../../profiles/symbols/loader_ver061_toc48_main57/MAIN3410PQ.json) served via `panadiga://symbols/<train_id>/<module>`.
- **Direct Low-Latency RAG Indexing ([`crates/panadiga-rag/src/indexer/ssot_atlas.rs`](../../crates/panadiga-rag/src/indexer/ssot_atlas.rs)):** indexes `profiles/traps/` and `profiles/forensic_atlas/` directly for fast, low-token RAG/MCP queries (`panadiga mcp`).

---

## 5. Gate 4: `Mn103Vm` Emulator Proof of `Loader ver0.61` `Nor-FROM` Boot

Test **`mn103vm_proves_nor_autoboot_loader_ver061_norfrom_boot_and_escape_hatch`** in [`sata_hdd_autoboot/mod.rs`](../../crates/panadiga-core/src/re/sata_hdd_autoboot/mod.rs) executes real **`Loader ver0.61`** machine code (`eh55_loader_ver061_256k.bin`) against a **Copy-on-Write** `8 MiB` NOR dump (`eh55_nor_8mb_ic5001_repaired.bin`) inside **`Mn103Vm` + `SpansionNorFlash`**, leaving **`SA0..SA10` byte-identical**:

1. **Exact `FUN_4400894B` (`0x4400894B..0x44008B98`) ABI flow:**
   - Initializes `loaded_from_nor` (`-12(A3)`) to `0`, then calls **`FUN_4400A1F1`** (`NFD_FileExists` — exact entry **`0x4400A1F1`**, not `0xA1F2`) passing `.data` string **`0x504800AF`** (`"fvu.exe\0"`).
   - **Negative path (`D0 != 0`, `"fvu.exe"` absent):** branches to **`0x440089F5`** (`Update-Disk` via SD/DVD). Confirmed that real `eh55_nor_8mb_ic5001_repaired.bin[0x40000..0x80000]` is `100% 0xFF` and BGO contains no `"fvu.exe"`.
   - **Positive path (`D0 == 0`, `"fvu.exe"` present in BGO):** logs `"Read fvu program from Nor-FROM to 0x%x.\n"`, calls `memcpy(0x90400000, 0x84040000, 0x40000)` (`0x4402A34A`) + `memcmp(...)` (`0x4402A30A`), sets `-12(A3) = 1`, and jumps to **`0x44008B98`**, loading **`SA11..SA14` (`0x84040000..0x8407FFFF`)** with exact SHA-256 verification and chaining into Stage-2 HDD boot without modifying immutable **`SA0..SA10`**.
2. **Cleanup caveat (`0x44009BED`) & 3rd escape hatch:**
   - When `startup reason = [70]`, `0x44008914` invokes `0x44009BED` (`cleanup_bgo_temp_files`), leaving virgin `SA11..SA14` (`0x40000..0x80000`) untouched. The emulation suite also verifies the **3rd escape hatch** (`PANA__SD.FRM` recovery fallback).

### 5.1 Live Stage-1 Dry-Run Proof (`P4ZU` via SD) & `config1` / `IC7501` Forensic Audit (`2026-11-23`)
- **Live Gate 4 Stage-1 hardware proof ([`com10_portao4_etapa1_sd_dryrun_20261123.log`](../evidence/com10_portao4_etapa1_sd_dryrun_20261123.log), `EV-20261123-025`):** with **zero NOR writes**, the 256 B SDRAM `P4ZU` self-unpacker + `Loader ver0.61` `zlib 1.2.2` + `0x91040000` trampoline unpacked and ran `Fvu ver0.61` (`check-sum[0x316E619]`) and booted full `MAIN 3410PQ` from `SAMSUNG HM251JI` behind `IDE2SAT2` (`HEADER:0094407E`, `91,312 B` UART, `overflow=NO`).
- **`config1` (`ID 0x0117`) audit & BGO regression postmortem (`EV-20261123-026`):** in table `0x57AED0A8` (RAM base `0x53184B44` at `0x5751702A`), **`ID 0x0117` (TV System)** resides at **`+0x12F0` (`page 9 + 0x00F0`)**, with machine-code polarity **`0x00 = TV_NTSC` (`480i 60 Hz`)** and **`0x01 = TV_PAL` (`576i 50 Hz`)**. While Phase 1 of [`bgo_heal_nfrom_ntsc_20261123.log`](../evidence/bgo_heal_nfrom_ntsc_20261123.log) closed all FTL tag holes (eliminating `bgo_writeD(ID=0xF01)` failures), Phase 2 wrote `0x01` to `PID 0x000C` (Page 0, `+0x00EC` = **`ID 0x00C6`** `0xFF -> 0x01`), leaving real Page 9 untouched in `TV_PAL`. Additionally, `PID 0x01AC` (`fs4.dat` page 131) maps via `P1 L2P` to `SA55 (F2_02) slot 18` whose tag is `0x0000` (`bgo_read_D(ID=0x1AC) return error(2)`), and Brazilian `IC7501` (`MN67788RA`) rejects European `3410PQ` TBUS opcodes (`0xA9`, `0xC9`, `0xE3`, `0xE4`, `0xF9`, `0xFA`) with `0x7F`.

### 5.2 Live Bloco A BGO CoW Surgery (Zero Erases, `TV_NTSC` + `config1[0]` & `PID 0x01AC` Restoration — `EV-20261123-027`)

- **Physical pre-flight & FTL health (`COM10`):** `VDD=3.26V`, `NSW5V=1`, and `BGO_INSPECT` healthy (`sectors=45`, `erased=4`, `valid=41`, `corrupt=0`, `dup_lbn=0`, `zmarker_bad=0`, `healthy=YES`), with `0` intermediate `0xFFFF` holes across all 40 `F2` sectors. A pre-surgery backup (`178,176 B`, SHA-256 `5A6BDC8C5AEBB4EB0FB80AEA549DCFD8CEDCC102528011B2ADBD40B15767B099`) was saved prior to any write.
- **Dynamic live FAT walk (`0` sector erases):** live traversal resolved `P1=SA39` and open block `open=SA37` (`active_l2p=1037`, `blank_sps_total=22`), writing exclusively into the next virgin slots of `SA37`:
  1. **`config1[0]` (rotated at `PID 0x0044`, `SA48 slot 83`):** exact diff against backup showed only `+0x0EC: bak=0xFF live=0x01` — restored to `+0x0EC=0xFF` via CoW at `SA37 slot 32` (`L2P tid=0x0004 (sp=232,s=1)->(sp=234,s=0, word=0x0240)`).
  2. **`config1[9]` (rotated at `PID 0x003E`, `SA60 slot 60`, `+0x0F0=01 00 00 00`):** written `00 00 00 00` (`TV_NTSC`, 480i 60 Hz) via CoW at `SA37 slot 33` (`L2P tid=0x0003 (sp=202,s=4)->(sp=235,s=0, word=0x0242)`).
  3. **`fs4.dat[131]` (`PID 0x01AC`, located at `SA37 slot 18 tag=0x0000`):** restored from the clean payload (`SHA-256 B58B55FC010B651869BE74EA35FF6B5D7E413F4FA0F38DDF91C767391AA6271F`, matching `eh55_nor_8mb_ic5001_repaired.bin` `SA53 slot 112`) via CoW at `SA37 slot 34` (`L2P tid=0x001A (sp=178,s=3)->(sp=236,s=0, word=0x0244)`).
- **Post-write readback (before reboot):** `BGO_INSPECT healthy=YES`, `corrupt=0`, `dup_lbn=0`, `zmarker_bad=0`, `SA37 holes=0`, and all 3 tags, payloads, and hashes verified 100% intact in [`bgo_bloco_a_surgery_ntsc_pid0c_pid1ac_20261123.log`](../evidence/bgo_bloco_a_surgery_ntsc_pid0c_pid1ac_20261123.log).
- **`BOOTTRACE 75000` window & post-boot telemetry:** on the subsequent boot ([`boottrace_bloco_a_ntsc_cured_20261123.log`](../evidence/boottrace_bloco_a_ntsc_cured_20261123.log)), the `PANAMON1>` `[41]` double-reset delayed `[STAGE2] Boot MAIN @ 0x57000000` until `+49019ms`; the 75 s capture (`bytes=19369` UART at `75021956us`) closed at `+51263ms` (`System assert!! File:src/fpcmain.c Line:963 Err:0`) during the `~29.6s` `FPC RCV TIMEOUT GATE=15` wait, before MAIN printed `AVOUT_init`/`avout_tv_system`/`0x1AC`. Immediately afterward, `POST_STATUS` (`vdd=3.27V nsw5v=1 drops=41`) and passive `pico_heartbeat_ops.py` (`ONLINE_HEALTHY`, `vdd_mv=3258`, `nsw5v=1`, `drops=41`, `POWERED / AWAKE`, `wdt_recoveries=0`, `reboot_src=CLEAN`) verified clean boot completion with the unit staying awake.

### 5.3 Live Bloco B CoW Surgery (`TV_NTSC` Cured + `PID 0x01C1`) & Isolation of `L1489` (`EV-20261123-028`)

- **Live Bloco B BGO CoW surgery over `COM10` ([`bgo_bloco_b_surgery_ntsc_pid1c1_20261123.log`](../evidence/bgo_bloco_b_surgery_ntsc_pid1c1_20261123.log)):** with a `177,664 B` pre-surgery backup saved outside the repo (`pre_bloco_b_bgo_surgery_20261123.bin`, SHA-256 `7576FF4AC4C41C0E0A00AFB4051DB8D19770F8F01CC7E9C9896965B4C26ECF43`), live traversal resolved `P1=SA41` (`sp=167`) and `open=SA29`:
  1. `config1[9]` (`PID 0x003E`, rotated in `SA37 slot 33 -> SA29 slot 58`): wrote `+0x0EC=0x01` and `+0x0F0=01000000` (`TV_NTSC`, `L2P tid=0x0003 (sp=163,s=0)->(sp=167,s=3)`).
  2. `fs4.dat[132]` (`PID 0x01C1`, previously `0xFF` in `SA37 slot 19` shared with `0x0F20`): restored clean payload (`SHA-256 E0B56378B146CF67A03486CED4681F7A4EC30A74112D950717E67E1CDFC5A522`) into `SA29 slot 59` while preserving `SA37 slot 19` (`tag=0x0F20`), yielding `0` L2P mismatches, `SA29 holes=0`, and `healthy=YES`.
- **Subsequent live `MAIN 3410PQ` boot trace ([`boottrace_bloco_b_ntsc_cured_20261123.log`](../evidence/boottrace_bloco_b_ntsc_cured_20261123.log)):** confirmed `AVOUT_init() success.`, `avin_set_now_state() sys tvsystem = TV_NTSC(0).`, `avout_tv_system = 0`, `AVOUT_V_Mute: onoff=0`, and front-panel VFD **`DV`**, while live BGO writes hit **line 1489**:
  - `[nfrom] Status flag change error[1].` $\rightarrow$ `Status change error[-3].` $\rightarrow$ `File rewrite end error[-3].` $\rightarrow$ `Rewrite error[-3].`
  - followed by `11x` `[nfrom] bgo_writeD(ID=0xF07/0xF01) return error(19).` (latched error lock `0x530FD6A4 == 1`) and `[SYS_CNTL]##### NFROMError Power off !!!!! #####`.

### 5.4 Exhaustive 8 MiB IPL Diff (`S12/S13A/S13B/S14`) & `Mn103Vm` Rollover Proof of Rebuilt Kit (`EV-20261123-029`)

1. **Complete static root cause (exact 4 remaining gaps vs native `eh55_nor_8mb_ic5001_repaired.bin` IPL `@ 0x8401_E800..0x8402_6500`):** earlier SD kits (`sd_unbrick_kit_sata_bridge_stage2_3410pq` and `portao4_sd_dryrun_3410pq`) carried only `S1..S11` (`165 B`). Exhaustive byte diff across the driver and all 32-bit RAM literals proved zero missed sites outside `0x574A_6000..0x574B_4500` and isolated 4 final patches in [`main_bgo_retarget/mod.rs`](../../crates/panadiga-core/src/re/main_bgo_retarget/mod.rs):
   - **`S12` (`MAIN3060NP: 0x574B44C0` / `MAIN3410PQ: 0x574ACA0A`):** `P1` compaction `sp` ceiling `cmp 0x1F -> 0xFF, d0` via cave trampoline `0x570003C5`.
   - **`S13A` (`MAIN3060NP: 0x574AFBB8..FBBA` / `MAIN3410PQ: 0x574A8102..8104`):** `mov 0x2000, a3` replaced inline with `3x NOP` (`CB CB CB`) so `P1` (`param_2 == 1`) spare-sector allocation keeps `a3 = 0x10000` (matching native IPL `@ 0x840202AD`); without `S13A`, every `P1` rollover fails with `error=1`, latches the lock bit, and triggers `bgo_writeD error(19)`.
   - **`S13B` (`MAIN3060NP: 0x574AFC18` / `MAIN3410PQ: 0x574A8162`):** `mov 8, d2` adjusted to `mov 4, d2` (`0x08 -> 0x04`), bounding the 4-entry `P5` reserve `memmove` and protecting `P5[3]` (`sec_map_plus1_va + (45 + 3) * 2`).
   - **`S14` (`MAIN3060NP: 0x574B3E59` / `MAIN3410PQ: 0x574AC3A3`):** `P1` format `sp` ceiling `cmp 0x20 -> 0x100, r5` via cave trampoline `0x570003D7` (expanding the SDRAM stub cave to **`201 B`** at `0x57000320..0x570003E9`, leaving `23 B` zero padding through `0x57000400`).
2. **Deterministic `Mn103Vm` verification (`MAIN3060NP` & `MAIN3410PQ`):**
   - **Negative control (`no-S13A`):** failed at `iter 0` (`sp=0xFF -> 0x100`) with `rc = -3`, compaction error `[1]`, `File rewrite end error[-3]`, `lock = 1`, `bgo_writeD(0xF07) error(19)`, and `rc = -2` on the subsequent write.
   - **Positive proof (`S1..S14 + S13A/S13B`):** passed `Part A` (`8` `NFD_Write(22)` writes advancing `sp > 0x1F`) and `Part B` (`10` writes crossing `sp=0xFF -> 0x100`, `P5[3] == 0xFFFF`, `P5` spare allocated & programmed, idempotent BGO remount settlement, and `NvramOnly` readback) across both `3060NP` and `3410PQ` as well as Gate 4 `cow_nor`.
3. **Rebuilt `MAIN3410PQ` Stage-2 SD kit VALIDATED ON REAL SILICON (`EV-20261123-030`):** generated with `--main-bgo-retarget --sata-bridge-reset-fix --sata-bridge-stage2-boot` and passed `verify-sd --audit-profile hardened` with `0` fails (`PANA__SD.FRM` `7,181,464 B`, SHA-256 `2dc075f2c01fbb8e798d450eac806ed0db6113f2b93c1be31bef626a99a889de`; extracted `MAIN3410PQ` `13,003,052 B`, SHA-256 `cda7b2cb960d878b40ae64d768b0138aa855e35f258d67d1a8befc47156f0c3a`). On live `COM10` bench (`N4_HARDWARE`):
   - **Boot 1 (`WAITTX_FVU`, [`boottrace_sata_bridge_gate3_3410pq_s14_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_3410pq_s14_boot1_fvu_20261123.log)):** `triggered=YES`, `fvu program check-sum[0x21647BA]`, `File ID[0xC8] seek 0xF1800 size=0x5E7C98`, **`check_sum = 0x2f82c81a`**, `Vefify OK` × 2 (`0x00010000` and `0x1D1A5970`), and `success to update mainsoft.`.
   - **Boot 2 cold boot from `SAMSUNG HM251JI` (`BOOTTRACE 90000 NORMAL`, [`boottrace_sata_bridge_gate3_3410pq_s14_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_3410pq_s14_boot2_main_20261123.log)):** `134,268 B` (`95,399 B` UART, `1,935` lines, `ovf=NO`, `ferr=0`, `perr=0`, `CtrlEdges: NSW5V=0`); `[STAGE2] Boot MAIN @ 0x57000000` ran `MAIN 3410PQ` with `[SYS_BOOT]AVOUT_init`, `TV_NTSC(0)`, `avout_tv_system = 0`, `AVOUT_V_Mute: onoff=0`, completed **`:::SYS_RSV::: >> nfrom back up ok.` (L1690)** and **`:::SYS_RSV::: >> sys_rsv init success.` (L1692)**, mounted `startup: HDD opened`, transitioned to `SYS_CNTL_POW_ON`, `### HDD WAKEUP ###`, `sys_navi` / `sys_media READY`, `SYS_CNTL_SetSysState[EE]`, and `setgui end. tsk=106` with **`0` occurrences** of `Status flag change error`, `File omap update error`, `File rewrite end error`, `bgo_writeD error(19)`, `NFROMError Power off`, or `U99`, remaining awake in post-boot `ONLINE_HEALTHY` (`VDD=3.26V`, `NSW5V=1`, `NSW5V_DROPS=74` unchanged).

---

## 6. Verified SHA-256 Artifact & Capture Table

| Artifact / Log | Size (Bytes) | Verified SHA-256 | Role / Gate |
| :--- | :---: | :--- | :--- |
| [`boottrace_sata_bridge_gate2_20261122.log`](../evidence/boottrace_sata_bridge_gate2_20261122.log) | `7,536` | `f8c1b5699986334f1a97ef004319eaeef880b3447dc4d80d04c1db82c92d6865` | Gate 2 real-HW pass (`IDE2SAT2`) |
| [`boottrace_sata_bridge_gate3_boot1_20261122.log`](../evidence/boottrace_sata_bridge_gate3_boot1_20261122.log) | `7,557` | `4be424ba7e96859b824537e4f6940a777d4d3540c75fa5a7753211df54c60748` | Gate 3 attempt 1 (`src/nfd_api.c:228`) |
| [`boottrace_sata_bridge_gate3_boot2_20261122.log`](../evidence/boottrace_sata_bridge_gate3_boot2_20261122.log) | `7,402` | `4d35e08ed4ca9a0cb5e7c7e8c9846494e8966c46831bc57745a473e0935c665e` | Gate 3 attempt 2 (pre-retarget boot) |
| [`boottrace_sata_bridge_nosd_only_20261122.log`](../evidence/boottrace_sata_bridge_nosd_only_20261122.log) | `9,021` | `3120123e29115694e99b51eb1cf120daeb346683b36fc5ad5091026b2a34ac2e` | No-SD SATA `IDE2SAT2` diagnostic trace |
| [`boottrace_sata_bridge_nosd_warm_20261122.log`](../evidence/boottrace_sata_bridge_nosd_warm_20261122.log) | `7,536` | `15abe196a8a6a1611907b6ee25f020f47a352f025dcf7b8e8d377d6b0513faab` | No-SD warm boot trace |
| [`boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log) | `4,906` | `455a86083636cf4bcc2c49d33ffd55be1391e6e9c12edb7ebd15791dbdacd2d3` | Gate 3 Boot 1 `FVU)` (`0x32836a32`, `Vefify OK` × 2) |
| [`boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log) | `55,917` | `acbee64c1710e015dc32f707bff4acd3ce25cb1224575ab38753a7b3ff95595a` | **Gate 3 Boot 2 LIVE PASS (`PiE-OS 3060NP`, `TM LI`)** |
| [`com10_portao4_etapa1_sd_dryrun_20261123.log`](../evidence/com10_portao4_etapa1_sd_dryrun_20261123.log) | `91,489` | `87707afa1e4a63e18eabc62b2008cf7d526e2efb2284fc8317d77bc3bb82b2e7` | **Gate 4 Stage-1 LIVE PASS (`P4ZU` `0x316E619` -> `MAIN3410PQ`, `91,312 B` UART, `0` NOR writes)** |
| [`bgo_heal_nfrom_ntsc_20261123.log`](../evidence/bgo_heal_nfrom_ntsc_20261123.log) | `76,081` | `52cd7675528e1b294f6b6098717e2a2428a05b1f89d175042bfad025dcbf2985` | BGO Phase-1 tag-hole cure + `PID 0x000C` postmortem trace |
| [`bgo_bloco_a_surgery_ntsc_pid0c_pid1ac_20261123.log`](../evidence/bgo_bloco_a_surgery_ntsc_pid0c_pid1ac_20261123.log) | `12,169` | `df864ea9fac42e016f3f2b532a6e26f304a10769d05d34b1e9308d39f62a98d2` | Live Bloco A BGO CoW surgery (0 erases): `config1[0]` (`PID 0x0044`), `config1[9]` `TV_NTSC` (`PID 0x003E`), and `fs4.dat[131]` (`PID 0x01AC`) at `SA37` slots `32..34` + readback (`EV-20261123-027`) |
| [`boottrace_bloco_a_ntsc_cured_20261123.log`](../evidence/boottrace_bloco_a_ntsc_cured_20261123.log) | `27,588` | `113eb37723f551446ecef41539261857a19f33ed4c080de59225406d3dc1dfca` | Post-Bloco-A `BOOTTRACE 75000` (`+51263ms` in `FPC RCV TIMEOUT` after `[41]` double-reset; clean boot confirmed by `ONLINE_HEALTHY` heartbeat) |
| [`bgo_bloco_b_surgery_ntsc_pid1c1_20261123.log`](../evidence/bgo_bloco_b_surgery_ntsc_pid1c1_20261123.log) | `10,745` | `a4347281362152ef9f32c512e287879a9b5d42be1793234f5940b288b44373d8` | Live Bloco B BGO CoW surgery (`SA29` slots `58..59`: `config1[9]` `TV_NTSC` + `fs4.dat[132]` `PID 0x01C1`, `0` mismatches, `EV-20261123-028`) |
| [`boottrace_bloco_b_ntsc_cured_20261123.log`](../evidence/boottrace_bloco_b_ntsc_cured_20261123.log) | `92,895` | `d5497da7f15858f42d72c26c9ae5e8ac2f8bc5493e81fcb9a8718aaeb0a77685` | Post-Bloco-B live boot (`MAIN 3410PQ`, `TV_NTSC(0)`, `AVOUT_V_Mute: onoff=0`, `VFD DV`, captures `L1489` `Status flag change error[1]` / `error(19)`) |
| [`boottrace_sata_bridge_gate3_3410pq_s14_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_3410pq_s14_boot1_fvu_20261123.log) | `5,092` | `33b0cc171292cbd02e235609e0bc3748812867d6a1d5281eb7e55cc1a6ef7d12` | **LIVE Boot 1 `MAIN3410PQ` `S1..S14+S13A/S13B` (`size=0x5E7C98`, `check_sum=0x2f82c81a`, `Vefify OK` × 2, `EV-20261123-030`)** |
| [`boottrace_sata_bridge_gate3_3410pq_s14_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_3410pq_s14_boot2_main_20261123.log) | `134,268` | `5ad28a99f7b8692d93a36f27f16201340c7aaaa76a7872f9e7d8f7a748eef30a` | **LIVE Boot 2 `MAIN 3410PQ` (`TV_NTSC(0)`, `nfrom back up ok.`, `sys_rsv init success.`, `setgui end. tsk=106`, `0` BGO/`U99` errors, `EV-20261123-030`)** |
| `sd_kit_3410pq_bgo_complete/PANA__SD.FRM` *(scratch)* | `7,181,464` | `2dc075f2c01fbb8e798d450eac806ed0db6113f2b93c1be31bef626a99a889de` | Rebuilt `S1..S14+S13A/S13B` `MAIN3410PQ` Stage-2 SD kit passing hardened `verify-sd` (`EV-20261123-029`, silicon-validated in `EV-20261123-030`) |
| `sd_unbrick_kit_sata_bridge/DIGADIAG.FRM` | `262,144` | `305cc2eebc09bbaa3bb5c5d75c0395eb16831bed4b9f80eafd57b9945e6a762d` | Base `ver061` `256 KiB` raw diag payload |
| `sd_unbrick_kit_sata_bridge/PANA__SD.FRM` | `7,563,224` | `58d501fa63e69571afdcb7cbc1e6c5644e3dcb29ee4b713d7faa4ed8e703a2b0` | Gate 2 SD update image |
| `sd_unbrick_kit_sata_bridge_stage2/DIGADIAG.FRM` | `262,144` | `305cc2eebc09bbaa3bb5c5d75c0395eb16831bed4b9f80eafd57b9945e6a762d` | Gate 3 Stage-2 `ver061` diag payload |
| `sd_unbrick_kit_sata_bridge_stage2/PANA__SD.FRM` | `7,563,304` | `42b007eb7e35caeeba19afc0d105f92a909312b9b7fd92da4e29d03076ef8523` | Gate 3 Stage-2 `MAIN3060NP` `8 MiB` (`0x32836a32`) |
| `eh55_loader_ver061_256k.bin` | `262,144` | `9f5c448d8879106be5ca3da6d6ac649026d50fbc191251ae0f4a003efca63800` | `Loader ver0.61` (`SA0..SA10`) used in VM proof |
| `main3060np_unpacked.bin` | `13,427,852` | `075f552ed583ba1835c0d44d1b533ff5a016b8d81d15d7166240098461ddcca7` | Stock unpacked `MAIN3060NP` |
| `hdd_partition_firmware_0x02000000.bin` | `6,173,264` | `6ec63de059c00b287a0e8a1a4102a8007bbccf3210a552bfb45118a03b85c898` | European reference HDD partition `0x02000000` (`M6:PAL:DVD.HDD`) |
| `eh55_main3410pq_unpacked.bin` | `13,003,052` | `c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1` | Unpacked `MAIN3410PQ` (`M6:PAL` w/ `NTSC`/`PAL` + `AIOF PAL_M` drivers) |
| [`com10_live_video_osd_diag_20261124.log`](../evidence/com10_live_video_osd_diag_20261124.log) | `23,096` | `FA9EA01E82C8AC7EF0AE8A4B8B300EE6AE2BD818AE17EC6A7E0B596F4B11CB82` | Live volatile `ndb>` diagnostic on `MAIN3060NP` + `STANDBY_GATE_STOP` trace (`EV-20261124-005`) |
| [`com10_live_volatile_patch_20261124.log`](../evidence/com10_live_volatile_patch_20261124.log) | `6,404` | `BD4B0F32A0D1F9A993C4CCCDF0C9ADB9F8772E7B175427608FA72465746A1365` | Live volatile `IC3001`/`AVDD`/`AIOF_FS0` SDRAM patch in `MAIN3060NP` (`0.702s`, `EV-20261124-005`) |
| [`com10_m6930lb_3410pq_boot1_fvu_20261124.log`](../evidence/com10_m6930lb_3410pq_boot1_fvu_20261124.log) | `10,355` | `1543B9B3D0322509723F985DD657464B09579D2276058549DF224E490EDE2536` | **LIVE Boot 1 `FVU` `MAIN3410PQ` `M6930LB` (`check_sum=0x2f84d497`, `Vefify OK` × 2, `EV-20261124-005`)** |
| [`com10_m6930lb_3410pq_boot2_main_20261124.log`](../evidence/com10_m6930lb_3410pq_boot2_main_20261124.log) | `42,889` | `245562E6B3F4DE9A29462C8338E80889E2B48DFAA01D1E9C01EC7B6DE6F1FBE5` | **LIVE Boot 2 `[HDD_AUTOBOOT]` `MAIN3410PQ` `M6930LB` + `ndb>` (`0x53186DBC=0x80`, `iic_err=0`, `EV-20261124-005`)** |
| [`com10_m6930lb_3410pq_live_video_fix_20261125.log`](../evidence/com10_m6930lb_3410pq_live_video_fix_20261125.log) | `16,510` | `8986CE243A0F314CF97D6AB21388817F3CD146A23230A218AE91636D72114487` | **LIVE `480i 60 Hz` (`0x88000000=0x8020`, `0x515CD08C=0`) + `AVDD_DACOUT1=0x0D00` w/ `0x515A2444=1` (`EV-20261125-001`)** |
| `sd_unbrick_kit_m6930lb_br/PANA__SD.FRM` | `7,181,488` | `03a47de8c0c32401c03806bbc27333a722c4bdd777bc6f57b6fcd80322d2fb76` | Turnkey 1-SD-card `MAIN3410PQ` 8-site `M6930LB` kit w/ `0x504395FC=0x86F2435C` stamp (`verify-sd --audit-profile hardened` PASS, `EV-20261125-001`) |

---

## 7. Cross References

- **Hardware Guide (`StarTech IDE2SAT2` & `Type B` FFC Cable):** [`HARDWARE_GUIDE_STARTECH_IDE2SAT2_AND_TYPE_B_FFC_CABLE.md`](HARDWARE_GUIDE_STARTECH_IDE2SAT2_AND_TYPE_B_FFC_CABLE.md)
- **Anti-Trap Geometry & SSOT Forensic Atlas Guide:** [`ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md`](ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md)
- **Forensic Ledger & Project Status:** [`ledger.json`](../evidence/ledger.json) · [`STATUS.md`](../STATUS.md) · [`debug_logs/README.md`](../debug_logs/README.md)
