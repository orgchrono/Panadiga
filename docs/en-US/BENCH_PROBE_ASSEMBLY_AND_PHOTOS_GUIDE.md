> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/GUIA_MONTAGEM_E_FOTOS_PROBE_BANCADA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Illustrated Bench Probe Assembly & Practical Lab Guide
## `P7501` Asynchronous UART (`115200 8N1`) Bench Probe Assembly for Panasonic DMR-EH55 (`VEP79138` Board)

**Document:** `BENCH_PROBE_ASSEMBLY_AND_PHOTOS_GUIDE.md`  
**Classification:** Laboratory Assembly & Practical Engineering Runbook  

---

## 1. Engineering Scope & Purpose

This guide provides detailed mechanical assembly instructions, electrical wiring steps, and bench procedures for the **`P7501` Asynchronous UART (`115200 8N1`) + Discrete Boot/Reset Control Probe** built on the **Raspberry Pi Pico (`RP2040` / `RP2350`)** microcontroller.

This instrument enables:
1. Intercepting cold startup sequences (`CD diag boot mode` / `XFWE` serial boot) on **Matsushita `AM34` / `MN103S` (`VEP79138`)** without desoldering `IC5001`.
2. Operating bidirectional UART telemetry (`UARTP2M_T` / `UARTM2P_T`) and the **GDB RSP** bridge over TCP port `3333` for Ghidra/IDA analysis.
3. Monitoring live `SW_UART` (`GP26`), Open-Drain `XRESET` rise timing (`GP5`), and the `NSW5V` power rail (`GP3` locked strictly as `Hi-Z Input` with pulls disabled), while protecting the Spansion `S29JL064H` (`IC5001`) NOR Flash.

> [!NOTE]
> ### Technical CAD Rigor vs. Stylized Illustrations (Rejecting "PowerPoint" Drawings)
> Stylized dark-mode art diagrams and heuristic AI drawings have been **strictly excluded from this repository**. In physical hardware labs, visual ambiguity destroys silicon. Every graphic in this guide obeys:
> 1. **Official Datasheet CAD Geometry (JST ZR `B6B-ZR-3R4`):** True molded shell proportions, center locking latch window, thin front wall vs. thicker reinforced rear wall, 45° Pin 1 polarizing chamfer, and authentic PCB footprint pads (**square pad on Pin 6** vs. **round pads on Pins 1–5**).
> 2. **IEEE Std 315 / IEC 60617 Compliance:** Clean black-on-white technical schematics, normalized engineering title blocks, net tracing, `NSW5V` clamping rules, and clear separation between signal ground (IEEE triangle) and metallic chassis ground (IEEE chassis symbol).

---

## 2. CRITICAL BENCH WARNING: Unambiguous `GND` Pin Identification & `NSW5V` Safety

> [!CAUTION]
> ### MANDATORY LAB DISCLAIMER — GROUND PIN (`GND`) & `NSW5V`
> * **Official Panasonic Service Manual (`P7501 [B6B-ZR-3R4]`, p. 62):** **`GND` IS STRICTLY PIN 4**.
> * **Left $\rightarrow$ Right Physical PCB Order (between `FP50003` and `FP50004`):** Looking left to right, pins appear in descending canonical order: **Pin 6 (`UARTP2M_T`, square pad), Pin 5 (`NSW5V`), Pin 4 (`GND`), Pin 3 (`UARTM2P_T`), Pin 2 (`XRESET`), and Pin 1 (`SW_UART`)**. Counting left to right starting from `FP50003`, `GND` is the **3rd pin** (which corresponds to **Pin 4** in canonical manual numbering 1..6 starting from `FP50004`).
> * **SHORT-CIRCUIT & `NSW5V` HAZARD:** NEVER assume Pin 1 (`SW_UART`, `~3.25V`), Pin 5 (`NSW5V`, `~5.15V` via `D7510` from `IC1506` / `~3.29V` at rest), or Pin 6 (`UARTP2M_T`) is ground! Furthermore, Pico pin `GP3` wired to Pin 5 (`NSW5V`) **MUST ALWAYS stay configured as `Hi-Z Input` with pulls off (NEVER drive push-pull!)**.
> * **INVIOLABLE GOLDEN RULE:** Before plugging in the Raspberry Pi Pico or powering the board, **put your DMM on audible continuity mode and verify a steady $0.0\,\Omega$ beep between the chosen ground wire and bare unpainted chassis metal.**

---

## 3. High-Resolution Technical Photos & `VEP79138` Board Topography

