> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Pico P7501 C11 Upgrades, Ghidra `??` Root Causes & Python-to-Rust Parity Matrix

> **Companion References:** [`AUDITORIA_COMPLETA.md`](../../AUDITORIA_COMPLETA.md) · [`GHIDRA_UNDEF_AUDIT_REPORT.md`](../pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md) · [`MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md`](./MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md)

---

## 1. Why `??` (`Undefined`) Appears in Ghidra MN103S Disassembly

An exhaustive read-only audit across all five binaries in `C:\ghidra\projects\Panadiga_MN103S` confirmed that `??` gaps do **not** reflect dump corruption or serial framing loss. Every gap falls into one of three deterministic structural classes:

| Binary | Init Size | Instructions | `??` (`Undef`) | Gap Count | Dominant Drivers |
| :--- | ---: | ---: | ---: | ---: | :--- |
| `/BOOT.BIN` | `262,144 B` | `178,876 B` | `67,100 B` (`25.60%`) | `170` | Literal pools/unimpl (`41,508 B` / `27` gaps), clean code islands (`22,910 B` / `93` gaps), strings (`2,559 B` / `29` gaps), ptr table (`24 B` @ `0x44035EB0`) |
| `/eh55_frm_PROG061_BOOT.bin` | `262,144 B` | `666 B` | `261,478 B` (`99.75%`) | `9` | Single base entry point; `6` gaps >=64 B containing `0xCF`/`0xF8 0xFE` prologs (top gap `0x50406DAA`: `157,394 B`) |
| `/EH55_DRV_S211.bin` | `1,441,792 B` | `7,613 B` | `1,434,179 B` (`99.47%`) | `26` | Clean code islands (`780,547 B` / `19` gaps) + literal pools/unimpl (`653,632 B` / `7` gaps) |
| `/DRV_decomp.bin` | `1,441,792 B` | `727,726 B` | `666,076 B` (`46.20%`) | `1,957` | Pools/unimpl (`417,487 B` / `341`), islands (`153,222 B` / `292`), strings (`84,014 B` / `860`), ptrs (`2,543 B` / `34`) |
| `/EH55_MAIN_3410PQ.bin` | `13,003,052 B` | `825,478 B` | `12,177,574 B` (`93.65%`) | `1,986` | Pools/unimpl (`8,168,656 B` / `136`), clean code islands (`4,004,135 B` / `1,706`; `1,399` >=64 B w/ prologs), `3,935` `0xF9` prefix fails |

Specific MN103S / AM33 Sleigh behaviors verified verbatim in the disassembly gaps:
- **AM33 unimplemented Sleigh opcodes:** `0xF9 0xEA <reg>` (`MOVHU (Rm0+), Rn2`, e.g., `0x44011553`, `0x50416355`, `0x5700A836`), `0xF9 0xFA <reg>`, `0xFF 0xF0 0xFC` (`0x504009E4`, `0x57000D2C`), and `0xF0 0xC0` (`0x504313E6`).
- **Trap 4 mid-instruction branch tail-sharing:** linear disassembly marks shared target bytes `+1` as `??`.
- **Unexplored clean islands:** dispatch-table handlers behind `<8sIII>` and CDB arrays lack direct calls until resolved by `panadiga re audit` / `panadiga ghidra export-all`.

---

## 2. Pico `P7501` C11 Firmware Upgrades & Rebuilt UF2 Artifacts

Modular C11 modules under [`firmware/picofw/`](../../firmware/picofw/) (strict `-Wall -Wextra -Werror`, all production `.c/.h` files <= `339` lines) were compiled for RP2040 + RP2350 via `python scripts/build.py --pico --no-cache` and embedded in `crates/panadiga-core/src/pico/firmware_bins/`:
- **Unified UF2 (`panasonic_p7501_uart_unified.uf2`):** `522,752 B` (`SHA-256: 2d568f7f4c1d3b12c83721eef4a5666a59666ead5fe8ee7359915e45962aa405`)
- **RP2040 UF2 (`panasonic_p7501_uart_rp2040.uf2`):** `268,288 B` (`SHA-256: 0f216f39a5eb863eee4a93b7a957bea872f467cc0c050c7d897a33a73d50e0d2`)
- **RP2350 UF2 (`panasonic_p7501_uart_rp2350.uf2`):** `254,464 B` (`SHA-256: c0cd3f9bd90401ea2699cfeeb2e8c067608c0db489ec4d724cf550ac74dab99d`)

