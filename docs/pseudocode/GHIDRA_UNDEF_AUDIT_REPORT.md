> 🌐 **Language / Idioma:** **Bilingual (PT-BR / EN-US)**
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit (EN)](../../COMPLETE_AUDIT.md) | [Auditoria Completa (PT-BR)](../../AUDITORIA_COMPLETA.md) | [Seven Traps (EN)](../en-US/MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md) | [Sete Armadilhas (PT-BR)](../pt-BR/ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md) | [Pseudocode Index](./INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Auditoria Exaustiva de `??` (`Undefined`) no Disassembly MN103S / AM34 (Ghidra) / Exhaustive `??` Undefined-Byte Disassembly Audit

> **Projeto Analisado (somente leitura) / Inspected Project (strictly read-only):** `C:\ghidra\projects\Panadiga_MN103S` (`mn10300:LE:32:default`)  
> **Artefatos complementares / Companion Artifacts:** [Índice de Pseudocódigo](./INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) · [Evidência Integral (26.555 linhas)](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) · [Raw Captures](./raw/doc_boot_core.txt)

---

## 1. Veredito Técnico / Technical Verdict

**PT-BR:** Os trechos marcados com **`??`** no disassembly linear do Ghidra **não são erro de dump nem perda de comunicação no hardware**. A auditoria exaustiva sobre os **5 programas binários** no projeto `Panadiga_MN103S` provou **três causas determinísticas e mensuráveis**:

**EN-US:** The byte ranges rendered as **`??`** in Ghidra's linear disassembly are **neither dump corruption nor hardware UART loss**. Exhaustive inspection across all **5 binary programs** in `Panadiga_MN103S` proved **three deterministic, measurable root causes**:

1. **Tabelas de dados, literal pools LE32 e strings embutidas (`data_literal_pool_or_unimpl_opcode`) / Embedded literal pools, LE32 constants & strings:** Constants and strings interleaved between functions not yet separated as typed data by linear sweep.
2. **Ilhas de código válidas não alcançadas (`unanalyzed_clean_code_island`) / Unexplored clean code islands:** Valid functions starting with standard MN103S prologues (`MOVM` `0xCF` / `ADD disp8,SP` `0xF8 0xFE`) reachable only via indirect dispatch tables (`<8sIII>`, ATAPI CDBs, switch tables).
3. **Otimizações de opcode + lacunas específicas do módulo SLEIGH `AM33/MN10300` / Compiler opcode optimizations + SLEIGH `AM33/MN10300` gaps:**
   - **Armadilha 4 / Trap 4 (Saltos em meio de instrução / Mid-instruction overlaps):** `BRA` jumping `-1` into byte `+1` of an adjacent multi-byte instruction.
   - **Opcodes `AM33` ausentes no SLEIGH padrão / Missing AM33 opcodes in stock SLEIGH:** Especially `0xF9 0xEA <reg>` (`MOVHU (Rm0+), Rn2`), `0xF9 0xFA <reg>`, prefix `0xFF 0xF0 0xFC` before prologues, and `0xF0 0xC0`.

---

## 2. Inventário Quantitativo por Binário

| Programa Ghidra | `funcs` | Bloco Inicializado | Instruções | Dados Tipados | Bytes `??` (`Undef`) | Gaps `??` | Principal Causa Observada |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | :--- |
| `/BOOT.BIN` | 843 | 262.144 B | 178.876 B | 16.168 B | 67.100 B (25,60%) | 170 | Pools/unimpl (41.508 B em 27 gaps) + ilhas limpas (22.910 B em 93 gaps) |
| `/eh55_frm_PROG061_BOOT.bin` | 5 | 262.144 B | 666 B | 0 B | 261.478 B (99,75%) | 9 | Carregado por endereço base único; 6 gaps >=64 B com prólogos `CF`/`F8FE` (maior gap `0x50406DAA`: 157.394 B) |
| `/EH55_DRV_S211.bin` | 15 | 1.441.792 B | 7.613 B | 0 B | 1.434.179 B (99,47%) | 26 | Ilhas limpas (780.547 B em 19 gaps) + pools/unimpl (653.632 B em 7 gaps) |
| `/DRV_decomp.bin` | 2.346 | 1.441.792 B | 727.726 B | 47.990 B | 666.076 B (46,20%) | 1.957 | Pools/unimpl (417.487 B / 341 gaps) + ilhas limpas (153.222 B / 292 gaps) + strings (84.014 B / 860 gaps) + ptrs (2.543 B / 34 gaps) |
| `/EH55_MAIN_3410PQ.bin` | 33 | 13.003.052 B | 825.478 B | — | 12.177.574 B (93,65%) | 1.986 | Pools/unimpl (8.168.656 B / 136 gaps) + ilhas limpas (4.004.135 B / 1.706 gaps; 1.399 gaps >=64 B com `CF`/`F8FE`) |

### Detalhes Auditados em `/BOOT.BIN`
- **Classificação dos 170 gaps:** `27` gaps (`41.508 B`) de `data_literal_pool_or_unimpl_opcode`, `93` gaps (`22.910 B`) de `unanalyzed_clean_code_island`, `29` gaps (`2.559 B`) de `text_or_strings` e `1` gap (`24 B` em `0x44035EB0..0x44035EC7`) de `pointer_or_jump_table`.
- **Amostras de fronteira / alinhamento e opcodes não decodificados pelo Sleigh:**
  - `0x4400493B`: `[dd 0e fe ff ff]` (`call32` de 5 bytes que falhou na varredura inicial por fronteira de unidade de código).
  - Opcodes `0xF9 0xEA c8` (`MOVHU (Rm0+), Rn2`): observados em `0x44011553`, `0x44012E3E`, `0x44019865`, `0x44019FB9`, `0x4401B415`, `0x4401FA63` e `0x4401FB5C`.

### Amostras Concretas nos Demais Estágios
- **`/eh55_frm_PROG061_BOOT.bin`**:
  - Maiores gaps com prólogos `CF`/`F8FE`: `0x50406DAA` (`157.394 B`), `0x5042D51E` (`76.514 B`), `0x50400913` (`25.629 B`), `0x5040032F` (`1.169 B`), `0x5040011E` (`527 B`).
  - Opcodes `??` capturados logo após retornos ou desvios: `0x504009E4` (`ff f0 fc`), `0x50416355` (`f9 ea c8` seguido de `f9 fa c9` antes de `RETS`), `0x504313E6` (`f0 c0`).
- **`/EH55_DRV_S211.bin`**:
  - Maiores gaps com prólogos `CF`/`F8FE`: `0x400D7345` (`366.715 B`), `0x40131578` (`191.112 B`), `0x400109F6` (`93.988 B`), `0x40006F2A` (`37.790 B`), `0x400285BB` (`31.429 B`).
  - Exemplos típicos de blocos de código logo após `RET`/`BRA` deixados em `??`: `0x40003849`, `0x40006E09`, `0x40131C81`.
- **`/EH55_MAIN_3410PQ.bin`**:
  - `3.935` falhas iniciadas por prefixo `0xF9` (padrões consecutivos de `f9 ea c8` em `0x5700A836`, `0x5700A83C`, `0x5700A842`; pares `[f9 ea c8][f9 fa c9]` em `0x5700C051..0x5700C062`; `ff f0 fc` em `0x57000D2C`).
  - Maiores ilhas não exploradas com prólogos `CF`/`F8FE`: `0x57773665` (`344.148 B`), `0x5765AF6F` (`342.440 B`), `0x5713CDC9` (`218.018 B`), `0x57403559` (`165.691 B`), `0x573B6512` (`163.043 B`).

---

## 3. Como o Ecossistema Panadiga Trata Isso Sem Mutar o Banco `.gpr` / Read-Only Ecosystem Resolution

1. **CLI Nativa Rust / Native Rust CLI (`panadiga re`):**
   - `panadiga re audit --input <BIN>` automatically detects all **7 static MN10300 trap families** (`orphan_code_pointers`, `cfi_constant_sites`, `mid_instruction_overlaps`, `setlb_loop_count`, `am33_f9_ea_sites`, `am33_ff_pre_rets_sites`) directly in Rust ([`crates/panadiga-core/src/re/mn10300_traps/`](../../crates/panadiga-core/src/re/mn10300_traps/mod.rs)); see also [Part V of `EVIDENCIA`](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md#L26534) for all **10** Ghidra/SLEIGH decompiler trap classes (including `#8 SUB fb 9c`, `#9 NOT Dm1,Dn0 -> XOR`, and `#10 BTST SIMM,Rn0 -> mn_todo()`).
   - `panadiga re xrefs`, `panadiga re drv-cli`, and `panadiga re drv-errors` extract cross-references, `<8sIII>` dispatch tables, and the `0x34`-byte Shift-JIS error dictionary.
2. **Gerador Jython / Jython Generator (`panadiga ghidra export-all`):**
   - Exports [`tools/ghidra/ghidra_mn103_fix_decompilation.py`](../../tools/ghidra/ghidra_mn103_fix_decompilation.py) and [`tools/ghidra/ghidra_auto_unbrick_audit.py`](../../tools/ghidra/ghidra_auto_unbrick_audit.py) so operators can recover orphan prologues, fix stack-purge, and resolve cross-references safely inside Ghidra.

---

## 4. Plano de Remoção 100% de `??` & Patch SLEIGH Verificado / 100% Zero-`??` Recovery Plan & Verified SLEIGH Patch

**PT-BR:** Para eliminar os `??` e os `mn_todo()` sem mutar `C:\ghidra\projects\Panadiga_MN103S`, o repositório agora inclui o patch unificado [`tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch`](../../tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch) e o roteiro de 4 passes [`tools/ghidra/sleigh_patches/README.md`](../../tools/ghidra/sleigh_patches/README.md), **compilado e verificado com `sleigh.bat -u -f` do Ghidra 12.1.4 (`exit 0`, zero conflitos de construtores)**.

**EN-US:** To eliminate `??` gaps and `mn_todo()` stubs without mutating `C:\ghidra\projects\Panadiga_MN103S`, the repository now ships the unified diff [`tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch`](../../tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch) and 4-pass headless runbook [`tools/ghidra/sleigh_patches/README.md`](../../tools/ghidra/sleigh_patches/README.md), **compile-verified with Ghidra 12.1.4 `sleigh.bat -u -f` (`exit 0`, zero constructor conflicts)**.

- **12 defeitos SLEIGH corrigidos (`am33.sinc` + `mn103.sinc`):** inclusão de `:MOVHU (Rm0+), Rn2` (`0xF9 0xEA`), `:SYSCALL` (`0xF0 0xC0`) e `:BREAK` (`0xFF`); correção de `SUB` 3-op (`am33.sinc:184`), `SUB SIMM` (`186-188`), `MOV Rm2,(IMM32,Rn0)` (`98`), `MOVBU` 8-bit (`130-135`), `AND IMM32,EPSW` (`273`), `NOT`/`BTST`/`ADDC`/`SUBC`/`ASR`/`ASL`/`ROR`/`ROL` (`am33.sinc`), além de `BCLR` (`| ~` → `& ~` em `mn103.sinc:519-522`), `NOT Dn0` (`503`), `EXTB`/`EXTH` (`367, 369`) e `SUBC` (`427`).
- **Pipeline Headless de 4 Passes (em clone `Panadiga_MN103S_ZeroUndef`):** (1) compilação isolada de `mn103.sla` + importação dos 6 estágios incluindo `eh55_loader_ver061_256k.bin`; (2) tipagem de strings ASCII/Shift-JIS, tabelas `<8sIII>`/CDB/BGO/Feistel e padding `0xFF`; (3) varredura exaustiva de prólogos `0xCF`/`0xF8 0xFE` + bookmark de sobreposições Trap #4; (4) `ghidra_mn103_fix_decompilation.py` (stack-purge=0, MMIO `volatile`) + gate final `undefined_code_bytes == 0`.