### 3.1 Official `P7501` Connector Schematic (Panasonic Service Manual)
Below is the high-resolution capture cropped directly from the main board schematic page:

![Official Panasonic P7501 Schematic](../assets/images/schematic_p7501_panasonic_official.png)

*Key elements visible in the factory schematic:*
- Designated **`P7501`**, part number **`B6B-ZR-3R4`**, net label **`TO JIGU`**.
- **Pin 1:** `SW_UART` (`(D)`) — `~3.25V` Hi-Z / `~2.69V` PD via `Q7510`; LOW enters `XFWE` serial boot and cuts off `Q7514`.
- **Pin 2:** `XRESET` — Master Open-Drain active-LOW reset (`GP5`, `~20 µs` Hi-Z rise via `QR7503`).
- **Pin 3:** `UARTM2P_T` (`MAX RXD`) — Target serial RX (`GP4` Pico TX `115200 8N1` -> AM34 RX via `Q7514`, with onboard `R7570 10 kΩ`).
- **Pin 4:** **`GND` (Chassis Ground symbol, $0.0\,\Omega$)**
- **Pin 5:** `NSW5V` (`X_SW_5R2V`) — `~5.15V` rail (`IC1506` via `D7510`, `~3.29V` resting; keep `GP3` ALWAYS `Hi-Z Input`!).
- **Pin 6:** `UARTP2M_T` (`MAX TXD`) — AM34 serial TX (`115200 8N1` -> `GP2` Pico RX via `R7535`/`R7572 10 kΩ`, square PCB pad).

---

### 3.2 Physical Top-Side Topography on `VEP79138` & JST ZR CAD Specification

Test header **`P7501`** sits nestled between two vital flat-flex connectors:
* **To the Left (Pin 6):** 10-way flat-flex connector **`FP50003`** (optical drive spindle motor).
* **To the Right (Pin 1):** 8-way flat-flex connector **`FP50004`** (tray loading motor & limit switches).

Below is the vector CAD mechanical drawing derived from JST's official package geometry and the PCB IPC-7351 footprint:

![Physical Topography and P7501 Pinout](images/panasonic_dmr_eh55_pcb_p7501_orientation.png)

#### Mechanical Keying & Physical Invariants of JST ZR (`B6B-ZR-3R4`):
1. **Plastic Latch Window (Locking Notch):**  
   The locking tab of mating plug `ZHR-6` snaps into the **rectangular cutout on the front wall** facing the outer edge of the PCB.
2. **Wall Thickness Asymmetry (Thick Rear vs. Thin Front):**  
   JST polyamide housings are intentionally asymmetric:
   - **Rear Wall (Thick / Reinforced):** $0.60\text{ mm}$ solid resin facing inward toward the PCB core.
   - **Front Wall (Thin):** $0.45\text{ mm}$ flexible wall accommodating the `ZHR-6` retention latch.
3. **45° Corner Chamfer at Pin 1:**  
   The outer corner adjacent to Pin 1 features a molded $45^\circ$ chamfer providing tactile and visual index keying on the bench.
4. **PCB Copper Footprint Signature (Square Pad on Pin 6 vs. Round Pads on Pins 1..5):**  
   Inspecting copper pads directly on the solder side:
   - **Pin 6 (`UARTP2M_T`, left next to `FP50003`):** **SQUARE** copper pad.
   - **Pins 5 through 1:** **CIRCULAR / ROUND** copper pads.
5. **Pin Pitch:**  
   Exact **$1.50\text{ mm}$** center-to-center spacing; total housing width $10.50\text{ mm}$.

```text
       MECHANICAL ORIENTATION OF P7501 [B6B-ZR-3R4] (CANONICAL SERVICE MANUAL PINOUT)
       Thick Rear Wall (0.60mm) facing inward toward center of PCB (top)
       Locking Latch Window facing outer edge of PCB (bottom)
       
       10-Way Flat (FP50003)                                          8-Way Flat (FP50004)
       [Left Side]                                                             [Right Side]
            ┌──────────────────────────────────────────────────────────────────┐
      Pin 6 │ ■ UARTP2M_T [SQUARE PAD]  TX UART AM34 (115200 8N1) -> Pico GP2  │
      Pin 5 │ ● NSW5V     [Round Pad]   ~5.15V/3.29V (GP3 ALWAYS Hi-Z Input!)  │
      Pin 4 │ ● GND       [Round Pad]   Canonical Ground (0.0Ω to Chassis!)    │
      Pin 3 │ ● UARTM2P_T [Round Pad]   RX UART AM34 <- Pico GP4 (via Q7514)   │
      Pin 2 │ ● XRESET    [Round Pad]   Master Reset (Open-Drain GP5 -> 0V)    │
      Pin 1 │ ● SW_UART   [Round Pad]   Boot Select / ADC0 (GP26)              │
            └──────────────────────────────────────────────────────────────────┘
```

