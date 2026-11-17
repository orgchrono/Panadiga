> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/FIRMWARE_PICO_P7501_GUIA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Panasonic P7501 Async UART & Boot Probe Firmware Engine (RP2040 / RP2350)

Official bench controller firmware built for **Raspberry Pi Pico (RP2040)** and **Raspberry Pi Pico 2 (RP2350)** to perform boot capture, electrical diagnostics, and asynchronous UART communication (**`115200 8N1`**) across factory service connector **`P7501` (`B6B-ZR-3R4`, labeled `TO JIGU`)** on the **Panasonic DMR-EH55** recorder (**Matsushita `AM34` / `MN103S`** SoC + Syscon `IC7501`, board `VEP79138`).

---

## 1. System Overview

The modular C11 firmware in [`firmware/pico-p7501/`](../../firmware/pico-p7501/) implements strict ISO C11 hardware control (with production modules organized across `common/`, `hal/`, `uart/`, `diag/`, `flash/`, `protocol/`, `gdb/`, and `cli/`):

1. **PIO1 Full-Duplex Multi-Baud UART Engine + Legacy PIO0 (`p7501_uart_duplex.pio` / `uart/p7501_uart_pio_engine.c`):**
   * Deterministic passive reception via **`PIO1` (`SM0`)** on **`GP2`** (`P7501` Pin 6 `UARTP2M_T` / `MAX TXD`) and dedicated transmission via **`PIO1` (`SM1`)** on **`GP4`** (`P7501` Pin 3 `UARTM2P_T` / `MAX RXD` through transistor `Q7514`), featuring fixed-point fractional dividers for dynamic rates (`BAUD` and autobaud `BAUDSCAN`), live streaming (`UARTSTREAM`), microsecond-delta RAM logging (`HEXCAP`, zero-drop at `115200 8N1`), trigger-based injection (`WAITTX`), and transparent USB-CDC ↔ SC0 bridging (`BRIDGE`).
   * Retains the legacy `PIO0` engine (`p7501_uart.pio` / `uart/p7501_uart_capture.c`) for backward compatibility with `BOOTCAP` / `UARTMON` / `JIGBOOT`.
2. **Safe Open-Drain Reset Control (`XRESET` / `GP5`):** `GP5` (`P7501` Pin 2) toggles strictly between `0V` (active reset) and `Hi-Z` (unpulled input, `~10–20 µs` rise time through `QR7503`), preventing contention against Panasonic's reset supervisor.
3. **Strict Guard on `NSW5V` (`GP3` / `P7501` Pin 5) & `SW_UART` Control (`GP26` / `ADC0`, Pin 1):**
   * **`GP3` (`NSW5V` / `X_SW_5R2V`):** Supplied by `IC1506` (`~5.15V` upstream via `D7510`, resting at `~3.29V` across `R7570`/`R7572`). **ALWAYS remains a `Hi-Z Input` with pulls disabled** and is never driven push-pull.
   * **`GP26` (`SW_UART`):** Samples voltage via `ADC0` (`~3.24–3.25V` Hi-Z, `~2.70V` pull-down, `~3.26V` pull-up) and pulls `LOW` strictly for the exact serial-boot sampling window (`XFWE`, e.g., `JIG:350`), restoring `ADC0 Hi-Z` via `p7501_hw_restore_sw_uart_adc0()` whenever `GP4` TX must pass through `Q7514`.
