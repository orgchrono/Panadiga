---
name: panadiga-architecture-and-guardrails
description: >-
  Guarda arquitetural do ecossistema: Rust-First Logic, Functional Core Imperative Shell (FCIS),
  MVVM DAG unidirecional, SSOT (profiles/ e ssot export-all), SoC, SRP, DRY (jscpd 0% clones),
  anti-god-modules, anti-regressão, anti-hardcode, anti-flakiness e protocolo de criação ou
  edição de novos módulos. Use ao desenhar refatorações, criar módulos ou revisar acoplamento.
---

# Arquitetura, Functional Core/Imperative Shell & Guardrails Anti-Regressão

Esta skill impede degradação estrutural (god-modules, hardcode de modelos, duplicidade `jscpd`, vazamento de regras de domínio para UI e testes frágeis) durante a evolução do Panadiga e em novos projetos.

## 1. Quando Usar & Handoffs
- Ative antes de criar módulos novos, dividir arquivos grandes, alterar fronteiras entre Rust/TS/C11/Python ou investigar regressão/dívida técnica.
- Para implementação específica por stack, combine com `.agents/skills/panadiga-rust-backend-dev/SKILL.md`, `.agents/skills/panadiga-typescript-frontend-dev/SKILL.md`, `.agents/skills/panadiga-pico-flashing/SKILL.md` ou `.agents/skills/panadiga-rag-and-mcp-engineering/SKILL.md`.
- Árvore de decisão de posicionamento de módulos, DAG MVVM e checklist anti-regressão em [architecture-fcis-mvvm-and-anti-patterns.md](./references/architecture-fcis-mvvm-and-anti-patterns.md).

## 2. Invariantes Arquiteturais Não-Negociáveis
1. **Rust-First + Functional Core, Imperative Shell (FCIS):** Toda regra de negócio, criptografia, MEIHFS, cálculo de `MAIN_LBA`, segurança Spansion e policy `T0..T4` vive em código puro dentro de `panadiga-profiles` e `panadiga-core`. CLI Clap, MCP, Tauri IPC e React são cascas imperativas finas.
2. **Zero Hardcode de Modelo Comercial:** Identifique placas pela tupla **`(model, region, board_rev, flash_jedec_id)`** via SSOT `profiles/` + `panadiga ssot export-all` — nunca escreva `if model == "DMR-EH55"` espalhado em código.
3. **Zero Duplicidade (`jscpd` 0% Clones):** `.jscpd.json` bloqueia `> 0` clones em `apps/desktop/src`, `apps/desktop/src-tauri/src`, `packages/oxlint-plugin-panadiga/src`, `crates`, `firmware/pico-p7501`, `tools` e `scripts`. Extraia helpers compartilhados ao detectar lógica parecida.
4. **Anti-God-Modules & SRP:** Divida módulos por subdomínio (`vault/`, `extrhead/`, `bgo_unbrick/`, `pico/{cli,diag,flash,gdb,uart}/`). Limites front: `complexity <= 22`, `max-depth <= 4`, `import/no-cycle`.
5. **Anti-Hardcode, Anti-Supressão & Zero Mocks em Produção:** Proibido prose fora de `src/locales/`, números mágicos fora de `src/constants/`, `eslint-disable`/`@ts-ignore`/`@ts-expect-error`, e mocks fora de `apps/desktop/src/test/mocks/` ou flags `--mock` explícitas de runner.

## 3. Protocolo Canônico ao Editar ou Criar um Módulo Novo
1. **Localize a Camada Certa:** Consulte [architecture-fcis-mvvm-and-anti-patterns.md](./references/architecture-fcis-mvvm-and-anti-patterns.md) para decidir entre `panadiga-profiles`, `panadiga-core`, `panadiga-cli`/`mcp`/`src-tauri`, camada MVVM de `apps/desktop/src/`, `firmware/pico-p7501/<pasta>/` ou `scripts/`.
2. **Verifique Reuso Antes de Escrever:** Rode `panadiga_search_code` ou `rg -n` para não duplicar parsers, validadores SHA-256 ou constantes já existentes (`jscpd`).
3. **Preserve Contratos Cross-Layer:** Mudança em DTO/constante/schema Rust exige atualizar `ssot export-all`, Zod schemas / tipos TS, mocks de teste e testes de contrato no mesmo pacote de alterações.
4. **Previna Flakiness:** Zero `sleep` fixo em testes, zero dependência de ordem de execução ou rede externa em unit/E2E, timeouts explícitos e cleanup RAII (`TempDir` / `BenchSession`).

## 4. Armadilhas & Anti-Padrões
- Colocar cálculos de setor, offsets Spansion ou validação de porta serial no React/TS.
- Importar `@tauri-apps/*` ou `services/` diretamente dentro de `components/` ou `views/` (`panadiga/mvvm-strict-boundaries`).
- Editar arquivos gerados (`apps/desktop/src/models/generated/profiles.ts`, `firmware/pico-p7501/generated/`, `tools/ghidra/ghidra_ssot_maps.json`) em vez de editar `profiles/` + rodar `panadiga ssot export-all`.

## 5. Daqui pra Frente & Verificação
- **Em novos projetos:** Instale desde o Dia 0 um plugin de fronteira de camadas + detector `jscpd` threshold `0` + separação SSOT JSON -> geradores multi-linguagem.
- **Verificação:** `python scripts/validate.py` Gates 1–5 (`clippy`, `oxlint`, `vitest`, `jscpd 0%`, `uxlint 100/100`) e Gate 7 (`ssot export-all`).
