> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/RELATORIO_TECNICO_CLAUDE_OPUS_UNBRICK_DMR_EH55.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# TECHNICAL REVERSE ENGINEERING, CROSS-VALIDATION & LIVE UNBRICK DOSSIER
## Target: Panasonic DMR-EH55 (Matsushita AM34 / MN103S RISC Core)
**Addressee:** Claude Opus / Senior Reverse Engineering Team  
**Document:** `RELATORIO_TECNICO_CLAUDE_OPUS_UNBRICK_DMR_EH55.md` (`TECHNICAL_REPORT_FOR_CLAUDE_OPUS_UNBRICK_DMR_EH55.md`)  
**Date:** November 12, 2026 *(Historical bench snapshot)*  
**Hardware Status:** Connected live via Raspberry Pi Pico (`COM10`) on factory service connector `P7501 (TO WRITE JIGU)` — **`SDDIAG` and `CD diag boot mode` confirmed live!**

> [!NOTE]
> **Historical Record Fidelity Note (2026-11-12 Snapshot):** This file is the faithful English translation of the mid-session technical brief prepared on November 12, 2026. Subsequent disassembly and bench runs (`Steps 5–50`, documented in [`EH55_REVERSE_ENGINEERING_SESSION_FAITHFUL_REPORT.md`](./EH55_REVERSE_ENGINEERING_SESSION_FAITHFUL_REPORT.md) and [`RE_REPORT_FEISTEL_TOC_EXTRHEAD_AND_UART0_GATES.md`](./RE_REPORT_FEISTEL_TOC_EXTRHEAD_AND_UART0_GATES.md)) refined three preliminary hypotheses recorded in Sections 4 and 5 below:
> 1. **`PANA_DVD.FRM` Modules:** Slot 0 (`DRV S211`, `720,896 B`) is the Optical Drive firmware, Slot 1 (`PROG061`, `262,144 B`) is the FVU updater bootloader, and Slot 2 (`MAIN3060NP`) is Main OS Firmware.
> 2. **Feistel Cryptography:** Broken 100% (`16`-round 64-bit Feistel ECB + `PANA_FEISTEL_SBOX_256` + `EXTRHEAD` partial encryption / strided Adler-32).
> 3. **Optical Error `0x80040904` / `0x83A7`:** Proven in `DRV_decomp.bin` (`0x400e8c1e`) to be `0.5mW発光ができない(HFｵﾝ密調整)` (*0.5 mW laser emission failure*), not the `CLOSE SW` mechanical switch.

---

## 0. AUDIT AND TECHNICAL STANDPOINT (VALIDATION OF PRELIMINARY REPORT)

Before presenting the raw captures to Claude Opus, we validated the preliminary survey item-by-item against live hardware on `COM10` and repository binaries.

### 0.1 What Is 100% Validated and Proven on Hardware
1. **True Electrical Mapping of `P7501 (TO WRITE JIGU)`:** Extracted directly from the official Service Manual (Sections `13.8 Timer 1/4` and `13.10 Timer 3/4`). Pin 1 (`SW_UART` / `GP26`) simultaneously drives pin 80 of the Timer MCU (`IC7501`) and the collector of NPN transistor `Q7510`, whose emitter grounds **`XFWE`** toward pin 32 of the Digital PCB.
2. **Exact Cause of `U99` in Normal Mode:** `Loader ver0.61 [06/03/06]` calls `BGO driver init` before initializing front-panel communication (`*** timer correspond initialize ***`). Because NOR partition `[nfrom]`/`BGO` (or `EEMUL` micro-sectors `SA134..SA141`, `0x807F0000..0x807FFFFF`) on the Spansion `S29JL064H` triggers `BGO driver init failed[1].`, boot aborts early and the front panel times out with `U99`.
3. **Bypassing via `JIGBOOT` (`SW_UART = LOW`):** Pulling `SW_UART` low across reset forces the AM34 into **`***** serial boot mode *****`**, **completely bypassing `BGO driver init`**, initializing the front Timer (`Main hung-up time changed 255[s].`), and opening 3 factory recovery vectors in sequence:
   - **Vector 1:** `fmu` over UART (`!!! start fmu to read & exec fvu. !!!`)
   - **Vector 2:** Front SD Card slot (`SD init start...` / VFD showing **`SDDIAG`**)
   - **Vector 3:** ATAPI Optical Drive (`*** CD diag boot mode ***`)

