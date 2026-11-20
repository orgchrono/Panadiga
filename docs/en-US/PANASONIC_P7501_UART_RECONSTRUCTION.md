> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/PANASONIC_P7501_UART_RECONSTRUCTION.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Technical Specification: `P7501` Async UART (`115200 8N1`) Interface & AM34 / MN103S Architecture

Technical specification for physical interfacing, boot capture, diagnostic probing, and memory architecture support on the **Panasonic DMR-EH55** recorder (**Matsushita `AM34` / `MN103S`** SoC + Syscon **`IC7501`**, board **`VEP79138`**, connector **`P7501`** `[B6B-ZR-3R4]` `"TO JIGU"`).

> [!WARNING]
> **Critical Empirical Erratum — Panasonic DMR-EH55 (`P7501` vs. Legacy `CN55001` / `DWire32A`):**
> Exhaustive cross-checking of the 3 official **DMR-EH55** service manuals (`CHM0605028CE`, `DSD0603003CE` Vol.1/Vol.2) combined with live electrical and logic captures via RP2040 (`COM10`: `PINCHECK`, `BUSDIAG`, `UARTPROBE`, `JIGBOOT`, `UARTMON`) **definitively proved that `P7501` (`B6B-ZR-3R4`, `TO JIGU`) on the DMR-EH55 is NOT `DWire32A`, but a dual-target Asynchronous UART `115200 8N1` + Boot/Reset Control interface**:
> - **Zero occurrences in EH55 manuals:** `MN2DS`, `MN2WS`, `CN55001`, `P55001`, `DWire`, and `IC55007` do not exist in DMR-EH55 schematics (they belong to another DIGA chassis — note that the Digital PCB carries the main Matsushita SoC labeled `VEP79138K` in the EH55 manuals). In the EH55, `VEP79138K` is the Digital PCB, `P59001` interconnects Main PCB $\leftrightarrow$ Digital PCB (`UARTM2P`, `UARTP2M`, `XFWE`), and `P7501` sits on the Main PCB wired to `IC7501` / Main SoC.
> - **Verified Physical Mapping on `P7501` ($\leftrightarrow$ Pico RP2040):** Pin 1 **`SW_UART`** (`GP26/ADC0`, Hi-Z `3.24–3.25V`, Pull-Down `2.68–2.69V`), Pin 2 **`XRESET`** (`GP5`, rise time `20 µs`), Pin 3 **`UARTM2P_T` / MAX RXD** (`GP4`, probe TX blocked by `Q7514` if `SW_UART=LOW`), Pin 4 **`GND`**, Pin 5 **`NSW5V` (`X_SW_5R2V`, ~5.15V nominal — keep `GP3` `Hi-Z`!)**, Pin 6 **`UARTP2M_T` / MAX TXD** (`GP2`, probe RX, the sole active `115200 8N1` UART TX pin).
> - See **Section 10** of [REVERSE_ENGINEERING_BOOTLOADER_AND_OPTICAL_DRIVE_MN103S.md](REVERSE_ENGINEERING_BOOTLOADER_AND_OPTICAL_DRIVE_MN103S.md) for full discrete circuit analysis (`IC1506`, `D7510`, `B7501`, `Q7510`, `Q7514`, `QR7503`, `C7519`, `K7503`).

---

## 1. True Interface Identification: `P7501` Async UART (`115200 8N1`)

Contrary to initial literature hypotheses based on other Panasonic chassis (`PanaX NEO` / `PX-ADPDWA` / `DWire32A`), the 6-pin service header **`P7501` (`JST B6B-ZR-3R4`, `TO JIGU`)** on the DMR-EH55 Main PCB `VEP79138` is an **asynchronous UART `115200 8N1` bus** coupled to a discrete transistor multiplex stage (`Q7510`, `Q7514`, `QR7503`) and switched supply rail **`NSW5V` (`IC1506` via `D7510`)**:

* **Processor Core:** Matsushita **AM34 / MN103S** (32-bit RISC architecture, `MN10300` family), linked via board-to-board harness **`P59001`** between Digital PCB `VEP79138K` and Syscon **`IC7501`**.
* **Inter-Board Bus Signals (`P59001`):** `UARTM2P`, `UARTP2M`, `XFWE`, `SCLK`, `SBPTM`, `SBMTP`, `XMPREQ`, `XINTM`, `XINTP`.