4. **Non-Blocking LED Driver with Manual Lock (`hal/p7501_led.c` & `hal/p7501_led_logic.c`):** Pure 10 Hz state machine driven by an IRQ-safe `repeating_timer` (`GP25` on Pico/RP2040 and Pico 2/RP2350; tick fallback to `CYW43_WL_GPIO_LED_PIN` on Pico W / Pico 2 W), enforcing `MANUAL_LOCK` so periodic `STATUS` polling never overrides operator selection (`LED <IDLE|ACTIVE|GDB|FAULT|OFF|AUTO>`).
5. **USB CDC + Optional BLE Serial Console (`PANADIGA> `):** Interactive `115200 bps` command shell compatible with standard terminals (PuTTY, TeraTerm), `panadiga` CLI, and the Tauri Desktop/Android app (`panadiga-desktop` / APK). `pico_w` and `pico2_w` builds also enable a Bluetooth LE Nordic UART Service bridge (`PANADIGA_BLE_ENABLED=1`).
6. **GDB RSP Bridge & Fail-Loud Injection/Sanitization:** Embeds a **GNU Remote Serial Protocol (RSP)** server (`GDB`) plus `AUTODETECT` / `INJECT_LOADER CONFIRM` / `ERASE_NVRAM CONFIRM` primitives that refuse to operate blindly when `DCSR=0xFFFFFFFF` (no synchronous monitor connected on `P7501`); for units locked out without desoldering monitors, the verified recovery write path for Spansion `S29JL064H` (`IC5001`) is an external TSOP-48 programmer (`SVOD 4` / `XGecu T48`) + `panadiga nvram clean` or autonomous `DIGADIAG.FRM` via `SDDIAG`.

---

## 2. Build Toolchain Requirements

To compile the firmware from source:
* **CMake** $\ge$ 3.13 (on `PATH`)
* **ARM GCC Toolchain:** `arm-none-eabi-gcc` $\ge$ 10.3
* **Build System:** `Ninja` (recommended) or `MinGW Makefiles`
* **Raspberry Pi Pico SDK:** configured via environment variable `PICO_SDK_PATH` (imported by [`firmware/pico-p7501/pico_sdk_import.cmake`](../../firmware/pico-p7501/pico_sdk_import.cmake))

---

## 3. Building the Multi-Platform Firmware (RP2040 and RP2350)

The subsystem in [`firmware/pico-p7501/`](../../firmware/pico-p7501/) provides native multi-board support (`pico`, `pico_w`, `pico2`, `pico2_w`), orchestrated by cross-platform builder [`scripts/build.py`](../../scripts/build.py) and the native Rust CLI (building *out-of-tree* in `target/pico-build/`):

### Option A: Unified Repository Builder (Windows / Linux / macOS)
```bash
# Builds RP2040 + RP2350, generates panasonic_p7501_uart_unified.uf2, and syncs crates/panadiga-core/src/pico/firmware_bins/
npm run build:pico
# Or directly via Python:
python scripts/build.py --pico
```

### Option B: Manual CMake + Native UF2 Merge via Rust CLI
```bash
cmake -S firmware/pico-p7501 -B target/pico-build/pico -G Ninja -DCMAKE_BUILD_TYPE=Release -DPICO_BOARD=pico
cmake --build target/pico-build/pico --config Release

cmake -S firmware/pico-p7501 -B target/pico-build/pico2 -G Ninja -DCMAKE_BUILD_TYPE=Release -DPICO_BOARD=pico2
cmake --build target/pico-build/pico2 --config Release

cargo run -p panadiga-cli -- pico merge-uf2 \
  --rp2040 target/pico-build/pico/panasonic_p7501_uart.uf2 \
  --rp2350 target/pico-build/pico2/panasonic_p7501_uart.uf2 \
  --out target/pico-build/out/panasonic_p7501_uart_unified.uf2
```

### Generated Binary Artifacts
Builds synchronize self-contained `.uf2` images into `crates/panadiga-core/src/pico/firmware_bins/` (embedded in the Rust binary) and into `dist/`:
* **`panasonic_p7501_uart_rp2040.uf2`** $\leftarrow$ Image for Raspberry Pi Pico / Pico W and compatible RP2040 boards (264 KB SRAM).
* **`panasonic_p7501_uart_rp2350.uf2`** $\leftarrow$ Image for Raspberry Pi Pico 2 / Pico 2 W (RP2350 ARM Cortex-M33 / Hazard3 RISC-V, 520 KB SRAM).
* **`panasonic_p7501_uart_unified.uf2`** $\leftarrow$ Universal image concatenating RP2040 + RP2350 blocks into a single `.uf2`.
* Auxiliary images (`.elf`, `.bin`, `.hex`, `.map`) generated by `pico_add_extra_outputs(panasonic_p7501_uart)` remain in `target/pico-build/`.