---

### 0.2 Four Critical Corrections & Discoveries Added in This Revision (Argon Standpoint)

> [!IMPORTANT]
> The four points below were absent or incomplete in earlier notes and are **essential** so Claude Opus resolves the problem without hitting dead-end traps:

1. **New Live Capture — `SDDIAG` Mode (`startup reason = [5]`) and 2 GB SD Card Physical Layer Diagnosis (`-91006 / -90002`):**
   - When the user inserted a **2 GB SD card** (initially containing photos, and later `PANA_DVD.FRM` in root) into the DMR-EH55 front SD slot, the VFD displayed **`SDDIAG`**!
   - Live serial telemetry on `COM10` was captured for both runs (Section 3.3 below): driver `[SDCD]` physically detected the mechanical card-present switch (`[SDCD] sdcd_insrmv_check:SD Card inserted`), yet aborted **before mounting the FAT partition** with:
     `[SDCD]This SD Card is't memory card` $\rightarrow$ `[SDCD]SetBusWidth:SD Status ERROR -91006` $\rightarrow$ `SD init failed[-90002].`
   - **Reconstructed C Flow of `sdcd_init()` in `Loader ver0.61`:**
     ```c
     if (!sdcd_insrmv_check()) { /* Reads only mechanical CD pin 20 -> GND on P6802 */
         printf("[SDCD] sdcd_insrmv_check:SD Card remove \n");
         return -90001;
     }
     printf("[SDCD] sdcd_insrmv_check:SD Card inserted \n");
     if (sdcd_check_memory_card() != OK) { /* Sends CMD55 + ACMD41 on SCMD (pin 14) */
         printf("[SDCD]This SD Card is't memory card \n");
     }
     if ((err = sdcd_set_bus_width_4bit()) != 0) { /* Sends ACMD6 + ACMD13 on SCMD/DAT0..3 */
         printf("[SDCD]SetBusWidth:SD Status ERROR %d\n", err); /* -91006 */
         printf("[SDCD] Set SD Bus Width fail ERROR:%d\n", err);
         printf("[SDCD] === This Card can't be controled ==== \n");
         return -90002;
     }
     ```
   - **User-Confirmed Hardware Fact:** The card under test is **neither SDHC nor a microSD adapter**, but an **original legacy full-size Fujifilm 2 GB pure SDSC card**!
   - **Why does an original 2 GB Fujifilm SDSC card fail `sdcd_check_memory_card()` + `SetBusWidth (-91006)` before reading `PANA_DVD.FRM`?**
     Only **two physical/protocol causes** remain:
     1. **Electrical Cause (Most Likely — `P6802`/`P53501` Ribbon, `D3R3V` Rail at `LB6801`, or Oxidation on `P6801` 9 Pins):** In the schematic ([page_113.png](../assets/images/re_dmr_eh55/page_113.png) and [sd_card_schem.png](../assets/images/re_dmr_eh55/sd_card_schem.png)), signal `CD` (`SD Card inserted`) sits at the **outer edge** of the 20-pin FFC (`P53501 Pin 1 / P6802 Pin 20`) and simply grounds to chassis inside the slot when the card shell slides in — triggering even if card power is `0V` or data lines are open! Meanwhile `D3R3V` (`Pins 10–11` via `LB6801`), `SCMD` (`Pin 7/14`), `CLK` (`Pin 5/16`), and `DAT0..3` traverse the center of the 20-pin FFC and the 9 spring contacts inside socket `P6801`. Any drop on `LB6801` or poor contact across `P53501`/`P6802`/`P6801` makes the AM34 print `SD Card inserted` while reading a dead/compromised `DAT`/`SCMD` bus, firing `This SD Card is't memory card` and `-91006`!
     2. **SDSC 2 GB vs 1 GB Protocol Detail (`CSD v1.0 READ_BL_LEN = 10`):** In SD 1.01/1.10 (`CSD_STRUCTURE = 0`), max capacity using 512-byte blocks (`READ_BL_LEN = 9`) is **1 GB** ($4096 \times 512 \times 512\text{ B}$). To encode **2 GB**, 2 GB SDSC cards set **`READ_BL_LEN = 10` ($1024\text{ bytes/block}$)** in `CSD` while using FAT16 with 32 KB clusters.

