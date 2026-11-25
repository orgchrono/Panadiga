> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/RELATORIO_SESSAO_ENGENHARIA_REVERSA_EH55_100_FIEL.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# 100% Faithful Reverse Engineering and Live Debugging Session Report (`DMR-EH55` / `COM10` / `P7501`)

> **Documented Period:** `2026-11-06` to `2026-11-13` (Steps `1`–`50` static analysis + live captures via Pico RP2040 on `COM10` + CAD electrical audit of Panasonic service manuals).  
> **Scope:** Rigorous chronological record of **everything tested, measured, proven, and discarded** during the real bench session and binary analysis, explicitly distinguishing live hardware (`DMR-EH55` / `Loader ver0.61 [06/03/06]`) from the static reference image (`DMR-EH58` / `Loader ver0.63 [07/10/24]` inside `PANA_DVD.FRM`).

---

## 1. Executive Summary of Proven Final Verdicts

1. **Actual Protocol on `P7501 (TO WRITE JIGU)` in the `DMR-EH55`:**  
   `P7501` is **NOT `DWire32A` / synchronous JTAG debug (`CN55001`)**. It is an **Asynchronous UART port (`115200 8N1`) + Discrete Boot/Reset Control**, simultaneously confirmed by official service manuals (`DMR-EH55P/PC North America`, `EC/EB/EF Vol. 1 & 2`, `EC/EE`) and electrical/logic telemetry on `COM10`.
2. **Two Independent Faults Confirmed on the Bench Board (`U99`):**
   - **Fault 1 (Main PCB — Spansion NOR / `[nfrom]`/`BGO`):** In `***** normal mode *****`, the bootloader calls `BGO driver init` (`0x4400836f`) **before** accessing the HDD, receives return code `1` from `0x4401e636`, prints **`BGO driver init failed[1].`**, and skips loading `main soft` from HDD (`0x440065bc`), branching directly into emergency recovery (`0x44007812`) — producing `Please wait` $\rightarrow$ `U99` on the front panel.
   - **Fault 2 (`MATSHITA R6-MV4` Optical Drive — `0.5 mW` Laser):** In `CD diag boot mode` (or fallback from `normal mode`), after ticks `59..53` showing `X(80020401)` (*Logical Unit in process of becoming ready*), the drive fails at tick `52` with **`[RTSD] === DRIVE ACCESS ERROR === ( 80040904 )`** and **`SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00]`**, counting down to `Drive start failed[-2147219196].` followed by a cold reset. The Shift-JIS table inside `DRV_decomp.bin` (`0x400e8c1e`, stride `0x34`) proves that **`0x83A7`** is **`0.5mW発光ができない(HFｵﾝ密調整)`** (*Unable to emit 0.5 mW laser during HF ON fine adjustment*), **not** an IDE error flag stored in NVRAM.
3. **Bypassing the `BGO` Lock via `Pin 1 (SW_UART = LOW)`:**  
   Pulling `P7501 Pin 1 (SW_UART / GP26)` `LOW` during `XRESET` triggers **`***** serial boot mode *****`**, which **skips the `BGO` error**, immediately initializes **`*** timer correspond initialize ***`** (`startup reason = [70]`, `Main hung-up time changed 255[s].`), and executes the recovery chain (`fmu` $\rightarrow$ `SD init start...` $\rightarrow$ `*** CD diag boot mode ***`).

---

## 2. Faithful Session Chronology (From Initial Code to Live Bench)

### Phase A — Engineering Restructure and CI Pipeline (`2026-11-06` to `2026-11-11`)
* Consolidated the repository under `C:\Users\danie\Documents\Panadiga`, eliminating legacy hardcoded paths, unifying CLI and Desktop around `panadiga.exe` with transparent `.gz` loading, MVVM/FP architecture in Rust + React/TypeScript, and **9 mandatory Quality Gates** in `scripts/validate.py` (Rust `fmt/check/test`, `tsc`, Vitest, `jscpd 0%`, `uxlint`, Playwright, Ghidra export, C11 host `-Werror`/`/WX`, and Python `py_compile`/`mypy --strict`/`pytest`).
* Fixed critical USB CDC initialization bugs on the Pico (`DTR` + timeout/prompt framing) until achieving a stable connection on `COM10` (`VID:2E8A PID:000A` $\rightarrow$ `PING PONG`).

