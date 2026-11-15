> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/AUDITORIA_ADVERSARIAL_FIRMWARE_PICO_E_DISASSEMBLY_PROG061_MAIN.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Adversarial Audit of the Pico Firmware (`P7501`), LED Fix & `PROG061` / `MAIN_3410PQ` Disassembly

> **Scope & Empirical Grounding:** This dossier consolidates: (1) the modular refactoring (`SRP / SoC / SSOT / DRY / FP`) of C11 modules under [`firmware/pico-p7501/`](../../firmware/pico-p7501/), (2) adversarial auditing of firmware and Desktop/Rust parsers against live bench measurements on **`COM10` (`Panasonic DMR-EH55` via `P7501`)**, (3) root cause and permanent resolution of the **onboard LED** controller (`RP2040`/`RP2350`/`CYW43`), and (4) MN10300/AM34 disassembly findings in **`eh55_frm_PROG061_BOOT.bin`** and **`EH55_MAIN_3410PQ.bin`** verified via PyGhidra (`Panadiga_MN103S`).

---

## 1. Modular Decomposition in Strict C11 (`SRP / SoC / SSOT / DRY`)

Monoliths in [`firmware/pico-p7501/`](../../firmware/pico-p7501/) were decomposed into cohesive single-responsibility modules with **production files and headers capped at $\le 257$ lines** (`cli/p7501_cli.c` at `257` lines; `uart/p7501_uart_pio_engine.c` at `246` lines), compiled cleanly under ISO C11 `-Wall -Wextra -Werror` on ARM/RISC-V and `/std:c11 /W4 /WX` on Windows host tests:

| Subsystem | Created / Refactored Files | Single Responsibility (SRP / SoC) |
| :--- | :--- | :--- |
| **`common/`** | [`p7501_hex.h`](../../firmware/pico-p7501/common/p7501_hex.h), [`p7501_hex.c`](../../firmware/pico-p7501/common/p7501_hex.c) | **DRY SSOT** for hex byte decoding (`p7501_decode_hex_bytes`, supporting spaces, `:`, and `,`), case-insensitive comparisons (`p7501_ascii_ieq`), uppercase copy (`p7501_ascii_upper_copy`), and tokenization (`p7501_next_token`). Eliminated duplicate parsers across `cli/`, `uart/p7501_fmu_probe.c`, and `gdb/`. |
| **`hal/`** | [`p7501_led_logic.h`](../../firmware/pico-p7501/hal/p7501_led_logic.h), [`p7501_led_logic.c`](../../firmware/pico-p7501/hal/p7501_led_logic.c), [`p7501_led.h`](../../firmware/pico-p7501/hal/p7501_led.h), [`p7501_led.c`](../../firmware/pico-p7501/hal/p7501_led.c) | Clean split between **Pure FP Logic (`p7501_led_logic.c`, host-testable)** ↔ **HAL IO/IRQ (`p7501_led.c`)** unifying `GP25` (`RP2040`/`RP2350`) and `CYW43_WL_GPIO_LED_PIN` (`Pico W`/`Pico 2 W`). |
| **`diag/`** | [`p7501_diag_electrical.c`](../../firmware/pico-p7501/diag/p7501_diag_electrical.c), [`p7501_diag_busdiag.c`](../../firmware/pico-p7501/diag/p7501_diag_busdiag.c), [`p7501_diag_pwrtrace.c`](../../firmware/pico-p7501/diag/p7501_diag_pwrtrace.c) | 6-pin electrical matrix (`PINCHECK`), bus activity check (`BUSDIAG`), and time-stamped ADC0 + edge logger (`PWRTRACE`). |
| **`flash/`** | [`p7501_flash_cli.h`](../../firmware/pico-p7501/flash/p7501_flash_cli.h), [`p7501_flash_cli.c`](../../firmware/pico-p7501/flash/p7501_flash_cli.c), [`p7501_ram_loader.c`](../../firmware/pico-p7501/flash/p7501_ram_loader.c) | Flash/EEMUL CLI handlers (`AUTODETECT`, `CFI`, `FLASH_SIZE`, `INJECT_LOADER`, `ERASE_NVRAM`) decoupled from `p7501_cli.c`. |
| **`uart/`** | [`p7501_uart_pio_engine.c`](../../firmware/pico-p7501/uart/p7501_uart_pio_engine.c), [`p7501_uart_baud.c`](../../firmware/pico-p7501/uart/p7501_uart_baud.c), [`p7501_uart_stream.c`](../../firmware/pico-p7501/uart/p7501_uart_stream.c), [`p7501_uart_tx.c`](../../firmware/pico-p7501/uart/p7501_uart_tx.c), [`p7501_fmu_batch.c`](../../firmware/pico-p7501/uart/p7501_fmu_batch.c) | `PIO1` engine (`SM0` RX / `SM1` TX in `p7501_uart_duplex.pio`), autobaud (`BAUD`/`BAUDSCAN`), zero-drop capture (`UARTSTREAM`/`HEXCAP`), timed TX trigger (`WAITTX`), transparent bridge (`BRIDGE`), and FMU scan (`FMUBATCH`). |
| **`cli/` & `tests/`** | [`p7501_cli.c`](../../firmware/pico-p7501/cli/p7501_cli.c), [`p7501_cli_help.c`](../../firmware/pico-p7501/cli/p7501_cli_help.c), [`test_uart_codec.c`](../../firmware/pico-p7501/tests/test_uart_codec.c) | Thin command router (`257` lines), help/banner output, and C11 host unit tests for codec, `JIG:<ms>` args, hex parser, and LED FSM. |

---

## 2. LED Bug Root Cause & Deterministic Fix (`p7501_led.c`)

Previously, it seemed as if *"the board itself overrode LED patterns"*. Code inspection and bench reproduction revealed **three combined defects**:

1. **Silent overwrite by telemetry polls (`STATUS`):** Routine UI/host polls or main-loop state updates called `p7501_led_set_mode(...)`, wiping operator selections (`LED GDB`, `LED FAULT`, etc.).
2. **Stall during blocking captures:** Synchronous UART captures (`115200 bps` / `3000–8000 ms`) monopolized the core while `p7501_led_tick()` ran only outside captures.
3. **Deferred pin application & missing manual lock:** Pattern changes waited until the next tick, and neither `LED OFF` nor `LED AUTO` existed.

### Implemented & `COM10`-Verified Solution

* **Pure FSM + `MANUAL_LOCK` (`p7501_led_logic.c`):**
  * `p7501_led_state_set_manual(&s_led_state, mode)` locks `lock = MANUAL`, preventing `p7501_led_set_auto_mode(...)` calls from changing the LED until `LED AUTO` (`p7501_led_unlock_auto`) is executed.
  * Distinct `10 Hz` patterns: `OFF`, `HEARTBEAT_IDLE`, `BUS_ACTIVE`, `GDB_RSP`, and `FAULT`.
* **IRQ-Safe Hardware Timer (`p7501_led.c`):**
  * On standard boards (`RP2040`/`RP2350`, `GP25`), `p7501_led_init()` installs `add_repeating_timer_ms(-100, led_timer_callback, NULL, &s_led_timer)` guarded by `save_and_disable_interrupts()` / `restore_interrupts()`, so LED blinking continues uninterrupted during `UARTSTREAM`/`HEXCAP`/`WAITTX`.
  * On Wi-Fi/BLE boards (`PANADIGA_BLE_ENABLED=1`), skips SPI calls to `CYW43` inside IRQ context and advances cooperatively via `p7501_led_tick()` / `p7501_led_sleep_ms()`.

---

## 3. Adversarial Audit: Firmware ↔ Bench (`COM10`) & Desktop/Rust Parsers

### 3.1 Critical Defects Resolved During Audit

| Severity | Source | Defect Observed in Live Test / Review | Fix & Empirical Proof |
| :---: | :--- | :--- | :--- |
| **CRITICAL** | `uart/p7501_uart_stream.c` (`HEXCAP`) | `HEXCAP 3000 NORMAL` dropped bytes on `COM10` (`"Chip se"`, `"uncoress"`, `overflow=YES`) because `flush_hexcap_row()` called blocking `fflush(stdout)` over USB CDC every 16 bytes, overflowing the 8-word `PIO1` FIFO at `115200 bps` (`86.8 µs/byte`). | `(byte, gap_us)` samples now buffer into static RAM (`HEXCAP_RAM_BUF_CAP = 2048`) across the RX window and print only after `p7501_pio_end_session()`. Subsequent live capture: **`348 bytes, framing_err=0, overflow=NO`** (`+88 µs`/byte steady). |
| **CRITICAL** | `flash/p7501_flash_cli.c` & `gdb/gdb_rsp_cpu.c` | Risk of pseudo-success when target/synchronous monitor was offline (`DCSR=0xFFFFFFFF`). | Enforced **fail-loud**: `AUTODETECT`, `INJECT_LOADER`, `ERASE_NVRAM`, and `GDB` commands (`G`, `c`, `s`, `?`) detect open bus `P7501_BUS_RESP_OPEN_HIGH` and explicitly abort (`[AUTODETECT] ERROR: No synchronous monitor online (DCSR=0xFFFFFFFF).`). |
| **HIGH** | `diag/p7501_diag_pwrtrace.c` | Obsolete macro names (`P7501_PIN_SW_UART`, `P7501_ADC_MAX_COUNT`) broke ARM SDK compilation after refactor. | Migrated to SSOT constants in `p7501_types.h`: `P7501_PIN_SW_UART_ADC`, `P7501_ADC_VREF`, `P7501_ADC_RESOLUTION`, `P7501_ADC_CH_VDD`. |
| **HIGH** | `flash/p7501_ram_loader.c` | Redefined `SPANSION_MICROSECTOR_SIZE_BYTES`, conflicting with `ram_loader_payload.h`. | Removed local duplicate (single SSOT in `ram_loader_payload.h`). |
| **HIGH** | `crates/panadiga-core`, `diagnostics.rs`, `diagnosticParser.ts` | Target Bootrom logs (`BGO driver init failed[1].`, `SD init failed[-90002].`, `ERROR -91006`) tricked desktop parsers into marking successful Pico runs (`WAITTX: OK`) as `failed`. | Classifier now matches Pico-originated fatal errors (`<cmd>: error`, `[<cmd>] error:`, `[safety interlock]`, `[handshake] failed`, `unknown command:`) and gives precedence to final `<CMD>: OK`; timeouts scale to `125,000 ms` and `spec.rs`/`protocol.py`/mocks expose all `PIO1` commands. |
| **MEDIUM** | `Q7514` Circuit (`SW_UART` vs `TXD`) | Holding `GP26` `LOW` (`XFWE`) continuously blocks `UARTM2P_T` (`GP4` via `Q7514`) during `wait 1`. | Added `JIG:<ms>` argument (e.g., **`WAITTX 8000 JIG:350 "wait 1" 0D 4000`**): holds `SW_UART=LOW` for `350 ms` (or `2500 ms` for cold boot past `DDR CHECK`) and releases `GP26` to `ADC0 Hi-Z` before injecting TX. |

