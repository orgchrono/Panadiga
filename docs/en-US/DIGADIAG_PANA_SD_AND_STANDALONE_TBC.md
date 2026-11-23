> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/DIGADIAG_PANA_SD_E_TBC_AUTONOMO.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Technical Dossier: `DIGADIAG.FRM` vs `PANA__SD.FRM` / `PANA_DVD.FRM`, SD `-91006` Gate & Standalone TBC Feasibility (DMR-EH55 / MN103S)

> **Scope & Forensic Evidence Base:** Derived from verified MN103S disassembly of the real **DMR-EH55 NOR bootloader (`Loader ver0.61 [06/03/06]`**, mirrored in `prog061_dec.bin` `sd_load.o` @ `0x5040eca9` / `0x5040ed7c` / `0x5040ea7d` and March 2006 NOR `flash.bin` @ `0x440085d9` / `0x440095ab` / `0x4400bc70`), contrasted with `/BOOT.BIN` (**`Loader ver0.63 [07/10/24]`**, DMR-EZ485V, NOR `0x44000000`), alongside `PROG061` (`0x50400000`), `EH55_MAIN_3410PQ.bin` (`0x57000000`), `MAIN3060NP` (`0x57000000`), `PANA_DVD.FRM`, `hdd_partition_firmware_0x02000000.bin`, and official `VEP79138` schematics (`na_p49_blk1.png`, `na_p50_blk2.png`, `vol2_p125_interconnect.png`).

---

## 1. `.FRM` Container Formats & How the Recorder Consumes Them

Confusing **`DIGADIAG.FRM`** with **`PANA__SD.FRM`** and **`PANA_DVD.FRM`** — and conflating the real **DMR-EH55 bootloader (`Loader ver0.61 [06/03/06]`)** with the newer **`Loader ver0.63 [07/10/24]`** (`/BOOT.BIN` from DMR-EZ485V) — is the #1 pitfall during DMR-EH55 unbrick recovery:

| Container File | Reader Stage | Real DMR-EH55 Format (`Loader ver0.61 [06/03/06]`) | `/BOOT.BIN` Format (`Loader ver0.63 [07/10/24]`, EZ485V) | Verified Payload Breakdown | Unbrick & Field Role |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`DIGADIAG.FRM`** | **Stage 1: `sd_diag_load`** (`0x4400bc70` / `0x5040eca9` in `ver0.61`; `FUN_4400a97c` in `ver0.63`) | **Exact `262,144 B` (`0x40000`) uncompressed raw binary** (starting `dc 14 00 00 00` = `JMP 0x50400014`, **no `EXTRHEAD` wrapper**); loaded straight into **`0x90400000`** (uncached alias of `0x50400000`) and jumped to `0x50400000` after `*** start diag ***` | Single **`EXTRHEAD`** container (exact **`131,072 B` / `0x20000`**, tag `"FVU "`, `dst=0x50400000`, `adler_step=4`), read into **`0x93fc0000`** and unpacked via `uncompress()` | Single MN103S binary (`262,144 B` at `0x50400000`), e.g. BGO-patched `PROG061` (`bypass` / `PANAMON1`) or `NOR_DMP` | **Pre-MAIN rescue entry**: runs before `NVRAM U99` and HDD/ODD mount; supplying `131,072 B` (`EXTRHEAD`) to `ver0.61` causes `boot_fat_read` to truncate at `0x20000 != 0x40000` (`0x5040ed67`) and fail with **`SD diag load failed[0].`** |
| **`PANA__SD.FRM`** | **Stage 2: `PROG061`** (`sd_read_firm_list` @ `0x5040ed7c` / `sd_find_firm_entry` @ `0x5040ea7d` in `ver0.61`) | **Plaintext `48-byte` (`0x30`)** TOC entries, up to 128 slots (**`0x1800 B` = `6,144 B`**, mode **`sd48`**), checked via `list_sum_u16` | **Plaintext `64-byte` (`0x40`)** TOC entries, up to 128 slots (**`0x2000 B` = `8,192 B`**, mode **`sd64`**) | Typically 3 slots: `DRV S211`, `PROG061 `, `MAIN3060NP` (or `MAIN3410PQ`), **`7,531,744 B`** in stock `sd48` (`7,533,792 B` in `sd64`) | Full SD update flashed into NOR, persisting **`MAIN3410PQ` TBC Passthrough (`E-E`) + dynamic `>200 GB` HDD** |
| **`PANA_DVD.FRM`** | **Optical Disc Update** (`CD-R` / `DVD-R`) | **Feistel-encrypted `48-byte`** `"Firm list"` TOC; dual layer (outer Feistel per module + internal partial Feistel `EXTRHEAD` on `DRV`/`MAIN`) | Same optical `48 B` Feistel format | Empirical (`7,531,744 B`): `[0] DRV S211` (`0x001800`, `720,896 B` -> `1,441,792 B @ 0x90202000`), `[1] PROG061 ` (`0x0B1800`, `262,144 B` raw after outer decrypt), `[2] MAIN3060NP` (`0x0F1800`, `6,542,560 B` -> `13,427,852 B @ 0x57000000`) | Canonical upstream source used to extract/rebuild `DIGADIAG.FRM` and `PANA__SD.FRM` |