```text
       CANONICAL SERVICE CONNECTOR P7501 [B6B-ZR-3R4] ("TO JIGU" — 1.5mm Pitch)
       ┌───────────────────────────────────────────────────────────────────────┐
Pin 1  │ ● SW_UART   │ Boot / Serial Enable (Hi-Z ~3.25V; LOW -> XFWE / cuts Q7514)
Pin 2  │ ● XRESET    │ Hardware Reset (Active-LOW Open-Drain via QR7503, ~20µs rise)
Pin 3  │ ● UARTM2P_T │ MAX RXD (Pico TX -> AM34 RX via Q7514; onboard R7570 10kΩ)
Pin 4  │ ● GND       │ Common Ground (0.0Ω directly to Metal Chassis!)
Pin 5  │ ● NSW5V     │ X_SW_5R2V (IC1506 ~5.15V / D7510, ~3.29V idle — KEEP Hi-Z!)
Pin 6  │ ● UARTP2M_T │ MAX TXD (Pico RX <- AM34 TX 115200 8N1; Square PCB Pad)
       └───────────────────────────────────────────────────────────────────────┘
```

---

## 2. Electrical Hardware Interface Design (RP2040 / RP2350 $\leftrightarrow$ `P7501`)

Interconnecting the **Raspberry Pi Pico (RP2040 / RP2350)** to `P7501` requires strict adherence to the discrete network on `VEP79138` (`R7570`/`R7572` `10 kΩ`, `R7562` `4.7 kΩ`, `R7559` `10 kΩ`, `R7528` `4.7 kΩ`):

| Canonical `P7501` Pin | Schematic Signal | Circuit Role (`VEP79138`) | RP2040 / RP2350 GPIO | Conditioning / Protection | Electrical Function & Measured Behavior |
| :---: | :--- | :--- | :---: | :--- | :--- |
| **Pin 1** | `SW_UART` `(D)` | `Q7510` Collector, `QR7503` Base, `IC7501` Pin 80 | **`GP26 / ADC0`** (Pin 31) | Optional `1.0 kΩ` series | `3.24–3.25V` Hi-Z; `2.68–2.69V` pull-down. `LOW` drains `XFWE` and cuts off `Q7514`. |
| **Pin 2** | `XRESET` | Reset line `QR7503` | **`GP5`** (Pin 7) | **Open-Drain Mode** (opt. `100 Ω` series) | Driven `0V` when active; released to `Hi-Z` with `~20 µs` rise time. Never drive `+3.3V` push-pull! |
| **Pin 3** | `UARTM2P_T` (`MAX RXD`) | Via `Q7514` to `UARTM2P`, `R7570 10 kΩ` | **`GP4`** (Pin 6, UART TX) | No external pull-up (opt. `100 Ω` series) | Probe TX (`115200 8N1`) to board RX. Requires `SW_UART = Hi-Z` so `Q7514` conducts. |
| **Pin 4** | **`GND`** | `GND_DIGITAL` | **`GND`** (Pin 38) | Low-impedance direct tie | **Common `0V` reference ($0.0\,\Omega$ directly to chassis ground).** |
| **Pin 5** | **`NSW5V` (`X_SW_5R2V`)** | Supply `IC1506` (`~5.15V`) via `D7510` | **`GP3`** (Pin 5) | **STRICTLY `Hi-Z Input` no pulls** | Static rail at `~3.29V` idle (`~5.15V` upstream). **NEVER drive push-pull!** |
| **Pin 6** | `UARTP2M_T` (`MAX TXD`) | Output via `R7535` / `R7572 10 kΩ` | **`GP2`** (Pin 4, UART RX) | Optional `100 Ω` series | Sole active UART TX pin on board (`<00>DDR CHECK: OK`, `CD diag boot mode`). Square PCB pad. |

### Critical Electrical Assembly Rules

1. **Absolute Guard on `NSW5V` (`GP3` / Pin 5):**
   * `IC1506` (`SI-3011ZFE`) outputs `~5.15V` reaching `R7570`/`R7572` and `IC7501 VDD` via `D7510`. Accordingly, RP2040/RP2350 `GP3` must stay locked in `GPIO_IN` with `gpio_disable_pulls(3)` throughout execution. **Never** power the Pico from `P7501` and **never** drive `GP3` high or low.
