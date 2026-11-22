> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/FIRMWARE_PICO_P7501_GUIA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Panasonic P7501 Async UART & Boot Probe Firmware Engine (RP2040 / RP2350)

Official bench controller firmware built for **Raspberry Pi Pico (RP2040)** and **Raspberry Pi Pico 2 (RP2350)** to perform boot capture, electrical diagnostics, and asynchronous UART communication (**`115200 8N1`**) across factory service connector **`P7501` (`B6B-ZR-3R4`, labeled `TO JIGU`)** on the **Panasonic DMR-EH55** recorder (**Matsushita `AM34` / `MN103S`** SoC + Syscon `IC7501`, board `VEP79138`).

---

## 1. System Overview (Zero-Gadget `P0_FW_RO` Architecture)

The modular C11 firmware in [`firmware/pico-p7501/`](../../firmware/pico-p7501/) implements strict ISO C11 hardware control organized cleanly across `cli/`, `common/`, `diag/`, `flash/`, `gdb/`, `generated/` (`p7501_ssot.h`), `hal/`, `tests/`, and `uart/`:

> [!NOTE]
> **Zero-Gadget Probe Decision (v0.2.0):** The Raspberry Pi Pico (`RP2040` / `RP2350`) probe deliberately attaches **zero extra standalone hardware gadgets** (no `SSD1306` OLED, buzzer, or probe push-buttons). Screen rendering, audio/visual cues, and **DIGA Commander IR** run on the **Android smartphone (USB-C OTG / BLE NUS)** and **Desktop Tauri 2 Workbench**, keeping probe firmware lean inside `P0_FW_RO` (`256 KiB`: `RP2040` `text=160,208 B` / `bss=73,280 B`; `RP2350` `text=157,552 B` / `bss=72,900 B`).

1. **PIO1 Full-Duplex Multi-Baud UART Engine + PIO0 (`p7501_uart_duplex.pio` / `uart/p7501_uart_pio_engine.c`):**
   * Deterministic passive reception via **`PIO1` (`SM0`)** on **`GP2`** (`P7501` Pin 6 `UARTP2M_T` / `MAX TXD`) and dedicated transmission via **`PIO1` (`SM1`)** on **`GP4`** (`P7501` Pin 3 `UARTM2P_T` / `MAX RXD` through transistor `Q7514`), featuring fixed-point fractional dividers for dynamic rates (`BAUD` and autobaud `BAUDSCAN`), live streaming (`UARTSTREAM`), microsecond-delta RAM logging (`HEXCAP`, zero-drop at `115200 8N1`), trigger-based injection (`WAITTX`), and transparent USB-CDC ↔ SC0 bridging (`BRIDGE`).
   * Retains `PIO0` capture engines (`p7501_uart_duplex.pio` + `uart/p7501_uart_capture.c`), `p7501_logic_sampler.pio`, and `p7501_sds_sampler.pio` for `BOOTCAP` / `UARTMON` / `JIGBOOT` / `PIOLOGIC` / `SDSNIFF`.
2. **Safe Open-Drain Reset Control (`XRESET` / `GP5`):** `GP5` (`P7501` Pin 2) toggles strictly between `0V` (active reset) and `Hi-Z` (unpulled input, `~10–20 µs` rise time through `QR7503`), preventing contention against Panasonic's reset supervisor.
3. **Strict Guard on `NSW5V` (`GP3` / `P7501` Pin 5) & `SW_UART` Control (`GP26` / `ADC0`, Pin 1):**
   * **`GP3` (`NSW5V` / `X_SW_5R2V`):** Supplied by `IC1506` (`~5.15V` upstream via `D7510`, resting at `~3.29V` across `R7570`/`R7572`). **ALWAYS remains a `Hi-Z Input` with pulls disabled** and is never driven push-pull.
   * **`GP26` (`SW_UART`):** Samples voltage via `ADC0` (`~3.24–3.25V` Hi-Z, `~2.70V` pull-down, `~3.26V` pull-up) and pulls `LOW` strictly for the exact serial-boot sampling window (`XFWE`, e.g., `JIG:350`), restoring `ADC0 Hi-Z` via `p7501_hw_restore_sw_uart_adc0()` whenever `GP4` TX must pass through `Q7514`.
