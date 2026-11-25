> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/DIGADIAG_SALTO_50400000_E_ESCRITA_PROTEGIDA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# DIGADIAG `JMP 0x50400000` & Guarded Read/Write Architecture

This engineering note specifies two cross-stack upgrades implemented across **Pico RP2040/RP2350 C11 firmware**, **Rust core/CLI/MCP**, **Python tools**, and the **Tauri desktop application**:
1. Canonical **`0x50400000` entry alignment** for **`DIGADIAG.FRM`** payloads — both the **raw uncompressed `262,144 B` (`0x40000`) binary** loaded to **`0x90400000`** on real DMR-EH55 **`Loader ver0.61 [06/03/06]`**, and the `131,072 B` (`0x20000`) plaintext `EXTRHEAD` container loaded at `0x93fc0000` on **`Loader ver0.63 [07/10/24]`** (`/BOOT.BIN` from DMR-EZ485V).
2. Transition from read-only inspection to **Guarded Read/Write (`Safe R/W`)** with strict hardware lockouts preserving the Spansion S29JL064H Bootloader IPL.

---

## 1. `DIGADIAG` Jump `0x50400000` vs Live RSP RAM Stub (`0x51800000`)

Cross-binary analysis confirmed canonical **`0x50400000`** execution entry across both bootloader generations, despite distinct SD container formats:
- **Real DMR-EH55 NOR (`Loader ver0.61 [06/03/06]`**, mirrored in `prog061_dec.bin` `sd_load.o` @ `0x5040eca9` / `0x5040ed7c` / `0x5040ea7d` and March 2006 NOR `flash.bin` @ `0x440085d9` / `0x440095ab` / `0x4400bc70`): Stage 1 reads `DIGADIAG.FRM` as an **exact `262,144 B` (`0x40000`) uncompressed raw binary** (starting `dc 14 00 00 00` = `JMP 0x50400014`, without `EXTRHEAD`) directly into **`0x90400000`** (uncached alias of `0x50400000`) and branches straight to **`0x50400000`** (`JMP 0x50400000` @ `0x44008601`) after `*** start diag ***`; and `PANA__SD.FRM` uses plaintext **`48-byte` (`0x1800 B`, mode `sd48`, `7,531,744 B`)** TOC entries.
- **`Loader ver0.63 [07/10/24]` (`/BOOT.BIN` from DMR-EZ485V**, `FUN_4400a97c` / `FUN_44006dbc` / `FUN_44006fdd`): loads the **`131,072 B` (`0x20000`) `EXTRHEAD` container** to **`0x93fc0000`** and decompresses it to **`0x50400000`**, pairing with plaintext **`64-byte` (`0x2000 B`, mode `sd64`)** TOC entries in `PANA__SD.FRM`.
- Meanwhile, live GDB RSP UART payloads (`$M` / `$X` / `$G`) target free SDRAM at **`0x51800000`**.

`resolve_nor_stub_load_addr(explicit, wrap_digadiag)` in [`crates/panadiga-core/src/re/nor_dumper/mod.rs`](../../crates/panadiga-core/src/re/nor_dumper/mod.rs) enforces this split deterministically:
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
| **Pico C11 GDB RSP** (`firmware/picofw/gdb/gdb_rsp_memory.c`) | `$M` / `$X` write to Internal SRAM (`0x00000000..=0x0003FFFF`), Core MMIO (`0x10000000..=0x1000FFFF`), SDRAM (`0x40000000..=0x43FFFFFF`), Top 64 KiB NVRAM, OCD MMIO (`0xC0000000..=0xC000FFFF`) | Overlap with `0x80000000..=(nvram_start - 1)` rejected with `E05`; `NSW5V` (`GP3`) remains Hi-Z. |
| **Pico C11 `PANARSP` & `DIAGWATCH`** (`uart/p7501_pana_rsp_codec.c`, `uart/p7501_diag_watch_codec.c`) | `PANARSP` supports `WMEM` / `WRITE32` / `DUMPNOR`; `DIAGWATCH` drives/watches `PANAMON1` UART0 sessions | `PANAMON1` (`0x50439494` / offset `0x039494` in `27,496` B slack; emitted with MN103S compiler `CALL ..., [], 0` ABI + `46 B` `PUTC_D3_BYTES` `SC0STR & 0xA0`/`0x00040000` timeout) supports `'B'`, `'R'`, `'W'`, `'D'`, `'F'`, `'J'`, `'E'`, `'U'`, `'Q'` — `'W'`/`'E'` block `SA0..SA3` IPL with `E05` and `'U'` unlocks Upper NOR `SA4..SA133`. |
| **Rust Core / CLI / MCP** (`nvram`, `matshita_emu`) | `panadiga nvram patch --input ... --output ... --offset ... --hex ...`; MCP `panadiga_clean_nvram` (`action: "patch"`) | Copy-on-Write mandatory (`output != input`); writes confined to `0..65535` byte NVRAM slice. |
| **Tauri IPC & UI** (`storage.rs`, `NvramPatchCard`, `Sc0BridgePanel`) | `patch_nvram_command`, `$M51800000` guarded SDRAM probe in `Sc0BridgePanel` | Mock & live Tauri guards block writes outside 64 KiB NVRAM or targeting IPL addresses with `E05`. |