2. **Why `PANA_DVD.FRM` Module 3 and HDD Partition 1 Have `0` Non-Padding Encrypted Blocks in Common (`0xFF*8` and `0x00*8`):**
   - Statistical analysis showed `PANA_DVD.FRM` and HDD (`panasonic DMR EH-55 Firmware hdd_100.bin`) share identical 8-byte ciphertext blocks for `0xFF*8` (`03 50 65 C8 7B B8 E0 90`) and `0x00*8` (`E5 D1 F3 4A 51 8B 26 90`), proving **64-bit ECB block encryption with the same master key**.
   - Yet body block intersection between Module 3 and HDD Partition 1 is **zero (`0`)**. Why?
   - **Cryptographic Answer:** As proven by the boot log (`uncompress dst=90202000 len=160000 src=84080040 len=acc01 HEADER:5627DD54, DECOMP:5627DD54`), **Panasonic compresses binaries (`zlib` / `LZMA`) BEFORE applying padding (`0xFF`/`0x00`) and Feistel ECB encryption!** Across compressed streams, any tiny firmware revision delta shifts the compressor's sliding window, altering 100% of body 8-byte ciphertext blocks while leaving only trailing padding blocks identical!

3. **Repository Trap — Archived `panasonic_dmr-eh55_flash_dump.bin` (`8 MB`) Is NOT a Real Flash Dump:**
   - Byte-by-byte inspection proved that the earlier synthetic fixture (`panasonic_dmr-eh55_flash_dump.bin`) held only **99 non-`0xFF` bytes** across 8 MB (an ASCII stub header `PANASONIC DMR-EH55 AM34 IPL BOOTLOADER VEP79138` in the first 63 bytes plus `55 AA 10 00` markers every 8 KB across `0x7F0000..0x7FE007`).
   - Thus, **the native plaintext code of `Loader ver0.61 [06/03/06]` still resides strictly inside the physical Spansion `S29JL064H` (`IC5001`) chip on the bench board**.

4. **Electronic Trap During `wait 1 second.` (`fmu` Serial) — `Q7514` Cutoff Under `XFWE = LOW`:**
   - In the official schematic (`Timer 3/4`, [na_p64_topleft.png](../assets/images/re_dmr_eh55/na_p64_topleft.png)), `P7501` Pin 3 (`MAX RXD` / `GP4`) enters the collector of NPN transistor **`Q7514`**, whose emitter drives `UARTM2P` (AM34 RX).
   - However, **the Base of `Q7514` is biased through `R7528 (4.7 kΩ)` from the `XFWE` line itself!**
   - Consequently, **while the Pico holds `GP26` (`SW_UART`) `LOW (0V)`, `XFWE` sits at `~0.1V` and `Q7514` is in CUTOFF (open switch), blocking every byte sent by `GP4` from reaching the AM34!**
   - To transmit commands over `GP4`, the Pico must:
     1. Hold `GP26 = LOW` only across `XRESET` (`GP5`) until AM34 latches boot mode (`serial boot mode`).
     2. **Keep `GP3` (`Pin 5: NSW5V` / Line `C`) ALWAYS `Hi-Z Input` with pulls disabled (NEVER drive push-pull!)**, since `IC1506` drives `NSW5V` from `~5.15V` via `D7510` while the recorder is powered.
     3. **Release `GP26` to `Hi-Z`** before transmitting on `GP4`, letting `XFWE` rise and bias `Q7514` ON!

---

## 1. HARDWARE ARCHITECTURE & SYSTEM TOPOLOGY

* **Main SoC:** **Matsushita `AM34` / `MN103S`** (32-bit RISC core).
* **System NOR Flash:** Spansion `S29JL064H` (8 MB / 64 Mbit, TSOP-48, `IC5001` on Digital PCB).
* **Top-Boot NVRAM / EEMUL:** Top micro-sectors `SA134` through `SA141` (`0x807F0000` to `0x807FFFFF`, 64 KB).
* **Front-Panel / Timer MCU:** Matsushita `IC7501` (controls VFD display, front keys, SD card detect `CD`, and reset chain).
* **Optical Drive:** Proprietary Panasonic ATAPI/IDE DVD-RAM drive (Servo DSP `MN102`).
* **Hard Disk Drive (HDD):** PATA/IDE attached via StarTech SATA $\leftrightarrow$ IDE adapter jumpered as Master.

