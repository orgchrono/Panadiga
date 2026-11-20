> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# MN103S / AM34 Seven Ghidra Traps & Automated Resolution (+ Trap #8 SLEIGH)

This reference documents the **seven recurring Matsushita MN10300 / MN103S (`AM34`) disassembly and decompilation traps** plus **Trap #8 (SLEIGH `SUB` operand reversal)** encountered across `BOOT.BIN`, `PROG061`, `DRV`, and `MAIN_3410PQ`, together with the automated Rust analyzers (`crates/panadiga-core/src/re/mn10300_traps/mod.rs`), Python helpers (`scripts/re/mn103_traps.py`), bounded MN103S VM (`crates/panadiga-core/src/re/mn10300_emu/mod.rs`), and exportable Ghidra Jython scripts (`tools/ghidra/ghidra_mn103_fix_decompilation.py`).

> [!NOTE]
> All static analyzers and Jython scripts operate purely in memory or generate portable Python scripts via `panadiga ghidra export-all`. Live Ghidra binary project databases (`C:\ghidra\projects\Panadiga_MN103S`) are never mutated directly by automated tests or generators.

---

## 1. Summary of the Seven MN103S Traps

| # | Trap Name | Typical Signature / Artifact | Resolution in `panadiga-core` & Ghidra |
| :---: | :--- | :--- | :--- |
| **1** | Orphan Functions & Pointer / CLI Dispatch Tables | Unreferenced code islands behind `<8sIII>` tables (`0x400DF5C0`), ATAPI CDB arrays, or jump slots (`0x5040EA30`) | `parse_drv_cli_table`, `parse_atapi_cdb_dispatch`, and `scan_code_pointer_tables` recover handler pointers while filtering unimplemented stub addresses (`0x40014559`). |
| **2** | Indirect `BCLR 0xff` / `BSET` RMW Side Effects | Bitwise hardware flag mutations (`0x5771AC87`, `0x40130EFA`) hidden by Sleigh pointer aliasing | `classify_drv_cli_command` + explicit symbol typing annotate `PASS`/`MMODE`/`SLOG` SRAM flags (`0x0CD8`, `0x0D78`). |
| **3** | Post-Call Unreachable Block Pruning & `SETLB`/Stack-Return Anomalies | Truncated decompiler blocks (`0x574DD208`) after custom calling sequences or non-standard stack frames | `ghidra_mn103_fix_decompilation.py` fixes stack-purge depths and restores flow across truncated post-call regions. |
| **4** | Mid-Instruction Branch Overlap Collisions (`??`) | Branch targets pointing inside multi-byte instructions (`BRA 0x5040ea2f` landing inside a `0x5040ea2e` op), producing `??` in disassembly | **Expected silicon optimization**, not corruption: `build_instruction_spans` + `detect_mid_instruction_overlaps` detect every overlapping jump (`[start_va + 1, end_va)`). |
| **5** | Spansion CS1 NOR Flash CFI Command Cycles | Writes/reads to `0x84200554`, `0x84200AAA`, `0x84200000` misread as arbitrary heap pointers | `is_flash_cfi_command_va` and `scan_cfi_flash_constant_hits` tag Flash JEDEC/CFI ports (`0x000`/`0x554`/`0x555`/`0xAAA`/`0xAAB` inside `0x8000..`/`0x8400..`) as CFI state transitions (`CfiStage` in `Mn103Vm`). |
| **6** | `SETLB`..`Lcc` Hardware Loops & Negative `A3` Frames | `SETLB` (`0xDB`) .. `LNE` (`0xD9`) loops (`in_LAR - 4`) and negative `A3` frame offsets (`0x50404F12`) | `decode_setlb_loops` locates loop bounds; `Mn103Vm` models `LAR`/`LIR` hardware loop registers deterministically. |
| **7** | Cross-Revision Symbol Shifts, CP932 Error Table & LE32 False Hits | Handlers shifted between `DRV_decomp` (`PASS` @ `0x4013420A`) and `EH55_DRV_S211` (`PASS` @ `0x40130EFA`), 52-byte (`0x34`) Shift-JIS table (`0x400E779A`), and mid-instruction LE32 constants (`0x4006F5E2`) | `identify_firmware_revision`, `parse_drv_mecha_error_table`, and `filter_mid_instruction_constant_hits` partition genuine pointers from mid-instruction false positives. |
| **8** | Systematic `SUB` Operand Reversal in Ghidra SLEIGH | 3-operand `SUB` (`fb 9c ..`, e.g., `0x4400c77b` [byte `+1` `0x4400c77c`], `0x4400caeb`, `0x4400d525`, `0x4400e0c6` [byte `+1` `0x4400e0c7`], `0x4400e948`) decompiled as `src - dst` (`Rm - Rn`) instead of `dst - src` (`Rn - Rm`) | Direct byte audit against `BOOT.BIN` + SLEIGH spec (`am33.sinc:184` vs `mn103.sinc:421` `f1 ..`); `Mn103Vm` models true silicon `dst = dst - src` in `ops.rs`. |

---

