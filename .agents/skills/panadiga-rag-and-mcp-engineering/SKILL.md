---
name: panadiga-rag-and-mcp-engineering
description: >-
  Uso operacional e engenharia de extensão do motor Zero-Error RAG de 6 camadas
  (crates/panadiga-rag) e do Servidor MCP nativo em Rust (crates/panadiga-mcp, 26 tools,
  9 famílias de resources incluindo panadiga://symbols/<train_id>/<module> e 4 prompts + bridge Ghidra MCP). Use para consultar conhecimento
  indexado com economia de tokens ou para criar novas tools MCP, resources e indexadores RAG.
---

# Engenharia & Uso do Zero-Error RAG e Servidor MCP Nativo

Esta skill cobre tanto a utilização eficiente das **26 tools MCP** (incluindo `panadiga_symbol_lookup`) e do **RAG de 6 camadas** (com indexador e guardrail de símbolos destrutivos) quanto as regras rígidas para expandir ambos com novas capacidades.

## 1. Quando Usar & Handoffs
- Use para buscar documentação, esquemas, manuais PDF, símbolos e código via RAG/MCP ou ao adicionar novas ferramentas/indexadores nos crates `panadiga-rag` e `panadiga-mcp`.
- Combine com `panadiga-rust-backend-dev` ao escrever código Rust e `panadiga-validate` antes de concluir alterações.
- Catálogo completo de tools, resources, prompts e camadas em [rag-mcp-catalog-and-extension.md](./references/rag-mcp-catalog-and-extension.md).

## 2. Invariantes Não-Negociáveis
1. **Framing JSON-RPC Limpo (`stdio`):** `panadiga-mcp` e `panadiga-rag` usam `#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]`. Toda telemetria vai via `tracing` para `stderr`; jamais use `println!`.
2. **Guardrail Físico Inviolável (`crates/panadiga-rag/src/safety/`):** `HardwareSafetyValidator` e `safety/patterns.rs` nunca podem ser enfraquecidos; respostas que envolvam hardware devem respeitar bloqueios de `5V`, `NSW5V`, `SA0..SA10`, `0xC000_100C` e locks de disco.
3. **Nenhuma Lógica de Negócio Duplicada no MCP:** Handlers MCP em `crates/panadiga-mcp/src/tools/` são adaptadores finos (Imperative Shell) que validam JSON de entrada e delegam integralmente a `panadiga-core` / `panadiga-profiles` / `panadiga-rag`.
4. **Limite de Resource & Traversal Guard:** `resources.rs` limita payloads `resources/read` a `32 KiB` e restringe paths relativos em `panadiga://docs/*`; documentos grandes devem orientar o uso de `panadiga_rag_query` ou `panadiga_search_code`.
5. **Paridade de Contagem:** Ao adicionar/remover tools MCP, atualize testes em `panadiga-mcp`, `scripts/tests/test_docs_parity.py` e contadores sincronizados em docs (`README*`, `AGENTS.md`, `.github/RELEASE_NOTES.md`).

## 3. Uso Canônico em Bancada & Pesquisa
```bash
# Status do corpus RAG e consulta direta via CLI
cargo run -q -p panadiga-cli -- rag status --json -
cargo run -q -p panadiga-cli -- rag query "loader ver0.61 bgo 45 setores" --json -

# Inspecionar schemas MCP nativos e readiness do servidor stdio
cargo run -q -p panadiga-cli -- mcp --tools
cargo run -q -p panadiga-cli -- mcp --ping

# Bridges configuradas em .mcp.json:
# python -m scripts.panadiga_cli_bridge mcp
# python -m scripts.panadiga_cli_bridge ghidra-mcp --ghidra-server http://127.0.0.1:8080/
```
- Prefira tools MCP específicas antes de rodar builds ou greps pesados: `panadiga_memory_map`, `panadiga_safety_check`, `panadiga_pico_catalog`, `panadiga_search_manuals`, `panadiga_search_visuals`, `panadiga_search_code`, `panadiga_rag_query`, `panadiga_symbol_lookup`, `panadiga_project_status`.

## 4. Como Criar uma Nova Tool MCP ou Extensão RAG
### Nova Tool MCP (`crates/panadiga-mcp/src/`)
1. Implemente e teste a lógica pura em `panadiga-core` ou `panadiga-profiles` primeiro.
2. Adicione a definição JSON Schema no módulo temático apropriado (`tools/defs_core.rs`, `tools/defs_re.rs` ou `tools/defs_re_ext.rs` — mantenha módulos coesos, sem god-file).
3. Implemente o handler fino no módulo de execução correspondente (`tools/forensics.rs`, `tools/forensics_re*.rs`) reutilizando helpers compartilhados de erro/validação (`jscpd` 0%!).
4. Registre no roteador em `tools/mod.rs` e cubra com testes de schema e dispatch em `panadiga-mcp`.

### Novo Indexador ou Camada RAG (`crates/panadiga-rag/src/`)
1. Pipeline das 6 camadas em `pipeline.rs`: `QueryGuard` (`guardrails.rs`) -> `HierarchicalIndexer` (`indexer/{markdown,pdf,code,symbols,visual,bm25,vector}.rs`) + `HybridRetriever` (`retriever.rs`) -> `RetrievalCritic` (`critic.rs`) -> `GroundedGenerator` (`generator.rs`) -> `FactCritic` (`critic.rs`) -> `HardwareSafetyValidator` (`safety/`).
2. Ao adicionar nova fonte de corpus, adicione indexador dedicado em `src/indexer/`, conecte `ChunkSourceFilter` e cubra com testes determinísticos em `src/tests.rs`.

## 5. Armadilhas & Anti-Padrões
- Esquecer que `panadiga_hdd_flash_inspect` é estritamente dry-run/planner e que `panadiga_eh55_bgo(action="live_apply")` exige `confirm_upper_nor=true` + `backup_dir`.
- Copiar sugestão de comando legado presente no texto de `prompts.rs` sem conferir a CLI real (o subcomando válido é sempre `panadiga pico capture-nor --slice ...`).

## 6. Verificação
```bash
cargo test -p panadiga-rag -p panadiga-mcp
cargo run -q -p panadiga-cli -- mcp --tools
```
Confirme `tools.len()` esperado, zero warnings clippy (`-D warnings`) e `rag query` retornando citações válidas.