### 3.2 Live Hardware Verification (`COM10`, RP2040 Unified UF2)

Universal firmware flashed via `BOOTSEL` on `D:\` (`SHA-256: 4ed6e74c781402c6aa400e360ecc38f351c5aa6af6306a4a52109ceba87a7793`), attached to **DMR-EH55** via `P7501` harness:

* **`STATUS`:** `VDD: 3.25V (OK)`.
* **`LED GDB` -> 3x `STATUS` (`0.75 s` interval) -> `LED STATUS` -> `LED OFF` -> `LED AUTO`:**
  * Preserved `mode=GDB_RSP | lock=MANUAL` immune to `STATUS` polling, toggling hardware `pin=1` → `pin=0` on IRQ timer.
* **`AUTODETECT`:** `[AUTODETECT] ERROR: No synchronous monitor online (DCSR=0xFFFFFFFF). Flash geometry unknown.` (fail-loud confirmed).
* **`PINCHECK`:** `GP26=3.24V (Hi-Z) / 2.70V (PD) / 3.26V (PU)`, `XRESET Release Rise Time (Hi-Z): 10 us`, `Post-Reset 250ms Transitions: GP2=872, GP3=0, GP4=0, GP5=0`, **`PINCHECK: OK`**.
* **`BAUDSCAN 3500 NORMAL`:** `edges=2124 min_low_us=8 max_low_us=54 raw_bps=125000 snapped_baud=115200`, **`BAUDSCAN: OK`**.
* **`PWRTRACE 3000 JIG:350`:** `GP26_SWUART: min=3.21V max=3.30V | Edges: RXD=2124 NSW5V=0 TXD=0 (final NSW5V=1)`, **`PWRTRACE: OK`**.
* **`UARTSTREAM 3500 NORMAL` & `HEXCAP 3000 NORMAL`:** `348 bytes, framing_err=0, overflow=NO`, capturing `DDR CHECK: OK`, `uncompress dst=90202000 len=160000 src=84080040 len=acc01`, `HEADER:5627DD54, DECOMP:5627DD54`, `Loader ver0.61 [06/03/06]`, `BGO driver init failed[1].`, `Main hung-up time changed 255[s].` (`+744207 us` BGO decompression gap, `+88 us/byte` UART burst).
* **`WAITTX 8000 JIG:350 "wait 1" 0D 4000` and `1B0D`:** both fired on `wait 1` after `!!! start fmu to read & exec fvu. !!!`, advancing to `*** timer correspond initialize ***`, `startup reason = [70]`, `SD init start...`, `[SDCD]SetBusWidth:SD Status ERROR -91006`, and `*** CD diag boot mode ***` (**`WAITTX: OK (triggered=YES, rx=715 bytes)`**).
* **`ISOLATE`:** `[BUS] P7501 pins placed in High-Impedance Isolation (NSW5V/XRESET Hi-Z, GP26 ADC0).`

---

## 4. Technical Comparison: C11 + PIO vs. C++ vs. MicroPython

Choosing **strict ISO C11 + PIO (`p7501_uart_duplex.pio`)** is directly justified by electrical and timing constraints measured on the bench:

1. **Critical Timing Margin (`~8 µs` – `88 µs`):** At `115200 bps`, each bit is `8.68 µs` (`min_low_us=8` observed in `BAUDSCAN`) and bytes arrive every `88 µs`. As proven by the `HEXCAP` overflow bug, even a single C `fflush()` over USB CDC overflowed the 8-word `PIO1` FIFO. Garbage-collected runtimes (MicroPython) introduce millisecond allocation stalls that would break sub-millisecond triggers (`WAITTX`) and zero-drop capture.
2. **Electrical Safety Determinism (`NSW5V` / `XRESET` / `Q7514`):** `GP3` (`NSW5V`) must never leave `Hi-Z Input` and `GP5` (`XRESET`) must operate strictly open-drain (`0V` / `Hi-Z`, `10 µs` rise). Bare-metal C11 eliminates dynamic constructors, RTTI, and exception paths across reset sequences.
3. **C++ vs. C11:** C++20 with `-fno-exceptions -fno-rtti` provides no throughput gain over C11 + PIO, whereas pure C11 directly supports clean host FFI testing (`test_uart_codec.c` / `test_gdb_rsp_host.c`) and keeps binaries minimal.

---

## 5. MN10300 Disassembly Discoveries (`PROG061` & `MAIN_3410PQ`)

Analysis conducted via PyGhidra scripts against project `Panadiga_MN103S` (`C:\ghidra\projects\Panadiga_MN103S.gpr`):

### 5.1 `/eh55_frm_PROG061_BOOT.bin` (`0x50400000..0x5043FFFF`)

| Address / Symbol | Size | Identified Routine & Decompiled Behavior |
| :--- | :---: | :--- |
| **`FUN_50404ec0`** (`0x50404EC0`) | `148 B` | UART line editor (`gets`): uses `func_0x50404ca5` (`getc`) and `func_0x50404d15` (`putc`); handles `\r` by emitting `\r\n` + NUL terminator; handles `\b` (`0x08`) visually via `0x08 0x20 0x08`. |
| **`FUN_50404f12`** (`0x50404F12`) & **`FUN_50404f3a`** (`0x50404F3A`) | `32 B` / `88 B` | Interactive FVU monitor pause routines printing **`"---- Hit Any Key ----"`** (`0x50432DD0`) and **`"---- Hit 'Q' to Escape / Any Other Key to Continue ----"`** (`0x50432E20`). `FUN_50404f3a` reads a key, uppercases lowercase ASCII via attribute LUT at `0x50438461` (`& 2` → `- 0x20`), and returns `ch != 'Q'` (`0x51`). |
| **`FUN_5040ea30`** (`0x5040EA30`) | `75 B` | FMU container `list_sum` validator: byte-sums the first `0x2E` (`46`) descriptor bytes and checks against `*(short *)(buf + 0x2E)`, printing `"[fmu] error! list_sum is invalid.\n"` (`0x504349F6`) on mismatch. |
| **`FUN_5040df40`** (`0x5040DF40`) | `1058 B` | **Timer-MCU (`IC7501`) Flash Update State Machine:** issues drive power-down (`0x84 .. 0xFF`, `func_0x5040ba7e(0xf)`), waits `3000 ms`, toggles **`P5OUT`** (`&= 0xF7`) and **`0xDB000500`** (`&= 0xFF3F` / `\|= 0x40`), sends **`0x8B`**, adjusts Timer serial baud divider in **`0xDB000100`** (`(v & 0xFFF0) \| 10`), sends `16` sync `0x00` bytes (`30 ms` spacing), completes handshake **`0xB0`** (expects `-0x50` echo) and **`0xB3`** (expects `-0x4D` echo), triggers erase/reset with **`0xA7`** + **`0xD0`**, polls status **`0x70`** (bits `0x80`, `0x20`, `0x02` across 2 returned bytes) up to 2 attempts, and invokes write (`func_0x5040e41b`) + verify (`func_0x5040e6dc`). |
| **`FUN_504068bd`** (`0x504068B0`..`0x504069BF`) | — | Drive/tray startup loop inside FVU boot entering an infinite retry branch (`BRA 0x504069bf`) if `0x504F00F0 != 0` after checking `0x50480288`/`0x50480284`/`0x5040c779` and `0x504F00F4`. |

### 5.2 `/EH55_MAIN_3410PQ.bin` (`0x57000000..0x57C6692B`) — `+$OK#9A` Serial Stub & Reference Chain

