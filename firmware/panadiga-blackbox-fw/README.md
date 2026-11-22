# Panadiga Open-Source Hardware-Interoperability Prototype & Feasibility Spike (`AM34 / MN103S`)

> **Status & Provenance Notice:** `[EXPERIMENTAL PROTOTYPE / FEASIBILITY SPIKE — NOT LEGALLY CLEAN-ROOM]` — Experimental SDRAM (`0x50400000`) Stage-1.5 monitor (`PANAMON2`) & standalone Video TBC bring-up harness for Matsushita AM34 (`MN2DS0009` / compatible DIGA SoCs). Because this prototype resides in the same repository alongside disassembled Panasonic firmware artifacts and register maps, it is an **open-source hardware-interoperability feasibility spike**, **NOT** an isolated "clean-room" implementation under copyright separation rules (no Chinese-wall isolation is claimed or enforced).

## Design Goals & Silicon Constraints

1. **SDRAM Stage-1.5 Slot Only (`0x50400000`, `256 KiB`)**:
   - Loaded into volatile SDRAM by existing Panasonic SD/CD loaders (`DIGADIAG.FRM`) without touching CS1 IPL Flash (`0x84000000..0x8403FFFF`).
2. **Real-Silicon Hardware Requirements (Beyond `Mn103Vm` ISA Simulation)**:
   - **No Hardware MPU / Software-Only MMIO Guard (`0xC000100C..0xC000100F`)**: Compile-time static assertions and `am34_safe_read32`/`am34_safe_write32` wrappers reject accesses to `0xC000100C` (watchdog/bus-lock register) and unmapped DAVIS ranges (`0x89000080..0x8A000000`), as well as writes to IPL Flash (`0x80000000..0x8003FFFF`, `0x84000000..0x8403FFFF`). However, Stage-1.5 AM34 runs with **no active hardware MPU**: raw pointer dereferences outside these wrappers are NOT hardware-trapped by the CPU.
   - **Mandatory Atomic IRQ Masking (`CLI`) & SDRAM Vector Table Before NOR Operations**: Any code later issuing Spansion `S29JL064H` CFI erase/program commands MUST disable interrupts (`CLI`) and point exception/interrupt vectors to SDRAM before leaving NOR `Read Array` mode; an unmasked IRQ fetching an ISR vector from NOR mid-erase/program hangs or permanently bricks the device.
   - **`IC7501` TBUS Power-Supervisor Watchdog**: On real `VEP79138` hardware, `IC7501` shuts down or resets the main rails unless TBUS keepalive frames are serviced within the supervisor window (`panadiga unbrick tbc-plan` / Pico bench keepalive).
   - **`Mn103Vm` Scope**: The workspace `Mn103Vm` emulator checks instruction decoding, register state, and synthetic MMIO bounds only; it **cannot** validate real SDRAM/flash bus wait-states, I/D-cache coherency, Spansion CFI toggle-bit timing, DMA, or IRQ races.
3. **Standalone Video TBC & `PANAMON2` Telemetry**:
   - Uses UART0 (`115200 8N1` via `P7501 TO JIGU`) with deterministic single-line telemetry (`PANAMON2>`).
   - Provides experimental `AIOF` / video encoder probe stubs (`NTSC`, `PAL-M`, `PAL`, `SECAM`).

## Containerized Cross-Toolchain Requirement

Building runnable AM34 machine code requires a **reproducible containerized `mn10300-elf-gcc` / `am33_2.0-linux-gnu-gcc` cross-compiler environment** (historical host toolchains are obsolete/unavailable natively on modern OSes):

```bash
# Inside a pinned mn10300-elf cross-compiler container (e.g. Debian/crosstool-ng AM33/MN103S):
mn10300-elf-gcc -std=c11 -Os -Wall -Wextra -Werror -ffreestanding -nostdlib \
  -T am34_sdram.ld crt0_am34.S panamon2_tbc.c -o panamon2_tbc.elf
mn10300-elf-objcopy -O binary panamon2_tbc.elf panamon2_tbc.bin
```

> **CI / Host Validation:** On host CI machines without an MN103S cross-compiler container, `panadiga` performs static source/guard checks only (`scripts/validate.py` Gate 8 compiles Pico/host C11 modules). Execute AM34 SDRAM experiments strictly behind `ADuM` USB galvanic isolation with external Pico `P7501` capture on UART (`115200 8N1`).