```mermaid
flowchart TD
    PICO["Raspberry Pi Pico (RP2040 on COM10)"] -- "6 Pins (B6B-ZR-3R4)" --> JIG["P7501: TO JIGU"]
    
    subgraph TIMER_PCB ["Timer PCB (Front Panel)"]
        JIG -- "Pin 1: SW_UART (GP26)" --> Q7510["NPN Q7510 (2SD0601A)"]
        JIG -- "Pin 1: SW_UART (GP26)" --> IC7501_80["IC7501 Pin 80 (SW_UART)"]
        JIG -- "Pin 2: XRESET (GP5 Open-Drain)" --> QR7503["Reset Supervisor QR7503"]
        JIG -- "Pin 3: MAX RXD (GP4 TX)" --> Q7514["NPN Q7514 (Base via R7528 <- XFWE)"]
        JIG -- "Pin 5: NSW5V (GP3 Hi-Z!)" --> PULLUPS["Pull-ups R7527, R7559, R7562"]
        JIG -- "Pin 6: MAX TXD (GP2 RX)" <-- AM34_TX["UART TX 115200 8N1"]
        IC7501_80 <--> QR7503
    end

    subgraph DIGITAL_PCB ["Digital PCB (Main Logic Board)"]
        Q7510 -- "XFWE Line (Pin 32)" --> AM34["Matsushita AM34 SoC (MN103S RISC)"]
        Q7510 -- "Biases Base via R7528" --> Q7514
        Q7514 -- "UARTM2P Line (RXD)" --> AM34
        AM34 -- "16-bit Parallel Bus (Native Code)" --> FLASH["Spansion S29JL064H (8 MB Flash IC5001)"]
        AM34 -- "PATA/IDE Bus" --> OPTICAL["DVD-RAM Drive (MN102 Servo)"]
        AM34 -- "PATA/IDE Bus" --> HDD["HDD via StarTech Bridge (Master)"]
        AM34 -- "SD Bus (P53501 <-> P6802)" --> SDCARD["Front SD Card Slot (P6801)"]
    end
```

---

## 2. EXACT PINOUT OF CONNECTOR `P7501 (TO WRITE JIGU)`

| Pin on `P7501` | Panasonic Signal | Pico Pin | Circuit & Function on DMR-EH55 Hardware |
| :---: | :---: | :---: | :--- |
| **1** | `SW_UART` | `GP26` | Connects to Line `(D)`, **`IC7501` Pin 80 (`SW_UART`)**, and **`Q7510` (`2SD0601A0L`)** Collector. Pulling `LOW` grounds **`XFWE`** (forcing `serial boot mode` on AM34 and `SDDIAG` on front VFD), while cutting off `Q7514`. |
| **2** | `XRESET` | `GP5` | Connects to Line `(E)` / `QR7503` Collector. Master system reset (active low, always drive Open-Drain). |
| **3** | `MAX RXD` | `GP4` | Connects to `UARTM2P_T` / **`Q7514`** Collector. Emitter feeds `UARTM2P` (AM34 RX). Conducts only when `XFWE` is HIGH (`R7528 4.7 kΩ`). |
| **4** | `GND` | `GND` | Common ground (`0V`). |
| **5** | `NSW5V` | `GP3` (**ALWAYS `Hi-Z`!**) | Line `(C)` fed at `~5.15V` from `IC1506` via `D7510` to tops of `R7527 (4.7 kΩ)`, `R7559 (10 kΩ)`, and `R7562 (4.7 kΩ)` (`~3.29V` idle). **MUST stay `Hi-Z Input` with NO pulls (NEVER push-pull!)** |
| **6** | `MAX TXD` | `GP2` | `UARTP2M_T` net via `R7535 (100 Ω)` directly from Matsushita AM34 serial output `UARTP2M` (`115200 8N1`). |

---

## 3. THE 3 INITIAL LIVE TELEMETRY CAPTURES ON `COM10`

### 3.1 Capture 1: Normal Mode (`BOOTCAP` — `SW_UART` Hi-Z $\rightarrow$ `U99` Error)

