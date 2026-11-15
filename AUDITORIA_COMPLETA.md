# Auditoria Técnica Consolidada — Ecossistema Panadiga (`DMR-EH55`)

> 🌐 **Idioma / Language:** **Português (Brasil)** | [English (`COMPLETE_AUDIT.md`)](./COMPLETE_AUDIT.md) · [Portal `docs/INDEX.md`](./docs/INDEX.md) · [Auditorias Históricas (`docs/audits/`)](./docs/audits/README.md) · [Pseudocódigo Ghidra (`docs/pseudocode/`)](./docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) · [Logs `COM10` (`docs/debug_logs/`)](./docs/debug_logs/README.md)

> **Documento Vivo na Raiz do Repositório** · As auditorias históricas pré-refatoração de `2026-11-11` foram preservadas em [`docs/audits/`](./docs/audits/README.md).  
> **Escopo:** Arquitetura modular atual (`crates/`, `firmware/pico-p7501/`, `apps/desktop/`, `scripts/`), upgrades do firmware Pico `P7501`, diagnóstico completo de `??` (`Undefined`) no Ghidra, exposição de pseudocódigo/logs de bancada, paridade Python → Rust e invariantes físicos/criptográficos.

---

## 1. Mapa Arquitetural Atual do Workspace

O repositório opera como um workspace Rust unificado ([`Cargo.toml`](./Cargo.toml), edição `2024`) integrado a firmware bare-metal C11/PIO (`RP2040`/`RP2350`), frontend Desktop/Android Tauri 2.0 (`React + TypeScript`) e automação de build/validação/desenhos técnicos em Python:

| Camada / Crate | Caminho | Responsabilidade Principal | Maior Arquivo Atual |
| :--- | :--- | :--- | :--- |
| **Core Nativo** | [`crates/panadiga-core/`](./crates/panadiga-core/) | Forense MEIHFS, NVRAM `EEMUL`, cripto Feistel/TOC/`EXTRHEAD`, geradores Ghidra, SSOT Pico, RE `mn10300`/`mn10300_traps`/`mn10300_emu`, emulador `matshita_emu`, `log_decoder` | `atapi.rs` (`457 L`) |
| **Motor RAG Zero-Error** | [`crates/panadiga-rag/`](./crates/panadiga-rag/) | Indexação recursiva de `docs/` (BM25 + vetores), `QueryGuard`, `FactCritic`, `HardwareSafetyValidator` (fail-closed IPL / sobretensão / `JTAG`) | `safety.rs` (`376 L`) |
| **Servidor MCP (`stdio`)** | [`crates/panadiga-mcp/`](./crates/panadiga-mcp/) | JSON-RPC 2.0 com **18 ferramentas nativas** (`panadiga_*`) para agentes de IA e auditoria offline/bancada | `defs_re.rs` (`242 L`) |
| **CLI Nativa (`panadiga`)** | [`crates/panadiga-cli/`](./crates/panadiga-cli/) | Superfície unificada (`nvram`, `hdd`, `frm`, `headers`, `scan`, `manuals`, `pico`, `ghidra`, `rag`, `memory-map`, `verify-artifact`, `gdb-bridge`, `mcp`, `unbrick`, `re`, `desktop`/`interactive`) | `cli_tests.rs` (`388 L`) |
| **Desktop / Tauri Backend** | [`apps/desktop/src-tauri/`](./apps/desktop/src-tauri/) | Comandos IPC (`crypto_manuals`, `hardware`, `storage`, `frm_pack`, `unbrick`), transporte serial/RSP e parser diagnóstico | `hardware.rs` (`403 L`) |
| **Frontend React / TS** | [`apps/desktop/src/`](./apps/desktop/src/) | Views MVVM estritas, i18n (`en-US`/`pt-BR`), visualizador de docs em `docs/`, `Centro Unbrick`, `Bancada Pico` | `mockUnbrickHandlers.ts` (`427 L`) / `usePicoTransport.ts` (`359 L` prod) |
| **Firmware Pico C11/PIO** | [`firmware/pico-p7501/`](./firmware/pico-p7501/) | Sondagem `P7501` via `PIO0`/`PIO1`, GDB RSP, `DIAGWATCH`, `PANARSP`, `DRVPANA`, `SDSNIFF` + 4 suítes de teste em host | `p7501_diag_watch_codec.c` (`316 L` prod) / `test_pana_logic.c` (`366 L` test) |
| **Scripts & Geradores** | [`scripts/`](./scripts/) & [`tools/ghidra/`](./tools/ghidra/) | Orquestrador CI 9 Gates (`validate.py`), build multi-alvo (`build.py`), desenhos CAD IEEE/IEC (`scripts/dev/`), Jython Ghidra (`tools/ghidra/`) | `tui.py` (`449 L`) / `ghidra_mn103_fix_decompilation.py` (`743 L`) |