1. **Primary Discovery — Remote Debug Serial Stub (`FUN_574f0775` @ `0x574F0775`, `413 B`):**
   * The main OS embeds a **complete RSP/serial packet interpreter** bound to strings at **`0x57A7243E..0x57A72485`**:
     `_dev_SERIAL`, `-$E01#A6` (`0x57A7244A`), `-$E00#A5` (`0x57A72453`), `+$OK#9A` (`0x57A7245C`), `-$E02#A7` (`0x57A72464`), `,` (`0x57A7246D`), `-$E03#A8` (`0x57A7246F`), `#`, `:`, `-$E04#A9` (`0x57A7247C`).
   * **Exact Flow of Handler `FUN_574f0775` and Decompiled Subroutines (`0x574F06E2..0x574F1323`):**
     * **`FUN_574f06e2` (`0x574F06E2`) / `FUN_574f0735` (`0x574F0735`):** RSP loop entry; sets RX buffer to `0x5177FE42` and TX buffer to `0x51780282` (using `0x15` NAK or `+$` prefix).
     * **`FUN_574f0913` (`0x574F0913`, `290 B`):** RSP frame receiver via direct UART polling (`*puRam515b2e48 & 0x10`, byte read from `*pcRam515b2e4c`); expects leading `'$'` (error transmits `0x15` + `-$E01#A6`). On binary write command `'X'`, parses byte length between `','` (`0x2C`) and `':'` (`0x3A`) so raw `0x23` (`'#'`) inside binary data never terminates the frame prematurely.
     * **`FUN_574f0775` (`0x574F0775`, `413 B`):** Computes mod-256 8-bit sum over inner frame bytes and verifies against the 2 trailing hex digits after `'#'` (mismatch sends `-$E00#A5`); dispatches by opcode `cmd = buf[1]`:
       * `'m'` / `'x'` / `'c'` → **`FUN_574f0a33` (`0x574F0A33`, `560 B`):** hex memory read (`'m'`, using table `0x57A72417` in `0x400`-byte blocks), raw binary read (`'x'`, `0x440`-byte blocks), or compressed read (`'c'`, via `func_0x574f0fef`/`0x574f102b`/`0x574f11bd` with `0x8000 | len` header), appending `'#'` + 2-digit hex checksum; returns `-$E03#A8` (`0x57A7246F`) on parse error.
       * `'M'` / `'X'` → **`FUN_574f0c63` (`0x574F0C63`, `320 B`):** hex memory write (`'M'`, validating chars via `*(byte *)(ch + 0x57B96BED) & 0x44` and LUT `0x57A723F7`, sending `-$E04#A9` on invalid hex) or raw binary write (`'X'`); replies **`+$OK#9A`** (`0x57A7245C`).
       * `'g'` → **`FUN_574f0da3` (`0x574F0DA3`, `223 B`):** serializes `0x70` (`112`) bytes from the saved CPU register context pointed to by `iRam5316ae8c` (224 hex chars + `#checksum`).
       * `'Z'` (`'Z0'` / `'Z1'`) → replies **`+$OK#9A`**; `'Z0'` clears active loop flag (`in_R4 = 0`), releases `uRam577e5dc4` (`func_0x5770d53e`/`0x5770d5bc`), and sets **`0xC0001002 |= 0x80`** before returning.
       * Any other opcode → replies `-$E02#A7`.
   * **Complete Kernel Driver Registration & Tables (`_dev_SERIAL`):**
     * **Dynamic registration (`FUN_574f0538` @ `0x574F0538`, `73 B`):** registers `"_dev_SERIAL"` (`0x57A7243E`) via `func_0x5770897c(param_1, 0x5177f640)` and installs default I/O callbacks **`0x574f12f7`** (1-byte read at `*puRam515b2e50`) and **`0x574f1323`** (`func_0x5770d625(uRam577e5dc8)`).
     * **Kernel Device Table (`0x577E4E80..0x577E4EC0`):** 12-byte records `(u32 dev_id, u32 drv_idx, char *name)`: `[0] (1, 1, "_mn864600ieee1394")`, `[1] (2, 0x101, "_mn864600ake")`, `[2] (3, 2, "_fp_path")`, `[3] (4, 3, "_key_path")`, and **`[4] (5, 4, 0x577E4ED0 "_dev_SERIAL")` at `0x577E4EB0..0x577E4EBC`**.
     * **`_dev_SERIAL` Driver Vtable (`0x577E4D94..0x577E4DB4`):** `dev_id = 5` (`0x577E4D94`), `open = 0x574F0E82`, `close = 0x574F0F19`, `read = 0x574F0F15` (calls `FUN_574f0f3e`, draining the 2048-byte ring buffer `0x5177F642[uRam515b2e54 & 0x7FF]`, decrementing `iRam515b2e58` and signaling `uRam577e5dc4`), `write = 0x574F0F17`, `ioctl_a = 0x574F0F1D`, `ioctl_b = 0x574F0F1B`, `poll = 0x574F0F9D`.
     * **Serial Interrupt Vector & Init:** `0x577E4E50` maps `0x00030015` → **`0x574F0F9F`** (`SC0` ISR for vector `0x15`); `0x57C59F54` points to **`0x574F0590`** (`serial.a` main RX/RSP routine); and an earlier suspected `CALL16` at `0x574F828B` → `0x574F0735` proved to be a mid-instruction false positive at offset `+1` of `0x574F828A` (`[fc cd aa 84 4f 57] MOV 0x574f84aa,D1`). Real 32-bit MN103S calls use the 7-byte opcode `0xDD <rel32_le> <regs_u8> <imm8>` (detailed in Section 5.4).
2. **Libraries & Devices String Table (`0x57C15400..0x57C1555F` & `0x57A52D98`):**
   * Confirmed build provenance in `EH55_MAIN_3410PQ.bin`: `DVR_SYSTEM_LIB:rtscdd.a K0.58 (2006/07/05):RTSD` (`0x57C15403`), `rtsdauth.a Ver0.15-MV3 (2005/12/14)` (`0x57C15433`), **`DVR_SYSTEM_LIB: serial.a Ver0.33 (2005/06/28):Debug Serial`** (`0x57C15470`/`0x57C15480`), `sys_boot.a Ver6.0.076.03 (2006/03/14): sys_boot(M6)` (`0x57C154AB`), `sys_cd.a Ver 7.06 (2005/11/02): CD Media App` (`0x57C154EF`), `sys_cntl.a Ver6.0.143.19 (2006/05/12)` (`0x57C1552A`).
   * Descriptor `_dev_SERIAL` at `0x577E4ED0` (next to `_mcp_graph`, `_key_path`, `_fp_path`, `_mn864600ake`, `_mn864600ieee1394`) plus register names `SC0STR` (`0x57A52D98`), `SC0RXB`, `SC0TXB`, `SC0ICR`, `SC0CTR`. *(Note: `main3060np_unpacked.bin` offsets used in early scans are shifted by `~+0x838C0` relative to `EH55_MAIN_3410PQ.bin`, whose image covers `0x57000000..0x57C6692B`.)*
3. **Attribution Correction (`FUN_574f8354` / `FUN_574f862b` / `FUN_574f79f2`):**
   * Decompilation showed `FUN_574f8354` (`48 B`) and `FUN_574f862b` (`142 B`) read hardware IDs from **EEPROM/NVRAM** (addresses `0xE4`, `0xD0`, `0xD3`, returning codes `'H'`/`'I'`/`'J'` / default `'L'`), feeding `"src/sys_cd_apl_main.c"` (`0x57AE2F53`, `"CD APL Start!! \n"` at `0x57AE2F69`), and are **not** part of the serial protocol.
4. **Debug Menu `ENC_MENU` (`0x57074B4F`..`0x57074CAA`):**
   * Serial command loop exited by `0x1A` (`Ctrl-Z`), printing `" Not supported!! \n"` (`0x577F926F`) when out of bounds and dispatching 16 handlers through pointer table `0x577F9284`.

### 5.3 Comparison of SD Loaders (`BOOT.BIN` vs `eh55_frm_PROG061_BOOT.bin`) and Ready-to-Flash SDSC Files

Cross-referencing `/BOOT.BIN` (`ver0.63`) and `/eh55_frm_PROG061_BOOT.bin` (`ver0.61`) clarified how each stage reads `DIGADIAG.FRM` and `PANA__SD.FRM` from an **SDSC ($\le 2\text{ GB}$, FAT16)** card:

| Stage / Binary | Decompiled Functions | Behavior for `DIGADIAG.FRM` | Behavior for `PANA__SD.FRM` |
| :--- | :--- | :--- | :--- |
| **1st Stage (`BOOT.BIN` / Flash `0x44000000`)** | `FUN_44006cd9` → `FUN_44007ba5` → `FUN_4400a97c` (`SDDIAG`) and `FUN_44006fe7` → `FUN_4400a8c7` / `FUN_4400aa5f` / `FUN_44007a12` (`FVU`) | When `SW_UART = LOW` (`serial boot mode`, `param_2 == 2`), prints `"SDDIAG"`, inits card (`FUN_4400a247`), opens `"DIGADIAG.FRM"`, reads **`0x20000` (`131,072`) bytes** to `0x93FC0000`, calls `uncompress(&DAT_93fc0000)` (`plaintext EXTRHEAD zlib` → `0x50400000`), prints `"*** start diag ***"`, and jumps to `0x50400000`. | Opens `"PANA__SD.FRM"`, reads **`0x2000` (`8,192`) bytes** of plaintext TOC (**128 x 64-byte entries**, `list_sum` at `+0x3E` verified by `FUN_4400a6d0`), and loads each module without outer Feistel encryption (`FUN_44007a12`). |
| **2nd Stage (`PROG061` / RAM `0x50400000`)** | `FUN_5040ebfe`, `FUN_5040ed7c`, `FUN_5040ea7d`, `FUN_5040ea1c`, `FUN_5040722b`, and `FUN_5040eca9` *(orphan: 0 callers)* | `FUN_5040eca9` (`0` static callers) reads `"DIGADIAG.FRM"` (`0x40000` B -> `0x90400000`); **Stage 1 `/BOOT.BIN` rejects** raw `DIGADIAG.FRM` and requires `131,072` B `EXTRHEAD`. | When `iRam50480284 == 5`, `FUN_5040ebfe`/`FUN_5040ed7c` reads `0x1800` B (`48B` TOC) inside `PROG061`, whereas factory `PANA__SD.FRM` (`7,533,792` B) and `/BOOT.BIN` validate **`64B` (`0x2000`)** entries (`list_sum` at `+0x3E`). |