### Phase B — Synthetic Mock Discovery & Passive Serial Adapter Failure (`2026-11-12`)
* **Forensic audit of `panasonic_dmr-eh55_flash_dump.bin` (`8,388,608 B`):** Entropy and byte analysis proved that `8,388,509 B` (`99.999%`) were `0xFF`, leaving only `99` synthetic bytes (`PANASONIC DMR-EH55 AM34 IPL BOOTLOADER VEP79138` and a `55 AA 10 00...` fixture for Rust unit tests). Static analysis of real code immediately moved to the `PANA_DVD.FRM` package extracted from `DVDRAM.7z` (`DMR-EH58`, `BOOT.BIN` `ver0.63` + `DRV.BIN`).
* **Why `FT232RL` and Arduino (without ATmega) failed before the Pico:** Passive 3-wire USB-Serial converters (`TX/RX/GND`) do not control `Pin 1 (SW_UART)` or `Pin 2 (XRESET)` reset timing required to switch `XFWE` toward Digital PCB `VEP79138K`.
* **Why synchronous `DWire32A` `HANDSHAKE` returned `DCSR: 0xFFFFFFFF`:** On the harness plugged into `P7501`, `GP3` was connected to `Pin 5 (NSW5V)` (static pull-up rail, **not `SCLK`**) and `GP4` to `Pin 3 (MAX RXD)` (idle-HIGH pull-up via `R7570`, **not bidirectional `SDATA`**).

### Phase C — Physical Electrical Inspection (`PINCHECK` / `BUSDIAG`) & CAD Manuals (`2026-11-12` to `2026-11-13`)
* **Physical orientation measured on the board:** The user identified connector **`P7501` (`B6B-ZR-3R4`, 6 pins, `1.5 mm` pitch)** squeezed between the optical drive FFCs **`FP50003`** (10/11-pin, Spindle Motor, on the left) and **`FP50004`** (8-pin, Loading Motor + `OPEN_SW`/`CLOSE_SW`, on the right) — see [page_113.png](../assets/images/re_dmr_eh55/page_113.png) and [fp50004_zoom.png](../assets/images/re_dmr_eh55/fp50004_zoom.png).
* **Multimeter measurements by the user:** On every pin except `GND`, the multimeter read **`~3.3V` idle**, seemingly contradicting an initial reading of label `NSW5V (5.15V)`. Vector CAD inspection in [`na_p62_zoom.png`](../assets/images/re_dmr_eh55/na_p62_zoom.png), [`na_p64_topleft.png`](../assets/images/re_dmr_eh55/na_p64_topleft.png), [`crop_p55_IC1506_zoom.png`](../assets/images/re_dmr_eh55/crop_p55_IC1506_zoom.png), and [`crop_p64_Q7510_Q7514_QR7503.png`](../assets/images/re_dmr_eh55/crop_p64_Q7510_Q7514_QR7503.png) reconciled 100% of the numbers:

| `P7501` Pin | Signal in Panasonic Schematic | Pico GPIO (`COM10`) | Verified Real Circuit (CAD `NA p.55/62/63/64/83`) | Empirical Measurement (`PINCHECK` / `BUSDIAG` / `COM10`) | Safety Rule |
| :---: | :--- | :---: | :--- | :--- | :--- |
| **1** | `SW_UART` (`Line D`) | `GP26 / ADC0` | Collector of **`Q7510 (2SD0601A0L)`** + Base of **`QR7503 (UNR511300L)`** + `IC7501` pin 80 (`SW_UART`). Base of `Q7510` at `3.30V` (`R7562 4.7 kΩ`) and Emitter (`XFWE`) at `3.30V` (`R7559 10 kΩ`). | `Hi-Z: 3.24–3.25V`, **`PD: 2.68–2.69V`** ($3.30\text{V} - V_{BC}$), `PU: 3.26–3.27V`. | `LOW` forces `serial boot`; **`Hi-Z` enables TX via `Q7514`**. |
| **2** | `XRESET` (`Line E`) | `GP5` | `XRESET` reset network via `QR7503`. | `STRONG HIGH`; `Hi-Z` rise time measured at **`20 µs`**. | Drive strictly as **Open-Drain** (`LOW` / `Hi-Z`). |
| **3** | `UARTM2P_T` (`MAX RXD`) | `GP4` (Pico TX) | `R7570 (10 kΩ)` pull-up to `NSW5V`; enters Collector of **`Q7514`** toward `UARTM2P` (RX pin on Digital PCB `P59001`). | **`0` transitions in `BUSDIAG`**; physically isolated from SoC when `SW_UART = LOW` (since `Q7510` cuts off `Q7514` via `R7528 4.7 kΩ`). | Send TX commands with **`SW_UART = Hi-Z`** (`Q7514` conducting). |
| **4** | `GND` | `GND` | `0V` reference. *(Note: preliminary drafts discussed pin 3 due to reversed pin numbering; in the official `B6B-ZR-3R4` spec on p.62 it is Pin 4).* | `0.00V` | Mandatory common ground. |
| **5** | `NSW5V` (`X_SW_5R2V`, `Line C`) | `GP3` | Supplied by `IC1506 (SI-3011ZFE, ~5.15V)` through `D7510` toward `R7570/R7572 (10 kΩ)` and `IC7501 VDD` (wired-OR with `B7501`); varies in standby/cutoff depending on active rail. | **`0` transitions in `BUSDIAG`** (`NOT SCLK`); safe Hi-Z reading ~`3.29V`. | **ALWAYS `Hi-Z Input` with NO pulls — NEVER drive push-pull!** |
| **6** | `UARTP2M_T` (`MAX TXD`) | `GP2` (Pico RX) | `UARTP2M` output from AM34 SoC via `P59001` with `R7572 (10 kΩ)`. | **Only active UART line:** `872` / `1006` transitions in `250 ms`; **`2124`** (`normal`) / **`3308`** (`JIG`) in `0..1.3 s` @ `115200 8N1`. | `Hi-Z` input + weak internal pull-up. |