2. **Short-Circuit Prevention on `XRESET` (`GP5` Open-Drain):**
   * `GP5` toggles direction only: `0V` (output low) to assert reset, and unpulled `GPIO_IN` (`Hi-Z`) to release the line.
3. **Coupling `SW_UART` $\leftrightarrow$ `Q7514` (`UARTM2P_T`):**
   * When `SW_UART = LOW` (`0V`), `Q7510` pulls `XFWE` low (forcing serial/JIG boot mode) while simultaneously biasing `Q7514` into cutoff (via `R7528 4.7 kΩ`), physically isolating Pin 3 (`UARTM2P_T`) from the SoC `UARTM2P`. For TX bits on `GP4` to reach the SoC, `SW_UART` (`GP26`) must be released to **`Hi-Z`**.
4. **No External `4.7 kΩ` Pull-Up on `UARTM2P_T`:**
   * Board `VEP79138` already includes `R7570` (`10 kΩ`) and `R7572` (`10 kΩ`). Do not solder an external `4.7 kΩ` pull-up between `P7501` and Pico `3V3_OUT`.
5. **Hardware Write-Protect Check on Spansion Pin 14 (`WP#/ACC`, `IC5001`):**
   * On Spansion `S29JL064H70TFI00` (`IC5001`, Top Boot), holding Pin 14 (`WP#/ACC`) at `0V` hardware-protects the top two micro-sectors (`SA140` and `SA141`, range `0x7FC000`–`0x7FFFFF`). Verify voltage with a multimeter before any in-circuit Flash erase (`3.3V` = unprotected).

---

## 3. PIO State Machine (RP2040 / RP2350) for `115200 8N1` Async RX

The **PIO (`PIO0`)** subsystem deterministically samples bytes from `UARTP2M_T` (`GP2`, `P7501` Pin 6) at **`115200 8N1`** (8x oversampling at `921,600 Hz`) with zero ARM CPU interrupt overhead and without ever touching `GP3` (`NSW5V`).

### Verified PIO Assembly Source ([`firmware/pico-p7501/p7501_uart.pio`](../../firmware/pico-p7501/p7501_uart.pio))

```pio
.program p7501_uart_rx

; =========================================================================
; Panasonic P7501 Asynchronous 115200 8N1 UART Receiver (Passive Input Only)
; Oversampling: 8 SM cycles per bit (921,600 Hz @ 115200 baud)
;
; Pin mapping:
;   IN / JMP pin: GP2 (P7501 Pin 6 UARTP2M_T, MAX TXD -> Pico RX)
;   GP3 (P7501 Pin 5 NSW5V) is NEVER touched by PIO and stays High-Z Input.
; =========================================================================

.wrap_target
start:
    wait 0 pin 0            ; Wait for falling edge of Start bit (GP2 == 0)
    set x, 7        [10]    ; 8 data bits; wait 1.5 bit periods (12 cycles total to bit 0 center)
bitloop:
    in pins, 1              ; Shift LSB-first data bit into ISR
    jmp x--, bitloop [6]    ; Loop every 8 cycles (1 + 7 cycles)
    jmp pin, good_stop      ; Verify Stop bit is HIGH
    irq 4 rel               ; Framing error signaled on IRQ 4
    wait 1 pin 0            ; Wait for idle HIGH recovery
    jmp start
good_stop:
    push block              ; Deliver MSB-aligned byte in upper 8 bits of RX FIFO word
.wrap
```

---

## 4. Discrete Boot Timing (`SW_UART` + `XRESET`) & `P7501` Primitives

When running `JIGBOOT` or discrete boot profiles via [`p7501_uart_bitbang.h`](../../firmware/pico-p7501/uart/p7501_uart_bitbang.h):