```text
DDR CHECK: OK

Chip select = 1
F/E Address = 0x90202000 , Drive(compress) Size = 0xACC50
uncompress dst=90202000 len=160000 src=84080040 len=acc01
HEADER:5627DD54, DECOMP:5627DD54

F/E reset release finish ...

***** Loader ver0.61 [06/03/06] *****
***** normal mode *****
BGO driver init failed[1].
[RTSD] ATAPI Software Reset 
[RTSD] Device Reset Occurred(0x62900) 
** can not close tray **
** startup drive **
X(80020401)59,X(80020401)58,X(80020401)57,X(80020401)56,X(80020401)55,X(80020401)54,X(80020401)53,
[RTSD] === DRIVE ACCESS ERROR === ( 80040904 )
*** CDB  [00 00 00 00 00 00 00 00 00 00 00 00] 
*** SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00] 
H_STAT [0] T_STAT [2]
X(80040904)52,X(80040904)51,...,X(80040904)0,Drive start failed[-2147219196].
```

---

### 3.2 Capture 2: Serial Mode WITHOUT SD Card (`JIGBOOT` — `SW_UART = LOW`, `startup reason = [70]`)

```text
DDR CHECK: OK

Chip select = 1

***** Loader ver0.61 [06/03/06] *****
***** serial boot mode *****

!!! start fmu to read & exec fvu. !!!
wait 1 second.
*** timer correspond initialize ***
startup reason = [70]
Main hung-up time changed 255[s].
SD init start...
[SDCD] sdcd_insrmv_check:SD Card remove 
[SDCD] === This Card removed ==== 

... SD init end
SD init failed[-90001].
*** CD diag boot mode ***
F/E Address = 0x90202000 , Drive(compress) Size = 0xACC50
uncompress dst=90202000 len=160000 src=84080040 len=acc01
HEADER:5627DD54, DECOMP:5627DD54

F/E reset release finish ...
*** RTSC Initialize ***
[RTSD] ATAPI Software Reset 
[RTSD] Device Reset Occurred(0x62900) 
X(80062900)X(80023a00)** startup drive **
X(80020401)59,...,X(80040904)0,Drive start failed[-2147219196].
```

---

### 3.3 Capture 3: Serial Mode WITH Photo SD Card Inserted (`JIGBOOT` $\rightarrow$ VFD `SDDIAG`, `startup reason = [5]`)

```text
DDR CHECK: OK

Chip select = 1

***** Loader ver0.61 [06/03/06] *****
***** serial boot mode *****

!!! start fmu to read & exec fvu. !!!
wait 1 second.
*** timer correspond initialize ***
startup reason = [5]
Main hung-up time changed 255[s].
SD init start...
[SDCD] sdcd_insrmv_check:SD Card inserted 
[SDCD]This SD Card is't memory card 
[SDCD]SetBusWidth:SD Status ERROR -91006
[SDCD] Set SD Bus Width fail ERROR:-91006
[SDCD] === This Card can't be controled ==== 

... SD init end
SD init failed[-90002].
*** CD diag boot mode ***
F/E Address = 0x90202000 , Drive(compress) Size = 0xACC50
uncompress dst=90202000 len=160000 src=84080040 len=acc01
HEADER:5627DD54, DECOMP:5627DD54

F/E reset release finish ...
*** RTSC Initialize ***
[RTSD] ATAPI Software Reset 
[RTSD] Device Reset Occurred(0x62900) 
X(80062900)X(80023a00)** startup drive **
X(80020401)59,X(80020401)58,X(80020401)57,X(80020401)56,X(80020401)55,X(80020401)54,
```

**Comparative Table of `[SDCD]` Driver Return Codes:**
| Scenario in Front SD Slot (`P6801`) | `startup reason` | `[SDCD]` Message | Final Code |
| :--- | :---: | :--- | :---: |
| No card inserted | `[70]` | `sdcd_insrmv_check:SD Card remove` | `SD init failed[-90001].` |
| Modern SDHC/SDXC ($>2\text{ GB}$) or bus fault | **`[5]` (`SDDIAG`)** | `sdcd_insrmv_check:SD Card inserted` $\rightarrow$ `This SD Card is't memory card` $\rightarrow$ `SetBusWidth:SD Status ERROR -91006` | `SD init failed[-90002].` |
| Standard **SDSC ($\le 2\text{ GB}$, FAT12/FAT16)** | **`[5]` (`SDDIAG`)** | Full SD 1.x init (`ACMD41` + `ACMD6` 4-bit OK) | Searches for `DIGADIAG.FRM` / `fvu` firmware on SD card! |