## 2. Why `??` Appears in Ghidra MN103S Disassembly (Trap 4)

Panasonic's MN103S compiler deliberately shares tail bytes inside multi-byte encodings (Trap 4, e.g. `BRA 0x5040ea2f` jumping one byte inside an opcode beginning at `0x5040ea2e`). When Ghidra's linear sweep disassembles `0x5040ea2e`, byte `+1` is already claimed by that instruction, causing the overlapping reference or trailing byte to display `??`. `detect_mid_instruction_overlaps` isolates those exact overlaps mathematically without altering raw binary bytes.

---

## 3. Bounded Offline MN103S Mini-VM (`Mn103Vm`)

For offline validation of patches and prologues (`crates/panadiga-core/src/re/mn10300_emu/mod.rs`), `Mn103Vm` models `D0..D3`, `A0..A3`, `SP`, `PC`, `MDR`, `LIR`, `LAR`, and `PSW` (with undefined-flag poisoning):

- **Implemented ISA scope**: Core `0x00..=0xEF` moves/ALU/branches (`CALL`, `RET`, `SETLB`, `Lcc`), `MOVM` (`0xCE`/`0xCF` for `D2/D3/A2/A3/OTHER`), and verified subsets of prefixes `0xF0`, `0xF1`, `0xF2`, `0xF8`, and `0xFC`.
- **Fail-closed safety**: Prefixes `0xF3..=0xF7`, `0xF9..=0xFB`, `0xFD..=0xFF`, `RTI`, hardware MMIO/interrupts, and any write to protected IPL Flash raise [`Mn103Fault`] (`IplWriteProtected`, `InvalidOpcode`, `UnmappedRead`/`UnmappedWrite`).

---

## 4. Regenerating & Exporting Ghidra Scripts

```powershell
cargo run -p panadiga-cli -- ghidra export-all --dir tools/ghidra
```

Exports:
- `tools/ghidra/ghidra_am34_memory_map.py`
- `tools/ghidra/ghidra_importar_simbolos_panadiga.py` / `ghidra_import_symbols_panadiga.py`
- `tools/ghidra/ghidra_auto_unbrick_audit.py`
- `tools/ghidra/ghidra_mn103_fix_decompilation.py`
- `tools/ghidra/ghidra_conectar_debugger.py` / `ghidra_connect_debugger.py`

---

## 5. Trap #8: Systematic Semantic Operand Reversal in `SUB` (`fb 9c ..` / `f1 ..`) in SLEIGH `mn10300:LE:32:default`

During the Ghidra 12.1.4 audit of the FAT12/FAT16/FAT32 and SD drivers in `C:\ghidra\binaries\BOOT.BIN` (`Loader ver0.63 [07/10/24]`, base `0x44000000`), a **systematic operand-reversal bug in the `GhidraMn10300` SLEIGH processor specification** was proved for 3-operand `SUB Rm, Rn, Rd` (`fb 9c ..`), distinct from 2-operand `SUB Dm, Dn` (`f1 0x`):

- **Real MN103S / AM33 silicon semantics:** Under Matsushita's `src, dst` operand convention, 2-operand `SUB Dm, Dn` (`f1 0x`) computes `Dn = Dn - Dm` (`dst = dst - src`, accurately modeled in `Mn103Vm` `crates/panadiga-core/src/re/mn10300_emu/ops.rs`), and 3-operand AM33 `SUB Rm, Rn, Rd` (`fb 9c ..`) computes **`Rd = Rn - Rm`** (`dst1 - src`).
- **Ghidra SLEIGH defect (`mn10300:LE:32:default`):** In `GhidraMn10300/data/languages/am33.sinc` (line 184), `:SUB Rm2, Rn0, Rd2 is op8=0xfb; op8=0x9c; Rm2 & Rn0; Rd2 & op4l=0` emits `{ local val:4 = Rm2; update_sub(val, Rn0); Rd2 = val; }`, computing **`Rm2 - Rn0` (`src - dst`)** instead of `Rn0 - Rm2` (`dst - src`). By contrast, 2-operand `:SUB Dm1, Dn0` (`f1 0x`) in `mn103.sinc` (line 421) emits `update_sub(Dn0, Dm1)` = `Dn0 - Dm1` (correct direction), though `am33.sinc:186-188` (`SUB SIMM8/24/32, Rn0`) and `mn103.sinc:427` (`SUBC Dm1, Dn0` `f1 8x`) also exhibit SLEIGH defects by calling `update_add`.

### Verified Byte-Level Sites in `BOOT.BIN`