4. **Non-Blocking LED Driver with Manual Lock (`hal/p7501_led.c` & `hal/p7501_led_logic.c`):** Pure 10 Hz state machine driven by an IRQ-safe `repeating_timer` (`GP25` on Pico/RP2040 and Pico 2/RP2350; tick fallback to `CYW43_WL_GPIO_LED_PIN` on Pico W / Pico 2 W), enforcing `MANUAL_LOCK` so periodic `STATUS` polling never overrides operator selection (`LED <IDLE|ACTIVE|GDB|FAULT|OFF|AUTO>`).
5. **USB CDC + Optional BLE Serial Console (`PANADIGA> `):** Interactive `115200 bps` command shell compatible with standard terminals (PuTTY, TeraTerm), `panadiga` CLI, and the Tauri Desktop/Android app (`panadiga-desktop` / APK). `pico_w` and `pico2_w` builds also enable a Bluetooth LE Nordic UART Service bridge (`PANADIGA_BLE_ENABLED=1`).
6. **GDB RSP Bridge & Fail-Loud Injection/Sanitization (2-Layer NOR Policy):** Embeds a **GNU Remote Serial Protocol (RSP)** server (`GDB`) plus `AUTODETECT` / `INJECT_LOADER CONFIRM` / `ERASE_NVRAM CONFIRM` primitives that refuse to run blindly if `DCSR=0xFFFFFFFF`. On already-bricked `U99` units, the primary no-desoldering path pairs **`SDDIAG` (`DIGADIAG.FRM` + `PANA__SD.FRM` `ver0.61` with `JIG:2500`)** or headless **Door #1 (`0x0B1800`)** $\rightarrow$ **`PANAMON1>`**, requiring a verified **8 MiB** read-only dump (`PANADMP1` + SHA-256) first and keeping **Layer 1 Critical Boot IPL (`SA0..SA10`) permanently locked (`E05`)** while authorizing explicit `CONFIRM_UPPER_NOR` BGO recovery (leaving `SVOD 4` / `XGecu T48` external TSOP-48 programming as physical fallback if IPL or SD slot fails).

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

### Generated Binary Artifacts & Verified Footprint
Builds synchronize the unified container under `crates/panadiga-core/src/pico/firmware_bins/` (embedded into the Rust binary) and slice per silicon family via `extract_uf2_family_slice`:
* **`panasonic_p7501_uart_unified.uf2`** (**`636,416 bytes` / `1,243` blocks**, `SHA-256: 1af8da3e15ca65abc78e1a4fb4d3281e99d34cc5c968186584243de717b14077`) $\leftarrow$ Universal `RP2040` + `RP2350` container:
  * **RP2040 (`pico`):** `text = 160,208 B` (`P0_FW_RO = 256 KiB`), `bss = 73,280 B`.
  * **RP2350 (`pico2`):** `text = 157,552 B` (`P0_FW_RO = 256 KiB`), `bss = 72,900 B` (`s_default_gdb_ctx` down to `6,736 B` / `0x1A50 B`).
* Auxiliary images (`.elf`, `.bin`, `.hex`, `.map`) generated by `pico_add_extra_outputs(panasonic_p7501_uart)` remain in `target/pico-build/`.

### Continuous Integration (CI/CD)
The CI workflow in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) and local gate runner (`python scripts/validate.py` / `npm run validate`) verify **38 C11 host test suites/functions across 4 modules** ([`test_gdb_rsp_host.c`](../../firmware/pico-p7501/tests/test_gdb_rsp_host.c), [`test_pana_diag_drv.c`](../../firmware/pico-p7501/tests/test_pana_diag_drv.c), [`test_pana_logic.c`](../../firmware/pico-p7501/tests/test_pana_logic.c), and [`test_uart_codec.c`](../../firmware/pico-p7501/tests/test_uart_codec.c)) with `-Wall -Wextra -Werror` plus dual RP2040/RP2350 ARM compile verification, alongside Rust (`337` tests / `6` crates), Vitest (`128`), E2E (`27`), `uxlint` (`386`), `jscpd` (`0%`), and Python (`mypy --strict` / `pytest`).

