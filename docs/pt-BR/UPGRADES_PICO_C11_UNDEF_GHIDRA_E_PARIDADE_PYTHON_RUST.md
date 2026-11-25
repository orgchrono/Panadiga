> 🌐 **Idioma / Language:** [English (US)](../en-US/PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# Upgrades C11 no Pico `P7501`, Causas de `??` no Ghidra e Matriz de Paridade Python → Rust

> **Documentos Relacionados:** [`AUDITORIA_COMPLETA.md`](../../AUDITORIA_COMPLETA.md) · [`GHIDRA_UNDEF_AUDIT_REPORT.md`](../pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md) · [`ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md`](../pt-BR/ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md)

---

## 1. Por que Aparecem Trechos com `??` (`Undefined`) no Ghidra MN103S

A auditoria exaustiva (somente leitura) sobre os **5 binários** em `C:\ghidra\projects\Panadiga_MN103S` provou que os trechos `??` **não são erros de dump nem falhas de comunicação**. Todos os intervalos `Undefined` pertencem a três classes estruturais bem definidas:

| Binário Ghidra | Bloco Inicializado | Instruções | Bytes `??` (`Undef`) | Total de Gaps | Causas Dominantes Observadas |
| :--- | ---: | ---: | ---: | ---: | :--- |
| `/BOOT.BIN` | `262.144 B` | `178.876 B` | `67.100 B` (`25,60%`) | `170` | Pools/unimpl (`41.508 B` / `27` gaps), ilhas limpas (`22.910 B` / `93` gaps), strings (`2.559 B` / `29` gaps), tabela de ponteiros (`24 B` @ `0x44035EB0`) |
| `/eh55_frm_PROG061_BOOT.bin` | `262.144 B` | `666 B` | `261.478 B` (`99,75%`) | `9` | Carregado por ponto de entrada único; `6` gaps >=64 B com prólogos `0xCF`/`0xF8 0xFE` (maior gap `0x50406DAA`: `157.394 B`) |
| `/EH55_DRV_S211.bin` | `1.441.792 B` | `7.613 B` | `1.434.179 B` (`99,47%`) | `26` | Ilhas de código limpas (`780.547 B` / `19` gaps) + literal pools/unimpl (`653.632 B` / `7` gaps) |
| `/DRV_decomp.bin` | `1.441.792 B` | `727.726 B` | `666.076 B` (`46,20%`) | `1.957` | Pools/unimpl (`417.487 B` / `341`), ilhas (`153.222 B` / `292`), strings (`84.014 B` / `860`), ponteiros (`2.543 B` / `34`) |
| `/EH55_MAIN_3410PQ.bin` | `13.003.052 B` | `825.478 B` | `12.177.574 B` (`93,65%`) | `1.986` | Pools/unimpl (`8.168.656 B` / `136`), ilhas limpas (`4.004.135 B` / `1.706`; `1.399` >=64 B com prólogos), `3.935` falhas iniciadas por `0xF9` |

Evidências exatas do módulo Sleigh MN103S / AM33 capturadas nos gaps:
- **Opcodes `AM33` ausentes no Sleigh padrão:** `0xF9 0xEA <reg>` (`MOVHU (Rm0+), Rn2`, ex.: `0x44011553`, `0x50416355`, `0x5700A836`), `0xF9 0xFA <reg>`, `0xFF 0xF0 0xFC` (`0x504009E4`, `0x57000D2C`) e `0xF0 0xC0` (`0x504313E6`).
- **Armadilha 4 (saltos em meio de instrução):** desvios que compartilham o byte `+1` de uma instrução multi-byte vizinha deixam o byte sobreposto marcado como `??`.
- **Ilhas limpas sem xref direta:** tratadores atrás de tabelas `<8sIII>` e CDBs ATAPI ficam ocultos até serem mapeados por `panadiga re audit` / `panadiga ghidra export-all`.

---

## 2. Upgrades C11 no Firmware Pico `P7501` e UF2s Recompilados

Os módulos em [`firmware/picofw/`](../../firmware/picofw/) (ISO C11 estrito com `-Wall -Wextra -Werror`, todos os arquivos de produção <= `339` linhas) foram recompilados para hardware real (`RP2040` e `RP2350`) via `python scripts/build.py --pico --no-cache` e incorporados em `crates/panadiga-core/src/pico/firmware_bins/`:
- **UF2 Unificado (`panasonic_p7501_uart_unified.uf2`):** `522.752 B` (`SHA-256: 2d568f7f4c1d3b12c83721eef4a5666a59666ead5fe8ee7359915e45962aa405`)
- **UF2 RP2040 (`panasonic_p7501_uart_rp2040.uf2`):** `268.288 B` (`SHA-256: 0f216f39a5eb863eee4a93b7a957bea872f467cc0c050c7d897a33a73d50e0d2`)
- **UF2 RP2350 (`panasonic_p7501_uart_rp2350.uf2`):** `254.464 B` (`SHA-256: c0cd3f9bd90401ea2699cfeeb2e8c067608c0db489ec4d724cf550ac74dab99d`)

Principais evoluções validadas nos testes de host (`firmware/picofw/tests/test_pana_diag_drv.c`):
1. **`DIAGWATCH [ms] [NORMAL|JIG[:ms]|NOPULL] <ação> [RELEASE|HOLD]`:** suporta payloads parametrizados no prompt `PANAMON1>`: `READ[:a[,n]]`, `WRITE:<a,v>`, `JUMP:<a>` e `ERASE:<a_nvram>`, além de `AUTO`/`BYPASS`/`FORMAT`/`DUMP`/`QUIT`/`NONE`. Travas compartilhadas impedem gravação na Flash IPL Spansion (`0x80000000..=0x807EFFFF`, `0x84000000..=0x847EFFFF` → `E05`) e restringem `ERASE` estritamente aos `64 KB` de NVRAM (`0x807F0000..=0x807FFFFF`, `0x847F0000..=0x847FFFFF`).
2. **`PANARSP REGS`:** decodifica 12 registradores MN103S de 32 bits (`96` caracteres hex LE32) via `p7501_rsp_decode_mn103_regs`.
3. **`DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd>`:** envia comandos de fábrica do drive `Matshita R6-MV4` com desbloqueio automático opcional via `PASS 0x4d415453 0x48495441` (ou sem senha com `NOPASS`).
4. **Exposição no App Desktop:** os novos diretórios `docs/{audits,pseudocode,debug_logs}` ficam disponíveis diretamente no painel de Documentação de Engenharia (`ManualsView` → [`EngineeringDocsPanel.tsx`](../../apps/desktop/src/components/manuals/EngineeringDocsPanel.tsx)) e indexados pelo motor `panadiga-rag`.

---

## 3. Matriz de Auditoria: Conversão Python → Rust

| Módulo(s) Python | Situação Atual | Equivalente / Destino em Rust | Decisão Técnica |
| :--- | :--- | :--- | :--- |
| `scripts/re/mn10300.py`, `scripts/re/mn103_traps.py` | **Paridade Rust Entregue** | `crates/panadiga-core/src/re/{mn10300,mn10300_traps}` + `panadiga re {audit, xrefs, drv-cli, drv-errors}` | Análise estática das 7 Armadilhas, opcodes `AM33`, xrefs e tabelas `DRV` já roda nativamente em Rust; Python mantido só para paridade em `scripts/tests/test_re_tools.py`. |
| `scripts/re/pana_frm.py` | **Substituído no Core** | `crates/panadiga-crypto/src/` (`panadiga frm`) + `panadiga re drv-cli` | Cifra Feistel/`EXTRHEAD`/`.FRM` e tabela `<8sIII>` do drive já possuem implementação completa em Rust. |
| `scripts/bench/decoders.py` | **Paridade Rust Entregue** | `crates/panadiga-core/src/pico/log_decoder/` (`panadiga pico decode-log`) + `apps/desktop/src-tauri/src/pico/diagnostics/parser.rs` | Decodificação offline de logs UTF-16LE/UTF-8 (`COM10`), erros `SDCD` e pacotes SCSI `SENSE` migrada para a CLI Rust (`panadiga pico decode-log`). |
| `scripts/bench/{nor_dumper,rsp_frames,rsp_dump}.py` | **Substituído no Core/Desktop** | `panadiga-core/src/re/nor_dumper.rs`, `src-tauri/src/pico/rsp_protocol.rs`, ferramenta MCP `panadiga_nor_dumper_stub` | Lógica de stubs NOR e pacotes GDB RSP já opera em Rust no backend Tauri e na CLI/MCP; cópias Python apoiam mocks de `pytest`. |
| `scripts/bench/{protocol,boot_harness,runner}.py` | **Manter Opcional (Bancada)** | `apps/desktop/src-tauri/src/pico/serial/` & `panadiga pico` | Útil para testes rápidos isolados via `pyserial` na `COM10` sem recompilar binários Rust (`test_boot_harness.py`). |
| `tools/ghidra/*.py` (7 scripts + `java/` 4 fontes) | **DEVE MANTER PYTHON/JAVA** | Scripts Jython gerados do SSOT Rust via `panadiga ghidra export-all`; suíte Headless 12.1.4 em `tools/ghidra/java/` | Executam dentro da JVM do Ghidra (`Jython 2.7` / Java) chamando `currentProgram`, `FlatProgramAPI` e `DecompInterface`. Rust permanece SSOT para o mapa AM34 e os scripts `.py`. |
| `scripts/build.py`, `validate.py`, `panadiga_ops.py`, `panadiga_cache.py`, `build_cache/*`, `build_lanes/*`, `tui.py` | **DEVE MANTER PYTHON** | Nenhum (orquestrador externo) | Gerencia os 9 Quality Gates e o build DAG de Cargo, Tauri, Ninja/ARM GCC, Gradle Android, `tsc`, Vitest, `jscpd`, `uxlint`, Playwright, `mypy --strict` e `pytest`. |
| `scripts/dev/{cad_primitives,schematic_sheet,generate_schematics_and_cad}.py` | **DEVE MANTER PYTHON** | Nenhum (gerador Matplotlib IEEE/IEC) | Geração offline de desenhos elétricos e CAD `300 DPI` em PNG. |
| `scripts/tests/*.py` | **DEVE MANTER PYTHON** | Coberto por `cargo test` no lado Rust | Validação `mypy --strict` + `pytest` (Gate 9) dos scripts auxiliares e de build. |