| Function | Exact Address & Raw Bytes in `BOOT.BIN` | Address / Alignment Note | Ghidra Pseudo-C Output (Inverted) | Real Hardware Semantics |
| :--- | :--- | :--- | :--- | :--- |
| **`FUN_4400c696`** | `0x4400c77b`: `fb 9c dc d0` (`SUB D1, D0, D1`), followed by `0x4400c782`: `f1 01` and `0x4400c787`: `f1 01` (`SUB D0, D1`) | `0x4400c77c` is byte `+1` (`9c`) of `fb 9c dc d0` | `ReservedSectors - TotSec32 - puVar1[-3] - puVar1[-4]` (`*(ushort *)(puVar1[-5] + 0x24) - *(int *)(puVar1[-5] + 0x34)`) | **`TotSec32 - ReservedSectors - puVar1[-3] - puVar1[-4]`** (FAT16 > 32 MB cluster math works properly on hardware) |
| **`FUN_4400ca59`** | `0x4400caeb`: `fb 9c cd c0` (`SUB D0, D1, D0`) | Immediately preceded by `0x4400cae8..0x4400caea`: `f8 03 f8` (`MOV (-0x8, A3), D0`) | `puVar1[3] = puVar1[-2] - puVar1[3]` (`chunk - remaining`) | **`puVar1[3] = puVar1[3] - puVar1[-2]`** (`remaining -= chunk`) |
| **`FUN_4400d32f`** | `0x4400d525`: `fb 9c cd c0` (`SUB D0, D1, D0`), plus `0x4400d7ef`, `0x4400d8b4`, `0x4400d921`, `0x4400d92e`, `0x4400d961`, `0x4400d98f`, `0x4400db58` (`f1 01` at `0x4400d9ad`/`0x4400d9d1`) | Forensic check: `0x4400d4b0` in `BOOT.BIN` is `e4` (`ADD D1, D0`) + `0x4400d4b1: f1 60` (`ADD D0, A0`) in the FAT16 `== 0x10` check; the read-clamp `SUB` is at `0x4400d525` | `puVar1[7] = *(int *)(puVar1[-5] + 0x44) - *(int *)(puVar1[-5] + 0x40)` (`Offset - FileSize`) | **`puVar1[7] = FileSize - Offset`** (clamps `0x40000` read down to `0x20000` on a 128 KiB `DIGADIAG.FRM`) |
| **`FUN_4400dfc8`** | `0x4400e0c6`: `fb 9c cd c0` (`SUB D0, D1, D0`), plus `0x4400e2f9`, `0x4400e366`, `0x4400e373`, `0x4400e3a6`, `0x4400e3d4` | `0x4400e0c7` is byte `+1` (`9c`) of `fb 9c cd c0` | `param_3 = _DAT_5056b6c4 - _DAT_5056b6c0` (`Offset - FileSize`) | **`param_3 = _DAT_5056b6c0 - _DAT_5056b6c4`** (`FileSize - Offset`) |
| **`FUN_4400e737`** | `0x4400e948`: `fb 9c cd c0` (`SUB D0, D1, D0`), plus `0x4400eb29`, `0x4400ed63` | Exact instruction start at `0x4400e948` | `*(int *)(puVar1[-6] + 0x14) = *(int *)(puVar1[-6] + 0x10) - *(int *)(puVar1[-7] + 0x2c)` (`CacheStart - FATSectors`) | **`FATSectors - CacheStart`** (remaining sectors in 3-sector FAT cache window) |

---

## 6. Real-Silicon MN103S Compiler ABI & UART0 Conventions (`PANAMON1` / `nor_dumper`)

Following live `COM10` bring-up of **`PANAMON1`** under `Loader ver0.61` (`641 B`, bypassing `BGO driver init failed[1].` and stalling prior to the prompt), full binary call-site and `uart0_putc` audits proved **two mandatory hardware/ABI rules**:

1. **Calls & Stack Frames (`100%` `CALL target, [], 0x00` / `RET ..., 0x00|0x20`):**
   - `1,386 / 1,386` calls in `PROG061` and `1,065 / 1,065` calls in `BOOT.BIN` emit **`CALL target, [], 0x00`** (`imm8 = 0`), allocate stack explicitly with `ADD -imm8, SP` / `ADD +imm8, SP`, and return via **`RET [], 0x00`** (or `MOVM [D2,D3,A2,A3], (SP); ADD -16, SP ... RET [D2,D3,A2,A3], 0x20`, verified in `PROG061 @ 0x50415CAD`), placing the caller return PC directly below any `ADD -N, SP` local frame (e.g., unlock flag at `(12, SP)` after `ADD -12, SP`).
2. **Bounded `SC0STR` Polling (`PUTC_D3_BYTES` `46 B`):**
   - Both `PROG061 FUN_50404d15` (`0x50404D37`) and `BOOT.BIN uart0_putc` (`0x440047C7`) poll `(SC0STR 0xD400200C) & 0xA0` (`TXBUSY | TXBF`) with a bounded loop counter (`0x1400000` in stock; `MOV 0x00040000, D1` preserving `D1` via `ADD -4, SP` in `PUTC_D3_BYTES`) rather than spinning forever.
   - Both [`crates/panadiga-core/src/re/prog061_uart_monitor/`](../../crates/panadiga-core/src/re/prog061_uart_monitor/mod.rs) and [`crates/panadiga-core/src/re/nor_dumper/stub.rs`](../../crates/panadiga-core/src/re/nor_dumper/stub.rs) enforce this ABI, verified via Ghidra (`SUMMARY_CALLS: total=55 bad=0`) and `Mn103Vm`.