```mermaid
flowchart TD
    IRQ["IRQ GP3 (NSW5V Falling Edge / Missing)"] --> HIZ["Force GP6/GP7 and UART TX Lines into Hi-Z Without Pulls"]
    CMD["Host Command (UART / USB-CDC / BLE / GDB RSP)"] --> CLI["p7501_cli.c (67 SSOT cmds + 9 aliases incl. IR)"]
    CLI --> GATE["p7501_bus_check_access()"]
    GATE -- "0xC000100C or IPL SA0..SA10 (256 KiB)" --> E05["PERMANENT E05 BLOCK"]
    GATE -- "Upper NOR without F14 + CONFIRM_UPPER_NOR" --> E05
    GATE -- "NVRAM SA134..SA141 or Unlocked Upper NOR" --> EXEC["PIO1 / PANARSP / DIAGWATCH / JEDEC"]
    CLI --> VAULT["QSPI Vault A/B (P0..P5 + P6_NOR_MIRROR): VAULT VERIFY <1..3>"]
    CLI --> CRC["CRC32 Nibble-LUT (16 entries): p7501_qspi_crc32_fast()"]
    CLI --> IRCMD["IR VERIFY <12hex> / IRTX / IRRX (Shared XOR + Parity Validator)"]
    EXEC --> DUMP["DUMPNOR / FAST_DUMPNOR: ff_blocks Tracker (<u32> in @P7501_RES)"]
```

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
| **`STATUS` / `VDD` / `WDT`** | `STATUS` \| `VDD` \| `WDT` | Samples `GP26 (ADC0)` on `P7501` Pin 1 (`SW_UART`), prints `VDD: X.XXV (OK / LOW / FLOATING)`, and reports Watchdog (`WDT`) traceability. |
| **`BANNER`** / **`HELP`** | `BANNER` \| `HELP` \| `?` | Prints the identification banner or complete command list. |
| **`BAUD` / `FRAMING`** | `BAUD [bps]` \| `FRAMING [8N1\|8E1\|8O1]` | Queries or configures **`PIO1` (`SM0`/`SM1`)** baud rate (`300..921600 bps`) and hardware parity framing using FP16 fractional dividers. |
| **`BAUDSCAN`** | `BAUDSCAN [ms] [NORMAL\|JIG[:ms]\|NOPULL] [APPLY]` | Glitch-immune dual-polarity (LOW/HIGH) bit-width cluster detector + PIO1 `IRQ 4` framing & ASCII counter on `GP2` (`APPLY` switches `PIO1`). **Note:** Changing baud in PuTTY/RealTerm only affects virtual USB CDC — `PIO1` controls physical `P7501` sampling. |
| **`UARTSTREAM`** | `UARTSTREAM [ms] [NORMAL\|JIG[:ms]\|NOPULL]` | Live capture via `PIO1` with timed `SW_UART` release (`JIG:2500` for `Loader ver0.61` or `JIG:350`) and automatic `ADC0` restoration. |
| **`HEXCAP`** | `HEXCAP [ms] [NORMAL\|JIG[:ms]\|NOPULL]` | RAM capture (`HEXCAP_RAM_BUF_CAP` = `2048` bytes) with `+us` deltas and hex+ASCII dump after window (zero overflow at `115200 bps`). |
| **`UARTTXHEX`** | `UARTTXHEX [LOW:]<hex\|BRK> [rx_ms]` | Injects arbitrary hex bytes or a `120 ms` `BREAK` pulse over `PIO1 TX` (`Q7514` requires `SW_UART` released unless prefixed with `LOW:`). |
| **`WAITTX`** | `WAITTX <ms> <NORMAL\|JIG[:ms]\|NOPULL> "<trig>" <hex\|BRK> [post_ms]` | Waits for trigger substring on `GP2` (e.g., `"wait 1"` after `JIG:350`) and injects payload/BREAK via `PIO1 TX` keeping `NSW5V` `Hi-Z`. |
| **`BRIDGE`** | `BRIDGE [baud] [NORMAL\|JIG[:ms]\|NOPULL]` | Transparent byte bridge USB-CDC ↔ `SC0` (`PIO1`), terminated by escape sequence `~~~` (or timeout). |
| **`PWRTRACE` / `BOOTTRACE`** | `PWRTRACE [ms] [boot]` \| `BOOTTRACE [ms] [boot\|PREJIG] [WAKE]` | `PWRTRACE` samples min/max `ADC0` + `RXD/NSW5V/TXD` edges; `BOOTTRACE` correlates PIO1 UART + `GP3..5` + `ADC0` (supports `PREJIG WAKE` for `IC7501` standby). |
| **`PIOLOGIC`** | `PIOLOGIC [ms] [boot] [khz] [CTRL]` | 4-channel `PIO0` logic sampler (`GP2..5`) for control-line edges and FMU/SC0 handshake timing. |
| **`SDSNIFF`** | `SDSNIFF [ms] [boot]` | Passive `Hi-Z` `PIO0` SD bus sniffer (`GP10..15`) with CRC16 and 4-bit data-lane verification. |
| **`JIGSWEEP`** | `JIGSWEEP [HOLD\|DELAY\|PREBOOT] [ms..]` | Automated `SW_UART` latch window & pre-DDR Bootrom timing sweep for deterministic `SDDIAG` capture. |
| **`BOOTCAP`** | `BOOTCAP [ms]` | Asserts Open-Drain `XRESET` with `SW_UART` released and captures `115200 8N1` boot UART stream (`PIO0`). |
| **`UARTMON`** | `UARTMON [ms]` | Passively listens on `GP2` (`UARTP2M_T`) without forcing reset or `SW_UART`. |
| **`JIGBOOT`** | `JIGBOOT [ms]` | Boots with `SW_UART = LOW` (`XFWE` / JIG mode) across `XRESET` pulse and captures `GP2` at `115200 8N1`. |
| **`JIGPIN`** | `JIGPIN [LOW\|HIZ\|STATUS]` | Drives Open-Drain control on `P7501` Pin 1 (`SW_UART` on `GP26`, historically nicknamed `"TW7501"` in legacy CLI strings — distinct from the physical `TW7501` pad on `IC7501`'s 5V backup rail). |
| **`UARTSEND`** | `UARTSEND [LOW:]<text>` | Transmits ASCII/bytes on `GP4` (`UARTM2P_T`) at `115200 8N1` (default `Hi-Z` enables `Q7514` conduction; `LOW:` prefix forces `SW_UART` low). |
| **`UARTPROBE`** | `UARTPROBE [sec]` | Runs serial probing on `P7501` and restores `GP26` to `ADC0 Hi-Z` afterward. |
| **`PANARSP`** | `PANARSP [ms] [boot] [WAIT:x] <act>` | `_dev_SERIAL` RSP bridge (`REGS`, `MEM`, `WMEM`, `DUMPNOR`, `CALL`, `NORUNLOCK:CONFIRM_UPPER_NOR`, `NORLOCK`, `JEDEC_ERASE:<addr>`, `JEDEC_PROG:<addr>,<hex>`). |
| **`DIAGWATCH`** | `DIAGWATCH [ms] [boot] [act] [WAKE]` | Autonomous `SC0` watchdog and resident `PANAMON1>` monitor (`NORUNLOCK:CONFIRM_UPPER_NOR` / `NORLOCK` dispatching two-step `'U'` → `'W'`/`'E'`). |
| **`DRVPANA`** | `DRVPANA [ms] [boot] [NOPASS] <cmd>` | Matshita `R6-MV4` optical drive factory CLI bridge with automatic `PASS 0x4d415453 0x48495441` unlock (or `NOPASS`). |
| **`IR` / `IRTX` / `IRRX`** | `IR [TX\|RX\|VERIFY <12hex>]` \| `IRTX ...` | 38 kHz Kaseikyo PWM synthesizer ([`p7501_ir_codec.c`](../../firmware/pico-p7501/diag/p7501_ir_codec.c)) on safe GPIOs `GP6..9`/`GP16..22`, RX capture, plus offline frame validation (`IR VERIFY <12hex>`: Panasonic OEM `0x02,0x20`, vendor parity + XOR `dev ^ sub ^ cmd` $\rightarrow$ `parity=OK\|ERR`). |
| **`VAULT`** | `VAULT [STATUS\|VERIFY <1..3>\|DUMP\|ERASE]` | QSPI Vault A/B (`P6_NOR_MIRROR`, `P7501_VAULT_PAYLOAD_SLOTS=3`, slot `0` = manifest). `VAULT VERIFY <1..3>` computes nibble-LUT XIP `CRC32` (`p7501_qspi_crc32_fast`) + `non_ff` byte count emitting `sub=VERIFY` (`ENOENT`/`ECRC` if empty/corrupt). |
| **`DUMPNOR` (`ff_blocks`)** | `PANARSP DUMPNOR ...` \| `FAST_DUMPNOR` | Streams inline blank-block (`0xFF` × 512 B) detection via `p7501_dumpnor_ff_tracker_t` and appends `ff_blocks=<u32>` to `@P7501_RES` and `PANARSP:` lines. |
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
| **`NOR_MAP`** | `NOR_MAP` | Scans all **142 Spansion `S29JL064H` sectors (`SA0..SA141`)** directly on the Pico (`IPL`, `NORFROM`, `EXTRHEADDRV`, 45 BGO sectors, Upper `EEMUL`, NVRAM), streaming `@P7501_PROG phase=MAP` push frames and `[NOR_MAP] ... healthy=YES|NO`. |
| **`FAST_READ`** | `FAST_READ <hex_va> <hex_len_mult_100>` | Uploads on-demand the verified `606 B` MN103S binary stub (`0x9043A000`, `CRC32 0x3D2D3D4A`), reads `256 B` pages with `0xFF` blank-page elision, streams `@P7501_PROG phase=READ` + `@P7501_CHUNK`, and verifies IEEE `CRC32`. |
| **`FAST_WRITE`** | `FAST_WRITE <hex_va> <even_hex_bytes> CONFIRM_UPPER_NOR` | High-speed BGO-window (`SA26..SA70`) programmer enforcing **permanent Layer-1 IPL `E05` lockout (`SA0..SA10`)**, on-Pico pre-flight **`1 -> 0`** monotonicity check, `@P7501_PROG phase=WRITE` push telemetry, and unconditional `U LOCK` on all exit paths. |
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
> Prefer **`PIO1` commands (`BAUD`, `BAUDSCAN`, `UARTSTREAM`, `BOOTTRACE`, `HEXCAP`, `PWRTRACE`, `WAITTX`, `DIAGWATCH`, `PANARSP`)** for new bench runs. For **`Loader ver0.61`**, use `JIG:2500` in `UARTSTREAM 12000 JIG:2500` to hold `SW_UART` through `DIGADIAG.FRM` loading (whereas `JIG:350` suits short bootrom windows); in both cases the timer automatically releases `GP26 -> ADC0 Hi-Z` so `Q7514` conducts TX. `ISOLATE` remains mandatory after legacy sessions (`UARTSEND` / `UARTPROBE`).

### Example 3: Injection / Erase with `CONFIRM` Lock & Bricked-`U99` No-Desoldering Compliance
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

*(Compliance Note: On an already-bricked `U99` DMR-EH55, first attempt **100% no-desoldering** recovery via `SDDIAG` `DIGADIAG.FRM` + `PANA__SD.FRM` `ver0.61` on SDSC FAT16 with `JIG:2500`, or headless **Door #1** `0x0B1800` $\rightarrow$ `PANAMON1>`, taking a mandatory full `8 MiB` SHA-256 read-only backup via `PANADMP1` before authorizing Layer 2 BGO `U CONFIRM_UPPER_NOR` / `NORUNLOCK:CONFIRM_UPPER_NOR` — Layer 1 Critical Boot IPL `SA0..SA10` stays permanently locked (`E05`). External TSOP-48 removal (**SVOD 4** / **XGecu T48** + `panadiga nvram clean`) remains the fallback if `SA0..SA10` IPL or SD hardware has failed.)*

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
| **Layer 1 — Critical IPL / Bootloader** | `0x80000000..0x8003FFFF` / `0x84000000..0x8403FFFF` (`256 KiB`: policy `SA0..SA3` in `64 KiB` units; physical `SA0..SA10` = `8×8K+3×64K`) | **Permanently read-only (`immutable RO`, never writable) → `E05`** | Never unlocked on the Pico probe so `P7501` serial boot recovery remains intact. |
| **Layer 2 — Upper NOR / BGO App** | `0x80040000..0x807EFFFF` / `0x84040000..0x847EFFFF` (policy `SA4..SA133`; physical `SA11..SA133`) | **Write-protected by default (`NvramOnly` → `E05`, forbidden until unlocked)** | Opt-in unlock via `$qPanadigaNorUnlock:CONFIRM_UPPER_NOR#<xx>`, `PANAMON1` (`U CONFIRM_UPPER_NOR`), or `PANARSP`/`DIAGWATCH` (`NORUNLOCK:CONFIRM_UPPER_NOR`). Houses EH55 `ver0.61` BGO (`0x84130000..0x843FFFFF`, `SA26..SA70` vs `ver0.63`/`PROG061` `0x84200000..0x843FFFFF`) & Upper EEMUL (`SA71..SA133`). |
| **Layer 3 — Top NVRAM / EEMUL** | `SA134..SA141` (`0x807F0000..0x807FFFFF`, `8 KiB` sectors) | **Writable by default** | On real EH55, `SA134` (`0x847F0000`) holds active `"1284"` header `0ff00000002c` (`6 B`) while `SA135..SA141` are `0xFF`. |

* **JEDEC SRAM Stubs (`0x51800000`), Uncached Mirror (`0x90400000`) & Real `S29JL064H` (~700 ms) Erase Timing:**
  * `PANARSP JEDEC_ERASE:<addr>` and `PANARSP JEDEC_PROG:<addr>,<hex>` (`1..=16` bytes per call) upload the [`p7501_pana_jedec.h`](../../firmware/pico-p7501/uart/p7501_pana_jedec.h) / [`p7501_pana_jedec_stubs.c`](../../firmware/pico-p7501/uart/p7501_pana_jedec_stubs.c) stub to `0x51800000`, signaling **`PANAJEDEC_OK`** or timeout **`PANAJEDEC_TO`**. Over `PANAMON1>`, [`p7501_diag_watch.c`](../../firmware/pico-p7501/uart/p7501_diag_watch.c) dispatches `'U'` before `'W'`/`'E'`.
  * **Uncached SDRAM Coherence (`0x90400000`):** when patching executable RAM in `PROG061` (`0x50400000`), target its uncached **`0x90400000`** mirror (e.g., `PANADMP1` `MOV` bounds at `0x9043950E` $\rightarrow$ `J 904394FA`, or test stubs at `0x90439B40`) so writes bypass write-back D-cache and are seen immediately by I-cache.
  * **~700 ms 64 KiB Erase vs DQ7 Polling:** physical `S29JL064H` 64 KiB sector erase requires ~`700 ms`, outlasting the bounded `0x00100000`-iteration DQ7 loop in `jedec_emit.rs`; callers must poll readback for `0xFF` × 32 after `OK:ERASE` before issuing `W` program writes.
* **Offline Copy-on-Write Fallback:** Image patching runs via `panadiga nvram patch-upper-nor` ([`nvram.rs`](../../crates/panadiga-cli/src/cli/handlers/nvram.rs)) with mandatory SHA-256 `--backup` (blocking the opening `256 KiB` unless `--allow-critical-ipl` is specified offline).
* **Deep-Dive on `IC7501` Standby, `TW7501`, `TBUS` & Real VFD States:** See the dedicated architecture dossier [`STANDBY_IC7501_TBUS_TW7501_AND_BOOT_STATES_ARCHITECTURE.md`](./STANDBY_IC7501_TBUS_TW7501_AND_BOOT_STATES_ARCHITECTURE.md) (`PLEASE WAIT -> U99` vs `HELLO -> U99`, `TW7501` + `[POWER]` standby stabilization physics, `FUN_44005cd0` `255s` handshake, and `PREJIG WAKE`).