* **Bug Discovered in the Pico Firmware (`COM10`):**  
  After running `JIGBOOT`, `UARTMON`, or `UARTSEND`, a subsequent `ISOLATE` command left `GP26` (`VDD_ADC`) configured as SIO/output `LOW`, causing `STATUS` to report **`VDD: 0.02V (LOW)`** even with the board powered. On the bench, running **`UARTPROBE 8` followed by `ISOLATE`** restored `adc_gpio_init(GP26)` $\rightarrow$ `VDD: 2.70V (OK)` (`PINCHECK: 3.25V | 2.69V | 3.27V`). The bug was permanently fixed in the C11 source at `firmware/picofw/hal/picofw_hw.c` (`p7501_hw_read_vdd` and `p7501_hw_isolate_bus_hiz`).

---

## 3. Complete Matrix of 10 Live Captures on `COM10` (`DMR-EH55`, `Loader ver0.61`)

All captures below were preserved in session capture archives (with key bench logs in [`docs/debug_logs/`](../debug_logs/README.md)) and fully decoded by [`scripts/bench/decoders.py`](../../scripts/bench/decoders.py):

| Log File | Size / Encoding | Mode / Path Observed | Live Errors / Milestones Logged | Experimental Conclusion |
| :--- | :---: | :--- | :--- | :--- |
| `normal_boot_90s.log` | `24,362 B` (UTF-16LE) | `***** normal mode *****` | `DDR CHECK: OK`, `Chip select = 1`, **`BGO driver init failed[1].`**, fallback `CD diag` (`0x80020401` $\rightarrow$ `80040904` / `SENSE 83 A7 09 04`). | Proves `normal mode` aborts before reading the HDD due to `[nfrom]`/`BGO` failure. |
| `bootcap_cover_shut_live.log` | `4,568 B` (UTF-8) | `***** normal mode *****` (top cover bolted) | Same path: `BGO driver init failed[1].`, `** can not close tray **`, `80040904` (`83 A7`). | Proved screwing down the top cover magnet clamp does not clear `can not close tray` / `80040904`. |
| `jigboot_20s.log` | `2,026 B` (UTF-8) | `***** serial boot mode *****` | Skips `BGO`, enters `!!! start fmu to read & exec fvu. !!!`, `wait 1 second.`, **`startup reason = [70]`**, `SD init failed[-90001]`. | Confirms `IC7501` unlock (`timer correspond initialize`). |
| `jigboot_70s.log` | `4,157 B` (UTF-8) | `***** serial boot mode *****` | Reaches end of `59..0` countdown in `*** CD diag boot mode ***`: `X(80040904)0,Drive start failed[-2147219196].` + cold reset. | Mapped the full 60-second ATAPI timeout cycle up to reboot. |
| `jigboot_85s_live.log` | `13,353 B` (UTF-8) | `***** serial boot mode *****` | Full sequence: `SD -90001` $\rightarrow$ `F/E Address = 0x90202000` $\rightarrow$ `X(80062900)X(80023a00)` $\rightarrow$ `80040904` (`83 A7`) $\rightarrow$ restart `<00>DDR CHECK: OK`. | Primary evidence of the continuous `JIGBOOT` loop. |
| `sd_fmu_attempt1.log` | `3,019 B` (UTF-8) | `serial boot` + Fujifilm 2GB SD (`MMAGF02GWECA-MB`) | `[SDCD]SetBusWidth:SD Status ERROR -91006`, `SD init failed[-90002]`, continues to `80040904` (`83 A7`). | Slot detects card and advances past `-90001`, failing at `ACMD13`/`SetBusWidth`. |
| `sd_fmu_attempt2.log` | `2,830 B` (UTF-8) | `serial boot` + SD reformatted FAT16 32KB (`FVU`/`PANA_DVD.FRM`) | Same error **`[SDCD]SetBusWidth:SD Status ERROR -91006`** / `-90002`. | Justified disassembling `GetSD_Status` (`0x44027826`) in `BOOT.BIN`. |
| `no_sd_flat_boot.log` | `2,884 B` (UTF-8) | `serial boot` without SD reader FFC | `-91006`/`-90002` persists on slot controller and `80040904` (`83 A7`) is identical. | Rules out short or lockup caused by the front panel/SD FFC. |
| `no_ide_cable_boot.log` | `3,019 B` (UTF-8) | `serial boot` **without IDE ribbon** (StarTech/HDD removed) | Identical (`80040904`, `SENSE 83 A7 09 04`). | **Refuted** hypothesis of a runtime error injected by the SATA/IDE bridge during boot. |
| `no_molex_boot.log` | `2,732 B` (UTF-8) | `serial boot` **without HDD Molex power** | Identical (`80040904`, `SENSE 83 A7 09 04`). | Isolates `80040904` strictly to the optical subsystem (`DRV`/OPU). |