### Continuous Integration (CI/CD)
The CI workflow in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) and local gate runner (`python scripts/validate.py` / `npm run validate`) verify Rust, TypeScript, C11 host tests ([`test_gdb_rsp_host.c`](../../firmware/pico-p7501/tests/test_gdb_rsp_host.c), [`test_pana_diag_drv.c`](../../firmware/pico-p7501/tests/test_pana_diag_drv.c), [`test_pana_logic.c`](../../firmware/pico-p7501/tests/test_pana_logic.c), and [`test_uart_codec.c`](../../firmware/pico-p7501/tests/test_uart_codec.c)), Python (`mypy --strict`, `pytest`), `jscpd`, Vitest, and Playwright.

---

## 4. Flashing the `.uf2` File onto the Raspberry Pi Pico

The RP2040 / RP2350 includes a native ROM USB bootloader (**UF2 Bootloader**). Automated flashing is also supported via `panadiga pico flash --port COM10`:

```text
        DRAG & DROP FLASHING ON RASPBERRY PI PICO
   ┌───────────────────────────────────────────────────────────┐
   │  1. Unplug the Pico USB cable                             │
   │  2. Hold down the white BOOTSEL button                    │
   │  3. Plug the USB cable into the host PC                   │
   │  4. Release the BOOTSEL button                            │
   │  5. Volume "RPI-RP2" (or "RP2350") mounts on the OS       │
   │  6. Copy "panasonic_p7501_uart_unified.uf2" to the drive  │
   │  7. The Pico reboots automatically into the new firmware! │
   └───────────────────────────────────────────────────────────┘
```

*(With the firmware already running over USB CDC, you can also send `BOOTSEL` at the `PANADIGA> ` prompt to enter UF2 mode without pressing the physical button.)*

---

## 5. USB CDC Serial Connection (Terminal & Tauri)

After rebooting, the Pico enumerates a USB CDC serial port (`COMx` on Windows, `/dev/ttyACM0` on Linux, `/dev/cu.usbmodem*` on macOS, or USB-OTG on Android):
* **Baud Rate:** `115200` (native 12 Mbps USB CDC)
* **Framing:** 8 data bits, no parity, 1 stop bit (`8N1`), no flow control

Upon connecting or pressing `<ENTER>`, the console prints the official banner from [`p7501_cli_help.c`](../../firmware/pico-p7501/cli/p7501_cli_help.c):

```text
====================================================================
   PANADIGA: Panasonic DMR-EH55 P7501 Async UART & Boot Bench Probe
   MCU Host : RP2040 (Pico / ARM Cortex-M0+)
   Bus      : P7501 (B6B-ZR-3R4) 115200 8N1 (PIO1) + SW_UART / XRESET / NSW5V(Hi-Z)
   [GDB-RSP] Active Server Mode (RP2040 8 KB Cache)
   Type HELP for available P7501 UART, PIO1, FMU, diag, Flash, LED & GDB commands.
====================================================================

PANADIGA> 
```

---

## 6. Complete Command Reference (`p7501_cli.c` / `p7501_cli_help.c`)

The firmware serves interactive human operators as well as Rust automation (`panadiga`), bench scripts (`scripts/bench/`), and the Tauri reactive UI (`SSOT` in [`crates/panadiga-core/src/ssot/spec.rs`](../../crates/panadiga-core/src/ssot/spec.rs)):

