# Panadiga — Analisadores Java Consolidados para NSA Ghidra (`tools/ghidra/java/`)

Este diretório centraliza e consolida os analistas Java (`.java`) do projeto **Panadiga** para engenharia reversa MN103S / AM34 (`BOOT.BIN ver0.63`, `eh55_frm_PROG061_BOOT.bin ver0.61` e módulos relacionados) no **Ghidra 12.1.4+**, substituindo os 11 scripts avulsos originalmente soltos na raiz `C:\ghidra\*.java`.

---

## 1. Estrutura Consolidada

| Arquivo | Função | Substitui (Legado `C:\ghidra\`) |
| :--- | :--- | :--- |
| [`panadiga/Mn103sAuditSupport.java`](panadiga/Mn103sAuditSupport.java) | Biblioteca compartilhada: catálogos verificados (`BOOT.BIN` e `PROG061`), resolução read-only de funções (`getFunctionContaining`), descarte determinístico `decomp.dispose()`, dump com bytes brutos e detector automático da **Armadilha #8 SLEIGH (`am33.sinc`)** | Boilerplate duplicado dos 10 scripts `Decompile*.java` |
| [`PanadigaBootFatSdDecompiler.java`](PanadigaBootFatSdDecompiler.java) | Decompilador consolidado por estágio/grupo/endereço para toda a pilha FAT12/16/32, drivers SD (`CMD17`/`CMD18`/OCR/RCA) e `sd_load.o` (`PROG061`), com alerta inline `fb 9c` / `fb|fd|fe 98` | `DecompileFat.java` .. `DecompileFat8.java`, `DecompileSdRead.java`, `DecompileCardType.java` |
| [`PanadigaMn103sDisasmDump.java`](PanadigaMn103sDisasmDump.java) | Dumper de disassembly MN103S por faixa (`START..END` ou presets), exibindo opcodes brutos, fallback `<undef db 0xXX>`, recuperação de início desalinhado (`getInstructionContaining`) e flags Trap #8 | `DumpAsm.java` |
| [`PanadigaMn103sTrapAudit.java`](PanadigaMn103sTrapAudit.java) | Auditor estrutural (catálogo ou toda a memória executável `--scope=exec`) de anomalias `SUB` no `am33.sinc` (linhas 184 e 186–188) e alinhamento das sondas históricas | *Novo analista* |

---

## 2. Bugs & Duplicações Corrigidos na Consolidação

1. **Fim de 10 clones idênticos:** todo o setup `DecompInterface`, resolução e impressão foi unificado em [`panadiga/Mn103sAuditSupport.java`](panadiga/Mn103sAuditSupport.java).
2. **Duplicação removida:** `0x4400dc64` (`boot_fat_read_part_sector`) aparecia repetido entre `DecompileFat5.java` e `DecompileFat6.java`.
3. **Sondas mid-function/mid-instruction tratadas sem `NULL` silencioso:**
   - `0x4400d335` (`DecompileFat.java`) cai `+3 B` dentro do prólogo de `FUN_4400d32f`; agora resolve automaticamente para `FUN_4400d32f (+0x6)`.
   - `0x4400c436` (`DecompileFat3.java`) cai `+1 B` dentro de `[f8 03 0c] MOV (0xc,A3),D0` no helper `0x4400c35a..0x4400c50f`; agora é reportado com contexto exato.
   - `0x4400c760` (`DumpAsm.java`) retornava **0 linhas** por cair `+1 B` dentro de `[f8 11 18] MOV D0,(0x18,A1)` em `0x4400c75f`; `PanadigaMn103sDisasmDump` recua via `getInstructionContaining(start)` e avisa no log.
4. **Segurança `-readOnly` & limpeza de recursos:** `decomp.dispose()` garantido em `finally`, e criação de funções (`--allow-create`) é opt-in (`false` por padrão) para evitar `ReadOnlyException` ou mutações acidentais em `Panadiga_MN103S`.
5. **Detecção automática da Armadilha #8 SLEIGH (`am33.sinc`):** identifica `fb 9c` (`SUB Rm2,Rn0,Rd2` com operandos invertidos em `am33.sinc:184`) e `fb/fd/fe 98` (`SUB SIMM,Rn0` usando `update_add` em `am33.sinc:186-188`).

---

## 3. Uso em Ghidra Headless (Read-Only, sem `-save`)

> **Regra do repositório:** Nunca grave alterações persistentes no projeto `Panadiga_MN103S` durante auditorias headless (`-noanalysis -readOnly`, sem `-save`).

```bat
REM 1. Auditar todo o stack FAT/SD de BOOT.BIN (com detecção de Trap #8):
C:\ghidra\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat C:\ghidra\projects Panadiga_MN103S ^
  -process "BOOT.BIN" -noanalysis -readOnly ^
  -scriptPath "C:\Users\danie\Documents\Panadiga\tools\ghidra\java" ^
  -postScript PanadigaBootFatSdDecompiler.java

REM 2. Filtrar apenas o driver SD ou endereços específicos:
C:\ghidra\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat C:\ghidra\projects Panadiga_MN103S ^
  -process "BOOT.BIN" -noanalysis -readOnly ^
  -scriptPath "C:\Users\danie\Documents\Panadiga\tools\ghidra\java" ^
  -postScript PanadigaBootFatSdDecompiler.java "--group=sd-driver"

REM 3. Dumpar disassembly alinhado + bytes brutos em BOOT.BIN (ex.: janela legada 4400c760..4400c7b0):
C:\ghidra\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat C:\ghidra\projects Panadiga_MN103S ^
  -process "BOOT.BIN" -noanalysis -readOnly ^
  -scriptPath "C:\Users\danie\Documents\Panadiga\tools\ghidra\java" ^
  -postScript PanadigaMn103sDisasmDump.java 4400c760 4400c7b0

REM 4. Auditoria global de Trap #8 SLEIGH SUB em toda a memória executável de BOOT.BIN:
C:\ghidra\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat C:\ghidra\projects Panadiga_MN103S ^
  -process "BOOT.BIN" -noanalysis -readOnly ^
  -scriptPath "C:\Users\danie\Documents\Panadiga\tools\ghidra\java" ^
  -postScript PanadigaMn103sTrapAudit.java "--scope=exec"

REM 5. Auditar PROG061 ver0.61 (sd_load.o + Update-Disc):
C:\ghidra\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat C:\ghidra\projects Panadiga_MN103S ^
  -process "eh55_frm_PROG061_BOOT.bin" -noanalysis -readOnly ^
  -scriptPath "C:\Users\danie\Documents\Panadiga\tools\ghidra\java" ^
  -postScript PanadigaBootFatSdDecompiler.java
```