---

## 4. Verified Static Findings in `BOOT.BIN` and `DRV_decomp.bin` (`Steps 1`–`50`)

* **Empirically Proven `MN103S` ISA Rule (`step41_verify_mn103_rel.py`):** In `call16 (0xCD)`, `jmp16 (0xCC)`, `call32 (0xDD)`, `jmp32 (0xDC)`, and `bra8 (0xCA)`, the base `PC` register is the **address of the current instruction** (`target = instruction_va + disp`), proven at `0x44004961 -> 0x44004808 (printf)`, `0x44009cb7 -> 0x44009b11`, and `0x4400b35d -> 0x440063d4`.
* **Exact Path of `BGO driver init failed[1].` (`BOOT.BIN`):** `0x44004abd` $\rightarrow$ `0x4400836f` $\rightarrow$ `0x440041dc` (`0x84380000` autoselect) $\rightarrow$ `0x44001c49` $\rightarrow$ `0x4401e636` returns `1` even before `0x44003fd0` (`Control information init`). Since `0x4400836f` returns `0xFFFFFFFF`, `0x44004acc` skips `0x440065bc(1)` (HDD) straight to `0x44007812`.
* **`wait 1 second.` in `serial boot mode` (`step40` / `step47`):** Executes only `delay_ms(1000)` (`0x440063d4`) followed by `*** timer correspond initialize ***`. Function `0x4400488a` (`uart_gets`, calling `uart_getc` `0x4400479f`) has **0 callers across the entire `BOOT.BIN`**.
* **`** can not close tray **`:** `0x44009c85` sends ATAPI CDB `1B 00 00 00 03` (`START/STOP UNIT`) via `0x44009b11`; any non-zero return code makes `0x44009cbe` print `** can not close tray **`, regardless of physical top-cover position.
* **`GetSD_Status` (`-91006` / `-90002`):** `0x44027eda -> 0x44027826` reads `ACMD13` and rejects `SD_CARD_TYPE` in the structure at `0x50574914` when outside the profile expected by the 2006/2007 loader.
* **Anatomy of `DRV_decomp.bin` (`1,441,792 B` = `0x160000`, decompressed from `DRV.BIN` + `0x40`):**
  - **Factory CLI Table (`0x400df5c0`, 62 `<8sIII` entries):** `[00] SLOGON` through `[61] CDRW`; output is redirected to circular RAM buffer `SLOG` (`0x889f1fe0`), drained to the host only by vendor ATAPI commands (`0x40020b51`, `0x40020be4`, `0x40020c12`).
  - **ATAPI Dispatch Table (`0x400de4b2`, 256 entries):** **74 active opcodes** (`62` SCSI/MMC + `12` Vendor Matsushita `0xE8..0xFE`, plus the LUT at `0x400dea64`).
  - **Sense LUT (`0x400de35c`) and Shift-JIS Mechanical Dictionary (`0x400e779a..0x400e8f50`, stride `0x34`):** Index `0x38` holds `04 09 04`, while `0x400e8c1e` stores `0x83A7` = **`0.5mW発光ができない(HFｵﾝ密調整)`**.