---

## 4. FORENSIC ANALYSIS OF OPTICAL DRIVE ATAPI ERROR (`80040904`)

* **Issued SCSI Command (CDB):** `[00 00 00 00 00 00 00 00 00 00 00 00]` (`TEST UNIT READY`).
* **Request Sense Response:** `70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00`
  * **Sense Key `04`:** *Hardware Error*.
  * **ASC `09` / ASCQ `04`:** Servo/mechanism failure (`0x80040904` = `-2147219196`), Matsushita specific subcode `83 A7` (subsequently decoded from `DRV_decomp.bin` at `0x400e8c1e` as `0.5mW発光ができない(HFｵﾝ密調整)` — *0.5 mW laser emission failure*).
* **Initial Physical Hypothesis (2026-11-12):** At the time of this capture, inspection of [page_113.png](../assets/images/re_dmr_eh55/page_113.png) examined `FP50004` (`Loading Motor` + `OPEN_SW`/`CLOSE SW`) adjacent to `P7501`; later tests with bolted cover and `DRV_decomp.bin` Shift-JIS table decoding isolated `80040904`/`0x83A7` strictly to laser emission failure in the optical pickup/drive (`DRV`/OPU).

---

## 5. PRELIMINARY CRYPTANALYSIS OF `PANA_DVD.FRM` & HDD PARTITIONS (HISTORICAL)

### 5.1 Structure of `PANA_DVD.FRM` (`7,531,744` bytes)
* **Global Header (`0x000000..0x000010`, 16 bytes):**
  `49 2F 05 60 CE DE 50 8C E3 C0 8A FD 49 B6 4A 9A`
* **Table of Contents (`0x000010..0x0000A0`, 3 entries $\times$ 48 bytes = 144 bytes):**
  * **Entry 0 (`0x0010`):** `EFDC1CDA13881BAB` | `74A78ACD789B84FF` | `2A95C0DDCD1C5FF1` | `696A334256D61E54` | `212C29514C08699AC00EBF203F432332`
  * **Entry 1 (`0x0040`):** `EFDC1CDA13881BAB` | `3B7E7EDBB6C57A53` | `23CB732B8CDAA69D` | `A126CE849FEF6DB3` | `20861ED612D3E904ADB8D5CA7B848D76`
  * **Entry 2 (`0x0070`):** `EFDC1CDA13881BAB` | `74A78ACD789B84FF` | `40DEDBF6662D4CB1` | `1AE6ADB49339BA00` | `83849E9CEC5502D951D74238823D31BB`
* **Module Correlation (Historical Note — Later Decrypted 100% into `DRV S211`, `PROG061`, `MAIN3060NP`):**
  * **Slot 0 (`0x000010` TOC / payload `0x001800..0x0B1800`):** Initially hypothesized as OSD; later proven byte-for-byte identical to `EH55_DRV_S211.bin` (`DRV S211`, `720,896 B`) $\leftrightarrow$ **HDD Partition 2** (`0x04000000`).
  * **Slot 1 (`0x0B1800..0x0F1800`, `262,144 B`):** `PROG061` FVU updater bootloader.
  * **Slot 2 (`0x0F1800..0x72EC00`, `6,542,560 B`):** Main Firmware (`MAIN3060NP`) $\leftrightarrow$ **HDD Partition 1** (`0x02000000`, `6,173,264 B`).

### 5.2 Confirmed Plaintext–Ciphertext Pairs (64-bit ECB KPA)
| Encrypted Block (8 Bytes) | Plaintext | Occurrences in `.FRM` | Occurrences in HDD | Meaning |
| :---: | :---: | :---: | :---: | :--- |
| `03 50 65 C8 7B B8 E0 90` | `FF FF FF FF FF FF FF FF` | **3,812** | **1,280** | Virgin NOR Flash padding (`0xFF * 8`) |
| `E5 D1 F3 4A 51 8B 26 90` | `00 00 00 00 00 00 00 00` | **196** | **3** | Zero padding (`0x00 * 8`, including `+0x38` and `+0x30` in HDD Partitions 1 & 2) |
| `4D 3C 5D BB 0C 35 77 77` | Fixed block descriptor | **1,280** | 0 | Inter-module padding pattern (`0x0AEFD0..0x0B17D0`) |

