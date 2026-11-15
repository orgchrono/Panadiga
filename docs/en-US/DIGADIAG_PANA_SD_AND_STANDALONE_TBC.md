# Technical Dossier: `DIGADIAG.FRM` vs `PANA__SD.FRM` / `PANA_DVD.FRM`, SD `-91006` Gate & Standalone TBC Feasibility (DMR-EH55 / MN103S)

> **Scope & Forensic Evidence Base:** Derived from verified MN103S disassembly of `/BOOT.BIN` (NOR `0x44000000`), `PROG061` (`0x50400000`), `EH55_MAIN_3410PQ.bin` (`0x57000000`), `MAIN3060NP` (`0x57000000`), container dumps in `sd_card_eh55/`, `PANA_DVD.FRM`, `hdd_partition_firmware_0x02000000.bin`, and official `VEP79138` schematics (`na_p49_blk1.png`, `na_p50_blk2.png`, `vol2_p125_interconnect.png`).

---

## 1. `.FRM` Container Formats & How the Recorder Consumes Them

Confusing **`DIGADIAG.FRM`** with **`PANA__SD.FRM`** and **`PANA_DVD.FRM`** is the #1 pitfall during DMR-EH55 unbrick recovery. Although all three share the `.FRM` extension, they target completely different boot stages and use different cryptographic wrappings:

| Container File | Reader Stage | Header / TOC Structure | Encryption Layers | Verified Payload Breakdown | Unbrick & Field Role |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`DIGADIAG.FRM`** | **Stage 1: `/BOOT.BIN`** (`SDDIAG` at NOR `0x44000000`) | Single **`EXTRHEAD`** container (exact **`131,072 B` / `0x20000`**, tag `"FVU "`, `dst=0x50400000`, `adler_step=4`, build `"PROG061"`) | **Plaintext** (neither outer TOC nor Feistel payload wrapping) | Single MN103S binary (`262,144 B` unpacked -> SDRAM `0x50400000`), e.g. BGO-patched `PROG061` (default `bypass`) or `NOR_DMP` | **Pre-MAIN rescue entry**: runs before `NVRAM U99` initialization and before HDD/ODD mount; executes **`bypass` / `UART_MONITOR_PANAMON1`**, or UART NOR dump (`auto_format` blocked fail-closed without override) |
| **`PANA__SD.FRM`** | **Stage 2: `PROG061`** (running at `0x50400000`) | **Plaintext `64-byte` (`0x40`)** TOC table, up to 128 slots (`8,192 B` TOC area), checked via `list_sum_u16` | Plaintext TOC; module payloads keep internal `EXTRHEAD` partial Feistel spans (`DRV`: `0x2800`, `MAIN`: `0x19000`) | Typically 3 slots: `DRV S211`, `PROG061 `, `MAIN3410PQ` (or `MAIN3060NP`), ~`7.53 MB` (`7,533,792 B`) | Full SD update flashed into NOR, persisting **`MAIN3410PQ` TBC Passthrough (`E-E`) + dynamic `>200 GB` HDD** |
| **`PANA_DVD.FRM`** | **Optical Disc Update** (`CD-R` / `DVD-R`) | **Feistel-encrypted `48-byte`** `"Firm list"` TOC table | **Dual layer**: full outer Feistel per module + internal `EXTRHEAD` partial Feistel (`DRV`/`MAIN`) | Empirical (`7,531,744 B`): `[0] DRV S211` (`0x001800`, `720,896 B` -> `1,441,792 B @ 0x90202000`), `[1] PROG061 ` (`0x0B1800`, `262,144 B` raw after outer decrypt), `[2] MAIN3060NP` (`0x0F1800`, `6,542,560 B` -> `13,427,852 B @ 0x57000000`) | Canonical upstream source used to extract/rebuild `DIGADIAG.FRM` and `PANA__SD.FRM` |

### End-to-End SD Boot Consumption Flow (`BOOT.BIN` -> `PROG061` -> `MAIN`)

