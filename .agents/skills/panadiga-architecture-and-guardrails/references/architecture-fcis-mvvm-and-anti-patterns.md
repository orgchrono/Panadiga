# Arquitetura FCIS, DAG MVVM, Anti-God-Modules & Guia de Criação de Módulos

## 1. Onde Colocar Cada Peça Nova (Árvore de Decisão)

| O que você precisa criar/alterar | Destino Arquitetural Exato | O que NUNCA fazer |
| :--- | :--- | :--- |
| Novo hardware/modelo/placa/flash/EEPROM/layout | `profiles/{soc_cores,socs,flash_chips,eeprom_chips,firmware_trains,nor_layouts,media_formats,boards,models}/*.json` + `panadiga ssot export-all` | Hardcode de IDs/modelos em Rust, TS ou C11 |
| Regra pura de parsing, crypto, MEIHFS, BGO, NOR, RE | `crates/panadiga-core/src/<domínio>/` (módulos coesos, sem IO de UI) | Colocar lógica de domínio em `apps/desktop/src/` ou handlers CLI/MCP |
| Subcomando CLI / Tool MCP / Comando Tauri IPC | Casca fina em `panadiga-cli` / `panadiga-mcp` / `apps/desktop/src-tauri` delegando a `panadiga-core` | Duplicar lógica entre CLI, MCP e `src-tauri` (`jscpd`!) |
| Nova camada/indexador ou regra RAG | `crates/panadiga-rag/src/{indexer,safety}/` | Enfraquecer `HardwareSafetyValidator` ou misturar logs em `stdout` |
| Driver/diagnóstico da sonda RP2040/RP2350 (`PicoFW`) | `firmware/picofw/{cli,common,diag,flash,gdb,hal,uart,tests}/` | Inchar `cli/`, editar `generated/` à mão ou exceder `256 KiB` `P0_FW_RO` |
| Nova feature no Workbench Desktop/Android | `apps/desktop/src/{models,constants,locales,utils,services,store,hooks,viewmodels,components,views}/` | Saltar camadas do DAG MVVM ou colocar mocks fora de `src/test/mocks/` |
| Pipeline batch RE/Ghidra/bancada/PDF | `scripts/re/`, `scripts/bench/` com `mypy --strict` + testes em `scripts/tests/` | Escrever direto em `profiles/boards/` sem quarentena |

## 2. Anti-God-Modules, SRP & Limites Estruturais
- **Fatie arquivos grandes por responsabilidade única (como feito em `vault/`, `extrhead/`, `bgo_unbrick/`, `pico/`):** um módulo de parser separado de validação, de codec binário e de IO/orchestration.
- **Métricas & Gates:** `jscpd --threshold 0` (zero clones nas 7 raízes de `.jscpd.json`), `oxlint` (`complexity <= 22`, `max-depth <= 4`, `import/no-cycle`), `mypy --strict scripts`, `cargo clippy -- -D warnings`.

## 3. Prevenção de Regressão, Hardcode, Dívida Técnica & Flakiness
- **Regressão:** Todo bug corrigido (ex.: `SA41 + 0x14`, `SW_UART` invertido, `#B1` no `PANAMON1>`, `BCLR`) ganha um teste determinístico reproduzindo exatamente o cenário de falha.
- **Hardcode:** Strings de UI -> `locales/{en-US,pt-BR}/`; magic numbers -> `constants/`; especificações de hardware -> `profiles/` + `ssot export-all`.
- **Dívida Técnica & Coerência Cross-Layer:** Mudanças em contratos entre Rust, C11, TS e Python devem regenerar `ssot export-all` / `ghidra export-all` e validar todos os 9 Quality Gates no mesmo lote.
- **Flakiness:** Proibido `sleep` fixo em testes, proibido acesso de rede externo em Vitest/Playwright/Pytest, sempre limpar arquivos temporários/portas seriais com `BenchSession`/`TempDir` e aguardar estado explícito (`expect(...)` do Playwright com timeout configurado).