### End-to-End SD Boot Consumption Flow (`Loader ver0.61` vs `/BOOT.BIN` `ver0.63`)

```text
[SD Card: FAT16 SDSC <= 2 GiB (<= 0xFFF5 clusters)]
   │
   ├──► Stage 1 (Real DMR-EH55): Loader ver0.61 [06/03/06]
   │       (mirrored in prog061_dec.bin sd_load.o @ 0x5040eca9 / 03/2006 NOR @ 0x440085d9, 0x440095ab, 0x4400bc70)
   │       1. Passes 1-bit GetSD_Status -> sends ACMD6 4-bit switch -> 2nd ACMD13 check
   │       2. Mounts FAT16 -> opens DIGADIAG.FRM -> requires EXACT 262,144 B (0x40000) raw uncompressed
   │          (if given 131,072 B EXTRHEAD, 0x5040ed67 cmp 0x40000, d0 fails -> "SD diag load failed[0].")
   │       3. Loads directly to 0x90400000 (uncached alias of 0x50400000), closes FAT, flushes caches,
   │          prints "*** start diag ***" and executes JMP 0x50400000 (@ 0x44008601)
   │          ├─► bypass (safe default): skips failing BGO checks while leaving CS1 IPL/RAM untouched
   │          ├─► UART_MONITOR_PANAMON1: enables pre-MAIN interactive monitor (P7501 115200 8N1)
   │          ├─► auto_format (BLOCKED fail-closed; requires --allow-unsafe-bgo-format: targets PROG061/ver0.63 CS1 0x84200000..0x843FFFFF; real EH55 ver0.61 BGO is 0x84130000..0x843FFFFF SA26..SA70)
   │          └─► NOR_DMP stub: streams 8 MiB NOR Flash (0x84000000) read-only over UART
   │
   ├──► Stage 2 (Real DMR-EH55): PROG061 at 0x50400000 -> sd_read_firm_list (0x5040ed7c) / sd_find_firm_entry (0x5040ea7d)
   │       └─► Reads PANA__SD.FRM in sd48 mode (48B plaintext TOC x 128 = 0x1800 B; stock 7,531,744 B) and programs NOR
   │
   └──► Contrast (Ghidra reference /BOOT.BIN): Loader ver0.63 [07/10/24] (DMR-EZ485V)
           └─► FUN_4400a97c reads 0x20000 B (131,072 B) EXTRHEAD into 0x93fc0000 + uncompress() -> 0x50400000;
               PANA__SD.FRM uses 64B plaintext TOC (0x2000 B, mode sd64)
```