```text
[SD Card: FAT16 SDSC <= 2 GiB]
   │
   ├──► Stage 1: /BOOT.BIN (NOR 0x44000000, SDDIAG loader)
   │       1. Passes 1-bit GetSD_Status (@ 0x44027826) -> sends ACMD6 4-bit switch (@ 0x44027F79) -> 2nd check (@ 0x44027F93)
   │       2. Loads DIGADIAG.FRM (strictly requires exact 131,072 bytes)
   │       3. Inflates EXTRHEAD ("FVU ") straight to 0x50400000 (Adler-32) and branches to entry
   │          ├─► bypass (safe default): skips failing BGO checks while leaving CS1 IPL/RAM untouched
   │          ├─► UART_MONITOR_PANAMON1: enables pre-MAIN interactive monitor (P7501 115200 8N1)
   │          ├─► auto_format (BLOCKED fail-closed; requires --allow-unsafe-bgo-format: erases CS1 0x84200000..0x843FFFFF / pointers 0x504F8464..6C)
   │          └─► NOR_DMP stub: streams 8 MiB NOR Flash (0x84000000) read-only over UART
   │
   └──► Stage 2: PROG061 (at 0x50400000) -> opens PANA__SD.FRM (64B plaintext TOC slots)
           └─► Programs updated firmware images (DRV S211, PROG061, MAIN3410PQ) with permanent patches
```

> **`MAIN3410PQ` vs `MAIN3060NP` Build Note:**
> HDD Partition 1 (`firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin`, `6,173,264 B`) unpacks via `panadiga frm unpack --mode main` to **exact `13,003,052 B` = `EH55_MAIN_3410PQ.bin`** (`VA 0x57000000`), matching all 3 TBC passthrough sites (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`) and the `src/hdd_cmd.c` LBA48 clamp (`0x574DD1FC`). Conversely, `PANA_DVD.FRM` slot `[2]` and factory `PANA__SD.FRM` (`7,533,792 B`) hold `MAIN3060NP` (`13,427,852 B` unpacked), where `odd_branch` moves to `0x4FE986`. `panadiga unbrick prepare-sd` enforces fail-closed build identity checks (`--expected-main-build` / `--allow-target-mismatch`) so `MAIN3410PQ` TBC/HDD patches are never silently misapplied.

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
   * Use genuine **SDSC `<= 2 GiB`** media formatted **FAT16** (never SDHC >= 4 GiB) and run `panadiga unbrick verify-sd <dir>` before bench testing.
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

# 3. Generate complete SD Unbrick Kit (DIGADIAG.FRM 128K safe default bypass + PANA__SD.FRM)
# Note: --bgo-mode auto_format (0x50401F3C) is blocked fail-closed and requires --allow-unsafe-bgo-format
panadiga unbrick prepare-sd --outdir sd_card_eh55_kit --bgo-mode bypass --passthrough-mode av_passthrough

# 4. Verify prepared SD card folder
panadiga unbrick verify-sd sd_card_eh55_kit

# 5. Inspect/patch MAIN3410PQ TBC Passthrough or generate _dev_SERIAL GDB RSP frames
panadiga unbrick passthrough --input firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin
panadiga unbrick passthrough --rsp-plan --callback-va 0x50401F3C

# 6. Assemble read-only MN103S NOR flash UART dumper stub (optional 128K DIGADIAG wrapper)
panadiga unbrick nor-stub --wrap-digadiag --out unbrick_out/DIGADIAG_NOR_DUMPER.FRM
```

* **Desktop App:** Open **`Unbrick Center`** (`Centro Unbrick`) in the sidebar for visual SD Kit generation, SD media verification, `.FRM` extraction, TBC Passthrough inspection/patching, live RSP frame calculation, and 128K NOR Dumper compilation.
* **MCP Server (`panadiga mcp`):** Exposes tool **#18 `panadiga_sd_unbrick_kit`** alongside `panadiga_bgo_unbrick` and `panadiga_main_passthrough`.