---

## 5. Honest Record of Discarded Hypotheses vs. Obtained Proofs

| Initial / Intermediate Hypothesis During Session | How It Was Tested / Audited | 100% Faithful Final Verdict |
| :--- | :--- | :--- |
| `P7501` was the synchronous JTAG/OCD `DWire32A` interface (`CN55001`). | `PINCHECK` + `BUSDIAG` on Pico + vector CAD of 3 EH55 manuals (`NA`/`EU`/`Vol.2`). | **Refuted for `DMR-EH55` (`P7501`).** `P7501` is Asynchronous UART `115200 8N1` + `SW_UART`/`XRESET`/`NSW5V`. `DWire32A` applies only to legacy OCD headers (`CN55001`). |
| `panasonic_dmr-eh55_flash_dump.bin` (`8 MB`) was a real dump to disassemble in Ghidra. | Entropy audit and byte count in `STEP 13841`. | **Refuted.** It was a synthetic mock for Rust unit tests (`99.999%` `0xFF` bytes). Analysis moved to real `PANA_DVD.FRM`. |
| Partition 2 (`0x04000000`, `720,896 B`) was an `OSD` module. | Zlib decompress + strings + CLI/ATAPI tables (`step5`–`step6`). | **Refuted.** It is the optical drive microcode `DRV.BIN` (`1,441,792 B` decompressed). |
| All `P7501` pins pulled up to `D3R3V` or needed a simple FT232RL. | CAD analysis `NA p.55/62/63/64/83` + `PINCHECK`. | **Refined with precision.** `Pin 5 (NSW5V)` comes from `IC1506 (~5.15V)` / `D7510` (keep `GP3` at `Hi-Z`!); `Pin 1` reads `~3.25V` Hi-Z and clamps to `~2.69V` across `Q7510` B-C junction (`3.30V - V_BC`). |
| `UARTSEND VER` with `SW_UART=LOW` would hit `FMU`/AMOBP prompt during `wait 1 second.`. | Live `UARTSEND` test (`0 bytes`) + CAD `Q7510`/`Q7514` + `step47_final_decode.py`. | **Doubly refuted.** In hardware, `SW_UART=LOW` cuts off `Q7514`, isolating `GP4`. In software, `BOOT.BIN` performs a plain `delay_ms(1000)` and `uart_gets` (`0x4400488a`) has **0 callers**. |
| `80040904` / `** can not close tray **` came from open cover or an IDE flag in NVRAM/EEPROM `IC7404`. | Tests without IDE ribbon/Molex, cover bolted, `step15b`..`step50`. | **Refuted.** Two supposed xrefs (`0x4006f5e1`, `0x401385eb`) were `call32 printf/strlen`; `0x83A7` is physical `0.5 mW` laser emission failure under HF ON (`DRV` + OPU), and `BOOT.BIN` does not access `IC7404` I2C. |
| After `JIGBOOT`/`UARTSEND`, `0.02V` in `STATUS` meant board power-off. | `PINCHECK` + `UARTPROBE 8` comparison on `COM10` + reading `firmware/picofw/hal/picofw_hw.c`. | **Pico firmware bug identified and fixed.** `p7501_hw_isolate_bus_hiz()` and `p7501_hw_read_vdd()` now clear SIO and call `adc_gpio_init(GP26)` when returning to ADC mode. |

---

## 6. Tools Created From This Session

- **Reusable Python Modules:** [`scripts/re/mn10300.py`](../../scripts/re/mn10300.py) and [`scripts/re/pana_frm.py`](../../scripts/re/pana_frm.py)
- **Non-Destructive Bench Ecosystem:** [`scripts/bench/protocol.py`](../../scripts/bench/protocol.py), [`scripts/bench/decoders.py`](../../scripts/bench/decoders.py), and [`scripts/bench/runner.py`](../../scripts/bench/runner.py)
- **Visual Catalog & Script Compendium:** [`VISUAL_CATALOG_EH55_SCHEMATICS_AND_PCB.md`](./VISUAL_CATALOG_EH55_SCHEMATICS_AND_PCB.md) and [`RE_SCRIPTS_COMPENDIUM_STEPS_01_50.md`](./RE_SCRIPTS_COMPENDIUM_STEPS_01_50.md)
