> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/DIGADIAG_SALTO_50400000_E_ESCRITA_PROTEGIDA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# DIGADIAG `JMP 0x50400000` & Guarded Read/Write Architecture

This engineering note specifies two cross-stack upgrades implemented across **Pico RP2040/RP2350 C11 firmware**, **Rust core/CLI/MCP**, **Python tools**, and the **Tauri desktop application**:
1. Canonical **`0x50400000` entry alignment** whenever a payload is packaged inside a 128 KiB plaintext `DIGADIAG.FRM` (`EXTRHEAD`).
2. Transition from read-only inspection to **Guarded Read/Write (`Safe R/W`)** with strict hardware lockouts preserving the Spansion S29JL064H Bootloader IPL.

---

## 1. `DIGADIAG` Jump `0x50400000` vs Live RSP RAM Stub (`0x51800000`)

Disassembly of `/BOOT.BIN` (`FUN_44006dbc` and `FUN_44006fdd`) confirmed that when the Stage-1 bootloader consumes `DIGADIAG.FRM` (`131,072` B `EXTRHEAD`) from FAT16 SD, it ALWAYS decompresses/copies the payload starting at `0x50400000` and jumps directly to `0x50400000`. Meanwhile, live GDB RSP UART payloads (`$M` / `$X` / `$G`) target free SDRAM at `0x51800000`.

`resolve_nor_stub_load_addr(explicit, wrap_digadiag)` in [`crates/panadiga-core/src/re/nor_dumper.rs`](../../crates/panadiga-core/src/re/nor_dumper.rs) enforces this split deterministically:
- `wrap_digadiag = true` -> defaults to **`0x50400000`** (`DIGADIAG_NOR_STUB_LOAD_ADDR` / `PANA_DIGADIAG_DEFAULT_LOAD_ADDR`).
- `wrap_digadiag = false` -> defaults to **`0x51800000`** (`DEFAULT_NOR_STUB_LOAD_ADDR`).

Synchronized across:
- `panadiga unbrick nor-stub --wrap-digadiag`
- `panadiga_nor_dumper_stub` MCP tool (`wrap_digadiag`)
- Tauri `parse_nor_dumper_params` in `apps/desktop/src-tauri/src/commands/unbrick_dto.rs`
- Desktop ViewModel `useUnbrickViewModel.ts` (`buildNorStub`) & `mockUnbrickHandlers.ts`

---

## 2. Guarded Read/Write Across All Stacks (`E05` IPL Air-Gap)

Writes are now permitted **exclusively** in volatile memory (SRAM, SDRAM, MMIO control registers) and the top 64 KiB writable NVRAM sector/disk Copy-on-Write images, while **Bootloader IPL NOR Flash (`0x80000000..=nvram_start - 1` / `0x80000000..=0x807EFFFF` and `0x84000000..=0x847EFFFF`) remains strictly read-only (`E05`)**:

| Component | Read/Write Extensions | Enforced Lockouts |
| :--- | :--- | :--- |
| **Pico C11 GDB RSP** (`firmware/pico-p7501/gdb/gdb_rsp_memory.c`) | `$M` / `$X` write to Internal SRAM (`0x00000000..=0x0003FFFF`), Core MMIO (`0x10000000..=0x1000FFFF`), SDRAM (`0x40000000..=0x43FFFFFF`), Top 64 KiB NVRAM, OCD MMIO (`0xC0000000..=0xC000FFFF`) | Overlap with `0x80000000..=(nvram_start - 1)` rejected with `E05`; `NSW5V` (`GP3`) remains Hi-Z. |
| **Pico C11 `PANARSP` & `DIAGWATCH`** (`uart/p7501_pana_rsp_codec.c`, `uart/p7501_diag_watch_codec.c`) | `PANARSP` supports `WMEM` / `WRITE32` / `DUMPNOR`; `DIAGWATCH` drives/watches `PANAMON1` UART0 sessions | `PANAMON1` (`0x50439494` / offset `0x039494` in `27,496` B slack) supports `'B'`, `'R'`, `'W'`, `'D'`, `'F'`, `'J'`, `'E'`, `'Q'` — `'W'`/`'E'` block IPL with `E05`. |
| **Rust Core / CLI / MCP** (`nvram`, `matshita_emu`) | `panadiga nvram patch --input ... --output ... --offset ... --hex ...`; MCP `panadiga_clean_nvram` (`action: "patch"`) | Copy-on-Write mandatory (`output != input`); writes confined to `0..65535` byte NVRAM slice. |
| **Tauri IPC & UI** (`storage.rs`, `NvramPatchCard`, `Sc0BridgePanel`) | `patch_nvram_command`, `$M51800000` guarded SDRAM probe in `Sc0BridgePanel` | Mock & live Tauri guards block writes outside 64 KiB NVRAM or targeting IPL addresses with `E05`. |