> **`MAIN3410PQ` vs `MAIN3060NP` Build Note & `sd48` vs `sd64`:**
> HDD Partition 1 (`firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin`, `6,173,264 B`) unpacks via `panadiga frm unpack --mode main` to **exact `13,003,052 B` = `EH55_MAIN_3410PQ.bin`** (`VA 0x57000000`), matching all 3 TBC passthrough sites (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`) and the `src/hdd_cmd.c` LBA48 clamp (`0x574DD1FC`). Conversely, `PANA_DVD.FRM` slot `[2]` (`7,531,744 B`) and factory `PANA__SD.FRM` (`7,531,744 B` in `ver0.61` `sd48` `0x1800 B` TOC, or `7,533,792 B` in `ver0.63` `sd64` `0x2000 B` TOC) hold `MAIN3060NP` (`13,427,852 B` unpacked), where `odd_branch` moves to `0x4FE986`. `panadiga unbrick prepare-sd` enforces fail-closed build identity checks (`--expected-main-build` / `--allow-target-mismatch`) so `MAIN3410PQ` TBC/HDD patches are never silently misapplied. For real `Loader ver0.61` targets, use `panadiga unbrick bgo --raw-prog061` (raw `262,144 B` / `0x40000` `DIGADIAG.FRM`) and `panadiga frm pack --mode sd48` (`0x1800 B` 48-byte TOC `PANA__SD.FRM`), reserving `131,072 B` (`0x20000`) `EXTRHEAD` + `sd64` (`0x2000 B`) for `Loader ver0.63`.

---

## 2. SD `-91006` 4-Bit Gate & Why Pico Cannot Force 1-Bit Over `P7501`

A recurring question was whether the Raspberry Pi Pico connected to `P7501` could force `/BOOT.BIN` to remain in **1-bit** SD mode to bypass the `-91006` error. Disassembly of `/BOOT.BIN` provides an unambiguous answer:

1. **Exact `/BOOT.BIN` Execution Path:**
   * **`0x44027826`:** First `GetSD_Status` call succeeds in **1-bit bus mode** (`0x89001850 = 0x80b8`).
   * **`0x44027F79`:** `/BOOT.BIN` issues `ACMD6(2)` for **4-bit bus width** and clears bit 15 (`0x89001850 &= 0x7fff`).
   * **`0x44027F93`:** Second `GetSD_Status` fails with **`-91006`** whenever an electrical fault compromises **`DAT1..DAT3`** (`P6801` -> `VEP73136A` -> `P6802` FFC -> `P53501` -> `MN2DS0009` — the dominant hypothesis after CMD/CLK/DAT0 already succeeded in 1-bit mode) or when card/controller data corrupts `SD_STATUS[2]` (`!= 0`, while **SDHC > 2 GiB** typically fails earlier on `CMD8`/OCR). Optical drive error **`0x83A7`** cannot be bypassed in read-only IPL NOR and only affects the secondary `CDDIAG` optical fallback when SD init fails first.
2. **Why `P7501` Pico Injection Cannot Override `/BOOT.BIN`:**
   * UART receive helpers inside `/BOOT.BIN` (`uart0_getc @ 0x4400479F` -> `console_getline @ 0x4400488A`) have **zero active callers** (unreferenced dead code in production IPL).
   * `P7501` does **not** expose the On-Chip Debug (`OCD`) clock (`DCSR = 0xFFFFFFFF`), making live halt/register override impossible during `/BOOT.BIN`.
3. **Working Mitigation & Bench Thermal Safety:**
   * **Thermal / Short-Circuit Guard Before Powering:** NEVER energize the unit with a heated FFC cable connected (especially the 18-pin ODD pickup flex), and NEVER hot-plug/unplug FFCs; first measure cold resistance to `GND` on **`D3R3V`/`D+3.3V`/`DR+5V`/`DR+12V`** and trace continuity across `P6801`/`P6802`/`VEP73136A`.
   * Use genuine **SDSC `<= 2 GiB`** media formatted **FAT16** (`<= 0xFFF5` clusters; FAT32 fails with `-10025`) with a **`262,144 B` (`0x40000`)** raw `DIGADIAG.FRM` for `Loader ver0.61` (supplying a `ver0.63` `131,072 B` `EXTRHEAD` file after passing 4-bit init triggers **`SD diag load failed[0].`**). Run `panadiga unbrick verify-sd <dir>` before bench testing.
   * Once electrical rails are verified safe, run **`SDSNIFF`** (passive Hi-Z PIO0 sniffer on `GP10..GP15` isolating per-lane `DAT0..DAT3` CRC16 + `ACMD13`) and `scripts/bench/boot_harness.py` (`ab-boot`) to isolate `DAT1..DAT3` wiring faults vs card rejection.

---

## 3. Can the Digital PCB (`MN2DS0009` / `VEP79138` `P59001`) Run Standalone TBC Outside the Drive?

**Engineering Verdict: Optical Drive (`ODD`) and mechanical HDD hardware can be completely removed to create a solid-state TBC processor box, BUT the Digital PCB (`P59001`) still requires the Main PCB (`P7402`).**

### Removable Subsystems (Enabled by `MAIN3410PQ` `AV_PASSTHROUGH`)
Patching the 3 verified sites in `MAIN3410PQ`:
1. **`fpc_status_ctrl` (`VA 0x5771ABFB`, offset `0x71ABFB`, 6B):** Bypasses front-panel controller check -> `mov 1, d0; ret; nop; nop` (`80 01 f0 fc cb cb`).
2. **`odd_state_branch` (`VA 0x574F7B8C`, offset `0x4F7B8C`, 4B):** Unconditional `jmp16 +0x00D0; nop` (`cc d0 00 cb`) to `0x574F7C5C`.
3. **`odd_init_status` (`VA 0x574F7C5C`, offset `0x4F7C5C`, 7B):** Forces ODD ready return -> `clr d0; nop x5` (`80 00 cb cb cb cb cb`).
* Pairing this with a **`StarTech IDE2SAT2` (`MASTER`) + CF/SSD** and dynamic LBA48 `>200 GB` unlock (`0x574DD1FC`) removes both the optical drive and mechanical HDD — while keeping live **`PANARSP` injection via `P7501` (`_dev_SERIAL @ 0x515B2E00`, 3-byte handshake `0e 04 02` + `0x15` NAK)** available on demand.

### Non-Removable Main PCB Dependencies (`na_p49_blk1.png`, `na_p50_blk2.png`, `vol2_p125_interconnect.png`)
Per `VEP79138` interconnect schematics across `P7402 <-> P59001`, the Digital PCB lacks analog front-end/back-end and autonomous power sequencing:
* **Analog Video Front/Back-End (`IC3001` on Main PCB):** Houses sync clamps, **`6M+12M LPF`**, and **`75 Ω`** output drivers.
* **Audio Matrix & DAC (`IC4001` on Main PCB):** Handles analog audio switching outside `P59001`.
* **Timer / Syscon MCU (`IC7501` on Main PCB):** Drives the `TBUS` handshake, `XRESET`, and power-good sequencing.
* **8+ Power Rails (`P7402 -> P59001`):** Multiple regulated rails originate on Main PCB/PSU and feed `P59001`.

**Practical Outcome:** You can shed the steel chassis, bulky DVD optical mechanism, and aging 3.5" IDE HDD cleanly by keeping only **Digital PCB + Main PCB + PSU + CF/SSD (`IDE2SAT2`) + `MAIN3410PQ (AV_PASSTHROUGH)`**.

---

## 4. Operator Guide: `panadiga unbrick` CLI, Unbrick Center UI & MCP

```bash
# 1. Inspect workspace binaries, PROG061 BGO status, MAIN3410PQ TBC sites & RSP plan
panadiga unbrick status

# 2. Extract and Feistel-decrypt modules from PANA_DVD.FRM
panadiga unbrick extract-frm --frm firmware_and_images/PANA_DVD.FRM --outdir unbrick_extracted --unpack

# 3. Generate SD Unbrick Kit (default --loader-profile ver061: 256 KiB raw + sd48 PANA__SD.FRM in firmware_and_images/sd_unbrick_kit; or --loader-profile ver063 for 128K EXTRHEAD + sd64)
# Note: --bgo-mode auto_format (0x50401F3C) is blocked fail-closed and requires --allow-unsafe-bgo-format
panadiga unbrick prepare-sd --loader-profile ver061 --bgo-mode uart_monitor --passthrough-mode av_passthrough

# Manual alternative for raw DIGADIAG.FRM only (ver0.61):
panadiga unbrick bgo --input firmware_and_images/extracted_partitions/prog061_dec.bin --mode uart_monitor --raw-prog061 --output firmware_and_images/sd_unbrick_kit/DIGADIAG.FRM

# 4. Verify prepared SD card folder (containers, Adler-32, list_sum_u16 & ver061/ver063 consistency)
panadiga unbrick verify-sd firmware_and_images/sd_unbrick_kit

# 5. Inspect/patch MAIN3410PQ TBC Passthrough or generate _dev_SERIAL GDB RSP frames
panadiga unbrick passthrough --input firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin
panadiga unbrick passthrough --rsp-plan --callback-va 0x50401F3C

# 6. Assemble read-only MN103S NOR flash UART dumper stub (128K EXTRHEAD for ver0.63; ver0.61 requires 262,144 B raw @ 0x90400000)
panadiga unbrick nor-stub --wrap-digadiag --out unbrick_out/DIGADIAG_NOR_DUMPER.FRM
```

* **Desktop App:** Open **`Unbrick Center`** (`Centro Unbrick`) in the sidebar for visual SD Kit generation with `loader_profile` selection (`ver061` default for EH55 or `ver063`) targeting `firmware_and_images/sd_unbrick_kit/`, SD media verification, `.FRM` extraction, TBC Passthrough inspection/patching, live RSP frame calculation, and `ver0.63` 128K / raw SDRAM NOR Dumper compilation.
* **MCP Server (`panadiga mcp`):** Exposes tool **#18 `panadiga_sd_unbrick_kit`** (`loader_profile`, actions `status`, `extract_frm`, `prepare_sd`, `verify_sd`) alongside `panadiga_bgo_unbrick` (`--raw-prog061` / `wrap_as_digadiag: false`) and `panadiga_main_passthrough`.

---

## 5. Regional Compatibility (`PAL-M`/`LBS`), Diskless TBC Blockers, `ES10/ES15/ES35V` vs `EH55` & Panel/IR Matrix

- **Regional `PAL-M`/`LBS` & `ifodev2` Matrix (`[GHIDRA STATIC EVIDENCE / STAGE 0]`):** Block `ifodev2` (`2,048 B`, `SHA-256: f4f22dd0...`, target `0x0055`) stores metadata exclusively in its `0x400..0x7FF` tail. Video subsystems **`AIOF` (`0x8800_0000`, `cps_pal_m 0x8021`)**, **`AVDD` (`0x8980_0000`)**, and **`IC3001`** (`C1AB00002379` vs `1979`) govern cross-region standards (`re::region_compat::tv_standard`), while `hdd/capacity_patch` spans `5 GB` (`0x0024C388` `SV0511D`) through `2 TB` and `re::tbc_appliance` tracks diskless TBC workflows.
- **Viable Diskless TBC Routes (`[GHIDRA STATIC EVIDENCE / STAGE 0]` vs `[UNDER INVESTIGATION / CH1 PHYSICAL BLOCK]`):** Inside `MAIN3410PQ`, `VDM_Init` and `SDM_Init` (plus `FUN_574f5c6a`/`FUN_574f469d`) enforce HDD coupling (`HDD Err -> P-OFF!`), requiring `Stage15DigadiagTbc` / `PANA__SD.FRM` (`PROG 0x0B1800 -> 0x90400000`) or `Nor-FROM 0x84040000` for standalone TBC operation while `CH1` prevents `MAIN` boot.
- **`ES10` / `ES15` / `ES35V` vs `EH55` Incompatibility Verdict (`[GHIDRA STATIC EVIDENCE / STAGE 0]`):** Firmware images from `DMR-ES10`/`ES15`/`ES35V` are NOT interchangeable with `DMR-EH55` and cause hardware/boot bricks due to SoC, memory, and power-rail mismatches: `MN2DS0009` vs `MN2DS0012-H`/`MN2DS00151DB`, `RFKFM6016KT` vs `24LC64`, `X_SW_12V`/`DR12V`/`DR5V`, and `ES35V` XIP execution (`0x130000..0x40FFFF`).
- **Front Panel `FRONT(L)` & `IR7501` (`[GHIDRA STATIC EVIDENCE / STAGE 0]`):** The `FRONT(L)` sub-PCB carries only `[POWER]`; remaining keys sit on Main PCB `KEYIN1..3` while `IR7501` (`PNA4618M09VT`, `X_SW_5R2V`) services Standby `SERV` (`[CH UP]+[OPEN/CLOSE]+[REC]` for 5 s in `Power OFF`) before handing off to `FPC_ReciveTASK`/`R_SERVICEMODE` in `MAIN` ([`EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md`](./EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md)).

---

## 6. Stage-0 Static Triangulation (`AIOF` / `IC3001` / `IC7501 M0`) & Door #1 Bench Runbook

### 6.1 Verified `AIOF` Register Table, Clock Sequencing & `IC3001` (`EH55_MAIN_3410PQ.bin`)

1. **Complete 82-Register `AIOF` Bank (`0x8800_0000..0x8800_00B4`, `{u32 name_ptr, u32 reg_off}` descriptors at `0x578935FC..0x57893888`):**
   - Mode & Clock Controls: `VMODESEL` (`+0x00`: `0x8020` `cps_ntsc`, `0x8021` `cps_pal_m`, `0x8027` `cps_pal`), `V` (`+0x66`: Preset `[93]` `clock_start = 0x8001`, Preset `[94]` `ddr_stop = 0x8039`), `TEST1` (`+0x6C`: Preset `[0]` `0xB040`; `AIOF_SoftReset` `@ 0x570D64E4` clears bit 15 `& 0x7FFF`).
   - TBC & Frame Sync: `TBCSS0..2` (`+0x1C/+0x1E/+0x20`), `TBC` (`+0x24`), `FS0..1` (`+0x2C/+0x2E`: Preset `[85]` `standard = 0x40F0`, Preset `[86]` `not_standard = 0xC0F0`), `YC3` (`+0x64`: Preset `[90]` `mvagc_line_tbc_on` bit 5 `0x0020`), `CONT` (`+0xA4`: `[80]` `ntsc_setup_lighter = 0x4000`, `[81]` `ntsc_setup_darker = 0x8000`).
   - Read-Only Telemetry & Emphasis (`SSOT` synced in `crates/panadiga-constants/src/constants/tbc_mmio.rs`): `RDACC` (`+0x80`), `RDMV` (`+0x82`), `RDDET` (`+0x84`), `RDVBID_WSS0..1` (`+0x86/+0x88`), `RDDET1` (`+0x92`), `RDFCNT` (`+0x94`), and `EMPHA` (`+0xB4`).
   - **98 Static Presets:** Pointers to `{u32 reg_off, u16 and_mask, u16 or_val, u32 extra}` tables at `0x578957D8..0x5789595C` paired with preset names at `0x57C1F57C..0x57C1F704` (`common`, `cps_ntsc`, `cps_pal_m`, `cps_pal`, `cps_secam`, `cpn_*`, `rgb_pal`, `rec656_*`, `mvagc_line_tbc_on/off`, `clock_start`, `ddr_stop`).
2. **`IC3001` (`C1AB00002379`) I2C Addressing at `0x570D627E` (`src/iic33.c`):**
   - The `AVIN` caller loads 8-bit I2C write address **`0x94`** (**7-bit `0x4A`**, read `0x95`) prior to invoking `0x573FF1B4` (`IIC1` `0xD840_1000` with Timer arbitration).

### 6.2 `M0` `IC7501 TBUS` Keepalive & Door #1 Bench Runbook (Pico + FAT16 SDSC + Multimeter)

* **`M0` Engine Verified on `Mn103Vm` ([`keepalive.rs`](../../crates/panadiga-core/src/re/tbc_appliance/keepalive.rs)):**
  - `build_ic7501_wdt_255s_frame()` produces `[0x8A, 0x02, 0x00, 0xFF, 0, ..., 0, 0x30]` (`sum - 0x5B = 0x30`);
  - `emit_m0_tbus_wdt_keepalive_stub()` emits position-independent MN103S machine code (callable from `PANAMON1>` `J 51800000` via `CALLS (A2)` $\rightarrow$ `RETS 0xF0 FC`), writing `0xDC01_0010..0xDC01_001F`, pulsing `0xDC01_0000` bit 4, and returning `D0 = 0x30`.
* **Door #1 `M0` Bench Runbook (`NoJigPanaSdProgSlot`, Pico + SDSC FAT16 + Multimeter Only):**
  1. **Pre-Power Check:** Verify Pin 4 `GND` (`0.0 Ω`), keep `GP3` (`NSW5V`) strict `Hi-Z Input` with no pulls, `GP5` (`XRESET`) strictly Open-Drain, and power the Pico via USB only (`P7501` Pin 1 `< 10 mA`).
  2. **Door #1 Headless Boot (No HDD / No JIG):** Insert a `<= 2 GiB` FAT16 SDSC card holding [`PANA__SD.FRM`](../../firmware_and_images/sd_unbrick_kit/PANA__SD.FRM) (`cdcc2cdf...`), leave HDD/ODD disconnected, and open `COM10` at `115200 8N1`. Upon `HDD_MAX_LBA get failed[0xE8000000]`, `Loader ver0.61` enters `PANAMON1>` directly.
  3. **`M0` Injection (`0x5180_0000`):** Upload `emit_m0_tbus_wdt_keepalive_stub()` using `W`, execute `J 51800000` (`< 200 s` cadence), and confirm with the multimeter that `DR_P_ON_H` / `DI_P_ON_H` (`NSW5V ~3.29V`) and `VEP79138` rails stay powered for `> 30 min` without `U99` shutdown. See also [`STATUS.md`](../STATUS.md).
