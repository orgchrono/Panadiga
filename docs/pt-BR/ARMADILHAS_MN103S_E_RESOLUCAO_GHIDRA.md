> 🌐 **Idioma / Language:** [English (US)](../en-US/MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# As 7 Armadilhas MN103S / AM34 e Resolução no Ghidra & Rust (+ Armadilha #8 SLEIGH)

Este guia detalha as **sete armadilhas recorrentes de disassembly e descompilação MN10300 / MN103S (`AM34`)** mais a **Armadilha #8 (inversão semântica de `SUB` no SLEIGH)** identificadas em `BOOT.BIN`, `PROG061`, `DRV` e `MAIN_3410PQ`, explicando inclusive os trechos com **`??` no disassembly** e documentando os resolvedores em Rust (`crates/panadiga-core/src/re/mn10300_traps/mod.rs`), Python (`scripts/re/mn103_traps.py`), Mini-VM (`crates/panadiga-core/src/re/mn10300_emu/mod.rs`) e scripts Jython (`tools/ghidra/ghidra_mn103_fix_decompilation.py`).

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
| **8** | Inversão Semântica Sistemática em `SUB` no SLEIGH Ghidra | `SUB` de 3 operandos (`fb 9c ..`, ex.: `0x4400c77b` [byte `+1` `0x4400c77c`], `0x4400caeb`, `0x4400d525`, `0x4400e0c6` [byte `+1` `0x4400e0c7`], `0x4400e948`) descompilado como `src - dst` (`Rm - Rn`) em vez de `dst - src` (`Rn - Rm`) | Auditoria direta contra opcodes `BOOT.BIN` + spec SLEIGH (`am33.sinc:184` vs `mn103.sinc:421` `f1 ..`); `Mn103Vm` modela corretamente `dst = dst - src` em `ops.rs`. |

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

---

## 5. Armadilha #8: Inversão Semântica de `SUB` (`fb 9c ..` / `f1 ..`) no SLEIGH `mn10300:LE:32:default`

Durante a auditoria dos drivers FAT12/FAT16/FAT32 e SD em `C:\ghidra\binaries\BOOT.BIN` (`Loader ver0.63 [07/10/24]`, base `0x44000000`, Ghidra 12.1.4), identificou-se uma **inversão sistemática de operandos no módulo SLEIGH `GhidraMn10300`** para a instrução de subtração de 3 operandos `SUB Rm, Rn, Rd` (`fb 9c ..`), distinguindo-a da forma de 2 operandos `SUB Dm, Dn` (`f1 0x`):

- **Semântica real da CPU MN103S / AM33:** Na convenção Matsushita (`src, dst`), a subtração de 2 operandos `SUB Dm, Dn` (`f1 0x`) computa `Dn = Dn - Dm` (`dst = dst - src`, conforme modelado corretamente em `Mn103Vm` `crates/panadiga-core/src/re/mn10300_emu/ops.rs`), e a forma estendida AM33 de 3 operandos `SUB Rm, Rn, Rd` (`fb 9c ..`) computa **`Rd = Rn - Rm`** (`dst1 - src`).
- **Bug no SLEIGH do Ghidra (`mn10300:LE:32:default`):** Em `GhidraMn10300/data/languages/am33.sinc` (linha 184), a regra `:SUB Rm2, Rn0, Rd2 is op8=0xfb; op8=0x9c; Rm2 & Rn0; Rd2 & op4l=0` executa `{ local val:4 = Rm2; update_sub(val, Rn0); Rd2 = val; }`, emitindo **`Rm2 - Rn0` (`src - dst`)** em vez de `Rn0 - Rm2` (`dst - src`). Em contraste, `mn103.sinc` (linha 421, `:SUB Dm1, Dn0 is op8=0xF1; op4=0x0 & Dm1 & Dn0`) executa `update_sub(Dn0, Dm1)` = `Dn0 - Dm1` (ordem correta), embora `am33.sinc:186-188` (`SUB SIMM8/24/32, Rn0`) e `mn103.sinc:427` (`SUBC Dm1, Dn0` `f1 8x`) também apresentem defeito ao chamar `update_add`.

### Sítios Verificados Byte a Byte em `BOOT.BIN`

| Função | Endereço & Bytes Reais em `BOOT.BIN` | Referência / Alinhamento | Pseudo-C Emitido pelo Ghidra (Invertido) | Semântica Real Executada no Hardware |
| :--- | :--- | :--- | :--- | :--- |
| **`FUN_4400c696`** | `0x4400c77b`: `fb 9c dc d0` (`SUB D1, D0, D1`), seguido por `0x4400c782`: `f1 01` e `0x4400c787`: `f1 01` (`SUB D0, D1`) | `0x4400c77c` é o byte `+1` (`9c`) de `fb 9c dc d0` | `ReservedSectors - TotSec32 - puVar1[-3] - puVar1[-4]` (`*(ushort *)(puVar1[-5] + 0x24) - *(int *)(puVar1[-5] + 0x34)`) | **`TotSec32 - ReservedSectors - puVar1[-3] - puVar1[-4]`** (cálculo de clusters FAT16 > 32 MB funciona normalmente) |
| **`FUN_4400ca59`** | `0x4400caeb`: `fb 9c cd c0` (`SUB D0, D1, D0`) | Precedido por `0x4400cae8..0x4400caea`: `f8 03 f8` (`MOV (-0x8, A3), D0`) | `puVar1[3] = puVar1[-2] - puVar1[3]` (`chunk - remaining`) | **`puVar1[3] = puVar1[3] - puVar1[-2]`** (`remaining -= chunk`) |
| **`FUN_4400d32f`** | `0x4400d525`: `fb 9c cd c0` (`SUB D0, D1, D0`), além de `0x4400d7ef`, `0x4400d8b4`, `0x4400d921`, `0x4400d92e`, `0x4400d961`, `0x4400d98f`, `0x4400db58` (`f1 01` em `0x4400d9ad`/`0x4400d9d1`) | Nota forense: `0x4400d4b0` em `BOOT.BIN` é `e4` (`ADD D1, D0`) + `0x4400d4b1: f1 60` (`ADD D0, A0`) no teste FAT16 `== 0x10`; o `SUB` do truncamento está em `0x4400d525` | `puVar1[7] = *(int *)(puVar1[-5] + 0x44) - *(int *)(puVar1[-5] + 0x40)` (`Offset - FileSize`) | **`puVar1[7] = FileSize - Offset`** (trunca a leitura de `0x40000` para `0x20000` quando `DIGADIAG.FRM` tem 128 KiB) |
| **`FUN_4400dfc8`** | `0x4400e0c6`: `fb 9c cd c0` (`SUB D0, D1, D0`), além de `0x4400e2f9`, `0x4400e366`, `0x4400e373`, `0x4400e3a6`, `0x4400e3d4` | `0x4400e0c7` é o byte `+1` (`9c`) de `fb 9c cd c0` | `param_3 = _DAT_5056b6c4 - _DAT_5056b6c0` (`Offset - FileSize`) | **`param_3 = _DAT_5056b6c0 - _DAT_5056b6c4`** (`FileSize - Offset`) |
| **`FUN_4400e737`** | `0x4400e948`: `fb 9c cd c0` (`SUB D0, D1, D0`), além de `0x4400eb29`, `0x4400ed63` | Início exato em `0x4400e948` | `*(int *)(puVar1[-6] + 0x14) = *(int *)(puVar1[-6] + 0x10) - *(int *)(puVar1[-7] + 0x2c)` (`CacheStart - FATSectors`) | **`FATSectors - CacheStart`** (limite restante do cache de 3 setores da tabela FAT) |

---

## 6. Convenção ABI Real do Compilador MN103S & UART0 em Silício Real (`PANAMON1` / `nor_dumper`)

Durante o teste ao vivo do monitor **`PANAMON1`** em `COM10` sob `Loader ver0.61` (`641 B`, contornando `BGO driver init failed[1].` e parando antes do prompt), a auditoria completa de chamadas e sub-rotinas `uart0_putc` nos binários reais estabeleceu **duas regras obrigatórias de ABI e I/O**:

1. **Chamadas & Retornos (`100%` `CALL target, [], 0x00` / `RET ..., 0x00|0x20`):**
   - `1.386 / 1.386` chamadas em `PROG061` e `1.065 / 1.065` chamadas em `BOOT.BIN` utilizam **`CALL target, [], 0x00`** (`imm8 = 0`), reservando quadro de pilha explicitamente com `ADD -imm8, SP` / `ADD +imm8, SP` e encerrando com **`RET [], 0x00`** (ou `MOVM [D2,D3,A2,A3], (SP); ADD -16, SP ... RET [D2,D3,A2,A3], 0x20`, verificado em `PROG061 @ 0x50415CAD`), ficando o endereço de retorno imediatamente abaixo da área alocada por `ADD -N, SP` (ex.: flag de unlock em `(12, SP)` após `ADD -12, SP`).
2. **Polling `SC0STR` com Timeout Limitado (`PUTC_D3_BYTES` `46 B`):**
   - `PROG061 FUN_50404d15` (`0x50404D37`) e `BOOT.BIN uart0_putc` (`0x440047C7`) verificam `(SC0STR 0xD400200C) & 0xA0` (`TXBUSY | TXBF`) com contador de timeout (`0x1400000` em fábrica; `MOV 0x00040000, D1` preservando `D1` via `ADD -4, SP` no `PUTC_D3_BYTES`), evitando travamento infinito se flags de status serial não limparem imediatamente.
   - Tanto [`crates/panadiga-core/src/re/prog061_uart_monitor/`](../../crates/panadiga-core/src/re/prog061_uart_monitor/mod.rs) quanto [`crates/panadiga-core/src/re/nor_dumper/stub.rs`](../../crates/panadiga-core/src/re/nor_dumper/stub.rs) adotam essa ABI, verificada em Ghidra (`SUMMARY_CALLS: total=55 bad=0`) e em `Mn103Vm`.

---

## 7. Patch SLEIGH Verificado (`am33.sinc` + `mn103.sinc`) e Pipeline 100% Zero-`??`

A auditoria linha a linha da extensão local `GhidraMn10300` (`Ghidra 12.1.4 PUBLIC`) consolidou **12 defeitos reais** em `am33.sinc` e `mn103.sinc` no patch unificado [`tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch`](../../tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch) (compilado com `sleigh.bat -u -f` em cópia isolada sem erros nem conflitos de construtores):

- **Construtores ausentes geradores de `??`:** `:MOVHU (Rm0+), Rn2` (`0xF9 0xEA`, causa dos `3.935` sítios `0xF9` em `MAIN` e dos laços `memcpy` em `Loader ver0.61` `0x440242b7..0x440245ab`), `:SYSCALL` (`0xF0 0xC0`) e `:BREAK` (`0xFF`).
- **Bugs semânticos de P-Code:** `am33.sinc:98` (`MOV Rm2,(IMM32,Rn0)` somando `Rm2` em vez de `Rn0`), `134-135` (`MOVBU` 16-bit `*:2` em vez de `*:1`), `184` (`SUB Rm2,Rn0,Rd2` invertido), `186-188` (`SUB SIMM` chamando `update_add`), `273` (`AND IMM32,EPSW` usando `|`), além de `mn103.sinc:519-522` (`BCLR` usando `temp | ~mask` em vez de `temp & ~mask`, causa raiz da Armadilha #2), `503` (`NOT Dn0` modelado como XOR, Armadilha #9), `367/369` (`EXTB`/`EXTH`) e `427` (`SUBC`).
- **Remoção de `mn_todo()`:** `NOT Rn0` (`am33.sinc:293`), `BTST SIMM, Rn0` (`295-297`, Armadilha #10), `MOV (Rm0+, disp)` (`78-83`), `MOVBU (IMM32, ...)` (`130-133`), `ADDC`/`SUBC` e deslocamentos/rotações `ASR`/`ASL`/`ROR`/`ROL`.
- **Execução em 4 Passes (clone read-only `Panadiga_MN103S_ZeroUndef`):** consulte o roteiro completo em [`tools/ghidra/sleigh_patches/README.md`](../../tools/ghidra/sleigh_patches/README.md).