---

## 6. STRATEGIC ACTION PLAN PROPOSED ON NOVEMBER 12, 2026

### Priority 1: Exploit `SDDIAG` with SDSC ($\le 2\text{ GB}$, FAT12/FAT16) or Pico SD Sniffer
Since the board confirmed entering **`SDDIAG`** (`startup reason = [5]`) and printing `[SDCD] sdcd_insrmv_check:SD Card inserted` *before* spinning up the optical drive:
1. **Real SDSC Card ($\le 2\text{ GB}$):** Format a standard SDSC card (`FAT16`) and test `DIGADIAG.FRM` in root while capturing `COM10` via `JIGBOOT`. *(Subsequent bench outcome proved: the `2 GB` Fujifilm card initialized 4-bit at `25000 Kbps` after `-91006` ribbon fix; real `Loader ver0.61` rejected `131,072 B` `EXTRHEAD` with `SD diag load failed[0].` and executed **raw `262,144 B` `DIGADIAG.FRM`** into `***** Fvu ver0.61 [05/11/01] *****`, motivating the canonical `ver061` + `sd48` kit in `firmware_and_images/sd_unbrick_kit/` and MN103S compiler `CALL ..., [], 0` ABI fixes for `PANAMON1`.)*
2. **Advanced Sniffer Idea (`SDSNIFF`):** Tap `CLK`, `CMD`, and `DAT0..DAT3` on `P6801`/`P6802` to trace LBA reads during `SDDIAG`.

### Priority 2: Bypass or Inspect `*** CD diag boot mode ***`
Investigate `FP50004` / `CLOSE SW` vs optical pickup `0x83A7` (`ATAPI` / `DRV`). *(Later resolved as laser `0x83A7`, making `SDDIAG` via `DIGADIAG.FRM` and `PANARSP PASSTHROUGH` the primary diskless/no-ODD paths.)*

### Priority 3: `fmu` Handshake Check During `wait 1 second.` via `Hi-Z` `GP26` / `Q7514`
In [`p7501_pico_main.c`](../../firmware/picofw/picofw_pico_main.c), keep **`GP3` (`NSW5V`) strictly `Hi-Z Input`** and release `GP26` (`SW_UART`) to `Hi-Z` after `serial boot mode` entry so `Q7514` conducts. *(Subsequent disassembly in Step 40/47 proved `BOOT.BIN` `uart_gets` has `0` callers in `wait 1 second.`, whereas `_dev_SERIAL` post-MAIN exposes full RSP control.)*

### Priority 4: Extract Native `Loader ver0.61` from NOR Flash `S29JL064H` (`IC5001`) — **COMPLETED ON BENCH**
Dump `IC5001` (`0x80000000..0x80800000`, or `0x84000000` uncached via `PANADMP1`) to recover the exact board-specific `Loader ver0.61`, `[nfrom]`/`BGO` state, and `EEMUL` NVRAM sectors. *(Bench completion record: sliced read-only `PANADMP1` capture via `PANAMON1>` patching uncached `0x9043950E` $\rightarrow$ `J 904394FA` produced [`eh55_nor_8mb_ic5001_dump.bin`](../../firmware_and_images/README.md) [`8,388,608 B`, `SHA-256: c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`], `eh55_loader_ver061_256k.bin` [`262,144 B`, `9f5c448d...`], and `eh55_nvram_64k_raw.bin` [`65,536 B`, `803d1465...`]. Table `0x440337A4` confirmed real EH55 `ver0.61` BGO occupies `45×64 KiB` sectors at `0x84130000..0x843FFFFF` [`SA26..SA70` vs `ver0.63`/`PROG061` `0x84200000..0x843FFFFF`], separate from Upper EEMUL `"1284"` [`SA71..SA141`, `0x84400000..0x847FFFFF`]. Formatting `SA70` + `1,629` pure `1->0` writes across 13 spares [`SA42..SA66`] + `6` bytes at `SA41+0x14` eliminated `BGO driver init failed[1].` (`U99`) and booted live on `COM10` through `***** normal mode *****` $\rightarrow$ `*** RTSC Initialize ***`.)*