1. **Open-Drain Pulse on `XRESET` (`GP5`):** `p7501_hw_reset_assert()` pulls `0V` for `15 ms`, and `p7501_hw_reset_release()` releases `GP5` to `Hi-Z` (~`20 µs` rise time via `QR7503`).
2. **Boot Sampling & UART Capture (`115200 8N1`):** Under `JIGBOOT` (`SW_UART = LOW` across reset release), the bootloader outputs `<00>DDR CHECK: OK` followed by `CD diag boot mode` on `GP2` (`UARTP2M_T`), reporting optical mechanism sense telemetry (`X(80020401)59...` $\rightarrow$ `X(80040904)` $\rightarrow$ `Drive start failed[-2147219196]`).
3. **Interactive TX / Bootrom Probing (`UARTSEND` / `FMUSCAN`):** To transmit characters at `115200 8N1` on `GP4` (`UARTM2P_T`), invoke `ENGAGE` with `SW_UART` at `Hi-Z` so `Q7514` stays forward-biased and passes bits to the SoC.
4. **Safe Bus Isolation (`ISOLATE` / `HIZ`):** Tri-states TX drivers at completion, restores `GP26` (`ADC0`) via `p7501_hw_restore_sw_uart_adc0()`, and keeps `GP3` (`NSW5V`) strictly `Hi-Z Input`.

---

## 5. Physical Memory Map of the AM34 (`MN103S`) Architecture

Per the **Matsushita AM33v2 / AM34** architecture specification (and Linux `arch/mn10300` / GNU Binutils conventions), the 32-bit linear address space is partitioned into cached mirrors and direct uncached physical windows:

| Address Range (Hex) | AM34 Architectural Region | Mapped Device on Board `VEP79138` |
| :--- | :--- | :--- |
| `0x00000000` – `0x007FFFFF` | Cached Boot Space | Mirror of Spansion `S29JL064H` NOR Flash (`IC5001`, 8 MB) |
| **`0x80000000` – `0x807FFFFF`** | **Direct Physical Space (`CS0`, Uncached)** | **Direct Physical Access to Spansion `IC5001` (`CS0` / `SB0`)** |
| **`0x84000000` – `0x847FFFFF`** | **Uncached `CS1` NOR Flash Window** | **Live `PANADMP1` / `PANAMON1>` window**: IPL (`0x84000000..0x8403FFFF`), real EH55 `ver0.61` BGO (`0x84130000..0x843FFFFF`, `SA26..SA70` vs `ver0.63` `0x84200000..0x843FFFFF`), Upper EEMUL (`SA71..SA141`, `0x84400000..0x847FFFFF`) |
| `0x8C000000` / `0x90000000` | Uncached SDRAM Space | Main SDRAM chips (`0x90400000` uncached alias of `0x50400000` used by `PANAMON1>` at `0x9043950E` $\rightarrow$ `J 904394FA` and `0x90439B40` for I-cache/D-cache coherence) |
| `0xC0000000` – `0xDFFFFFFF` | SoC Internal Peripherals | Serial Controllers (`SC0`–`SC2` at `0xD4002000`), Timers, GPIO, ATA/IDE |
| `0xFF000000` – `0xFFFFFFFF` | Core Control Registers | CPU core configuration & interrupt controllers |

> [!CRITICAL]
> **Golden Rule for CPU-Driven JEDEC Programming:**
> Every JEDEC erase or program command issued by the *RAM Loader* to the Spansion Flash **must target the uncached physical address window (`0x80000000` + offset)** and never the `0x00000000` cached mirror.
> Writing unlock cycles (`0xAA` / `0x55`) to `0x00000000` while L1 D-cache is active absorbs those writes inside cache lines, preventing bus cycles from ever toggling physical Flash pins.

---

## 6. "RAM Loader" Source Code (Native AM34 JEDEC Driver for Spansion `S29JL064H`)

When executing payloads in internal SRAM (`0x00004000`), the firmware embeds a position-independent micro-binary (< 200 bytes) running the 16-bit (`Word Mode`) JEDEC algorithm to erase the top 8 micro-sectors of 8 KB (`SA134` through `SA141`, range `0x7F0000` to `0x7FFFFF`).

### 16-Bit (`Word Mode`) Unlock Address Translation

In 16-bit word mode (`BYTE# = High`), standard 16-bit JEDEC unlock addresses (`0x555` and `0x2AA`) map onto CPU byte addresses via a 1-bit left shift:
* `Unlock Addr 1`: `0x555 << 1` = **`0xAAA`** (Data: `0x00AA`)
* `Unlock Addr 2`: `0x2AA << 1` = **`0x554`** (Data: `0x0055`)
* **Uncached Base Address:** `0x80000000`
* **Erase Targets (8 x 8 KB `EEMUL` micro-sectors):**
  * `0x807F0000` (`SA134`) through `0x807FE000` (`SA141`)

