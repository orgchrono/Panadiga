> 🌐 **Idioma / Language:** [English (US)](../en-US/MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# As 7 Armadilhas MN103S / AM34 e Resolução no Ghidra & Rust

Este guia detalha as **sete armadilhas recorrentes de disassembly e descompilação MN10300 / MN103S (`AM34`)** identificadas em `BOOT.BIN`, `PROG061`, `DRV` e `MAIN_3410PQ`, explicando inclusive os trechos com **`??` no disassembly** e documentando os resolvedores em Rust (`crates/panadiga-core/src/re/mn10300_traps/mod.rs`), Python (`scripts/re/mn103_traps.py`), Mini-VM (`crates/panadiga-core/src/re/mn10300_emu/mod.rs`) e scripts Jython (`tools/ghidra/ghidra_mn103_fix_decompilation.py`).

> [!NOTE]
> Os geradores e analisadores atuam somente de forma estática ou exportam scripts Jython via `panadiga ghidra export-all`. Os bancos binários de projeto do Ghidra (`C:\ghidra\projects\Panadiga_MN103S`) não são modificados diretamente.

---

## 1. Matriz das 7 Armadilhas e Resolvedores

| # | Armadilha | Assinatura Típica no Binário | Resolução em `panadiga-core` & Ghidra |
| :---: | :--- | :--- | :--- |
| **1** | Funções Órfãs e Tabelas `<8sIII>` / CDB | Rotinas sem xrefs diretas atrás de tabelas CLI (`0x400DF5C0`), slots ATAPI CDB ou ponteiros (`0x5040EA30`) | `parse_drv_cli_table`, `parse_atapi_cdb_dispatch` e `scan_code_pointer_tables` recuperam os ponteiros reais filtrando stubs (`0x40014559`). |
| **2** | Efeitos Colaterais Indiretos `BCLR 0xff` / `BSET` RMW | Escritas bitwise em flags de hardware (`0x5771AC87`, `0x40130EFA`) mascaradas por aliasing de ponteiros Sleigh | `classify_drv_cli_command` e tipagem simbólica marcam explicitamente flags de sessão em SRAM (`0x0CD8`, `0x0D78`). |
| **3** | Poda de Blocos Pós-Chamada e Anomalias `SETLB`/Retorno | Descompilador truncando blocos (`0x574DD208`) após prólogos/epílogos fora do padrão | `ghidra_mn103_fix_decompilation.py` corrige *stack-purge* e restaura o fluxo pós-chamada. |
| **4** | Colisão de Salto no Meio de Instrução (`??`) | Desvios caindo no interior de instruções multi-byte (ex.: `BRA 0x5040ea2f` dentro de `0x5040ea2e`) gerando `??` | **Otimização esperada do compilador**, não erro de leitura: `build_instruction_spans` + `detect_mid_instruction_overlaps` identificam saltos em `[start_va + 1, end_va)`. |
| **5** | Ciclos CFI Spansion CS1 em NOR Flash | Acessos a `0x84200554`, `0x84200AAA`, `0x84200000` confundidos com ponteiros RAM | `is_flash_cfi_command_va` e `scan_cfi_flash_constant_hits` reconhecem ciclos JEDEC/CFI (`0x000`/`0x554`/`0x555`/`0xAAA`/`0xAAB` em `0x8000..`/`0x8400..`) modelados por `CfiStage` na VM. |
| **6** | Loops de Hardware `SETLB`..`Lcc` e Frames `A3` Negativos | Loops `SETLB` (`0xDB`) .. `LNE` (`0xD9`) (`in_LAR - 4`) e acessos negativos em `A3` (`0x50404F12`) | `decode_setlb_loops` identifica limites de loop; `Mn103Vm` modela `LAR`/`LIR` determinísticos. |
| **7** | Deslocamento entre Revisões, Tabela CP932 e Falsos LE32 | Símbolos deslocados entre `DRV_decomp` (`PASS` `0x4013420A`) e `EH55_DRV_S211` (`PASS` `0x40130EFA`), tabela Shift-JIS de `0x34` bytes (`0x400E779A`) e constantes LE32 falsas (`0x4006F5E2`) | `identify_firmware_revision`, `parse_drv_mecha_error_table` e `filter_mid_instruction_constant_hits` separam referências legítimas de colisões em meio de instrução. |

---

## 2. Por que aparecem trechos com `??` no Disassembly?

Na arquitetura MN103S, o compilador Matsushita compartilha bytes de instruções vizinhas (Armadilha 4, como `BRA 0x5040ea2f` saltando no byte `+1` da instrução iniciada em `0x5040ea2e`). O disassembler linear marca conflito/bytes órfãos como `??` por já estarem alocados em outra instrução válida — portanto é esperado e auditável matematicamente por `detect_mid_instruction_overlaps`, sem indicar perda de bits no dump nem travamento do debugger.

---

## 3. Mini-VM Determinística MN103S (`Mn103Vm`)

Para validar patches e fast-paths offline (`crates/panadiga-core/src/re/mn10300_emu/mod.rs`), `Mn103Vm` emula `D0..D3`, `A0..A3`, `SP`, `PC`, `MDR`, `LIR`, `LAR` e `PSW`:

- **Escopo suportado**: instruções simples `0x00..=0xEF` (`CALL`/`RET`/`SETLB`/`Lcc`), `MOVM` (`0xCE`/`0xCF`) e subconjuntos verificados de `0xF0`, `0xF1`, `0xF2`, `0xF8` e `0xFC`.
- **Fail-closed**: prefixos fora de escopo (`0xF3..=0xF7`, `0xF9..=0xFB`, `0xFD..=0xFF`), `RTI`, MMIO/interrupções não mapeadas e tentativas de escrita na Flash IPL retornam `Mn103Fault::IplWriteProtected` / `InvalidOpcode`.

---

## 4. Exportação dos Scripts Ghidra

```powershell
cargo run -p panadiga-cli -- ghidra export-all --dir tools/ghidra
```
