> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/GUIA_BANCADA_P7501_SEM_DESSOLDA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# SURGICAL BENCH GUIDE: NO-DESOLDERING `P7501` ASYNC UART CONNECTION
## Diagnostics, Boot Capture & Bench Preparation — Panasonic DMR-EH55 (`VEP79138`)
### Matsushita `AM34` / `MN103S` + Syscon `IC7501` $\leftrightarrow$ Raspberry Pi Pico (`RP2040` / `RP2350`)

---

## 1. Introduction & Technical Scope

This manual details the safe bench procedure for connecting a **Raspberry Pi Pico (`RP2040`)** or **Pico 2 (`RP2350`)** to the 6-pin factory service header **`P7501` (`JST B6B-ZR-3R4`, labeled `"TO JIGU"`)** on board `VEP79138` inside the **Panasonic DIGA DMR-EH55** recorder, enabling **`115200 8N1`** boot capture, complete electrical line diagnostics, discrete boot/reset control, and a GDB RSP bridge without soldering on or desoldering components from the main PCB.

> [!IMPORTANT]
> **Verified Physical Alignment (`P7501` Async UART vs. No-Desoldering `SDDIAG`/`PANAMON1>` Recovery vs. `IC5001` Fallback):**
> Cross-verification across **5 official DMR-EH55 service manuals (`30` indexed technical PDFs)** and live bench measurements proved that **`P7501`** is an **Asynchronous UART port (`115200 8N1`)** coupled to discrete stage `Q7510` / `Q7514` / `QR7503` / `IC1506` / `D7510`.
> - The Pico probe on `P7501` captures live boot telemetry (`<00>DDR CHECK: OK`, `SD init`, `CD diag boot mode`), verifies electrical integrity across all 6 pins (`PINCHECK`, `BAUDSCAN`, `HEXCAP`, `PWRTRACE`, `UARTSTREAM`, `FMUSCAN`), and opens the `GDB` RSP bridge without desoldering.
> - **Primary No-Desoldering `U99` Path (`SDDIAG` + `PANAMON1>`):** On real `DMR-EH55` hardware (`Loader ver0.61 [06/03/06]`), `U99` typically stems from corruption inside the 45-sector **BGO FTL (`SA26..SA70`, `0x84130000..0x843FFFFF` — `BGO driver init failed[1].`)** and/or top `EEMUL` (`SA134..SA141`), and was **cured in 2 cycles directly over `P7501` + SDSC (`DIGADIAG.FRM` `262,144 B` raw `@ 0x50400000` + `PANA__SD.FRM` `sd48`, or headless Door #1 `@ 0x0B1800` → `PANAMON1>`) without desoldering `IC5001`**.
> - Removing Spansion `S29JL064H70TFI00` (**`IC5001`**) for external TSOP-48 programming (**XGecu T48** / **SVOD 4** + `panadiga nvram clean`) is reserved strictly as a **last-resort fallback** if Layer 1 IPL (`SA0..SA10`) or the SD card slot is physically damaged.

> [!WARNING]
> **Compliance & Risk Assessment: Why In-Circuit Recovery Is Worth Attempting on an Already-Bricked (`U99`) Unit:**
> * **Real operational hazards you must respect:** (1) AC power loss or disconnect during Flash writes; (2) real ~`700 ms` 64 KiB sector erase time on Spansion `S29JL064H` (requiring `0xFF × 32` readback polling after `OK:ERASE` before `'W'` programming); (3) Syscon `IC7501` `~44s` watchdog (`PLEASE WAIT` / `SELF CHECK`, pulsing `NSW5V=0` and `XRESET=0` if no valid SDSC or `TBUS` keepalive is active); and (4) **mandatory full 8 MiB read-only backup (`PANADMP1`) verified by SHA-256** prior to unlocking Layer 2 (`U CONFIRM_UPPER_NOR`).
> * **Why risk-benefit compliance favors trying in-circuit first on a bricked unit:** **Layer 1 Critical Boot IPL (`SA0..SA10`, opening `256 KiB`: `0x80000000..0x8003FFFF` / `0x84000000..0x8403FFFF`) is permanently write-locked (`E05` RO)** on the Pico probe, guaranteeing `Loader ver0.61` (`SDDIAG` + `P7501`) cannot be overwritten in-circuit. On an already-bricked `U99` unit, any interrupted Layer 2/3 operation remains re-entrant via SD/UART — avoiding the far higher physical risk of lifted SMD pads from hot-air TSOP-48 rework (`RSK-01`).

---

## 1.1 Universal Compatibility with RP2040 and RP2350 Boards (Pico 1 & Pico 2)

> [!IMPORTANT]
> **Full Compatibility with Raspberry Pi Pico (RP2040) and Raspberry Pi Pico 2 (RP2350):**
> The Panadiga ecosystem officially supports **Pico 1 (`RP2040` / `Pico W`)**, **Pico 2 (`RP2350` / `Pico 2 W`)**, and compatible silicon variants:
>
> * **RP2350 Advantages on the Bench:**
>   * **520 KB SRAM (vs. 264 KB on RP2040):** Expanded buffer for continuous high-speed serial captures and GDB RSP caching.
>   * **150 MHz Clock & PIO 2.0:** Deterministic `115200 8N1` UART sampling with `8x` oversampling (`921,600 Hz`) in `p7501_uart.pio` / `p7501_uart_duplex.pio`.
>   * **Native `3.3V` Logic + `NSW5V` (`GP3`) Guard:** Control/UART signals on `P7501` run at `3.3V LVTTL`, while Pin 5 (`NSW5V`) connects to the `~5.15V` upstream node from `IC1506` via `D7510` — and **MUST ALWAYS** remain a pull-disabled `Hi-Z Input` on the RP2040/RP2350.

---

## 1.2 Engineering Warning: `P7501` Async UART (`115200 8N1`) vs. Legacy JTAG / `DWire32A`

> [!CAUTION]
> **Why you must NOT use generic JTAG firmwares, passive 3-wire USB-TTL adapters without Open-Drain, or synchronous `DWire32A` protocols on `P7501`:**
>
> 1. **Actual DMR-EH55 Interface:** `P7501` is **NOT** JTAG (`TDI/TDO/TMS/TCK`) or synchronous `DWire32A` (`SDATA`/`SCLK`). It is `115200 8N1` Asynchronous UART (`UARTP2M_T` & `UARTM2P_T`) paired with discrete boot/reset signals (`SW_UART`, `XRESET`) and switched rail `NSW5V`.
> 2. **TX Blocking via `Q7514`:** Pulling `SW_UART` (Pin 1) `LOW` causes `Q7510` to pull `XFWE` low and force serial boot (`CD diag boot mode` / `SDDIAG`), while simultaneously placing `Q7514` into cutoff and disconnecting Pin 3 (`UARTM2P_T`) from the CPU. TX bits from `GP4` cross `Q7514` only when `SW_UART = Hi-Z`.
> 3. **Damage Hazard on `NSW5V` and `XRESET`:** Feeding external power or forcing push-pull HIGH on Pin 5 (`NSW5V`, `~5.15V` upstream) or Pin 2 (`XRESET`, `QR7503`) risks damaging either the Pico or board `VEP79138`. Exclusively run the native `panasonic_p7501_uart` firmware.

---

## 2. Connector Mechanical & Commercial Standard (`JST ZR 1.5mm` 6-Pin - `B6B-ZR-3R4`)

Service header **`P7501`** soldered on the DMR-EH55 Main PCB (**`VEP79138`**) is a **6-pin JST ZR connector with a `1.5 mm` pitch (`B6B-ZR-3R4`)**, labeled **`TO JIGU`** in the official Panasonic schematic.

### 2.1 Buying the Right Harness (Plug-and-Play Without Micro-Soldering)
To mate cleanly with the Raspberry Pi Pico without soldering directly to SMD header pins:
* **Recommended Search Terms:** `JST ZR / ZH 1.5mm 6-pin Female to Dupont 2.54mm Male jumper cable` (`ZHR-6` female housing with `SZH-002T-P0.5` crimp contacts).
* **Pitch (Pin-to-Pin Spacing):** Strictly **`1.5 mm`** center-to-center.
* **Incompatible Standards (Do NOT Force):** Never force **JST SH (`1.0 mm`)**, **JST PH (`2.0 mm`)**, or **JST XH (`2.54 mm`)**.
* **Physical Orientation:** The square solder pad on the PCB identifies **Pin 6 (`UARTP2M_T` / `MAX TXD`)**, sitting closest to FFC connector **`FP50003`**.

---

## 2.2 Canonical Verified Pinout of Connector `P7501` (`VEP79138`)

> [!CAUTION]
> **SCHEMATIC & BENCH GROUND TRUTH (`P7501`):**
> * **Pin 4 = `GND` ($0.0\,\Omega$ direct to metal chassis).**
> * **Pin 6 = `UARTP2M_T` (`MAX TXD`, SQUARE pad on PCB adjacent to `FP50003`) — active board UART TX output (`115200 8N1`).**
> * **Pin 5 = `NSW5V` (`X_SW_5R2V`) — static rail reading `~3.29V` idle (`~5.15V` upstream via `D7510` / `IC1506`). NEVER ground or drive push-pull!**

```text
          CANONICAL P7501 HEADER [B6B-ZR-3R4] ("TO JIGU") — VEP79138
                     ┌───────────────────────────────────────────┐
      Pin 1          │  [1] SW_UART   (~3.25V Hi-Z / LOW=XFWE)   │
      Pin 2          │  [2] XRESET    (Open-Drain Reset ~20µs)   │
      Pin 3          │  [3] UARTM2P_T (MAX RXD -> Pico GP4 TX)   │
      Pin 4 (GND)    │  [4] GND       (0.0Ω to Metal Chassis!)   │
      Pin 5          │  [5] NSW5V     (KEEP GP3 AT Hi-Z!)        │
      Pin 6 (Square) │  [6] UARTP2M_T (MAX TXD -> Pico GP2 RX)   │
                     └───────────────────────────────────────────┘
```

### Pinout & Measured Electrical Behavior Table (`COM10` + NA/EU Manuals)

| `P7501` Pin | Schematic Signal | Board Circuit (`VEP79138`) | Measured Level / Behavior | Pico Pin (`RP2040/RP2350`) | Role in Recovery / Diagnostics |
| :---: | :--- | :--- | :--- | :---: | :--- |
| **Pin 1** | **`SW_UART`** `(D)` | `Q7510` Collector, `QR7503` Base, `IC7501` Pin 80 | `3.24–3.25V` Hi-Z / `2.68–2.69V` PD | **`GP26 / ADC0`** (Pin 31) | Forces serial/JIG boot (`XFWE`) when `LOW` (cuts `Q7514`); keep `Hi-Z` for active TX. |
| **Pin 2** | **`XRESET`** | `QR7503` reset network | Strict Open-Drain (`0V` / `Hi-Z`, `~20 µs` rise) | **`GP5`** (Pin 7) | Target master reset. Never drive `+3.3V` push-pull! |
| **Pin 3** | **`UARTM2P_T`** (`MAX RXD`) | Driver via `Q7514` (`R7570 10 kΩ`) | `3.3V` LVTTL `115200 8N1` | **`GP4`** (Pin 6, TX) | Probe TX to AM34 RX (`SW_UART=Hi-Z` to conduct). No external pull-up. |
| **Pin 4** | **`GND`** | `GND_DIGITAL` | **`0.0V`** ($0.0\,\Omega$ to chassis) | **`GND`** (Pin 38) | Shared `0V` bench reference. |
| **Pin 5** | **`NSW5V`** (`X_SW_5R2V`) | `IC1506` (`~5.15V`) via `D7510` | `~3.29V` idle / `~5.15V` upstream | **`GP3`** (Pin 5) | Feeds `R7570`/`R7572`/`IC7501`. **ALWAYS KEEP `Hi-Z Input` with NO pulls!** |
| **Pin 6** | **`UARTP2M_T`** (`MAX TXD`) | `R7535` / `R7572 10 kΩ` | Active `3.3V` LVTTL `115200 8N1` boot UART | **`GP2`** (Pin 4, RX) | Only pin carrying active UART TX (`<00>DDR CHECK: OK`, `CD diag`). Square pad. |

### 3-Step Multimeter Check Before Plugging In the Pico
With the recorder **unplugged from AC mains** and the **BLACK probe on the metal chassis**:
1. **Verify `Pin 4 (GND)`:** In continuity mode, confirm **$0.0\,\Omega$** between **`P7501` Pin 4** and chassis ground.
2. **Verify `Pin 6 (UARTP2M_T)`:** Visually confirm the **square PCB solder pad** closest to FFC `FP50003`.
3. **Measure Idle Voltages (Standby/Powered):** With multimeter on DC `20V`, verify `Pin 1 (SW_UART) ≈ 3.25V`, `Pin 5 (NSW5V) ≈ 3.29V`, and no shorts between adjacent pins before plugging Pico USB.

---

## 2.3 Critical Hardware Verification: Spansion `S29JL064H` (`IC5001`) Pin 14 (`WP#/ACC`)

> [!CAUTION]
> **Hardware Write-Protect Lock:**
> Per the **Official Spansion S29JL064H Datasheet** (*Top Boot Block Architecture* section):
> * **Pin 14 (`WP#/ACC`)** on Flash chip **`IC5001`** hardware-protects the top two 8 KB micro-sectors: **`SA140` (`0x007FC000`)** and **`SA141` (`0x007FE000`)**.
> * If Pin 14 is pulled to ground (`0.0V`), JEDEC *Sector Erase* (`0x30`) commands targeting those sectors are ignored internally by the Flash array.
> * Before any in-circuit Flash write procedure, confirm $+3.3\text{V}$ on `IC5001` Pin 14 with a multimeter.

---

## 3. Bench Wiring Diagram: RP2040 / RP2350 $\leftrightarrow$ `P7501`

Board `VEP79138` **already integrates** pull-up resistors `R7570` (`10 kΩ`) and `R7572` (`10 kΩ`) tied to `NSW5V`, plus `R7562` (`4.7 kΩ`) and `R7559` (`10 kΩ`) on `Q7510`. Therefore, **do NOT add an external `4.7 kΩ` pull-up**.

### 3.1 Complete Bench Wiring Schematic

```text
       RASPBERRY PI PICO (RP2040 / RP2350)                           MAIN BOARD VEP79138
               BENCH CONTROLLER                                    CANONICAL HEADER P7501
    ┌───────────────────────────────┐                             ┌──────────────────────────────┐
    │                               │                             │                              │
    │  Pin 31 (GP26 / ADC0) ────────┼◄──[ R_PROT: 1.0kΩ (opt) ]───├───◄ Pin 1: SW_UART (D)       │
    │  (Boot Latch / ~3.25V Sense)  │     Q7510/R7562/R7559       │     (LOW->XFWE; Hi-Z->TX)    │
    │                               │                             │                              │
    │  Pin 7  (GP5 / XRESET) ───────┼───[ R_SER: 100Ω (opt) ]─────├───► Pin 2: XRESET            │
    │  (Emulated Open-Drain)        │     LOW=Reset / Hi-Z=Run    │     (QR7503, ~20µs rise)     │
    │                               │                             │                              │
    │  Pin 6  (GP4 / UART TX) ──────┼───[ R_SER: 100Ω (opt) ]─────├───► Pin 3: UARTM2P_T         │
    │  (115200 8N1 -> AM34 RX)      │     Onboard R7570 10kΩ!     │     (MAX RXD via Q7514)      │
    │                               │                             │                              │
    │  Pin 38 (GND / 0V Ground) ────┼─────────────────────────────┼════ Pin 4: Common GND        │
    │  (Chassis 0V Reference)       │   DIRECT GROUND TIE 0.0Ω    │     (0.0Ω to Chassis!)       │
    │                               │                             │                              │
    │  Pin 5  (GP3 / NSW5V Sense) ──┼◄──[ Hi-Z Input / NO PULLS ]─├───◄ Pin 5: NSW5V (X_SW_5R2V) │
    │  (NEVER Drive Push-Pull!)     │     IC1506 ~5.15V / D7510   │     (~3.29V idle)            │
    │                               │                             │                              │
    │  Pin 4  (GP2 / UART RX) ──────┼◄──[ R_SER: 100Ω (opt) ]─────├───◄ Pin 6: UARTP2M_T         │
    │  (115200 8N1 <- AM34 TX)      │     Onboard R7572 10kΩ!     │     (MAX TXD, Square Pad)    │
    │                               │                             │                              │
    └───────────────────────────────┘                             └──────────────────────────────┘
```

### 3.2 Mermaid Interconnection Diagram

```mermaid
flowchart LR
    subgraph PICO["Raspberry Pi Pico (RP2040 / RP2350)"]
        ADC_P["GP26 / Pin 31 (SW_UART)"]
        RESET_P["GP5 / Pin 7 (XRESET OD)"]
        TX_P["GP4 / Pin 6 (UART TX)"]
        GND_P["GND / Pin 38"]
        NSW_P["GP3 / Pin 5 (NSW5V Hi-Z)"]
        RX_P["GP2 / Pin 4 (UART RX)"]
    end

    subgraph TARGET["Connector P7501 (VEP79138 - TO JIGU)"]
        CN_1["Pin 1: SW_UART (~3.25V Hi-Z)"]
        CN_2["Pin 2: XRESET (Open-Drain)"]
        CN_3["Pin 3: UARTM2P_T (Q7514 RX)"]
        CN_4["Pin 4: GND (0.0 Ω Chassis)"]
        CN_5["Pin 5: NSW5V (~5.15V up -> Hi-Z!)"]
        CN_6["Pin 6: UARTP2M_T (TX 115200 8N1)"]
    end

    ADC_P <--> CN_1
    RESET_P --> CN_2
    TX_P --> CN_3
    GND_P === CN_4
    CN_5 -.->|"Hi-Z Input No Pulls"| NSW_P
    CN_6 --> RX_P
```

---

## 4. Critical Hardware Protection Rules

### 4.1 Survival Rule 1: `NSW5V` (`GP3` / Pin 5) ALWAYS `Hi-Z Input`
* Pin 5 (`NSW5V` / `X_SW_5R2V`) receives `~5.15V` from regulator `IC1506` through diode `D7510` feeding `R7570`/`R7572` and Syscon `IC7501` (`~3.29V` idle reading).
* **NEVER** configure **`GP3` (`NSW5V`)** as a push-pull output, never enable internal pull-up/pull-down resistors on `GP3`, and **never power the Raspberry Pi Pico from `P7501`**.

### 4.2 Survival Rule 2: Open-Drain Control of `XRESET` (`GP5`)
* Pin 2 (`XRESET`) ties to the board reset line (`QR7503` / system supervisor).
* **NEVER** drive `GP5` to push-pull HIGH (`+3.3V`).
* **Safe Mode (`p7501_hw.c`):**
  * **Assert reset:** `GP5` driven output **`0V`**.
  * **Release reset:** `GP5` configured as unpulled **`Hi-Z` input** (`~20 µs` natural rise time).

### 4.3 Rule 3: Harness Length & Absence of External Pull-Up
* Keep the JST/Dupont harness short ($\le 15\text{ cm}$) and solidly grounded (`Pin 4 <-> Pico Pin 38`).
* **Do NOT** wire an external `4.7 kΩ` pull-up to `3V3_OUT`: `VEP79138` already includes `R7570` (`10 kΩ`) and `R7572` (`10 kΩ`).

---

## 5. Standard Bench Operating Procedure (Step-by-Step)

```text
                     VERIFIED P7501 OPERATIONAL FLOW
  ┌─────────────────────────────────────────────────────────────────────┐
  │ 1. COLD CHECK: Pin 4 <-> Chassis = 0.0Ω; verify inter-pin isolation │
  ├─────────────────────────────────────────────────────────────────────┤
  │ 2. JST ZR-6 MATE: Plug harness into P7501 + USB into Pico           │
  ├─────────────────────────────────────────────────────────────────────┤
  │ 3. TERMINAL (115200 bps): Open console -> Prompt 'PANADIGA> '       │
  ├─────────────────────────────────────────────────────────────────────┤
  │ 4. ELECTRICAL CHECK ('STATUS' & 'PINCHECK'): ~3.25V Hi-Z / ~2.69V PD│
  ├─────────────────────────────────────────────────────────────────────┤
  │ 5. BOOT CAPTURE ('UARTSTREAM 12000 JIG:2500' / 'BOOTTRACE 15000')   │
  ├─────────────────────────────────────────────────────────────────────┤
  │ 6. TX / MONITOR ('ENGAGE' -> 'UARTSEND' / 'DIAGWATCH' / 'PANARSP')  │
  ├─────────────────────────────────────────────────────────────────────┤
  │ 7. NO-DESOLDER BGO CURE: 8 MiB Backup ('PANADMP1') -> 'PANAMON1>'   │
  ├─────────────────────────────────────────────────────────────────────┤
  │ 8. POST-TEST HYGIENE ('UARTPROBE 8' -> 'ISOLATE'): Back to Hi-Z!    │
  └─────────────────────────────────────────────────────────────────────┘
```

### Step 1: Cold Inspection & Resistance Checks (Unit Unplugged from AC)
1. Unplug the DMR-EH55 from AC mains.
2. With multimeter in continuity mode, verify **$0.0\,\Omega$** between **`P7501` Pin 4 (`GND`)** and the metal chassis.
3. Confirm no short-circuit exists among Pins 4 (`GND`), 5 (`NSW5V`), and 1 (`SW_UART`).

### Step 2: Mating the `P7501` $\leftrightarrow$ Raspberry Pi Pico Harness
1. Plug the `ZHR-6` (`1.5 mm`) connector into `P7501`, double-checking exact 6-pin mapping (`1->GP26`, `2->GP5`, `3->GP4`, `4->GND`, `5->GP3`, `6->GP2`).
2. Power the Pico strictly from its **USB** port connected to the workstation (or Android OTG / BLE on Pico W / 2 W).

### Step 3: Power-On & Opening the Console (`PANADIGA> `)
1. Plug the Panasonic recorder into AC mains and connect Pico USB.
2. Open the Pico serial port in the Desktop app (`panadiga-desktop`), CLI (`panadiga`), or serial terminal at `115200 bps`.
3. Press `<ENTER>` and verify prompt **`PANADIGA> `**.

### Step 4: Electrical Health Check (`STATUS` & `PINCHECK`)
```text
PANADIGA> PING
PONG
PANADIGA> STATUS
VDD: 3.25V (OK)
PANADIGA> PINCHECK
```
* **Expected `PINCHECK` values:** `SW_UART` (`GP26`) reads `~3.25V` `Hi-Z`, `~2.69V` pull-down (clamped via `Q7510` B-C junction), and `~3.27V` pull-up; `XRESET` (`GP5`) rises in `~20 µs` on release; `NSW5V` (`GP3`) reports strictly `Hi-Z Input`; `GP2` (`UARTP2M_T`) shows hundreds of active UART edges during boot.

### Step 5: Live Boot Capture (`UARTSTREAM` / `BOOTTRACE` / `HEXCAP` / `BAUDSCAN`)
```text
PANADIGA> BAUDSCAN 3500 NORMAL
PANADIGA> UARTSTREAM 12000 JIG:2500
PANADIGA> BOOTTRACE 15000 NOJIG
```
* Deterministic PIO1 capture on `GP2` (`115200 8N1`) displays complete AM34 bootloader telemetry (`<00>DDR CHECK: OK`, `SD init`, `CD diag boot mode`, `X(80020401)` / `X(80040904)` codes, and `*** RTSC Initialize ***`). *(For **`Loader ver0.61`**, timed `UARTSTREAM 12000 JIG:2500` holds `SW_UART` LOW long enough to clear `DDR CHECK: OK` and load `DIGADIAG.FRM` before releasing `GP26`; legacy PIO0 `JIGBOOT`/`BOOTCAP`/`UARTMON` remain available.)*

### Step 6: Interactive Serial TX (`ENGAGE` + `UARTSEND` / `DIAGWATCH` / `PANARSP`) & Cleanup (`UARTPROBE 8` + `ISOLATE`)
```text
PANADIGA> ENGAGE
[BUS] P7501 UART TX (GP4) engaged idle HIGH; NSW5V (GP3) kept Hi-Z Input.
PANADIGA> UARTSEND VER
PANADIGA> UARTPROBE 8
PANADIGA> ISOLATE
[BUS] P7501 pins placed in High-Impedance Isolation (NSW5V/XRESET Hi-Z, GP26 ADC0).
```
*(Remember: `UARTSEND` keeps `SW_UART = Hi-Z` so signals pass through `Q7514`; upon completion, `UARTPROBE 8` + `ISOLATE` leaves `GP26 / ADC0` clean for subsequent `STATUS` checks.)*

### Step 7: No-Desoldering In-Circuit Recovery (`SDDIAG` / Door #1 $\rightarrow$ `PANAMON1>` $\rightarrow$ 8 MiB Backup & `U99` BGO Cure)
1. **Generate the canonical `ver061` SD kit:** Run `panadiga unbrick prepare-sd --loader-profile ver061` to place `DIGADIAG.FRM` (`262,144 B` raw `@ 0x50400000`) + `PANA__SD.FRM` (`sd48`, `SHA-256: cdcc2cdf...`) onto an **SDSC FAT16 ($\le 2\text{ GB}$)** card (SDHC/SDXC cards fail `Loader ver0.61` 4-bit SD init).
2. **Enter `PANAMON1>`:** Boot from the SDSC card via **`SDDIAG`** (`UARTSTREAM 12000 JIG:2500`) or headlessly through **Door #1 (`NOJIG` via `PROG` slot `0x0B1800` when `CH1` fails `HDD_MAX_LBA get failed`)**. Confirm the `PANAMON1>[B/R/W/D/F/J/E/U/Q]` prompt with `?\r` at `300 µs` cadence.
3. **Mandatory read-only 8 MiB backup (`PANADMP1`):** Before modifying any Flash bit, dump the entire `0x84000000..0x847FFFFF` array by patching `MOV` bounds on uncached mirror `0x9043950E` (`J 904394FA`) or via `panadiga pico dump-nor`, and verify the resulting **SHA-256** digest.
4. **2-Cycle BGO cure under `E05` IPL lock:** Execute the `panadiga re eh55-bgo` repair plan using explicit Layer 2 unlock (`U CONFIRM_UPPER_NOR` $\rightarrow$ sector format / pure `1 -> 0` writes on BGO `SA26..SA70` with ~`700 ms` `0xFF × 32` erase polling $\rightarrow$ `U LOCK`), then verify progression to `***** normal mode *****` $\rightarrow$ `*** RTSC Initialize ***` via `BOOTTRACE 15000 NOJIG`.

---

## 6. Bench Troubleshooting Matrix

| Observed Symptom | Most Likely Cause | Immediate Corrective Action |
| :--- | :--- | :--- |
| `VDD: 0.02V (LOW)` after `UARTSEND` / `JIGBOOT` | Residual SIO direction state on `GP26` or recorder unpowered | Run `UARTPROBE 8` followed by `ISOLATE`, and check AC power to the recorder. |
| `PINCHECK` reports `GP2 = 0` transitions | Pin 6 wire (`UARTP2M_T`, square pad) loose or swapped | Check continuity from `P7501` square pad to `GP2` (Pico Pin 4) + common ground `Pin 4 <-> GND`. |
| `UARTSEND LOW:cmd` captures `0 bytes` | Expected hardware behavior: `SW_UART = LOW` cuts off `Q7514` | Send `UARTSEND <text>` without `LOW:` prefix after `ENGAGE` so `GP26` stays `Hi-Z`. |
| Unit stuck in `U99` (`BGO driver init failed[1].`) | Corrupted BGO FTL (`SA26..SA70`, `0x84130000..0x843FFFFF`) or `EEMUL` (`SA134..SA141`) in `IC5001` | **First (No desoldering):** Use SDSC FAT16 `ver061` kit (`SDDIAG` / Door #1) $\rightarrow$ `PANAMON1>` + 8 MiB backup + `U CONFIRM_UPPER_NOR` / `panadiga re eh55-bgo` (`SA0..SA10` stays `E05`-locked). **Last resort:** external TSOP-48 read + `panadiga nvram clean` on **XGecu T48** / **SVOD 4**. |
| Drops to `PLEASE WAIT` / `SELF CHECK` at `~44s` (`NSW5V=0` & `XRESET=0`) | Syscon `IC7501` watchdog expires without valid SDSC media or `TBUS` keepalive | Use an **SDSC FAT16 $\le 2\text{ GB}$** card with the `ver061` kit (avoid SDHC/SDXC), or run the `TBUS` keepalive when executing `M0` stubs without `MAIN`. |
| `HDD_MAX_LBA get failed` / `DAVIS CH1` reads `0x00002004` or top lid won't close | Reversed `P58001` pinout (`Pin k = ATA(41-k)`), `CS`/`Slave` jumper, wrong FFC polarity, or `0°` `N5EZZ0000003` height collision | Force **`MASTER`** jumper on `StarTech IDE2SAT2` (never `CS`/`Slave`). Use **Type A** FFC at `0°`; to close the lid with `N5EZZ0000003` rotated `180°`, use a **40-pin `0.5 mm` pitch (`20.5 mm` width) `Type B` FFC** (or DIY razor-blade strip + Kapton) and confirm `1:1` multimeter continuity. |
| Serial console unresponsive | Wrong COM/ttyACM port or USB CDC driver unbound | Confirm port with `panadiga pico list` and verify `115200 8N1`. |

---

## 7. Probe Architecture & Universal Compatibility Matrix

### 7.1. `P7501` Probe (`115200 8N1`) vs. Generic USB-Serial Adapters
* **Raspberry Pi Pico (`RP2040` / `RP2350`)** combines deterministic PIO UART (`p7501_uart.pio` / `p7501_uart_duplex.pio`) with synchronized **`SW_UART`** gating (`GP26` for `Q7510`/`Q7514`), Open-Drain **`XRESET`** (`GP5`), passive **`NSW5V`** guard (`GP3` always `Hi-Z`), and **`GDB` RSP** bridge.
* **Electrical Warning:** Never connect generic USB-Serial dongles directly to `P7501` without strict `3.3V` compliance, true Open-Drain on `XRESET`, and complete isolation of Pin 5 (`NSW5V`).

### 7.2. Compatibility Across NOR Flash Vendors (JEDEC CFI)
Target stub [`ram_loader_am34_target.c`](../../firmware/pico-p7501/flash/ram_loader_am34_target.c) and `panadiga nvram clean` support JEDEC/CFI Top-Boot geometries across **Spansion** (`S29JL064H`, `S29GL064N`, `S29AL016D`), **ST/Numonyx** (`M29W640FB`/`FT`), **Macronix** (`MX29LV640DT`), **Fujitsu** (`MBM29LV640`), and **Toshiba/Winbond**.

### 7.3. Compatibility Across the Panasonic DIGA Family
Top-Boot NVRAM (`EEMUL`) window layout across DIGA generations:
1. **8 MB NOR Flash (`0x807F0000..0x807FFFFF`):** `DMR-EH55`, `DMR-EH56`, `DMR-EH65`, `DMR-ES15`, `DMR-ES25`, `DMR-ES35V`, `DMR-EZ25`.
2. **4 MB NOR Flash (`0x803F0000..0x803FFFFF`):** `DMR-ES10`, `DMR-ES20`, `DMR-ES30V`, `DMR-EH50`, `DMR-EH60`, `DMR-E100H`, `DMR-E500H`.
3. **2 MB NOR Flash (`0x801F0000..0x801FFFFF`):** `DMR-E50`, `DMR-E55`, `DMR-E65`, `DMR-E80H`, `DMR-E85H`, `DMR-E95H`.
4. **16 MB NOR Flash (`0x80FF0000..0x80FFFFFF`):** `DMR-EX75`, `DMR-EX85`, `DMR-EX77`, `DMR-EX87`, `DMR-EX95V`, `DMR-EZ27`, `DMR-EZ45V`, `DMR-EZ48V`.
5. **Full Architecture Reference:** See [SYSTEM_ARCHITECTURE_AND_ENGINEERING_BLUEPRINT.md](SYSTEM_ARCHITECTURE_AND_ENGINEERING_BLUEPRINT.md).

---

## 8. Single-Piece 6-Pin Connector (`JST ZR-6`) & Tri-State Isolation (`ISOLATE` / `HIZ`)

Because **`P7501` (`B6B-ZR-3R4`)** uses a **single-piece 6-pin female housing (`1.5 mm`)**, there is no need to unplug individual wires between diagnostics and normal boot runs:

1. **Connected Harness Safety:**
   * **Pin 4 (`GND`):** Common ground ($0.0\,\Omega$).
   * **Pin 5 (`NSW5V` -> `GP3`):** Permanently stays an unpulled **`Hi-Z Input`** in Pico HAL — **zero current sunk or injected**.
   * **Pin 2 (`XRESET` -> `GP5`):** Idle state is unpulled **`Hi-Z` Open-Drain**.
   * **Pins 1, 3, and 6 (`SW_UART`, `UARTM2P_T`, `UARTP2M_T`):** Placed into safe tri-state isolation via **`ISOLATE`** (`HIZ`), letting the Panasonic board boot normally while connected.
2. **Firmware `ISOLATE` (`HIZ`) and `ENGAGE` Commands:**
   * **`ISOLATE` / `HIZ`:** Calls `p7501_hw_isolate_bus_hiz()`, tri-stating TX drivers into safe high impedance (`NSW5V` & `XRESET` `Hi-Z`, `GP26` `ADC0`).
   * **`ENGAGE`:** Calls `p7501_hw_engage_uart_bus()`, asserting idle HIGH on `GP4` (`UARTM2P_T`) for active TX while keeping `GP3` (`NSW5V`) strictly `Hi-Z Input`.

---

## 9. First-Connection Read-Only & Risk-Graduated Compliance Checklist

Before attempting any invasive write or recovery command:

1. **Step 1 — Read-Only Telemetry (`STATUS` $\rightarrow$ `PINCHECK`):** Verify `Pin 4 = GND`, clean `SW_UART` voltages (`~3.25V Hi-Z | ~2.69V PD`), strict `GP3 (NSW5V)` isolation, and edges on `GP2 (UARTP2M_T)`.
2. **Step 2 — Boot Capture (`UARTSTREAM 12000 JIG:2500` / `BOOTTRACE 15000 NOJIG` / `HEXCAP`):** Record the full `115200 8N1` startup log (`<00>DDR CHECK: OK`, `SD init`, `CD diag boot mode`, or `BGO driver init failed[1].`) for exact root-cause triage.
3. **Step 3 — Full 8 MiB Read-Only Backup (`PANADMP1` + SHA-256) & Cleanup (`ISOLATE`):** Before any Flash write (`EEMUL` or `BGO`), acquire a full SHA-256-verified read-only copy of `0x84000000..0x847FFFFF` and always finish serial probe runs in `ISOLATE`.
4. **Step 4 — Graduated `U99` Repair (In-Circuit under `E05` IPL Lock $\rightarrow$ TSOP-48 Fallback):** With the 8 MiB backup safely on disk, perform in-circuit repair inside `PANAMON1>` (`SA134..SA141` `EEMUL` enabled by default; Layer 2 BGO `SA26..SA70` unlocked only via explicit `U CONFIRM_UPPER_NOR` + ~`700 ms` / `0xFF × 32` erase polling, keeping **Layer 1 IPL `SA0..SA10` permanently locked with `E05`**). For an already-bricked `U99` recorder, trying this no-desolder recovery first carries zero risk to bootrom IPL; resort to physical TSOP-48 removal (**XGecu T48** / **SVOD 4** + `panadiga nvram clean`) only if `SA0..SA10` IPL or the SD slot hardware has failed.