All canonical `.FRM` files follow the `64B` layout validated in `/BOOT.BIN` and factory `PANA__SD.FRM`:
* **`DIGADIAG.FRM` (`sd_card_eh55/` root; historically also mirrored in deduplicated `OPCAO_2_DIAGNOSTICO_FVU_SD\`):** `131,072` bytes (`0x20000`, `SHA-256: 78779d805eab049c65c3eec5e2b0646927c18168c4c97ae8eb24488a1caf339f`) — `EXTRHEAD` container (`tag='FVU '`, `build='PROG061'`, `dst=0x50400000`) for direct boot via `SDDIAG`.
* **`PANA__SD.FRM` (`sd_card_eh55/` root; historically also mirrored in deduplicated `OPCAO_1_ATUALIZACAO_COMPLETA_SD\`):** `7,533,792` bytes (`SHA-256: edbe9ea8ec8a4905c5c891d37484205e2897c12a50f73d272a268ebe91bb7a1a`) — plaintext 64B TOC (`0x2000`) + `DRV S211`, `PROG061 `, and `MAIN3060NP`.
* **`OPCAO_3_FORMATO_VER061_RAW\` *(Obsolete / Non-bootable at Stage 1)*:** historical `--mode sd48` variant (`PANA__SD.FRM` with 48B `0x1800` plaintext TOC = `7,531,744` bytes, `2,048` B shorter than factory file, and raw `0x40000` = `262,144`-byte `DIGADIAG.FRM` rejected by `/BOOT.BIN`).

### 5.4 Root Cause of Non-Spinning Spindle (`CDDIAG` / `DRV S211` / `BOOT.BIN`), Complete Drive CLI Table & `MAIN_3410PQ` RSP Entry

1. **Root Cause of Non-Spinning Spindle Motor at Boot / `CDDIAG` (`DRV` + `BOOT.BIN`):**
   * **Gating by prior laser failure (`LD_OFSCAN` precedes spindle spin-up):** Inside disc startup routine **`FUN_400c3d1d`** (`/DRV_decomp.bin`, corresponding to `0x0be162` in `drv_s211_unpacked.bin`), drive firmware unconditionally runs **`FUN_40080241`** (`LD_OFSCAN`, DSP cmd `0x2c`, `0x07d2b3` in `s211`) **before** calling **`FUN_400c8eda(1, 25000)`** (DSP cmd `0x0f`, spindle spin, `0x0c313e` in `s211`).
   * **Retry path (`FUN_400c39cd`):** The alternate/retry branch triggered via **`FUN_40095555`** (when `(DAT_00000e41 & 0x80) == 0`) also calls **`FUN_40080387()`** (laser check / `LD_OFSCAN` setting `DAT_00001538 |= 0x7f` + `FUN_400db4a7(7)`) before reaching `FUN_400c8eda(1, 25000)`.
   * **Laser diode register setup & gating (`DAT_00001538`):**
     * **`FUN_400804d3(1)`** toggles pin `0x1c` → DSP reg `0x55 = 0xff` (`FUN_4001053c`), calls **`FUN_40080547(1)`** (`0x07d5a3` in `s211`) reading shadow EEPROM byte **`0x40160b97`** via `FUN_400d70b5(0xb97)` (or `0xb96` for `param_1 == 0`) and writing DSP reg `0x59` via `FUN_400ae1ab(0, 0x59)` if `!= 0xff`, then loads 9 DSP registers `[0x1e..0x24, 0x50, 0x5c]` (`DAT_40129d30`, at `0x12535c` in `s211`) via `FUN_400af605(0x500, ...)`.
     * **`FUN_40080241`** (`LD_OFSCAN`) sets bit 6 of `DAT_00001538` (`|= 0x40`). **`FUN_40080578`** requires `(DAT_00001538 & 0x40) != 0` (else logs `_DAT_00000de8 = 0x714`, returns `0x3d`) and non-zero `FUN_4008063b(0)` (else `_DAT_00000de8 = 0x715`) before invoking `FUN_400db4a7(0x23)`. **`FUN_40083396`** loads OPC/calibration LUTs (`DAT_40119778`, `DAT_401196dc`, `DAT_401197c8`) and fires `FUN_400db4a7(0x46)` / `0x2b` / `0x7c`.
     * **Abort with DSP status `0x03A7` (`0x83A7`):** When `LD_OFSCAN` fails returning DSP status `0x03A7`, `FUN_400db60c` (`0x0d5378` in `s211`) sets **`_DAT_00000de8 = (ushort)uVar4 | 0x8000` (`0x83A7`)**, aborting `FUN_400c3d1d` and making **`FUN_40094f10`** (`0x0902e4` in `s211`) transition `DAT_00000dd2` to `0x40` / `0x20` without ever energizing the spindle motor. (`FUN_400d6f39` also checks shadow EEPROM checksum/counters `_DAT_40161000`/`4016102e`/`40161030`, flagging `0x7104`/`0x7105`/`0x7106` on EEPROM failure.)
   * **Implication in `BOOT.BIN` (`CDDIAG`) & `PROG061` Status:**
     * In `/BOOT.BIN`, `cd_diag_boot_mode` calls **`FUN_44009bd7`**, sending ATAPI **`0x1e`** + **`0x1b` (`cdb[4]=2` EJECT)** only after `startup_drive(1)` succeeds; when `startup_drive(1)` fails, it jumps to **`FUN_440079a4()`** → `FUN_440056b1(0x80, 0)`, `FUN_440056b1(0x81, 1)`, `FUN_44005896()`, and halts in `while (true)`. Besides `EXTRHEAD` algo `0` (zlib/deflate at `FUN_44028393`), **`FUN_4400b6c6`** supports algos `2` (`FUN_4402ba08`) and `3` (`FUN_4402c22c`, LZMA) for images at `0x84070000` → `0x90202000` / `0x90362000`.
     * Confirmed that **`/eh55_frm_PROG061_BOOT.bin` equals `prog061_dec.bin`** (`0x50400000..0x5043FFFF`) and implements the updater **`***** Fvu ver%s [%s] *****`**, not an interactive laser/spindle test menu.

2. **Diagnostic Interfaces Present in `DRV` Firmware (`/DRV_decomp.bin` ↔ `drv_s211_unpacked.bin`):**
   * **Verified Cross-Mapping (`DRV_decomp` → `s211`):** `FUN_4001455d`→`0x0145aa`, `FUN_4001591c`→`0x015970`, `FUN_4001b718`→`0x01b676`, `FUN_4001b871`→`0x01b7cf`, `FUN_4002025e` (`FE`)→`0x020300`, `FUN_400277de`→`0x02791a`, `FUN_40027fcd`→`0x028109`, `FUN_4002803a`→`0x028176`, `FUN_400280ca`→`0x028206`, `FUN_4002a1ce`→`0x02a2ee`, `FUN_40080241`→`0x07d2b3`, `FUN_400802a8`→`0x07d31a`, `FUN_40080547`→`0x07d5a3`, `FUN_40094f10`→`0x0902e4`, `FUN_400c3d1d`→`0x0be162`, `FUN_400c8eda`→`0x0c313e`, `FUN_400d65e1`→`0x0d07e3`, `FUN_400db4a7`→`0x0d5213`, `FUN_400db60c`→`0x0d5378`, `FUN_400db7e8`→`0x0d5554`, `pin[0]`→`0x0d7540`, `DAT_40129d30`→`0x12535c`, `SLOGON`→`0x0d92a4`, `LD_OFSCAN`→`0x0dae02`, `0.5mW`→`0x0de266`, `MATSHITA`→`0x008ae6`, and CDB opcode map `0x400dea64` → `0x0d8760` (`s211`, pointer table at `0x0d86a0..`).
   * **Service Unlock & Drive Serial Console (`0x71` = `113` commands):**
     * Serial receiver **`FUN_400273bd`** reads `DAT_0000b002` (when `(DAT_0000b003 & 0x47) == 0`) into buffer `+0xce6` until `\r` (`0x0d`) and fires `FUN_400dd690(0, 0x2000)`.
     * Dispatcher **`drv_serial_cli_dispatch`** scans **`0x71` (`113`)** `20`-byte entries at `0x400df5c0` (`0x0d92a4` in `s211`), requiring **`(DAT_00000cd8 & 1) != 0`** or flag **`0x80`** at `+0x0c` (**`PASS`** at `0x4013420a` / **`FUN_400278a3`**, setting `DAT_00000cd8 |= 3` and configuring `0xffff896c`; or EEPROM unlock byte `0x80` in **`FUN_400c69df`** when `0x80|0x40` and sentinel `0xa7 == 0xa5` match). Main loop **`FUN_400138cc`** runs CLI lines (`FUN_40028489(0x100)` / `FUN_400274ab()`) only when `DAT_00000cd8 & 1` is active.
     * Primary command groups across the `113` entries: session/memory/EEPROM (`SLOGON`, `SLOGOFF`, `PASS`, `VER`, `RESET`, `LOGCUT`, `MMODE`, `SMODE`, `PRMPT`, `RB`, `RW`, `WB`, `WW`, `EEP`, `D`), logs (`MLOG`, `DSCLOG`, `CLOG`, `ACSLOG`, `TMPLOG`, `LFMLOG`), servo/laser/ODC (`PON`/`HON`, `POFF`/`HOFF`, `PCONT`, `PDOWN`, `PTIME`, `DSCC`, `DSC`, `FEP`, `DSCREG`, `LDD`, `CIRC`, `DRC`, `ODCMEM`, `ODCCTL`, `ODCSTS`, `ZTBL`, `ADR`), direct motor/tray control (`START`, `IDLE`, `STANDBY`, `STOP`, `SPEED`, `WRSPEED`, `RDSPEED`, `EJECT`, `LOAD`, `SEEK`, `RPM`, `JMON`, **`FUN_4002a1ce`** building `CDB 0x1B` via `FUN_4001b71d`), media (`DVDROM`..`DVDRWDL`), and laser/read diagnostics (`RFCHK`, `RFCHK2`, `BLER`, `WTADJ`, `MEASR`, `TMP`, `BER`, `BER2`, `RDPCA`, `TBETA`, `SVER`, `SCRIN`, `SCREXE`, `BLCHK`, `RDLNG`, `FORMAT`, `JIT`, `SUBQ`, `DATAID`).
     * Over ATAPI, besides `0x1e` + `0x1b` (`FUN_44009bd7` in `BOOT.BIN`; `FUN_4002a1ce` → `FUN_4001b71d` `CDB 0x1B` in `DRV`) and **`FUN_4002025e` (`FE` → `0x020300`)**, **`FUN_400201b5`** activates bit 7 (`|= 0x80`) or bits `0..6` (`|= 0x7f`) of **`DAT_00000e41` (`MMODE`)** after checking `_DAT_00000246 & 0xff20 == 0`, `(DAT_0000023d & 0x1f) == 0`, `(DAT_00000246 & 0x1f) == 3`, and matching 8 CDB bytes (`0x23e..0x245`) against `&DAT_400df438`.

3. **Definitive Clarification of RSP Stub (`+$OK#9A`) in `EH55_MAIN_3410PQ.bin`:**
   * **Normal vs. Exception Gating (`iRam515b270c == 0`):** `DD-CALL` at `0x574B5ACA` calls **`FUN_574aebdc`**, which **sets `uRam515b270c = 1` strictly while `FUN_574ae930()` runs** (exception/register dump handler) and resets `uRam515b270c = 0` on return. Therefore, inside **`FUN_574f0590`**, **`iRam515b270c == 0`** represents normal non-exception operation.
   * **RSP Entry Trigger `0x15` (`NAK`) & Magic Sequence `0x0E 0x04 0x02`:** In branch `iRam515b270c == 0`, `FUN_574f0590` synchronizes `uRam577e5dc4` (ROM constant `0x0f` at `0x577e5dc4`) and reads 1 byte via `FUN_574f08b6` (`*puRam515b2e48 & 0x10`, byte in `*puRam515b2e4c`):
     * Receiving **`0x15` (`NAK`)** clears bit 7 of `UNK_c0001002` (`&= 0x7f`) and enters the RSP loop (`FUN_574f0913` + `'m'`/`'x'`/`'c'` → `FUN_574f0a33`, `'M'`/`'X'` → `FUN_574f0c63`, `'g'` → `FUN_574f0da3` outputting via `FUN_574f0876` to `*puRam515b2e50`, `'Z0'`/`'Z1'` → `+$OK#9A`, `'Z0'` exits and sets `UNK_c0001002 |= 0x80`).
     * Other bytes enqueue into the `0x800`-byte ring buffer `0x5177f642` and track the 3-byte sequence **`0x0E -> 0x04 -> 0x02`** (`bRam515b2e60: 0 -> 3 -> 0xc3 -> iRam515b2e64 = 1`), suppressing the next register dump in `FUN_574ae930`.
   * **UART/Timer Initialization (`FUN_574efb97` & `FUN_574f0e82`) and Opcode Clarification:** **`FUN_574f0e82`** (`_dev_SERIAL` `open` at `0x577e4d98`) sets `TMPSCNT = 0x80`, `TM2BR = 7`, `TM2MD = 0x41/1/0x81`, `SC0ICR`, `Ramd4002000 = 0xc085` (using ROM constant `uRam577e5dc8 = 0x10`). **`FUN_574efb97`** registers callbacks `0x574f0498`/`0x574f0464` via `FUN_574f052f` (`"_dev_SERIAL"`, `0x5177f640`), calls `func_0x574ef7d2()`, and when `(bRam515b2e39 & 2) != 0` configures `TM2BR = 0x2b`, `Ramd4002000 = 0xc085`, and ring buffers `0x5316a260`/`0x5316aa70` (drained by `FUN_574efad8`). Genuine 32-bit MN103S `DD-CALL` sites to `FUN_574f8242` (`0x574F7B71`) and `FUN_574f8278` (`0x574F7CF4`, `0x574FDFEB`) were verified, ruling out the false positive at `0x574F828B`.

### 5.5 Live `CDDIAG` Capture (`COM10`), Exact Origin of `0x83A7` (`80040904`) & `BOOT.BIN` ↔ `DRV` Chain

Live bench capture (`COM10`, `UARTSTREAM` `JIG:350` + `NOPULL`, `0` framing errors, `overflow=NO`) with a **1.86 GB FAT16 SDSC** card installed observed the exact sequence:

1. **4-Bit SD Failure (`-91006` / `-90002`) → Automatic Fallback to `CDDIAG`:**
   * `[SDCD]This SD Card is't memory card` → `SetBusWidth:SD Status ERROR -91006` → `SD init failed[-90002].` → `*** CD diag boot mode ***`.
   * `BOOT.BIN` (MN103S) decompresses the optical drive MN103S firmware (`F/E Address = 0x90202000 , Drive(compress) Size = 0xACC50`, `uncompress dst=90202000 len=160000 src=84080040 len=acc01`, `HEADER:5627DD54, DECOMP:5627DD54`), releases F/E reset (`F/E reset release finish ...`), and enters `*** RTSC Initialize ***`.
2. **ATAPI Sense Packet (`80040904` / `0x83A7`) Captured Live & Decoded in `DRV_decomp.bin`:**
   * Right after `[RTSD] ATAPI Software Reset` and `[RTSD] Device Reset Occurred(0x62900)`, `startup_drive @ 0x44009cea` (in `BOOT.BIN`, sending `0x1B` + up to `60` iterations of `FUN_44009835()` `TEST UNIT READY` / `FUN_44009e77()` / `FUN_4400f715(0, 1, 0)`) logs `X(80020401)` then aborts with `=== DRIVE ACCESS ERROR === ( 80040904 )`:
     ```text
     *** CDB  [00 00 00 00 00 00 00 00 00 00 00 00]
     *** SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00]
     H_STAT [0] T_STAT [2]
     ```
   * **False Positives Refuted & Root Cause Confirmed:** `0x40005795` (opcode bytes `83 a7` = `MOV D0,D3` / `CMP D1,D3`) and `0x40129307` (FEP speed LUT) were not the source. In `/DRV_decomp.bin`, **`FUN_40015419`** builds the 18-byte Sense buffer at `DAT_889ed27e`: copies `SK/ASC/ASCQ = 04/09/04` from `(&DAT_400de35c)[_DAT_00000398 * 3]` (index `0x38` at `0x400de404`) and writes `_DAT_00000392` into bytes `[10..11]` (**`0x83A7`**, propagated from `_DAT_00000de8` via `FUN_40014eb8` / `FUN_40015356`; bootloader uses `FUN_40004852` with `_DAT_000001aa`/`0x1b0` and `&DAT_400089dc`).
   * Inside **`drv_mecha_error_table @ 0x400e3736`** (431 x `0x34`-byte records formatted by `FUN_40028c38`), entry **`0x83A7` (`a7 83`) at `0x400e8c1e`** holds the Shift-JIS string **`0.5mW発光ができない(HFｵﾝ密調整)`** ("Cannot emit 0.5 mW laser power during HF ON fine adjustment").
   * **Why the spindle does not spin:** Servo/APC calibration (`FUN_400919df` → `FUN_40092022` → `FUN_400db60c` → `FUN_400928f5` / `0x400929a0` / `0x40093974`, with `FUN_40093fad` at 5.0mW/APC calibration) aborts with `_DAT_00000de8 = 0x83A7` when the **0.5 mW** laser emission loop fails to close — halting `startup_drive` before energizing the spindle motor.
   * **Complementary `BOOT.BIN` Flow:** `FUN_4401e636` (called by `bgo_driver_init`) loops `1..4` (`FUN_4401f0bc(..., 4, 0x52, 0x14d)` → `FUN_4401ece7` / `0x44022f12` / `0x4401f044`), whereas `boot_panic_or_halt_7812` → `FUN_44007836` signals the front panel (`FUN_440054aa` → `FUN_4400599a` IPC `0xa3` to `IC7501`), attempts `FUN_44009c85()` (`close tray`) + `startup_drive(0)`, and ejects (`FUN_44009bd7()`) if the result differs from `0x80023a00`.

---

## 6. Technical Roadmap, `PANARSP` / `PIOLOGIC` / `BRIDGE` Audit & Applied Fixes

### 6.1 Physical Bench State (`COM10`) & Post-Flash Unified Firmware Verification

* **Unified UF2 Flashed on Hardware (`COM10` / `D:\` via `BOOTSEL`):** flashed [`panasonic_p7501_uart_unified.uf2`](../../crates/panadiga-core/src/pico/firmware_bins/panasonic_p7501_uart_unified.uf2) (`550,400` bytes, `SHA-256: 7704068D8898E35BD7569C1C3B3A0B1438659946EE3166A0013A92355858AE0F`) onto the board (`MCU Host : RP2040`, `VDD: 3.24V OK`), verifying `PING -> PONG`, `BANNER`, and `PIOLOGIC` / `PANARSP` in `HELP`.
* **Live Verification of `PIOLOGIC` + `PIO0` `SM0`/`PIO1` Coexistence (`COM10`):**
  * **`PIOLOGIC 1500 JIG:350 1000`:** `Initial: RXD=1 NSW5V=1 TXD=1 XRESET=0 | fifo_stall=NO | edges=1024 (TRUNCATED) | span=1033800us`, **`RXD (GP2) Transitions=1023 (minLow=8us minHigh=8us)`**, **`NSW5V (GP3) Transitions=0`** (Hi-Z preserved), `TXD (GP4) Transitions=0`, `XRESET (GP5) Transitions=1`, and **`[PIOLOGIC] XRESET_rise -> first_RXD_fall: 10130 us`**.
  * Immediately afterward (without resetting Pico): `PING -> PONG` and `UARTSTREAM 1500 NORMAL -> UARTSTREAM: OK (348 bytes, framing_err=0, overflow=NO)`, proving on live silicon that `pio_sm_claim()` protected `PIO0` `SM0`.
* **Extended Captures `CDDIAG`, `PANARSP` & `NORMAL` (`72 s`):**
  * **`PANARSP 16000 JIG:350 WAIT:__NEVER__ NAK` + `UARTTXHEX 0D 1200`:** captured `995 bytes` (`fifo_ovf=NO`) + `201 bytes`, observing `SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00]` (`80040904`) at seconds `52`/`51`.
  * **`UARTSTREAM 72000 NORMAL` + `BAUDSCAN 3000 NOPULL`:** captured **`11,488 bytes, framing_err=0, overflow=NO`** across the entire `59..0` countdown with `80040904` (`83 A7 09 04`), observing at `0,`: `** open tray **` → `** close tray **` → `** startup drive **` → `X(80020401)59,58,` and baud **`115200`** (`edges=885 min_low_us=8 max_low_us=53 raw_bps=125000`). All `PANARSP` probes (`G`, `MAGIC`, `NAK`) during boot returned `rsp=NO_REPLY, fifo_ovf=NO` for the reason proven in Section 6.3.

### 6.2 Audit & Applied Fixes Across Codebases (`pico-p7501`, `scripts/bench/` & Desktop UI)

1. **`PIO0` `SM0` Ownership Protection (`hal/p7501_hw.c` + `diag/p7501_diag_piologic.c`):**
   * *Finding:* `p7501_hw_init()` loaded `p7501_uart_rx_program` into `pio0` SM0 without `pio_sm_claim()`. `PIOLOGIC` claimed SM0 via `pio_claim_unused_sm(pio0, false)`, overwriting and disabling the legacy RX program on exit, and omitted `p7501_hw_isolate_bus_hiz()` / `XRESET` release on error.
   * *Fix:* `p7501_hw_init()` now calls `pio_sm_claim(s_pio_inst, s_pio_sm)`, and `p7501_cmd_piologic` releases pending `XRESET` + calls `p7501_hw_isolate_bus_hiz()`, flags `(TIMING_SKEW)` on `fifo_stall=YES`, and logs `span=<us>` before the `1024`-edge cap.
2. **Anti-Overflow Post-Trigger `PANARSP` & Full Extraction (`uart/p7501_pana_rsp*`):**
   * *Finding:* `PANARSP_RX_RAM_CAP` at `1536` bytes overflowed during `~2.5–5.6 KB` boot logs (`WAIT:<trig>`), discarding real RSP replies; `p7501_rsp_inspect_frame()` scanned from offset `0`; and `ptext[96]` truncated `$m` replies at `95` chars.
   * *Fix:* Expanded buffer to `4096` bytes with sliding pre-trigger compaction (`PANARSP_PRETRIG_KEEP = 512`), offset-stepped `p7501_rsp_inspect_frame_from`, untruncated `[PANARSP] PayloadHex: ...`, LED + `fifo_ovf=NO/YES` telemetry, and C11 tests in `tests/test_pana_logic.c`.
3. **Burst Zero-Drop USB-CDC ↔ `SC0` `BRIDGE` (`uart/p7501_uart_tx.c`):**
   * *Finding:* Previous loop blocked `getchar_timeout_us(500u)` and wrote single-byte TX (`p7501_pio_tx_bytes(&tx_b, 1u, false)`) without checking `framing_err` or `p7501_pio_had_rx_overflow()`.
   * *Fix:* Non-blocking `getchar_timeout_us(0u)` polling with up to `16`-byte TX batching (`~~~` preserved), `25 us` idle sleep, and summary line `BRIDGE: OK (rx=.. tx=.. framing_err=.. overflow=NO/YES)`.
4. **Read-Only Full Dump Client, CI Decoders & `Sc0BridgePanel`:**
   * Added `parse_piologic_output`, `parse_panarsp_output`, `MockBenchTransport` simulation, and **`ReadOnlyRspDumper` + `BridgeByteStream` + `MockAm34RspStream`** in `scripts/bench/rsp_dump.py` for read-only chunked `$m<addr>,<len>` over `BRIDGE` with 8-bit checksum validation, NAK retry, final SHA-256, and guaranteed `~~~` + `ISOLATE` teardown.
   * In [`Sc0BridgePanel.tsx`](../../apps/desktop/src/components/pico/Sc0BridgePanel.tsx), [`pt-BR.json`](../../apps/desktop/src/locales/pt-BR.json), and [`en-US.json`](../../apps/desktop/src/locales/en-US.json), added **`F/E Unlock`** shortcut (`PASS 0x4d415453 0x48495441`) and explicitly separated `MAIN` `_dev_SERIAL` (`P7501` `SC0`) from the drive service CLI (`F/E UART 0xB002/0xB004`, distinct from ATAPI bus `0x880000c0`).

### 6.3 Final Disassembly Findings (`NORMAL`, Dynamic `0x83A7`, `0xB002` UART & `TM2BR`)

Conclusive PyGhidra analysis (`Panadiga_MN103S`) explaining observed bench behavior:

1. **Why `NORMAL` Never Reaches `MAIN` (`EH55_MAIN_3410PQ.bin`) or Answers `PANARSP`:**
   * **`FUN_440049eb` (`0x440049EB`, `/BOOT.BIN`):** after `***** normal mode *****`, calls `bgo_driver_init()` (`FUN_4400836f`). Because `bgo_driver_init()` returns `!= 0` (`"BGO driver init failed[1]."`), `FUN_440049eb` **branches away before calling `fmu_sd_dispatcher(1)` (`0x440065BC`)** into **`boot_panic_or_halt_7812()` (`0x44007812`) → `FUN_44007836`**.
   * `FUN_44007836` executes `FUN_44005cd0()` (`"Main hung-up time changed 255[s]."`), `FUN_44009c85()` (`** can not close tray **`), `startup_drive(0)` (`59..0` for `60 s` with `80040904`), opens tray (`FUN_44009bd7()`), awaits `_DAT_50503760 == 1`, retries `** close tray **` + `startup_drive(0)`, and halts forever inside **`FUN_440079a4()`**.
   * Under `JIG` (`mode == 2`), `FUN_440049eb` enters `fmu_sd_dispatcher(2)` → `FUN_4006cd9(0, 2)` (`FVU` / `SDDIAG`), likewise without running `MAIN`. Thus with `BGO` failing and `0x83A7` active, **`_dev_SERIAL` (`FUN_574f0590` / `FUN_574f0775`) never starts on the unit**.
   * Inside `EH55_MAIN_3410PQ.bin`, `FUN_57001fee @ 0x57001FEE` sets `Ramd4002000 |= 0xc085` and **`TM2BR = 0xc5`**, while **`FUN_574efb97 @ 0x574EFB97`** (called by `FUN_57010c09`, `FUN_570cd208`, and `FUN_574f46ba`) sets **`TM2BR = 0x2b`** and ring buffer `0x5316a260` only when `(bRam515b2e39 & 2) != 0` (`FUN_574ef7d7 @ 0x574EF7D7` via `param_1 | 10`); the 4 static callers of `SC0_getline` (`FUN_574ad90f`, `FUN_574ad995`, `FUN_574ada17`, `FUN_574ae5dc`) have `REL = []`, `LE32 = []`.
2. **Dynamic Source of `0x83A7` in `/DRV_decomp.bin` (`Ram88000194`):**
   * The prior `0x4006f5e2` hit (`a7 83`) was only an operand inside `CALL 0x40027988`. The actual origin is **`FUN_400db60c @ 0x400DB60C`**: on servo interrupt bit `0x800` (`FUN_400dd3a2(1)`), it reads **`uVar4 = Ram88000194`** (`DRV_SERVO_MMIO`) and when `(uVar4 >> 16) & 0xf0ff == 1`, computes **`_DAT_00000de8 = (ushort)uVar4 | 0x8000`**. Thus Servo DSP status **`0x000103A7`** produces `0x83A7` (`0.5mW発光ができない(HFｵﾝ密調整)` at `0x400e8c1e`), triggered after calibrations such as `FUN_4008360f` (`_DAT_00000294 = 0x1ec9ec`, `_DAT_00000298 = 0x1ecaec`, `DAT_8800012a = 0xbf`). It is a genuine physical `0.5 mW` laser/APC/HF-on hardware failure.
3. **Physical Bus of the Drive Service CLI (`0xB002..0xB004`) vs. ATAPI (`0x880000C0`):**
   * `drv_serial_cli_dispatch @ 0x400280CA` reads and writes **`DAT_0000b002` (RX) / `DAT_0000b003` (status) / `DAT_0000b004` (TX)** — the **dedicated UART on the optical drive F/E board** — **not** `SC0` on `P7501` (`MAIN`/`BOOT.BIN` talks to the drive via ATAPI at `0x880000C0`).
   * Unlock command **`PASS`** (`FUN_4013420a @ 0x4013420A`) requires **two arguments** parsed by `FUN_40027cd8` / `FUN_40027d30`: **`PASS 0x4d415453 0x48495441`** (`"MATS"` and `"HITA"`) to set `DAT_00000cd8 |= 1` and unlock all `0x71` (`113`) CLI commands (`START`, `LDD`, `DSCREG`, `FEP`, `PON`, `STOP`, `D`, `EEP`).

### 6.4 Does the New Firmware Improve Testing and Full Dump Extraction?

* **Testing Improvement:** Yes, proven on `COM10`: `PIOLOGIC` deterministically validates via `PIO0` that `GP3` (`NSW5V`) maintains `0` transitions with pulls disabled, measures minimum UART bit width (`8 us` at `115200` bps) and `XRESET -> RXD` latency (`10130 us`), and coexists with `PIO1` (`UARTSTREAM` zero drop), while C11/Python/Vitest/Playwright prevent RSP/UI regressions.
* **Full Dump Extraction (8 MB NOR / DRAM / NVRAM) Under Current Board State:**
  * `PIOLOGIC` is unsuitable for large dumps (caps at `1024` edges).
  * `PANARSP` + `ReadOnlyRspDumper` over `BRIDGE` (`$m<addr>,<len>`) remains ready **once `MAIN` (`_dev_SERIAL`) is running**, yet cannot respond while `BOOT.BIN` stalls on `BGO driver init failed[1].`.
  * Practical full extraction vectors on this unit are:
    1. **SDSC Card via `SDDIAG` (`DIGADIAG.FRM`):** resolving the `-91006` / `-90002` `GetSD_Status` physical 4-bit block lets `BOOT.BIN` load a read-only RAM payload at `0x50400000` independently of the laser.
    2. **Optical Drive (`0x83A7`) or F/E UART `0xB002/0xB004`:** inspect pickup/FFC/HF/APC circuitry (or connect `BRIDGE` to the F/E board TX/RX pads with **`F/E Unlock` → `PASS 0x4d415453 0x48495441`**) to enable optical `CD-R`/`DVD-RAM` boot (`DIGADIAG.FRM` / `PANA_DVD.FRM`).
    3. **External Synchronous Monitor:** use `AUTODETECT` / `INJECT_LOADER` whenever a synchronous debug header beyond `P7501` is attached (`DCSR=0xFFFFFFFF` reflects lack of synchronous pins on `P7501`).

---

### 6.5 Final Verified Findings: Button Injection Over Serial, Pico Voltage Glitching, `-91006`, `$x`/`$g` & Full Drive CLI Table (`0x71`)

Direct verification across `/BOOT.BIN`, `/DRV_decomp.bin`, and `/EH55_MAIN_3410PQ.bin` closes the remaining bench hypotheses:

1. **Why Front-Panel Key Injection Does Not Work Over `SC0` (`P7501`):**
   * **Orphan serial RX inside `/BOOT.BIN`:** Blocking reader **`uart0_getc @ 0x4400479F`** (polls `(0xd400200c >> 4) & 1`, reads `SC0RXB = 0xd4002009`) and `console_getline @ 0x4400488A` have **zero references in the binary** (`rel_calls=[]`, `ptrs=[]`, `SC0RXB raw_refs=[]`): `/BOOT.BIN` has no `SC0` command or button parser.
   * **Dedicated front-panel bus (`IC7501`):** Panel keys/commands travel over shared-memory IPC with Timer MCU (`IC7501`) via **`FUN_4400534a`** (mailbox `DAT_dc010000`, `15`-byte packet + `sum - 0x5b` checksum at `DAT_dc010010`, interrupt `FUN_4400f400(0, 0x31, 0)`), not via `P7501` UART.
   * **Pre-`MAIN` Halt:** Because `bgo_driver_init()` fails and halts at `boot_panic_or_halt_7812()` → `FUN_440079a4()`, `MAIN` never executes.

2. **Feasibility of Pico "Voltage Glitching" on the `BGO` Branch:**
   * The check after `***** normal mode *****` executes in **`FUN_440049eb @ 0x44004ABD..0x44004ADD`**: `CALL 0x4400836f` (`bgo_driver_init`, returns `-1` after `"BGO driver init failed[%u]."`) → `CMP 0x0, D0` (`a0 00` at `0x44004ACA`) → **`BEQ 0x44004AD7`** (`c8 0b` at `0x44004ACC`) → `fmu_sd_dispatcher(1)` vs. `CALL 0x44007812` (`boot_panic_or_halt_7812`).
   * **Why `XRESET` won't glitch it and why VDD crowbarring is contraindicated:** `GP5` (`XRESET`) resets the entire MN103S CPU back to `0x40000000`; core supply fault injection (`VDD_CORE`) requires external MOSFET hardware + decoupling removal, carries physical damage risk, and even skipping `0x44004ACC` leaves BGO struct `_DAT_50480d0c` uninitialized downstream in `fmu_sd_dispatcher(1)`.
   * **Deterministic Safe Alternative (`JIG` → `SDDIAG`):** Unlike `NORMAL`, `JIG` (`mode == 2` in `FUN_44006cd9`) jumps directly at **`0x44006DBC: JMP 0x50400000`** (`dc 44 92 3f 0c`), running `DIGADIAG.FRM` (`EXTRHEAD` plaintext loaded to `0x50400000`) with zero electrical hazard.

3. **Exact Cause of `SDCD ERROR -91006` (`-0x1637E`) & SD Reader Limits in `/BOOT.BIN`:**
   * **`GetSD_Status @ 0x44027826`:** Sends `CMD55` (`0x37`) + `ACMD13` (`0x4d`) and reads `0x40` bytes (`abStack_88[0..63]`). Strictly requires **`cStack_86 == '\0'` (SD Status byte `[2]` `== 0x00`)**; otherwise sets `_DAT_50574914 = 4`, prints `"[SDCD]This SD Card is't memory card \n"`, and returns **`-0x1637e` (`-91006`)**.
   * **`FUN_4402791a @ 0x4402791A`:** In SDSC mode (`_DAT_505748f8 < 2`), enforces `CSD_STRUCTURE == 0` and max sector count **`_DAT_5057491c <= 0x3F6004`** (`4,153,348` sectors ≈ `2.126 GB`), returning `-0x16380` outside that range.

4. **Exact Wire Format of `$x`, `$g` (`EH55_MAIN`) & Complete `113` (`0x71`) Drive CLI Table:**
   * **Binary Read `$x<addr>,<len>` (`FUN_574f0a33 @ 0x574F0A33`):** Outputs `+` `$` followed by **unescaped raw bytes** (`0x00..0xFF`, so `0x23` `'#'` can occur inside data!), chunked in **`0x440` (`1,088`) bytes** (2x throughput over `$m` at `115200` bps) plus `#<xx>` mod-256 checksum.
   * **Register Dump `$g` (`FUN_574f0da3 @ 0x574F0DA3`):** Outputs **`0x70` (`112`) bytes** (`224` ASCII hex chars = `28` 32-bit LE words) starting at `*iRam5316ae8c`, handling trailing pointers at `+0x68` and `+0x6c` around `+0x64` (`100`).
   * **UART `BOOT.BIN` (`uart0_putc @ 0x440047C7`):** Polls `MOVHU (0xd400200c), D0; EXTHU D0; AND 0xa0, D0; CMP 0, D0; BNE` then stores `MOVBU D0, (0xd4002008)` (`SC0TXB`).
   * **Complete `drv_cli_cmd_table @ 0x400DF5C0` (`0x71` = `113` commands):** Indices `[0..112]` (`SLOGON` to `JIT`), with F/E UART initialization `FUN_400100b0` (`_DAT_0000b000 = 0xc083`, `DAT_0000a084 = 0x36`, `DAT_0000a080 = 0x80`). Destructive write/erase commands (`WB`, `WW`, `WRITE`, `WTLONG`, `ERASE`, `FORMAT` — last four `perm=0x38`) are blocked by host safety policies.

---

### 6.6 Applied Ecosystem Implementations, IDE/`CS` Handshake, `TEST Mode` → `BYE` → `U99` & Persistent Write Paths

1. **Ecosystem Implementations (`ef947607` + Rebuilt Unified UF2):**
   * **Pico Firmware (`firmware/pico-p7501/`):** added `PANA_RSP_ACT_XMEM` in [`p7501_pana_rsp.h`](../../firmware/pico-p7501/uart/p7501_pana_rsp.h), `parse_mem_spec(..., char opcode, ...)` and `p7501_rsp_inspect_frame_from` in [`p7501_pana_rsp_codec.c`](../../firmware/pico-p7501/uart/p7501_pana_rsp_codec.c) (validating `#<hh>` checksum when binary `$x` payloads contain `0x23`), full hex output & `XMEM:` dispatch in [`p7501_pana_rsp.c`](../../firmware/pico-p7501/uart/p7501_pana_rsp.c), help text in [`p7501_cli_help.c`](../../firmware/pico-p7501/cli/p7501_cli_help.c), and unit tests in [`test_pana_logic.c`](../../firmware/pico-p7501/tests/test_pana_logic.c). Rebuilt dual RP2040+RP2350 UF2 in [`panasonic_p7501_uart_unified.uf2`](../../crates/panadiga-core/src/pico/firmware_bins/panasonic_p7501_uart_unified.uf2).
   * **Python Bench & RE (`scripts/bench/` & `scripts/re/`):** [`rsp_dump.py`](../../scripts/bench/rsp_dump.py) gained `use_binary_x` (`_MAX_X_CHUNK = 0x440`), `decode_mn103s_g_registers()` (`0x70` / `112` bytes = `28` words), and `ReadOnlyRspDumper.read_registers()`; [`nor_dumper.py`](../../scripts/bench/nor_dumper.py) adds `validate_nor_dump_range()` restricted to `0x80000000..0x80800000` and `0x84000000..0x84800000`, `build_readonly_nor_dumper_stub()` (`DEFAULT_NOR_STUB_LOAD_ADDR = 0x51800000`), and `decode_nor_dump_stream()` (`PANADMP1`..`PANADONE`); [`pana_frm.py`](../../scripts/re/pana_frm.py) updated `parse_drv_cli_table(..., count=113)`, `DRV_UNLOCK_COMMAND` (`PASS 0x4d415453 0x48495441`), and `classify_drv_cli_command()`.
   * **SSOT & Desktop UI:** [`spec.rs`](../../crates/panadiga-core/src/ssot/spec.rs) exposes `XMEM:` under `PANARSP`; [`diagnosticParser.ts`](../../apps/desktop/src/services/diagnosticParser.ts) (`extractTelemetryFindings`) and [`decoders.py`](../../scripts/bench/decoders.py) classify `BGO_INIT_HALT`, `LASER_APC_05MW_83A7`, and `SD_STATUS_MINUS_91006`; [`Sc0BridgePanel.tsx`](../../apps/desktop/src/components/pico/Sc0BridgePanel.tsx) renders the 2-bus banner (`SC0` vs `0xB002..0xB004`), `$x#.. Bin` button, and automatic `BGO driver init failed` alert.

2. **True `DMR-EH55` IDE Bus Topology (`Master` Mandatory — Not `Slave`, Not `CS`) vs. `DMR-EH58`:**
   * **Adversarial Erratum (`DMR-EH55` vs `DMR-EH58`):** An earlier extrapolation cited `DAT_505733d4 == '\0'` and `Slave` (`ch=0, dev=1`) remapping. Direct inspection of DMR-EH55 schematics and binaries **`eh55_frm_PROG061_BOOT.bin`** and **`EH55_MAIN_3410PQ.bin`** proved **`DAT_505733d4` DOES NOT EXIST in `DMR-EH55`**. That variable exists only in `C:\ghidra\binaries\BOOT.BIN` (`Loader ver0.63 [07/10/24]`, **`DMR-EH58` / `DVDRAM.7z`**), where `FUN_44010df0 @ 0x44010e22` reads GPIO `P44` (`0xdb000404 & 0x10`) and `FUN_4400fc3f @ 0x4400fc4a` swaps `acStack`.
   * **Schematic Proof on `DMR-EH55` (`vol2_schematics.pdf` pp. 46, 52, 113, 125–126; `europe.pdf` pp. 32, 49):**
     * **Primary Channel (`ch = 0`, MMIO `0x89000400..0x89000558`, `Pri`):** Optical DVD-RAM Traverse Deck attached internally to Digital PCB F/E (`FP50002`, `FP50003`/`P50001`, `FP50004`) — it does **not** share the 40-pin HDD FFC.
     * **Secondary Channel (`ch = 1`, MMIO `0x89000C00..0x89000D58`, `Sec`):** HDD connected via dedicated 40-pin FFC (`Extension FFC (HDD - RAM/Digital P.C.B. Module/ 40 Pin)`) with dedicated `CS0-`, `CS1-`, `DA0..DA2`, `DMARQ`, `DMACK-`, `DIOR-`, `DIOW-`, `IORDY`, `INTRQ`, and `CSEL` (Pin 28).
   * **Disassembly & Decompilation Proof on `DMR-EH55` (`PROG061` + `EH55_MAIN_3410PQ`):**
     * **Per-Channel Init (`Pri` vs `Sec`):** In `PROG061`, `FUN_5040c700 @ 0x5040c700` calls `FUN_504141df(0, 0)` (ODD) and `FUN_504141df(1, 1)` (HDD), then ATAPI reset `FUN_50414268(0, 1, 0)`. In `FUN_504141df` (`PROG061`) and `FUN_574d1487` (`MAIN`), `param_1 == 0` calls `DAVIS_Connect_IDE(0)` and `param_1 == 1` calls `DAVIS_Connect_IDE(1)`. On errors, `FUN_50414ce8` (`PROG061`) and `FUN_574d21ee` (`MAIN`) log `"[RTSD] DAVIS Connect(Pri) ERROR: %04X \n"` (`0x50435298` / `0x57a5cf68`) or `"(Sec) ERROR"` (`0x504352c0` / `0x57a5cf90`).
     * **Hardcoded Table `acStack_70[4] = { 2, 0, 1, 0 }`:** Both `FUN_50414908(byte ch, byte dev)` (`0x50414908..0x50414c32`, `PROG061`) and `FUN_574d1de8(byte ch, byte dev)` (`0x574d1df8`, `MAIN`) fix via MN103S bytes `80 02 f8 92 34 0c f8 9e 35 80 01 f8 92 36 f8 9e 37`:
       * `acStack_70[0] = 2` (`ch=0, dev=0` — **Primary Master** = Type `2` ATAPI DVD-RAM ODD)
       * `acStack_70[1] = 0` (`ch=0, dev=1` — Primary Slave = `0` Unused)
       * **`acStack_70[2] = 1` (`ch=1, dev=0` — Secondary MASTER = Type `1` ATA HDD!)**
       * `acStack_70[3] = 0` (`ch=1, dev=1` — Secondary Slave = `0` Unused)
     * Thus `if (acStack_70[ch*2 + dev] == 1)` (Spin-Up `4000 ms`, `DAVIS_Get_DriveID(1, 1)`, `IDENTIFY DEVICE` `0xEC`, Auto Spin-Up) **executes strictly for `ch = 1, dev = 0` (Secondary MASTER)**.
   * **Why `Slave` Ignores the Disk, `CS` Collides/Stalls & `MASTER` Is Mandatory on `IDE2SAT2`:**
     * `DAVIS_Connect_IDE(1)` (`FUN_5041f61b` / `FUN_5710c6c3`) probes `Master` (`DEV=0`) and `Slave` (`DEV=1`) on `0x89000D38`, testing `DRDY` (`+0x38 & 0x40`) across `100` cycles, then `FUN_5041f782`/`FUN_5710c82a` verifies register consistency (`0x8000040d` on conflict) and configures `Set Feature (PIO)` + `MultiWord DMA MODE2`. `Slave` leaves Master empty; `CS` causes indeterminate `CSEL` / `0x8000040d` DMA conflicts.
     * **Unbreakable Rule for `DMR-EH55`:** jumper the HDD or **`StarTech IDE2SAT2`** explicitly to **`MASTER` (`Device 0` on Secondary IDE `ch = 1`) — NEVER use `CS` and NEVER use `Slave`**.

3. **Origin of `TEST Mode` (`STOP + CH UP + OPEN/CLOSE` → `BYE` → `U99`) & Persistent Write Paths:**
   * **DMR-EH55 Service Manual (pp. 23–24):** `[STOP] + [CH UP] + [OPEN/CLOSE]` for `5 s` enters **`TEST Mode`** (*initializes tuner & main parameters*), displays `PLEASE WAIT`, and ends with **`BYE`**. **`U99`** indicates Main-to-Timer (`IC7501`) communication failure; in Service Mode (`[REC] + [CH UP] + [OPEN/CLOSE]`), `[9][6]`/`[9][7]`/`[9][8]`/`[9][9]` clear Drive Error, History (`61..65`), and Timer Last Error (`F00`).
   * **HDD Writes (`BOOT.BIN` vs `EH55_MAIN`):** In `/BOOT.BIN`, the **sole** ATA DMA `0xCA` writer (`FUN_4400a0b0`) is **`FUN_44006ae4 @ 0x44006b31`** (`HDD INNER SECTOR MAIN RECOVERY START.`), copying backup `MAIN` from `HDD_MAX_LBA - 0x20000` back to LBA `0x10000` (plus logging to `[nfrom]` `0x50480016`). `BOOT.BIN` **writes NO error flags to the HDD**.
   * **`BGO` / `[nfrom]` on NOR Flash & `IC7501` EEPROM:**
     1. **`BOOT.BIN` (`BGO` `0x84200000..0x843F0000`):** `FUN_440236ed` + `FUN_4402426b` initialize `BGO` (`FUN_4401e636`). All bootloader NVRAM writes route via `FUN_440090a6` → `FUN_44008b02` (`_DAT_50480d0c == 1`) for 3 IDs (`0x5048009d`, `0x50480016`, `0x504800ee`). While `FUN_4401e636()` returns `1` (`BGO driver init failed[1].`), boot halts at `0x44004ACC`.
     2. **`EH55_MAIN` (`[nfrom]` `< 0x84080000` + `IC7501` EEPROM):** `FUN_574a00cc @ 0x574A00CC` (`NFD_Write`/`NFD_VerifyWrite`, append-only `"NFD_NMIWRITE_END"`) writes control files to NOR, while `SA134..SA141` (`0x7F0000..0x7FFFFF`) hold `55 AA 10 00..55 AA 17 00` blocks (`panasonic_dmr-eh55_flash_dump.bin` synthetic fixture stores `55 39 39 00` `"U99\0"` at `0x7FE004`). Meanwhile `FPC_TestModeControl()` (`0x5771abfb`), `FPC_InitializeSetting` (`FUN_5706b9f0`), `FPC_RE_INITIALIZE_SHIPPING` (`FUN_575db0a7`), and `sys_fact_Timer_eepromWrite_Key` (`FUN_57562828`) write persistent parameters into `IC7501` internal EEPROM.

---

### 6.7 Live Validation on `COM10` (`JIG:2500`), 4-Bit Cause of `-91006`, Pre-MAIN Pins & CLI-less `PROG061` Flow

1. **Live `COM10` Capture & Why `JIG:350` Failed While `JIG:2500` Entered `serial boot mode`:**
   * With reflashed Pico (`PANADIGA>`, `BANNER`), `UARTSTREAM 6000 JIG:350` from standby (`VDD: 0.03V`) released `SW_UART` at `350 ms` (default of [`p7501_parse_boot_spec`](../../firmware/pico-p7501/uart/p7501_uart_args.c) for bare `JIG`) before `/BOOT.BIN` finished `DDR CHECK: OK` and reached `detect_boot_mode @ 0x44006454`, dropping into `***** normal mode *****` → `BGO driver init failed[1].` (`348 bytes`, `framing_err=1`).
   * With `UARTSTREAM 12000 JIG:2500` (`VDD: 3.24V (OK)`), `SW_UART` stayed LOW past `DDR CHECK: OK` and `Chip select = 1`, deterministically entering **`***** Loader ver0.61 [06/03/06] *****` / `***** serial boot mode *****`** (`startup reason = [5]`, `SD init start...`, `-91006`, `SD init failed[-90002].`, `*** CD diag boot mode ***`; `938 bytes`, `framing_err=0`, `overflow=NO`).

2. **Pre-MAIN Pins (`/BOOT.BIN` `nvram_read_id @ 0x4400f5cb` + table `0x440356f0`) & Absence of Pre-MAIN RX:**
   * Table `0x440356f0` (`20` x `16`-byte records) shows `nvram_read_id` polls GPIO MMIO (`P0MD..P7MD` / `P0IN..P7IN` at `0xdb000000..0xdb000708`), not NVRAM/EEPROM:
     * **`ID 1`** (`0xdb000004` bit `0x02` = `P01`): sets `Chip select`; non-zero triggers `mode = 3` (**`***** support card mode *****`**), running `bgo_driver_init()` and `parse_support_card_hdr(0x40000400, &dst)`.
     * **`ID 2`** (`0xdb000104` bit `0x01` = `P10`): `== 0` enters **`***** FEonly mode *****`** (`feonly_mode @ 0x44004b06`), reading `LAST_ERR_LOG` (`0x504800ee`) if BGO initializes, then jumping to F/E code.
     * **`ID 11`** (`0x0b`: `0xdb000404` bit `0x10` = `P44`): checked in `FUN_44006cd9` (`mode == 2`, `JIG`) after `FUN_44007ba5()` (`SDDIAG`).
     * **`ID 12`** (`0x0c`: `0xdb000504` bit `0x01` = `P50`, `SW_UART`): `== 0` → `mode = 2` (`***** serial boot mode *****` → `fmu_sd_dispatcher(2)` → `FUN_44006cd9(0, 2)` → `FUN_44007ba5()` `SDDIAG`); `!= 0` → `mode = 1` (`***** normal mode *****`).
   * **Zero pre-MAIN RX or CLI in `DIGADIAG.FRM` (`PROG061`):** In `/BOOT.BIN`, `uart0_getc @ 0x4400479f` is called only by `console_getline @ 0x4400488a`, which has **0 callers**. In `/eh55_frm_PROG061_BOOT.bin` (`Fvu ver0.61 [05/11/01]` at `0x50400000`), full `PC-rel` (`CC/CD/DC/DD`) and `LE32` scans show **0 external callers** for `FUN_50404c78` (polling `Ramd400200c & 0x10` / `SC0RXB`), `FUN_50404ca5`, `FUN_50404cef`, `FUN_50404e5d`, `FUN_50404f0a`, and `FUN_50404f32`. Only TX functions (`0x50404dcd`, 507 calls, and `0x50404d15`, 2 calls) are invoked externally.

3. **Exact 1-Bit OK → 4-Bit NG Root Cause of `-91006` on SD Card:**
   * `FUN_4402791a` calls `GetSD_Status @ 0x44027826` (`CMD55 + ACMD13`) **twice**:
     1. The 1st call (`0x44027d77`, **1-bit** bus after `FUN_44026549` with `0x89001850 = 0x80b8`) **passed** (`cStack_86 == 0x00`), as did `ACMD51 SEND_SCR` and `CMD55 + ACMD6(2)` (`SET_BUS_WIDTH` 4-bit).
     2. Next (`0x44027f79`), the bootloader clears bit 15 of `0x89001850` (`0x89001850 &= 0x7fff`) to switch to **4-bit (`DAT[3:0]`)** and repeats `GetSD_Status()` (`0x44027f93`). Solely on that 2nd 64-byte PIO read (`FUN_4402772c` via `0x89001860`), `cStack_86` (`SD_STATUS[2]`) read non-zero, tripping `[SDCD]This SD Card is't memory card` / `-91006`.
   * Since on `VEP73136A` (`SD Card Schematic Diagram`) `DAT0..DAT3` route across `P6802 <-> P53501`, series resistors `RX6801`/`R6803` (`22Ω`), and pull-ups `RX6802`/`R6805` (`12kΩ` → `3R3V`) to socket `P6801`, failing specifically post-4-bit switch pinpointed a `DAT1..DAT3` physical contact/ribbon fault — subsequently confirmed and resolved in [`captura_sd_flat_fix_20261115_025903.log`](../debug_logs/captura_sd_flat_fix_20261115_025903.log) (4-bit bus initialized at `25000 Kbps`, `2 GB` `0x3C4000` LBAs), awaiting FAT16-formatted `DIGADIAG.FRM` (`131,072 B` = `0x20000`, `<= 0xfff5` clusters; FAT32 triggers `-10025`; see [`DIGADIAG_PANA_SD_AND_STANDALONE_TBC.md`](./DIGADIAG_PANA_SD_AND_STANDALONE_TBC.md)).

4. **`BGO` Failure (`FUN_4401e636` → `1`) Blocks Automatic Flag Clearing:**
   * In `/BOOT.BIN`, `FUN_4401e636()` inits `FUN_4401f0bc` and probes `(uVar5=1, uVar4=2..4)` via `FUN_4401f044`; any return outside `{0, 4}` yields `1` (`BGO driver init failed[1].`).
   * In `PROG061` (`FUN_50404fa7`), execution is autonomous: calls `FUN_5040a294()` (`FUN_50404592` at `0x84380000` + `FUN_5040248e(0,0,0)`), falling into `FUN_5040689d()` → `FUN_504068bd(1, ...)` (`while (iRam504f00f4 == 0)`) on failure. NOR flag cleaners (`FUN_50409df2()` / `FUN_50409e65()` for `0x504800a6`, `0x504800af`, `0x5048009d`) require `iRam504802b0 == 1` (`"BGO driver no init.\n"`), leaving `bypass` (`panadiga_bgo_unbrick`), `uart_monitor` (`PANAMON1>`), or external `IC5001` inspection as the clean recovery routes.