### Position-Independent C11 Source ([`firmware/pico-p7501/flash/ram_loader_am34_target.c`](../../firmware/pico-p7501/flash/ram_loader_am34_target.c))

```c
/**
 * @file ram_loader_am34_target.c
 * @brief Autonomous AM34 / MN103S position-independent SRAM payload.
 *
 * Injected into internal SRAM via P7501 / AM34 monitor transport
 * to erase the corrupted NVRAM emulation partition (EEMUL) on the Spansion
 * S29JL064H70TFI00 NOR Flash, clearing the DMR-EH55 U99 fatal state.
 *
 * Target Processor: Matsushita UniPhier / AM34 / MN103S
 * Target Hardware:  Panasonic DMR-EH55 Digital Board (VEP79138K)
 *
 * Compilation:
 *   mn10300-elf-gcc -mam33-2 -std=c11 -Os -fPIC -nostdlib -c ram_loader.c -o ram_loader.o
 *   mn10300-elf-objcopy -O binary -j .text.entry -j .text ram_loader.o ram_loader.bin
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include <stdint.h>
#include <stdbool.h>

#define FLASH_UNCACHED_BASE         0x80000000UL
#define JEDEC_UNLOCK_ADDR1_OFFSET   0x0AAAUL
#define JEDEC_UNLOCK_ADDR2_OFFSET   0x0554UL

#define JEDEC_CMD_AA                0x00AAU
#define JEDEC_CMD_55                0x0055U
#define JEDEC_CMD_ERASE_SETUP       0x0080U
#define JEDEC_CMD_SECTOR_ERASE      0x0030U
#define JEDEC_CMD_READ_RESET        0x00F0U

#define JEDEC_POLL_DQ7_MASK         0x0080U
#define JEDEC_ERROR_DQ5_MASK        0x0020U

#define SPANSION_NVRAM_BASE_ADDR    0x007F0000UL
#define SPANSION_NVRAM_END_ADDR     0x00800000UL
#define SPANSION_MICROSECTOR_STRIDE 0x00002000UL

#define FLASH_REG16(addr) (*(volatile uint16_t *)((uintptr_t)(addr)))
#define JEDEC_REG_UNLOCK1 FLASH_REG16(FLASH_UNCACHED_BASE + JEDEC_UNLOCK_ADDR1_OFFSET)
#define JEDEC_REG_UNLOCK2 FLASH_REG16(FLASH_UNCACHED_BASE + JEDEC_UNLOCK_ADDR2_OFFSET)

static bool spansion_erase_sector(uint32_t sector_offset) {
    volatile uint16_t *sector_ptr =
        (volatile uint16_t *)((uintptr_t)(FLASH_UNCACHED_BASE + sector_offset));

    /* 6-cycle JEDEC Sector Erase command sequence in 16-bit word mode */
    JEDEC_REG_UNLOCK1 = JEDEC_CMD_AA;
    JEDEC_REG_UNLOCK2 = JEDEC_CMD_55;
    JEDEC_REG_UNLOCK1 = JEDEC_CMD_ERASE_SETUP;
    JEDEC_REG_UNLOCK1 = JEDEC_CMD_AA;
    JEDEC_REG_UNLOCK2 = JEDEC_CMD_55;
    *sector_ptr       = JEDEC_CMD_SECTOR_ERASE;

    while ((*sector_ptr & JEDEC_POLL_DQ7_MASK) == 0U) {
        if ((*sector_ptr & JEDEC_ERROR_DQ5_MASK) != 0U) {
            if ((*sector_ptr & JEDEC_POLL_DQ7_MASK) == 0U) {
                *sector_ptr = JEDEC_CMD_READ_RESET;
                return false;
            }
            break;
        }
    }

    *sector_ptr = JEDEC_CMD_READ_RESET;
    return true;
}

void __attribute__((section(".text.entry"), noreturn)) _start(void) {
    for (uint32_t offset = SPANSION_NVRAM_BASE_ADDR;
         offset < SPANSION_NVRAM_END_ADDR;
         offset += SPANSION_MICROSECTOR_STRIDE) {
        (void)spansion_erase_sector(offset);
    }

    __asm__ volatile ("pi");

    while (1) {
        __asm__ volatile ("nop");
    }
}
```

