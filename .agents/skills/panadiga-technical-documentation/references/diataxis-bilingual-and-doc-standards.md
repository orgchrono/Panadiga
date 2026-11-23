# Padrões Diátaxis, Paridade Bilíngue, Mermaid & Anti-Drift

## 1. Mapa Diátaxis das 5 Trilhas (`docs/STATUS.md` & `docs/INDEX.md`)
- **Trilha A — Hardware de Bancada, `P7501`, Pico C11 & Standby `IC7501`** (Guias práticos + esquemáticos + standby)
- **Trilha B — Cura `U99`, NVRAM Spansion & Criptografia `.FRM`** (Teoria criptográfica + forense BGO)
- **Trilha C — Bootloader `MN103S`, Ghidra, GDB & Ferramentas RE** (Referência de armadilhas, scripts e pseudocódigo)
- **Trilha D — Storage `MEIHFS`, `CH1 IDE`/`DAVIS`, Drive `R6-MV4` & IR** (Arquitetura de disco + compêndio comunitário)
- **Trilha E — TBC Autônomo, Arquitetura, Perfis SSOT, Onboarding, Mobile & QA** (ADRs, testes, RAG e roadmap)

## 2. Regras de Diagramas & Formatação
- **Mermaid Suportados:** `flowchart TD/LR`, `graph TD/LR`, `sequenceDiagram`, `stateDiagram-v2`, `classDiagram`, `erDiagram`, `xychart-beta`.
- Coloque labels com parênteses/colchetes entre aspas: `id["S29JL064H (8 MiB)"]`; nunca use `<br>` ou HTML nos nós.
- Escape `$` literal fora de backticks (`\$100` vs `` `$HOME` ``) para não disparar renderização KaTeX acidental.

## 3. Governança Anti-Drift (Checklist para Projetos Grandes)
- **Snapshots Datados Imutáveis:** Auditorias pontuais em `docs/audits/*_YYYY-MM-DD.md` ficam congeladas como registro histórico; documentos vivos (`STATUS.md`, `INDEX.md`, `CONTRIBUTING.md`, `AGENTS.md`, `README*`) devem ser verificados em conjunto por teste automatizado.
- **Badges Epistêmicos Estritos:** `en-US` não pode vazar badges em português (`test_docs_parity.py`); `pt-BR` só usa `[PROVADO EM HARDWARE REAL]` quando houver entrada `N4_HARDWARE`/`N5_REPRODUCED` no `ledger.json`.
- **Auditoria Periódica de Sincronia:** Ao mexer em versão do workspace (`Cargo.toml` / `package.json`), número de tools MCP, número de logs ou dossiês, inclua um passo único para atualizar todos os documentos raiz sincronizados no mesmo commit.