---

## 4. Probe Wiring Electrical Schematic (`RP2040` / `RP2350` $\leftrightarrow$ `P7501`)

Below is the official IEEE Std 315 / IEC 60617 electrical probe schematic with normalized engineering title blocks and voltage guards:

![Pico Probe Assembly Electrical Schematic](images/p7501_pico_probe_assembly_schematic.png)

#### Electrical Coupling Rationale & Resistor Verdict:
1. **Bench Verdict — Should Resistors Be Added?**
   - **DIRECT WIRE MODE (Short Harness $\le 15\text{ cm}$):**  
     For short bench harnesses ($L \le 15\text{ cm}$), **logic signals `SW_UART`, `XRESET`, `UARTM2P_T`, and `UARTP2M_T` already use $3.3\text{V}$ LVTTL with factory pull-up and limiting resistors onboard (`R7570 10 kΩ`, `R7572 10 kΩ`, `R7535 100 Ω`, `R7559 10 kΩ`)**, eliminating any need for an external $4.7\text{ k}\Omega$ pull-up on `UARTM2P_T` (`GP4`).
   - **OBEY THE PIN 5 (`NSW5V` / `GP3`) CRITICAL RULE:**  
     Pin 5 (`NSW5V`) carries **`~5.15V` from `IC1506` via `D7510`** (`~3.29V` resting) to bias `R7570`/`R7572` and `IC7501`. Keep **`GP3` strictly configured as `Hi-Z Input` with pulls disabled (NEVER drive push-pull and NEVER power the Pico from Pin 5)**.
   - **OPTIONAL INLINE RESISTORS FOR PERMANENT JIGS ($> 15\text{ cm}$):**  
     Add **$R_1 \dots R_3 = 100\,\Omega$** in series on logic pins (`GP2`, `GP4`, `GP5`) to damp ringing and **$R_{\text{sense}} = 1.0\text{ k}\Omega$** on `GP26 / ADC0` (`SW_UART`).
2. **Open-Drain Driver on `XRESET` (`GP5`):**  
   The Pico strictly pulls `XRESET` down to $0\text{V}$ (active reset) and releases to high-impedance (`Hi-Z`, measured `~20 µs` rise through `QR7503`). **NEVER configure `GP5` as push-pull driving $+3.3\text{V}$!**
3. **Standardized Ground Separation:**  
   Schematic symbols distinguish Pico Signal Ground (IEEE triangle) from Panasonic Chassis Ground (IEEE chassis rake), tied together across a low-resistance ($< 0.1\,\Omega$) return bond.

### 4.1 Sequential Pin-to-Pin Wiring Table (Pins 1 to 6)

| Canonical Pin (`P7501`) | Matsushita Signal | Pico Pin (`RP2040/RP2350`) | Physical Order & `ZHR-6` Lead | Conditioning & Safety Rule |
| :---: | :---: | :---: | :--- | :--- |
| **Pin 1** | `SW_UART` `(D)` | `GP26 / ADC0` (Pin 31) | Rightmost pin (nearest 8-way `FP50004`). `ZHR-6` Wire 1. | `~3.25V` Hi-Z / `~2.69V` PD (`Q7510`). `LOW` enables serial boot & cuts off `Q7514`; `Hi-Z` unlocks UART TX. ($1.0\text{ k}\Omega$ opt.) |
| **Pin 2** | `XRESET` | `GP5` (Pin 7) | System Master Reset (`QR7503`). `ZHR-6` Wire 2. | **Strict Open-Drain** (`LOW` / `Hi-Z` ~20 µs rise). Optional $100\,\Omega$ series. |
| **Pin 3** | `UARTM2P_T` (`MAX RXD`) | `GP4` (Pin 6 / UART TX) | Pico TX `115200 8N1` -> AM34 RX via `Q7514`. `ZHR-6` Wire 3. | Onboard `R7570 10 kΩ` to `NSW5V` (**no external pull-up!**). Needs `SW_UART = Hi-Z` so `Q7514` conducts. |
| **Pin 4** | **`GND`** | **Pin 38 (`GND`)** | **Canonical ground (3rd pin L$\rightarrow$R). `ZHR-6` Wire 4.** | **MANDATORY: Direct low-resistance bond ($0.0\,\Omega$ to bare metal chassis)!** |
| **Pin 5** | `NSW5V` (`X_SW_5R2V`) | `GP3` (Pin 5) | `~5.15V` rail from `IC1506` via `D7510` (`~3.29V` idle). `ZHR-6` Wire 5. | **ALWAYS `Hi-Z Input` no pulls — NEVER drive push-pull and NEVER power Pico!** |
| **Pin 6** | `UARTP2M_T` (`MAX TXD`) | `GP2` (Pin 4 / UART RX) | Leftmost pin (nearest 10-way `FP50003`). **SQUARE Pad**. `ZHR-6` Wire 6. | AM34 UART TX `115200 8N1` (`R7535`/`R7572 10 kΩ`). Optional $100\,\Omega$ series. |

