---
name: panadiga-ghidra-workflow
description: >-
  Exporta o mapa de memória oficial AM34, sincroniza símbolos reversos e executa análises MN103S / Ghidra para o firmware Panasonic.
  Use esta skill ao iniciar sessões de engenharia reversa no Ghidra, atualizar símbolos, auditar as Armadilhas MN103S ou usar ferramentas CLI `panadiga re`.
---

# Fluxo de Trabalho Ghidra & Auditoria Estática AM34 / MN103S

O processador Matsushita **AM34** (`MN103S` / UniPhier) exige o mapa físico exato de memória, correção das **8 Armadilhas MN103S** (incluindo inversão `SUB` `fb 9c` do `am33.sinc` SLEIGH) e sincronização de símbolos gerados pela SSOT Rust.

## 1. Exportação dos Scripts via Rust SSOT
Na raiz do repositório:
```bash
cargo run -p panadiga-cli -- ghidra export-all --dir tools/ghidra
```
Isso gera/sincroniza os scripts Jython em `tools/ghidra/`:
- `ghidra_am34_memory_map.py`: Regiões físicas AM34 (`SRAM`, `SDRAM`, `Uncached SDRAM 0x9000_0000`, `Flash IC5001`, `MMIO AIOF/AVDD/DAVIS/IIC`, `OCD`).
- `ghidra_import_symbols_panadiga.py` / `ghidra_importar_simbolos_panadiga.py`: Pontos de entrada, vetores e funções identificadas.
- `ghidra_auto_unbrick_audit.py`: Varredura automatizada de estágios e sítios de patch BGO/TBC.
- `ghidra_mn103_fix_decompilation.py`: Recuperação de frames `CALL`, stack purge e switch tables no Ghidra.
- `ghidra_connect_debugger.py` / `ghidra_conectar_debugger.py`: Ponte de depuração GDB RSP via Pico (`127.0.0.1:3333`).

Scripts Java headless adicionais vivem em `tools/ghidra/java/`:
- `PanadigaBootFatSdDecompiler.java`, `PanadigaMn103sDisasmDump.java`, `PanadigaMn103sTrapAudit.java` e `panadiga/Mn103sAuditSupport.java`.

## 2. Auditoria Estática Nativa em Rust (`panadiga re`, Sem GUI)
```bash
# Auditoria Seven Traps + AM33 SLEIGH + heurísticas em BOOT.BIN / PROG061 / MAIN:
cargo run -p panadiga-cli -- re audit --input <BIN> --base 0x44000000

# Cross-references de chamada/ponteiro, DRV CLI e tabelas de erro:
cargo run -p panadiga-cli -- re xrefs --input <BIN> --target <HEX_VA>
cargo run -p panadiga-cli -- re drv-cli --input <DRV_DECOMP>
cargo run -p panadiga-cli -- re drv-errors --input <DRV_DECOMP>

# Suíte determinística end-to-end (BGO, TBC E-E, Região/PAL-M, HDD >200GB, PANAMON1 via Mn103Vm):
cargo run -p panadiga-cli -- re verify-patches
```

## 3. Descoberta Multi-SO do Ghidra & Execução
- **Variável universal**: defina `GHIDRA_HOME` apontando para o diretório raiz da instalação do Ghidra.
- **Windows**: descoberta automática em `%GHIDRA_HOME%`, `C:\ghidra\ghidra_*`, `C:\tools\ghidra`, `D:\ghidra`, `C:\Program Files\ghidra` e `%USERPROFILE%\ghidra` (`ghidraRun.bat` / `analyzeHeadless.bat`, ou `tools/iniciar_ghidra.cmd`).
- **Linux**: `/opt`, `/opt/ghidra`, `/usr/share`, `/usr/local/share`, `/usr/local/share/ghidra`, `~/ghidra` (`ghidraRun` ou `PATH`).
- **macOS**: `/Applications`, `~/ghidra` (`ghidraRun`) e sincronização automática para `~/ghidra_scripts`.
- No Ghidra: abra o binário com arquitetura `MN10300` / `AM33/AM34`, adicione `tools/ghidra/` em **Window -> Script Manager** e execute `ghidra_am34_memory_map.py` antes de `ghidra_import_symbols_panadiga.py` e `ghidra_mn103_fix_decompilation.py`.