| Command | Syntax | Verified Operational Behavior |
| :--- | :--- | :--- |
| **`PING`** | `PING` | Replies `PONG` for fast link health checks. |
| **`STATUS` / `VDD`** | `STATUS` \| `VDD` | Samples `GP26 (ADC0)` on `P7501` Pin 1 (`SW_UART`) and prints `VDD: X.XXV (OK / LOW / FLOATING)`. |
| **`BANNER`** / **`HELP`** | `BANNER` \| `HELP` \| `?` | Prints the identification banner or complete command list. |
| **`BAUD`** | `BAUD [bps]` | Queries or configures **`PIO1` (`SM0`/`SM1`)** baud rate using FP16 fractional divisor (`300..921600 bps`). |
| **`BAUDSCAN`** | `BAUDSCAN [ms] [NORMAL\|JIG[:ms]\|NOPULL] [APPLY]` | Glitch-immune dual-polarity (LOW/HIGH) bit-width cluster detector + PIO1 `IRQ 4` framing & ASCII counter on `GP2` (`APPLY` switches `PIO1`). **Note:** Changing baud in PuTTY/RealTerm only affects virtual USB CDC — `PIO1` controls physical `P7501` sampling. |
| **`UARTSTREAM`** | `UARTSTREAM [ms] [NORMAL\|JIG[:ms]\|NOPULL]` | Live capture via `PIO1` with timed `SW_UART` release (`JIG:350`) and automatic `ADC0` restoration. |
| **`HEXCAP`** | `HEXCAP [ms] [NORMAL\|JIG[:ms]\|NOPULL]` | RAM capture (`HEXCAP_RAM_BUF_CAP` = `2048` bytes) with `+us` deltas and hex+ASCII dump after window (zero overflow at `115200 bps`). |
| **`UARTTXHEX`** | `UARTTXHEX [LOW:]<hex\|BRK> [rx_ms]` | Injects arbitrary hex bytes or a `120 ms` `BREAK` pulse over `PIO1 TX` (`Q7514` requires `SW_UART` released unless prefixed with `LOW:`). |
| **`WAITTX`** | `WAITTX <ms> <NORMAL\|JIG[:ms]\|NOPULL> "<trig>" <hex\|BRK> [post_ms]` | Waits for trigger substring on `GP2` (e.g., `"wait 1"` after `JIG:350`) and injects payload/BREAK via `PIO1 TX` keeping `NSW5V` `Hi-Z`. |
| **`BRIDGE`** | `BRIDGE [baud] [NORMAL\|JIG[:ms]\|NOPULL]` | Transparent byte bridge USB-CDC ↔ `SC0` (`PIO1`), terminated by escape sequence `~~~` (or timeout). |
| **`PWRTRACE`** | `PWRTRACE [ms] [NORMAL\|JIG[:ms]\|NOPULL]` | Samples min/max `GP26_SWUART` via `ADC0` and counts edges on `RXD` (`GP2`), `NSW5V` (`GP3`), and `TXD` (`GP4`) during boot. |
| **`BOOTCAP`** | `BOOTCAP [ms]` | Asserts Open-Drain `XRESET` with `SW_UART` released and captures `115200 8N1` boot UART stream (`PIO0`). |
| **`UARTMON`** | `UARTMON [ms]` | Passively listens on `GP2` (`UARTP2M_T`) without forcing reset or `SW_UART`. |
| **`JIGBOOT`** | `JIGBOOT [ms]` | Boots with `SW_UART = LOW` (`XFWE` / JIG mode) across `XRESET` pulse and captures `GP2` at `115200 8N1`. |
| **`JIGPIN`** | `JIGPIN [LOW\|HIZ\|STATUS]` | Drives Open-Drain control on `P7501` Pin 1 (`SW_UART` on `GP26`, historically nicknamed `"TW7501"` in legacy CLI strings — distinct from the physical `TW7501` pad on `IC7501`'s 5V backup rail). |
| **`UARTSEND`** | `UARTSEND [LOW:]<text>` | Transmits ASCII/bytes on `GP4` (`UARTM2P_T`) at `115200 8N1` (default `Hi-Z` enables `Q7514` conduction; `LOW:` prefix forces `SW_UART` low). |
| **`UARTPROBE`** | `UARTPROBE [sec]` | Runs serial probing on `P7501` and restores `GP26` to `ADC0 Hi-Z` afterward. |
| **`PANARSP`** | `PANARSP [ms] [boot] [WAIT:x] <act>` | `_dev_SERIAL` RSP bridge (`REGS`, `MEM`, `WMEM`, `DUMPNOR`, `CALL`, `NORUNLOCK:CONFIRM_UPPER_NOR`, `NORLOCK`, `JEDEC_ERASE:<addr>`, `JEDEC_PROG:<addr>,<hex>`). |
| **`DIAGWATCH`** | `DIAGWATCH [ms] [boot] [act] [WAKE]` | Autonomous `SC0` watchdog and resident `PANAMON1>` monitor (`NORUNLOCK:CONFIRM_UPPER_NOR` / `NORLOCK` dispatching two-step `'U'` → `'W'`/`'E'`). |
| **`IRTX`** | `IRTX <RED\|SERV\|DEVSUB:CMD...> [GPpin] [repeat]` | 38 kHz Kaseikyo PWM synthesizer ([`p7501_ir_codec.c`](../../firmware/pico-p7501/diag/p7501_ir_codec.c)) constrained to safe spare GPIOs `GP6..9` / `GP16..22` (device `>= 0x01`). |
| **`FMUSCAN` / `FMUTEST` / `FMUBATCH`** | `FMUSCAN` \| `FMUTEST` \| `FMUBATCH` | Timing probes and byte scans against Bootrom/FMU over `P7501`. |
| **`PINCHECK`** | `PINCHECK` | 6-pin electrical matrix check on `P7501` (`Hi-Z`, pull-down, pull-up on `GP26`, edge count on `GP2`, `XRESET` rise time, `NSW5V`/`GP3` guard). |
| **`BUSDIAG` / `SNIFF`** | `BUSDIAG` \| `SNIFF [ms]` | Bus activity diagnostic and passive transition counting. |
| **`ISOLATE` / `HIZ`** | `ISOLATE` \| `HIZ` | Disables TX drivers and places interface pins into safe high-impedance (`NSW5V` and `XRESET` `Hi-Z`, `GP26` `ADC0`). |
| **`ENGAGE`** | `ENGAGE` | Sets `GP4` (`UARTM2P_T`) idle HIGH for active UART TX while keeping `NSW5V` (`GP3`) strictly `Hi-Z Input`. |
| **`RESET`** | `RESET` | Generates an active-low Open-Drain pulse on `XRESET` (`GP5`), then releases to `Hi-Z`. |
| **`HANDSHAKE` / `SCAN` / `BREAK_IN`** | `BREAK_IN [0..3]` \| `SCAN` | Discrete `SW_UART + XRESET` sequence profiles (`0..3`) and bus response check. |
| **`DUMP` / `AUTODETECT` / `CFI` / `FLASH_SIZE`** | `DUMP [addr] [words]` \| `FLASH_SIZE [2\|4\|8\|16]` | Monitor memory read, fail-loud check (`DCSR=0xFFFFFFFF` aborts), CFI `QRY` query, and `EEMUL` geometry selection. |
| **`INJECT_LOADER`** | `INJECT_LOADER CONFIRM` | Loads AM34 micro-driver (`ram_loader_am34_target.c`) into `0x00004000` (requires `CONFIRM` token and active synchronous monitor). |
| **`ERASE_NVRAM`** | `ERASE_NVRAM CONFIRM` | Executes JEDEC erase across NVRAM/EEMUL micro-sectors (requires `CONFIRM` token and active target bus access). |
| **`GDB`** | `GDB` | Switches the USB CDC channel into the **GDB Remote Serial Protocol (RSP)** server. |
| **`LED`** | `LED [IDLE\|ACTIVE\|GDB\|FAULT\|OFF\|AUTO\|STATUS]` | Selects a visual pattern with `MANUAL_LOCK` (`IDLE`, `ACTIVE`, `GDB`, `FAULT`, `OFF`), restores automatic control (`AUTO`), or queries status (`STATUS`). |
| **`BOOTSEL`** | `BOOTSEL` | Reboots the Pico directly into the USB UF2 ROM bootloader without pressing the physical button. |

---

## 7. Practical Session Examples

### Example 1: Initial Electrical Check (Safe Passive Mode, No Active TX)
```text
PANADIGA> PING
PONG
PANADIGA> STATUS
VDD: 3.25V (OK)
PANADIGA> PINCHECK
...
PANADIGA> ISOLATE
[BUS] P7501 pins placed in High-Impedance Isolation (NSW5V/XRESET Hi-Z, GP26 ADC0).
```

### Example 2: Multi-Baud PIO1 Capture, Timed JIG & LED Control
```text
PANADIGA> LED GDB
[LED] Pico GP25 (Standard Pico / RP2040 / RP2350) | mode=GDB_RSP | lock=MANUAL | pin=1
LED: OK (GDB_RSP, MANUAL_LOCK)

PANADIGA> BAUDSCAN 3500 NORMAL
... edges=2124 min_low_us=8 max_low_us=54 raw_bps=125000 snapped_baud=115200
BAUDSCAN: OK

PANADIGA> HEXCAP 3000 NORMAL
... (348 bytes in RAM zero drop: DDR CHECK: OK .. Loader ver0.61 [06/03/06]) ...
HEXCAP: OK (348 bytes, framing_err=0, overflow=NO)

PANADIGA> PWRTRACE 3000 JIG:350
... GP26_SWUART: min=3.21V max=3.30V | Edges: RXD=2124 NSW5V=0 TXD=0 (final NSW5V=1)
PWRTRACE: OK

PANADIGA> WAITTX 8000 JIG:350 "wait 1" 0D 4000
... [WAITTX] Trigger matched! Injecting payload via Q7514 (SW_UART=Hi-Z)...
WAITTX: OK (triggered=YES, rx=715 bytes)

PANADIGA> LED AUTO
[LED] Pico GP25 (Standard Pico / RP2040 / RP2350) | mode=HEARTBEAT_IDLE | lock=AUTO | pin=1
LED: OK (HEARTBEAT_IDLE, AUTO)
```

> [!NOTE]
> Prefer **`PIO1` commands (`BAUD`, `BAUDSCAN`, `UARTSTREAM`, `HEXCAP`, `PWRTRACE`, `WAITTX`)** for new bench runs. Under `JIG:350`, the hardware timer automatically releases `SW_UART` (`GP26 -> ADC0 Hi-Z`) after `350 ms` so TX injections via `Q7514` reach `UARTM2P_T` without interfering with bootrom sampling. `ISOLATE` remains mandatory after legacy sessions (`UARTSEND` / `UARTPROBE`).

### Example 3: Injection / Erase with `CONFIRM` Lock (When Monitor Is Active)
```text
PANADIGA> INJECT_LOADER CONFIRM
[INJECT_LOADER] Target Address : 0x00004000 (AM34 Internal SRAM)
[INJECT_LOADER] Flash Geometry : 8 MB (NVRAM Window: 0x807F0000..0x807FFFFF)
...
INJECT_LOADER: OK

PANADIGA> ERASE_NVRAM CONFIRM
[ERASE_NVRAM] Launching JEDEC Sector Erase on Spansion Flash (8 MB)
[ERASE_NVRAM] Target Memory Range: 0x807F0000 to 0x807FFFFF (64 KB)
[ERASE_NVRAM] Micro-sectors      : SA134 to SA141 (8 x 8 KB)
...
ERASE_NVRAM: OK
```

*(Note: If the unit is trapped in a `U99` loop without an interactive monitor exposed on `P7501`, use either `SDDIAG` via `DIGADIAG.FRM` or an external TSOP-48 read of Spansion `S29JL064H` [`IC5001`] combined with `panadiga nvram clean`.)*

---

## 8. GDB RSP Bridge Mode (Interactive Debugging with Ghidra / IDA Pro)

The firmware embeds a **GNU Remote Serial Protocol (RSP)** server governed by [`gdb/gdb_rsp.h`](../../firmware/pico-p7501/gdb/gdb_rsp.h), [`gdb_rsp_codec.c`](../../firmware/pico-p7501/gdb/gdb_rsp_codec.c), and [`gdb_rsp_memory.c`](../../firmware/pico-p7501/gdb/gdb_rsp_memory.c):
* Serial command: `GDB` (or autodetection of packets starting with `$`).
* Custom Upper-NOR governance packets in RSP:
  * `$qPanadigaNorUnlock:CONFIRM_UPPER_NOR#<xx>` — explicitly unlocks writes on **Layer 2 (`SA4..SA133`)**.
  * `$qPanadigaNorLock#<xx>` — restores the default `NorWritePolicy::NvramOnly` (`E05`) lock.
* Unified TCP/Serial bridge in the Rust CLI (including async translation over `P7501` UART because no synchronous `DWire32A` clock exists):
  ```bash
  # Live async Ghidra/GDB bridge over DIGADIAG PANAMON1> (R/W) or MAIN _dev_SERIAL:
  panadiga pico gdb-bridge --port COM10 --target-mode panamon1 --listen 127.0.0.1:3333
  panadiga pico gdb-bridge --port COM10 --target-mode panarsp  --listen 127.0.0.1:3333
  # Guided interactive multi-boot bench battery (Standby WAKE, PREJIG no-XRESET, BAUDSCAN, Firmware Error Dictionary):
  panadiga pico bench-wizard --port COM10
  panadiga pico bench-wizard --port COM10 --profile prejig --wake-timeout-ms 45000
  panadiga pico error-lookup 0x83A7
  ```
* Detailed step-by-step walkthrough for Ghidra, IDA Pro, and `gdb-multiarch`:
  * 📖 **[Interactive Hardware Debugging Guide: GDB, Ghidra, and IDA Pro](GDB_GHIDRA_IDA_DEBUGGING_GUIDE.md)**

---

## 9. Two-Layer NOR Write Policy & Real JEDEC SRAM Stubs

The flash subsystem ([`p7501_flash.c`](../../firmware/pico-p7501/flash/p7501_flash.c)) mirrors [`NorWritePolicy`](../../crates/panadiga-core/src/pico/rsp_codec/policy.rs) over the Spansion `S29JL064H` (`0x80000000..0x807FFFFF` / uncached mirror `0x84000000..0x847FFFFF`):

| Layer | Sectors & Bus Window | Pico Firmware Policy | Unlock / Governance Notes |
| :--- | :--- | :--- | :--- |
| **Layer 1 — Critical IPL / Bootloader** | `SA0..SA3` (`0x80000000..0x8003FFFF` / `0x84000000..0x8403FFFF`, `256 KiB`) | **Permanently read-only (`immutable RO`, never writable) → `E05`** | Never unlocked on the Pico probe so `P7501` serial boot recovery remains intact. |
| **Layer 2 — Upper NOR / BGO App** | `SA4..SA133` (`0x80040000..0x807EFFFF` / `0x84040000..0x847EFFFF`, `64 KiB` sectors) | **Write-protected by default (`NvramOnly` → `E05`, forbidden until unlocked)** | Opt-in unlock via `$qPanadigaNorUnlock:CONFIRM_UPPER_NOR#<xx>`, `PANAMON1` (`U CONFIRM_UPPER_NOR`), or `PANARSP`/`DIAGWATCH` (`NORUNLOCK:CONFIRM_UPPER_NOR`). |
| **Layer 3 — NVRAM / EEMUL** | `SA134..SA141` (`0x807F0000..0x807FFFFF`, `8 KiB` sectors) | **Writable by default** | Target sanitization region for `U99` recovery (`ERASE_NVRAM CONFIRM`). |

* **JEDEC SRAM Stubs (`0x51800000`):** `PANARSP JEDEC_ERASE:<addr>` and `PANARSP JEDEC_PROG:<addr>,<hex>` (`1..=16` bytes per call) upload the [`p7501_pana_jedec.h`](../../firmware/pico-p7501/uart/p7501_pana_jedec.h) / [`p7501_pana_jedec.c`](../../firmware/pico-p7501/uart/p7501_pana_jedec.c) stub to `0x51800000` (after policy enforcement in [`p7501_pana_rsp_parse.c`](../../firmware/pico-p7501/uart/p7501_pana_rsp_parse.c)), signaling **`PANAJEDEC_OK`** or timeout **`PANAJEDEC_TO`**. Over `PANAMON1>`, [`p7501_diag_watch.c`](../../firmware/pico-p7501/uart/p7501_diag_watch.c) dispatches `'U'` before `'W'`/`'E'`.
* **Offline Copy-on-Write Fallback:** Image patching runs via `panadiga nvram patch-upper-nor` ([`nvram.rs`](../../crates/panadiga-cli/src/cli/handlers/nvram.rs)) with mandatory SHA-256 `--backup` (blocking `SA0..SA3` unless `--allow-critical-ipl` is specified offline).
* **Deep-Dive on `IC7501` Standby, `TW7501`, `TBUS` & Real VFD States:** See the dedicated architecture dossier [`STANDBY_IC7501_TBUS_TW7501_AND_BOOT_STATES_ARCHITECTURE.md`](./STANDBY_IC7501_TBUS_TW7501_AND_BOOT_STATES_ARCHITECTURE.md) (`PLEASE WAIT -> U99` vs `HELLO -> U99`, `TW7501` + `[POWER]` standby stabilization physics, `FUN_44005cd0` `255s` handshake, and `PREJIG WAKE`).
