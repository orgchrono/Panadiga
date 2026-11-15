> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# MN103S / AM34 Seven Ghidra Traps & Automated Resolution

This reference documents the **seven recurring Matsushita MN10300 / MN103S (`AM34`) disassembly and decompilation traps** encountered across `BOOT.BIN`, `PROG061`, `DRV`, and `MAIN_3410PQ`, together with the automated Rust analyzers (`crates/panadiga-core/src/re/mn10300_traps/mod.rs`), Python helpers (`scripts/re/mn103_traps.py`), bounded MN103S VM (`crates/panadiga-core/src/re/mn10300_emu/mod.rs`), and exportable Ghidra Jython scripts (`tools/ghidra/ghidra_mn103_fix_decompilation.py`).

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