Key bench command enhancements verified by host tests (`firmware/picofw/tests/test_pana_diag_drv.c`):
1. **`DIAGWATCH [ms] [NORMAL|JIG[:ms]|NOPULL] <action> [RELEASE|HOLD]`:** supports parameterized `PANAMON1>` TX payloads `READ[:a[,n]]`, `WRITE:<a,v>`, `JUMP:<a>`, and `ERASE:<a_nvram>` alongside `AUTO`/`BYPASS`/`FORMAT`/`DUMP`/`QUIT`/`NONE`. Shared flash boundary helpers enforce IPL Read-Only protection (`0x80000000..=0x807EFFFF`, `0x84000000..=0x847EFFFF`) and restrict `ERASE` strictly to `64 KB` NVRAM (`0x807F0000..=0x807FFFFF`, `0x847F0000..=0x847FFFFF`).
2. **`PANARSP REGS`:** decodes 12 MN103S 32-bit registers (`96` hex chars LE32) via `p7501_rsp_decode_mn103_regs`.
3. **`DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd>`:** dispatches `Matshita R6-MV4` Factory Drive CLI commands with automatic `PASS 0x4d415453 0x48495441` unlock unless `NOPASS` or an explicit `PASS` command is provided.
4. **Desktop Integration:** exposed in the Engineering Docs viewer under `docs/{en-US,pt-BR,audits,pseudocode,debug_logs}` via [`apps/desktop/src-tauri/src/commands/crypto_manuals.rs`](../../apps/desktop/src-tauri/src/commands/crypto_manuals.rs) and [`EngineeringDocsPanel.tsx`](../../apps/desktop/src/components/manuals/EngineeringDocsPanel.tsx).

---

## 3. Python -> Rust Conversion Audit Matrix

| Python Module(s) | Status | Rust Equivalent / Replacement | Engineering Decision |
| :--- | :--- | :--- | :--- |
| `scripts/re/mn10300.py`, `scripts/re/mn103_traps.py` | **Rust Parity Added** | `crates/panadiga-core/src/re/{mn10300,mn10300_traps}` + `panadiga re {audit, xrefs, drv-cli, drv-errors}` | Native zero-overhead CLI/core parity completed; Python retained temporarily for `scripts/tests/test_re_tools.py` parity verification. |
| `scripts/re/pana_frm.py` | **Mostly Superseded** | `crates/panadiga-crypto/src/` (`panadiga frm`) + `panadiga re drv-cli` | Feistel/`EXTRHEAD`/`.FRM` and `DRV` table parsing live in Rust; `DRV_UNLOCK_COMMAND` & read/write classifications share parity. |
| `scripts/bench/decoders.py` | **Rust Parity Added** | `crates/panadiga-core/src/pico/log_decoder/` (`panadiga pico decode-log`) + `apps/desktop/src-tauri/src/pico/diagnostics/parser.rs` | Offline UTF-16LE/UTF-8 boot log + SCSI SENSE decoding now native in Rust CLI; `decoders.py` stays only for `test_bench_ecosystem.py`. |
| `scripts/bench/{nor_dumper,rsp_frames,rsp_dump}.py` | **Core Superseded in Rust** | `panadiga-core/src/re/nor_dumper.rs`, `src-tauri/src/pico/rsp_protocol.rs`, `panadiga mcp` (`panadiga_nor_dumper_stub`) | Production RSP/dumper stubs run in Rust & Tauri; Python copies serve standalone pyserial bench scripts & pytest mocks. |
| `scripts/bench/{protocol,boot_harness,runner}.py` | **Optional Keep (Lab CLI)** | `apps/desktop/src-tauri/src/pico/serial/` & `panadiga pico` | Kept for quick zero-compile pyserial lab automation (`COM10`) with fault-simulation mock harness (`test_boot_harness.py`). |
| `tools/ghidra/*.py` (7 scripts + `java/` 4 sources) | **MUST STAY PYTHON/JAVA** | Jython scripts generated from Rust SSOT via `panadiga ghidra export-all`; Headless 12.1.4 Java suite in `tools/ghidra/java/` | Runs inside Ghidra JVM (`Jython 2.7` / Java) calling `currentProgram` / `FlatProgramAPI` / `DecompInterface`. Rust stays SSOT and generates `.py` scripts. |
| `scripts/build.py`, `validate.py`, `panadiga_ops.py`, `panadiga_cache.py`, `build_cache/*`, `build_lanes/*`, `tui.py` | **MUST STAY PYTHON** | None (meta-build orchestrator) | Orchestrates Cargo, Tauri, Ninja/ARM GCC, Android Gradle, `tsc`, Vitest, `jscpd`, `uxlint`, Playwright, `mypy --strict` and `pytest`; must remain external to Cargo. |
| `scripts/dev/{cad_primitives,schematic_sheet,generate_schematics_and_cad}.py` | **MUST STAY PYTHON** | None (Matplotlib IEEE/IEC renderer) | Offline vector-to-PNG engineering schematic & CAD generator (`300 DPI` plots). |
| `scripts/tests/*.py` | **MUST STAY PYTHON** | Covered by Rust cargo tests on Rust side | Gate 9 (`mypy --strict` + `pytest`) contract verification for the remaining Python modules. |