---

## 5. Bill of Materials (BOM)

| Item | Qty | Reference | Technical Specification | Role in Circuit |
| :---: | :---: | :--- | :--- | :--- |
| **1** | 1 | Microcontroller | **Raspberry Pi Pico** (`RP2040`) or **Pico 2** (`RP2350`) / Pico W | `P7501` UART/PIO controller (`115200 8N1`) & GDB RSP bridge |
| **2** | 3 | $R_1, R_2, R_3$ | **$100\,\Omega \pm 1\%$** Metal Film Resistor ($1/8\text{W}$ or $1/4\text{W}$) *(optional)* | Optional damping on `UARTP2M_T` (`GP2`), `UARTM2P_T` (`GP4`), `XRESET` (`GP5`) |
| **3** | 1 | $R_{\text{sense}}$ | **$1.0\text{ k}\Omega \pm 1\%$** Metal Film Resistor *(optional)* | Optional series isolation on `SW_UART` / `ADC0` (`GP26`) |
| **4** | 1 | Wiring Harness | 6-way 1.5 mm female JST ZR (`ZHR-6`) **OR** AWG 30 flexible Kynar/PTFE wire | Connection to OEM `P7501 [B6B-ZR-3R4]` |
| **5** | 1 | Strain Relief | Heat-resistant Kapton tape or hot-melt strain relief | Secures leads against PCB pad peel |

---

## 6. Step-by-Step Assembly & Commissioning Procedure

### Step 1: Preparing Board `VEP79138`
1. Unplug the AC mains cord from the Panasonic DMR-EH55.
2. Remove the metal top lid (rear and side screws).
3. Visually locate adjacent ribbon sockets: **`FP50003`** (10-way on left) and **`FP50004`** (8-way on right).
4. Between them lies **`P7501`** (`B6B-ZR-3R4`), labeled *"TO JIGU"*.
5. If soldering directly to pads, clean flux zones with 99.8% IPA and apply no-clean flux.

### Step 2: Electrical Connection (`ZHR-6` Plug or Direct Kynar Leads)
* **If `B6B-ZR-3R4` header is populated:** Snap in the pre-crimped `ZHR-6` female plug directly.
* **If soldering direct wires:**
  1. Lightly tin all 6 pads (remembering **Pin 6 `UARTP2M_T`** is the square pad beside `FP50003`).
  2. Solder 6 short ($\le 15\text{ cm}$) AWG 30 Kynar wires.
  3. Tape down wire bundle onto an empty ground plane with Kapton tape for mechanical strain relief.

### Step 3: Mandatory Multimeter Verification Checklist
Before touching the Raspberry Pi Pico:
1. **Ground Continuity Test (`Pin 4 = GND`):**
   - Black meter probe on bare chassis steel.
   - Red meter probe on **Manual Pin 4 (3rd pin from left starting at `FP50003`)**.
   - Multimeter **MUST emit continuous continuity tone ($< 0.3\,\Omega$)**.
   - Probe all other 5 pins against chassis — **zero beeps** permitted!
2. **Inter-Pin Short Check:**
   - Measure resistance between `Pin 5 (NSW5V)`, `Pin 1 (SW_UART)`, `Pin 6 (UARTP2M_T)`, and `GND` (`Pin 4`): verify open circuit (no $0\,\Omega$ bridges!).

### Step 4: Connecting to Raspberry Pi Pico & Launching Workbench
1. Confirm Pico firmware built from `firmware/pico-p7501/` is flashed and re-check `GP2`, `GP3 (Hi-Z!)`, `GP4`, `GP5 (OD)`, `GP26`, `GND`.
2. Connect Pico USB cable to the workstation (NEVER back-power Pico through `P7501`) and start Panadiga Desktop:
   ```powershell
   npm run tauri dev
   ```
3. Open the **Pico Bench** view and verify the **Physical Wiring (`Pico -> P7501`)** card to start safe `115200 8N1` serial capture.
