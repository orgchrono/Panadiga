# Panadiga Blackbox Clean-Room Firmware (`AM34 / MN103S`)

> **Status:** `[STATIC RESEARCH / CLEAN-ROOM PROTOTYPE]` — Experimental SDRAM (`0x50400000`) Stage-1.5 replacement monitor (`PANAMON2`) & standalone Video TBC initialization harness for Matsushita AM34 (`MN2DS0009` / compatible DIGA SoCs).

## Design Goals

1. **Zero Proprietary Panasonic Code**: Clean-room C11 + GNU AS (`crt0_am34.S`) + GNU LD (`am34_sdram.ld`) targeting the **SDRAM Stage-1.5 slot (`0x50400000`, `256 KiB`)** loaded by Panasonic SD/CD loaders (`DIGADIAG.FRM`) without modifying CS1 IPL Flash (`0x84000000..0x8403FFFF`).
2. **Non-Negotiable Silicon Safety Guards**:
   - **Lethal MMIO Block (`0xC000100C..0xC000100F`)**: Compile-time static assertions and runtime address-filter checks unconditionally reject any read/write hitting the SoC watchdog/bus-lock register (`0xC000100C`).
   - **DAVIS Global Space Guard (`>= 0x89000080`)**: Blocks speculative accesses outside verified CH0/CH1 ATAPI register banks (`0x89000000..0x8900007F`) and CS0/CS1 alias areas.
   - **IPL Read-Only Barrier (`0x80000000..0x8003FFFF`, `0x84000000..0x8403FFFF`)**: Never issues NOR program/erase commands against bootloader sectors (`SA0..SA10`).
3. **Standalone Video TBC & `PANAMON2` Telemetry**:
   - Uses UART0 (`115200 8N1` via `P7501 TO JIGU`) with deterministic single-line telemetry (`PANAMON2>`).
   - Provides clean-room `AIOF` / video encoder timing probe stubs for `NTSC`, `PAL-M` (Brazilian `M6930LB` M-NTSC crystal matrix), `PAL`, and `SECAM`.

## Reference Toolchain Build (Cross-Compiler Required)

```bash
# Requires mn10300-elf-gcc or am33_2.0-linux-gnu-gcc
mn10300-elf-gcc -std=c11 -Os -Wall -Wextra -Werror -ffreestanding -nostdlib \
  -T am34_sdram.ld crt0_am34.S panamon2_tbc.c -o panamon2_tbc.elf
mn10300-elf-objcopy -O binary panamon2_tbc.elf panamon2_tbc.bin
```

> **Note:** On host CI machines without an MN103S cross-compiler, `panadiga` validates source structure and guard logic statically; execute hardware tests only through gated `DIGADIAG.FRM` SDRAM loads with external Pico monitoring on `COM10`.