### Consolidação dos Achados Históricos (`2026-11-11`)

As auditorias originais ([`docs/audits/`](./docs/audits/README.md)) apontavam monólitos (`> 1000 L` em Rust/TS/C), `Cargo.toml.bak`, divergência `dwire32a_jtag_research`, ausência do ID de família `0xE48BFF56` no merger UF2 e classificação falsa de falha em logs de boot (`BGO driver init failed[1]`, `-91006`). **Todos esses pontos estruturais foram corrigidos e encerrados:**
- Nenhum módulo de produção Rust, C11 ou TypeScript ultrapassa `457` linhas;
- Zero clones de código (`threshold: 0` em [`.jscpd.json`](./.jscpd.json));
- Classificador diagnóstico e timeouts (`apps/desktop/src-tauri/src/pico/diagnostics/`) endurecidos contra logs legítimos da Bootrom e janelas longas;
- Todos os codecs críticos C11 (`p7501_diag_watch_codec.c`, `p7501_drv_pana_codec.c`, `p7501_pana_rsp_parse.c`, `p7501_sds_codec.c`, etc.) possuem testes unitários nativos compilados no host com `-Wall -Wextra -Werror`.

---

## 2. Firmware Pico `P7501` (C11 / PIO) & Binários UF2 Recompilados

Os binários embarcados em [`crates/panadiga-core/src/pico/firmware_bins/`](./crates/panadiga-core/src/pico/firmware_bins/) foram recompilados para hardware real (`RP2040` e `RP2350`) via `python scripts/build.py --pico --no-cache` com **ARM GNU Toolchain (`14.2 rel1`)**:

| Artefato UF2 | Tamanho | SHA-256 Verificado |
| :--- | ---: | :--- |
| `panasonic_p7501_uart_rp2040.uf2` | `330.752 B` | `70d578710a79ec5ca27ee1dd1dc23893ceccd30cacfed75e6a0bf0a00c771435` |
| `panasonic_p7501_uart_rp2350.uf2` | `315.392 B` | `73b0ebf7fb0eb3337578e40f0b0c9764323283e8e88c3454fcba1a1e6b27b29f` |
| `panasonic_p7501_uart_unified.uf2` | `646.144 B` | `d97dd9600766adff2f3692b5add5ec533e41f5379e67eb58296d7918f8f4334d` |

### Recursos Ativos na Sonda (`p7501_cli_help.c`)
- **Monitoramento e Desbloqueio `DIAGWATCH`:** agora aceita ações simples (`AUTO`, `BYPASS`, `FORMAT`, `DUMP`, `QUIT`, `NONE`) e **ações parametrizadas** (`READ[:a[,n]]`, `WRITE:<a,v>`, `JUMP:<a>`, `ERASE:<a_nvram>`) mantendo travas **fail-closed (`E05`)**:
  - Escritas/apagamentos fora da RAM/NVRAM ou dentro da Spansion IPL (`0x80000000..=0x807EFFFF` e `0x84000000..=0x847EFFFF`) são rejeitados em tempo de parsing (`p7501_flash_is_protected_ipl_range` / `p7501_flash_is_nvram_microsector_addr`).
- **Debugger `_dev_SERIAL` (`PANARSP`):** suporta `REGS` (decodificação de 12 registradores MN103S em 96 chars hex little-endian em `p7501_rsp_decode_mn103_regs`), `MEM`, `WMEM`, `DUMPNOR` e `CALL`.
- **Console de Fábrica do Drive Óptico (`DRVPANA`):** executa comandos do front-end `Matshita R6-MV4` com envio automático opcional de `PASS 0x4d415453 0x48495441` (ou `NOPASS`).
- **Analisadores de Temporização & SD (`PIO0`/`PIO1`):** `SDSNIFF` passivo em `GP10..GP15` (CRC16/lane), `JIGSWEEP`, `PIOLOGIC`, `BOOTTRACE`, `PWRTRACE`, `WAITTX`, `BAUDSCAN` e `HEXCAP`.

---

## 3. Disassembly Ghidra MN103S, Diagnóstico de `??` e Exposição de Artefatos

Toda a evidência extraída em modo estritamente **somente leitura** do projeto `C:\ghidra\projects\Panadiga_MN103S` agora está versionada no repositório:

- **Pseudocódigo C e Disassembly Integral:** [`docs/pseudocode/EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`](./docs/pseudocode/EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) (`488.538 B`, `13.677` linhas, `SHA-256: 050e29aed745db3b...`) e [`docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md`](./docs/pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) + dumps brutos [`docs/pseudocode/raw/`](./docs/pseudocode/raw/).
- **Relatório Exaustivo de `??` (`Undefined`):** [`docs/pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md`](./docs/pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md) comprova que os trechos `??` não são falha de leitura do dump, e sim:
  1. **Literal pools / tabelas e strings ainda não tipadas** (`41.508 B` em `/BOOT.BIN`, `417.487 B` em `/DRV_decomp.bin`, `8.168.656 B` em `/EH55_MAIN_3410PQ.bin`);
  2. **Ilhas de código válidas com prólogo `0xCF`/`0xF8 0xFE` sem xref direta** atrás de tabelas `<8sIII>` e switch/CDB ATAPI (`93` ilhas / `22.910 B` em `BOOT.BIN`, `1.706` ilhas / `4.004.135 B` em `MAIN`);
  3. **Armadilha 4 (saltos em meio de instrução multi-byte)** e **opcodes `AM33` ausentes no Sleigh padrão** (`0xF9 0xEA <reg>` `MOVHU (Rm0+), Rn2` — `3.935` ocorrências `0xF9` em `MAIN` — além de `0xFF 0xF0 0xFC`, `0xF9 0xFA` e fronteiras como `0x4400493B` `dd 0e fe ff ff`).
- **Logs Reais de Bancada (`COM10` / `P7501`):** [`docs/debug_logs/`](./docs/debug_logs/README.md) concentra `DOSSIE_COMPLETO_EH55_UNBRICK_E_FIRMWARE.txt` e as **12 capturas únicas `.log`** (eliminando duplicatas idênticas em disco), decodificáveis via `panadiga pico decode-log`.

---

## 4. Auditoria de Conversão Python → Rust (`panadiga re`, `decode-log` & Matriz Completa)

O núcleo binário, criptográfico, forense e analítico foi consolidado em Rust, restando em Python apenas automação externa, Jython de JVM e desenho CAD. A análise detalhada encontra-se nos dossiês bilíngues:
- 🇧🇷 [`docs/pt-BR/UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md`](./docs/pt-BR/UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md)
- 🇺🇸 [`docs/en-US/PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md`](./docs/en-US/PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md)

Resumo das entregas recentes em Rust:
- **`panadiga re {audit, xrefs, drv-cli, drv-errors}`** ([`crates/panadiga-core/src/re/`](./crates/panadiga-core/src/re/) & [`crates/panadiga-cli/src/cli/handlers/re.rs`](./crates/panadiga-cli/src/cli/handlers/re.rs)): paridade CLI nativa para auditoria das 7 Armadilhas + lacunas Sleigh `AM33`, xrefs MN103S, tabela `<8sIII>` com guarda `PASS` e dicionário de erros `0x34` Shift-JIS.
- **`panadiga pico decode-log`** ([`crates/panadiga-core/src/pico/log_decoder.rs`](./crates/panadiga-core/src/pico/log_decoder.rs)): decodificação offline de capturas UTF-16LE (com ou sem BOM) / UTF-8, modos de boot, `SD = 0x...[LBA]`, erros `SDCD` (`-91006`, `-90002`, `-90001`, `0`), pacotes SCSI/ATAPI `SENSE[...]` (`0x83A7`) e diagnósticos automáticos.

---

## 5. Invariantes de Segurança Física & Verificação de Qualidade

1. **Segurança Elétrica (`VEP79138` / `P7501`):** `GP3` (`NSW5V`) sempre em **Hi-Z Input**; teto lógico `<= 3.35 V`; `GP5` (`XRESET`) estritamente dreno aberto (`0 V` / `Hi-Z`); `SW_UART` (`GP26`) em `ADC0 Hi-Z` fora de janelas controladas `JIG:<ms>`.
2. **Proteção de Bootloader NOR (`IC5001` Spansion):** Faixas IPL `0x80000000..=0x807EFFFF` e `0x84000000..=0x847EFFFF` bloqueadas em gravação em todas as camadas (firmware C11 `E05`, core Rust, servidor MCP e RAG `HardwareSafetyValidator`).
3. **Integridade do Banco Ghidra:** `C:\ghidra\projects\Panadiga_MN103S` nunca é alterado pelos testes, CLI ou validação CI.
4. **Pipeline de 9 Quality Gates ([`scripts/validate.py`](./scripts/validate.py)):** Rust (`fmt`/`check`/`test`), TypeScript (`tsc --noEmit`), Vitest, `jscpd` (`0%` clones), `uxlint`, Playwright E2E, exportação Jython Ghidra (`panadiga ghidra export-all`), testes host C11 (`-Wall -Wextra -Werror` / `/W4 /WX`) e Python (`py_compile`, `mypy --strict`, `pytest`).
