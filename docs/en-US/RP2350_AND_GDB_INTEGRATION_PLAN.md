> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/PLANO_INTEGRACAO_RP2350_E_GDB.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Engineering Plan: RP2350 (Pico 2) Support and GDB / Ghidra Integration via `P7501`

**Technical Architecture & Roadmap Document**  
**Subsystem:** Asynchronous UART Interface `P7501` (`115200 8N1`), GDB RSP Bridge, and Bench Tooling  
**Target:** Panasonic DMR-EH55 (`AM34` / `MN103S`, Syscon `IC7501`, Flash `IC5001`)  
**Host Controllers:** Raspberry Pi Pico (RP2040) & Raspberry Pi Pico 2 (RP2350)

---

## 1. Comparative Technical Evaluation: RP2040 vs RP2350

| Silicon Parameter | Raspberry Pi Pico (RP2040) | Raspberry Pi Pico 2 (RP2350) | Practical Impact on DMR-EH55 Reverse Engineering |
| :--- | :--- | :--- | :--- |
| **CPU Cores** | Dual-core ARM Cortex-M0+ @ 133 MHz | Dual-core ARM Cortex-M33 (or Hazard3 RISC-V) @ 150 MHz | Faster execution of parsers, serial capture at `115200 8N1`, and C decoders. |
| **Internal SRAM** | 264 KB SRAM divided into 6 banks | **520 KB SRAM** divided into 10 banks | **Critical Advantage:** RP2350 can hold a full 64 KB sector of the Spansion S29JL064H (`EEMUL`) in local Pico RAM or extended capture/telemetry buffers without dropping USB frames. |
| **PIO Subsystem** | 2 PIO blocks (8 state machines total) | **3 PIO blocks (12 state machines total)** | Allows running the `P7501` PIO engine (`p7501_uart.pio`) on one PIO block while dedicating other blocks to monitoring/sniffing boot lines (`SW_UART`, `XRESET`). |
| **PIO Instructions** | Base PIO instruction set | PIO 2.0 (extended instructions and enhanced FIFO) | Current PIO code (`p7501_uart.pio`) is 100% compatible with both RP2040 and RP2350. |
| **I/O Voltage** | Strict 3.3V (Pin 5 `NSW5V` always `Hi-Z`) | Strict 3.3V (Pin 5 `NSW5V` always `Hi-Z`) | **Full Electrical Safety:** Logic signals on `P7501` operate at 3.3V LVTTL, while `GP3` (`NSW5V`) remains strictly an unpulled `Hi-Z` input. |
| **Tolerance & Protection** | Standard CMOS | Hardware Glitch Protection & Secure Boot | Greater robustness against spurious switching noise induced by the DVD recorder's SMPS supplies. |

---

## 2. Crucial Technical Clarification: Passive JTAG / FTDI Adapters vs. Pico `P7501` Probe

> [!WARNING]
> **Why generic JTAG firmwares (`pico-dirtyJtag`, `UrJTAG`) or standard 3-wire USB-Serial adapters are NOT sufficient on connector `P7501`:**
>
> * **There is NO IEEE 1149.1 JTAG or synchronous interface on `P7501`:** Official schematics of board `VEP79138` confirm that connector **`P7501` (`TO JIGU`)** does not expose `TDI`/`TDO`/`TMS`/`TCK`. Driving those lines as JTAG causes electrical contention or communication failure.
> * **Verified Interface (`P7501`):** It is an **Asynchronous UART port (`115200 8N1`)** multiplexed through discrete hardware (`Q7510`, `Q7514`, `QR7503`): Pin 1 (`SW_UART`), Pin 2 (`XRESET` Open-Drain), Pin 3 (`UARTM2P_T`), Pin 4 (`GND`), Pin 5 (`NSW5V` — keep `Hi-Z`), and Pin 6 (`UARTP2M_T`).
> * **Advantage of the Pico Probe (`p7501_uart.pio`):** Passive serial adapters lacking synchronized control of `SW_UART` (`GP26`) and `XRESET` (`GP5`) neither enter `serial boot mode` nor avoid `Q7514` cutoff during TX; furthermore, software protection keeps `GP3` (`NSW5V`) strictly in `Hi-Z Input`.

---

## 3. Mechanical & Commercial Specification of Connector `P7501`

* **Official Connector Standard:** **6-pin JST ZR (1.5 mm Pitch)**, official Panasonic Part Number **`B6B-ZR-3R4`** (*Top Entry*), labeled *"TO JIGU"*.
* **Pin Spacing:** `1.5 mm` (center-to-center). Do not confuse with JST SH (1.0 mm), JST PH (2.0 mm), or JST XH (2.54 mm).
* **Commercial Search Guide:**  
  * Search for: `JST ZR / ZH 1.5mm 6-pin Female to Dupont 2.54mm Male cable` (`ZHR-6` female housing with `SZH-002T-P0.5` crimp terminals).
* **Bench Advantage:** Using a pre-made harness with a 1.5 mm housing removes any need for micro-soldering on the recorder's Digital PCB, connecting the Pico safely and plug-and-play.

---

## 4. 3-Phase Integration Roadmap

### Phase 1: Dual-Target RP2040 / RP2350 Support in C Firmware (Immediate)
* **Objective:** Enable compiling the native firmware for both architectures with zero C source modifications.
* **Deliverables:**
  * Update `CMakeLists.txt` with configurable target (`-DPICO_BOARD=pico` or `-DPICO_BOARD=pico2`).
  * Out-of-tree orchestration via `scripts/build.py --pico` (`npm run build:pico`) and native binary merging via `panadiga pico merge-uf2`.
  * Generation of UF2 artifacts:
    * `panasonic_p7501_uart_rp2040.uf2`
    * `panasonic_p7501_uart_rp2350.uf2`
    * `panasonic_p7501_uart_unified.uf2`

### Phase 2: GDB Remote Serial Protocol (RSP) Server Implementation on the Pico
* **Objective:** Connect high-level PC debugging tools (such as GNU Debugger `mn10300-elf-gdb`) directly to the Pico serial bridge over the USB CDC port.
* **Bridge Architecture:**
  ```text
  [ mn10300-elf-gdb ]  <--- GDB RSP Protocol (TCP/Serial) --->  [ Pico RP2040/RP2350 ]  <--- UART 115200 8N1 (P7501) --->  [ AM34 / MN103S ]
  ```
* **Supported GDB Commands in the Bridge:**
  * `?` : Query halt status (queries pause/active session state on the probe).
  * `g` / `G` : Read and write the AM34 register bank (`D0..D3`, `A0..A3`, `SP`, `PC`, `MDR`, `PSW`).
  * `m` / `M` : Read and write 32-bit words in SRAM and peripheral space (`0x80000000`).
  * `c` / `s` : Continue / Single-step instruction execution.

### Phase 3: Native Rust TCP <-> Serial Bridge & Continuous Telemetry
* **Objective:** Eliminate incompatible generic JTAG intermediaries (such as ARM OpenOCD) and provide a deterministic low-latency bridge integrated directly with the Tauri UI and GDB Multiarch (`set architecture mn10300`).
* **Strategy:**
  * Direct bidirectional USB CDC / BLE NUS forwarding via `panadiga pico gdb-bridge` on TCP port `127.0.0.1:3333`.
  * Introspective RSP packet ring buffer (`apps/desktop/src-tauri/src/pico/gdb_bridge.rs`) for real-time inspection in the desktop UI.
  * Native Ghidra Jython script generator (`tools/ghidra/`) derived from the Rust SSOT.