---

## 7. Firmware Disassembly & Static Analysis Guide (8 MB)

Once the Spansion `S29JL064H` (**`IC5001`**) is read on the bench (e.g., via an external SVOD 4 / XGecu T48 programmer), the intact 8 MB binary (`8,388,608` bytes) exposes Panasonic's proprietary OS internals.

### Step 1: Endianness Check & *Byte-Swap* Normalization

Depending on the TSOP-48 socket adapter and 16-bit word ordering in programmer software, the dump may be saved byte-swapped relative to little-endian CPU view:

* Open the dump in **HxD** and search for known ASCII strings: `PANASONIC`, `MATSUSHITA`, `DMR`, `MEIHFS`.
* If characters appear pairwise swapped (`APANSNOIC`, `AMSTSUHITA`), each 16-bit word is byte-reversed.
* Normalize byte order for decompiler analysis:

```python
with open('DMR_EH55_RAW.bin', 'rb') as f:
    data = f.read()
# Byte-swap every 16-bit word
swapped = bytearray(len(data))
swapped[0::2] = data[1::2]
swapped[1::2] = data[0::2]
with open('DMR_EH55_SWAPPED.bin', 'wb') as f:
    f.write(swapped)
```
*(Note: images physically flashed back to the chip must match the native byte order expected by the programmer).*

### Step 2: Disassembly with Official GNU `binutils` (`mn10300-elf-objdump`)

```bash
mn10300-elf-objdump -D -b binary -m am33-2 --adjust-vma=0x80000000 DMR_EH55_ORIGINAL.bin > dmr_eh55_asm.txt
```

### Step 3: Immediate Inspection Checklist in `dmr_eh55_asm.txt`

1. **Boot Vector at `0x80000000`:** Contains reset startup instructions, bus controller configuration, and initial `SP` setup.
2. **`EEMUL` Checksum Routine referencing `0x807F0000`:** Cross-references to `0x807F0000..0x807FFFFF` isolate NVRAM micro-sector validation.
3. **Serial Peripherals (`0xD4002000`):** Boot/diagnostic UART routines wired to **`P7501`**.

---

## 8. Bench Recovery & Diagnostic Method Comparison

| Criterion | Method 1: External Programmer (SVOD 4 / XGecu T48 on `IC5001`) | Method 2: `P7501` Async UART Probe (RP2040 / RP2350) |
| :--- | :--- | :--- |
| **Technical Maturity** | **100% proven for external TSOP-48 backup/recovery** | **Proven live on `COM10` for 8 MiB dump (`PANADMP1`) & surgical BGO `U99` repair (`PANAMON1>`) without desoldering** |
| **Physical Work** | Hot-air removal of `IC5001` (TSOP-48) | Non-destructive plug-in of 6-pin **`P7501`** (`B6B-ZR-3R4`) + SD `DIGADIAG.FRM` |
| **Operational Scope** | Full 8 MB raw dump and clean `EEMUL` / BGO write-back | Full 8 MiB capture (`eh55_nor_8mb_ic5001_dump.bin`), normal boot (`*** RTSC Initialize ***`), `FMU`/Sense diagnostics, `U99` repair |
| **Hardware Risk** | Minimal with experienced SMD rework | Minimal when keeping **`NSW5V` (`GP3`) `Hi-Z Input`** and `XRESET` (`GP5`) Open-Drain |
| **Software Tooling** | Native CLI `panadiga nvram clean` wipes `0x7F0000`–`0x7FFFFF` | Firmware `panasonic_p7501_uart` + `panadiga` CLI / Tauri UI |

For deterministic physical `U99` repair on units where the serial monitor does not expose direct Flash write access over `P7501`, reading `IC5001` externally and running **`panadiga nvram clean`** restores the 8 virgin (`0xFF`) micro-sectors, clearing the NVRAM panic permanently (while `BGO` recovery on live units proceeds via `DIGADIAG.FRM` or clean Flash image rebuild).
