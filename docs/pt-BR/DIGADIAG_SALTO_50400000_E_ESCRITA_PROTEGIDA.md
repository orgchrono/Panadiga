> 🌐 **Idioma / Language:** [English (US)](../en-US/DIGADIAG_JUMP_50400000_AND_GUARDED_RW.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# Salto `DIGADIAG` em `0x50400000` e Escrita Protegida (Guarded R/W)

Esta especificação documenta as atualizações transversais em **firmware C11 Pico (`RP2040`/`RP2350`)**, **Rust core/CLI/MCP**, **scripts Python** e **aplicação desktop Tauri**:
1. Alinhamento obrigatório em **`0x50400000`** para cargas **`DIGADIAG.FRM`** (seja o binário bruto descomprimido de **`262.144 B` / `0x40000`** carregado em **`0x90400000`** no **`Loader ver0.61 [06/03/06]`** do DMR-EH55, seja o contêiner `EXTRHEAD` de `131.072 B` / `0x20000` em `0x93fc0000` no **`Loader ver0.63 [07/10/24]`** de `/BOOT.BIN`).
2. Evolução de modo estritamente *read-only* para **Leitura/Escrita Protegida (`Guarded R/W`)** mantendo bloqueio físico/lógico (`E05`) sobre o Bootloader IPL Spansion S29JL064H.

---

## 1. Endereço de Carga: `DIGADIAG` (`0x50400000`) vs Stub Live RSP (`0x51800000`)

A engenharia reversa cruzada confirmou o alinhamento em **`0x50400000`** nas duas gerações de bootloader, com formatos de arquivo distintos no cartão SD:
- **DMR-EH55 real (`Loader ver0.61 [06/03/06]`**, espelhado em `prog061_dec.bin` `sd_load.o` @ `0x5040eca9` / `0x5040ed7c` / `0x5040ea7d` e NOR de março de 2006 `flash.bin` @ `0x440085d9` / `0x440095ab` / `0x4400bc70`): o Estágio 1 lê `DIGADIAG.FRM` como **binário bruto descomprimido de exatos `262.144 B` (`0x40000`)** (iniciando em `dc 14 00 00 00` = `JMP 0x50400014`, sem `EXTRHEAD`) diretamente para **`0x90400000`** (alias *uncached* de `0x50400000`) e salta diretamente para **`0x50400000`** (`JMP 0x50400000` @ `0x44008601`) após `*** start diag ***`; e `PANA__SD.FRM` usa TOC plaintext de **`48 bytes` (`0x1800 B`, modo `sd48`, `7.531.744 B`)**.
- **`Loader ver0.63 [07/10/24]` (`/BOOT.BIN` do DMR-EZ485V**, `FUN_4400a97c` / `FUN_44006dbc` / `FUN_44006fdd`): lê o contêiner `EXTRHEAD` de **`131.072 B` (`0x20000`)** para **`0x93fc0000`** e o descomprime para **`0x50400000`**, usando TOC plaintext de **`64 bytes` (`0x2000 B`, modo `sd64`)** em `PANA__SD.FRM`.
- Já os stubs enviados diretamente na SDRAM viva via GDB RSP (`$M`/`$X`/`$G`) permanecem em **`0x51800000`**.

A função `resolve_nor_stub_load_addr(explicit, wrap_digadiag)` em [`crates/panadiga-core/src/re/nor_dumper/mod.rs`](../../crates/panadiga-core/src/re/nor_dumper/mod.rs) centraliza essa regra:
- `wrap_digadiag = true` -> padrão **`0x50400000`** (`DIGADIAG_NOR_STUB_LOAD_ADDR`).
- `wrap_digadiag = false` -> padrão **`0x51800000`** (`DEFAULT_NOR_STUB_LOAD_ADDR`).

Sincronizado também em:
- CLI: `panadiga unbrick nor-stub --wrap-digadiag`
- MCP: `panadiga_nor_dumper_stub` (`wrap_digadiag`)
- Tauri: `parse_nor_dumper_params` em `apps/desktop/src-tauri/src/commands/unbrick_dto.rs`
- Frontend: `useUnbrickViewModel.ts` (`buildNorStub`) e `mockUnbrickHandlers.ts`

---

## 2. Leitura/Escrita Protegida (`Guarded R/W`) e Invariante `E05`

Operações de escrita foram liberadas exclusivamente para memória volátil (SRAM/SDRAM/MMIO) e para os **64 KiB de NVRAM** ou arquivos em **Copy-on-Write**, preservando a Flash **IPL Spansion (`0x80000000..=nvram_start - 1` / `0x80000000..=0x807EFFFF` e `0x84000000..=0x847EFFFF`) estritamente somente leitura (`E05`)**:

| Camada | Extensões R/W | Travas de Segurança |
| :--- | :--- | :--- |
| **Pico C11 GDB RSP** (`firmware/pico-p7501/gdb/gdb_rsp_memory.c`) | `$M`/`$X` em SRAM (`0x00000000..=0x0003FFFF`), MMIO (`0x10000000..=0x1000FFFF`), SDRAM (`0x40000000..=0x43FFFFFF`), Top 64 KiB NVRAM, OCD MMIO (`0xC0000000..=0xC000FFFF`) | Qualquer sobreposição com a região IPL retorna `E05`; pino `NSW5V` (`GP3`) permanece estritamente Hi-Z. |
| **Pico C11 `PANARSP` & `DIAGWATCH`** (`p7501_pana_rsp_codec.c`, `p7501_diag_watch_codec.c`) | `PANARSP` com `WMEM`/`WRITE32`/`DUMPNOR`; `DIAGWATCH` acoplado ao monitor UART0 `PANAMON1` | `PANAMON1` (`0x50439494` / offset `0x039494` em `27.496` B livres; compilado com ABI MN103S `CALL ..., [], 0` + `PUTC_D3_BYTES` `46 B` timeout `0x00040000` em `SC0STR & 0xA0`) executa `'B'`, `'R'`, `'W'`, `'D'`, `'F'`, `'J'`, `'E'`, `'U'`, `'Q'` bloqueando IPL `SA0..SA3` com `E05`. |
| **Rust Core / CLI / MCP** (`nvram`, `matshita_emu`) | `panadiga nvram patch --input ... --output ... --offset ... --hex ...`; MCP `panadiga_clean_nvram` (`action: "patch"`) | Exige Copy-on-Write (`output != input`) e faixa estrita `0..65535` B da NVRAM. |
| **Tauri IPC & UI Desktop** (`storage.rs`, `NvramPatchCard`, `Sc0BridgePanel`) | `patch_nvram_command`, botão `$M51800000` em `Sc0BridgePanel` | Validação Copy-on-Write no ViewModel, backend Rust e handlers mock com rejeição `E05` para endereços IPL. |
