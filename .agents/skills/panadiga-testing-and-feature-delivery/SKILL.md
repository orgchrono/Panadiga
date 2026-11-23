---
name: panadiga-testing-and-feature-delivery
description: >-
  Gestão de novas implementações (do requisito ao teste e entrega), criação de testes
  universais nas 4 stacks (Rust #[test], Vitest 5, Playwright 1.65, C11 host e Pytest),
  prevenção de teatro de testes e erradicação de flakiness (eliminação de temp dirs fixos
  sem contador atômico, isolamento de portas/servidores e zero waitForTimeout). Use ao
  implementar nova funcionalidade ou criar/endurecer suítes de teste.
---

# Gestão de Novas Implementações, Criação de Testes & Anti-Flakiness

Esta skill governa o ciclo completo de uma nova funcionalidade — da definição dos contratos à criação de testes determinísticos e não-frágeis em todas as camadas.

## 1. Quando Usar & Handoffs
- Ative ao planejar e implementar nova feature, corrigir bug com teste de regressão, ou auditar flakiness/teatro de testes.
- Para os 9 (+1) Quality Gates globais e acoplamento de PR, consulte `.agents/skills/panadiga-validate/SKILL.md`.
- Para regras específicas de linguagem, combine com `.agents/skills/panadiga-rust-backend-dev/SKILL.md` e `.agents/skills/panadiga-typescript-frontend-dev/SKILL.md`.
- Padrões de criação de testes por stack e catálogo anti-flakiness em [testing-patterns-and-anti-flakiness.md](./references/testing-patterns-and-anti-flakiness.md).

## 2. Fluxo Canônico de Entrega de Nova Feature (6 Passos)
1. **Contrato & SSOT Primeiro:** Defina o dado ou capacidade em `profiles/` / Rust `panadiga-core` antes de tocar UI ou scripts; verifique `CapabilityGate` e política fail-closed.
2. **Regeneração Declarativa:** Rode `panadiga ssot export-all` / `ghidra export-all` e alinhe schemas Zod + tipos TS + i18n (`en-US` e `pt-BR`).
3. **Implementação FCIS/MVVM:** Núcleo puro testável sem IO -> cascas finas (CLI/MCP/Tauri/Services/Store/ViewModel/Components).
4. **Testes em Pareamento (Happy Path + Fail-Closed):**
   - **Rust:** testes unitários puros + testes de integração rejeitando entradas inválidas (`E05`, `0xC000_100C`, path traversal, versões inválidas).
   - **Frontend TS:** testes Vitest com mocks exclusivamente em `apps/desktop/src/test/mocks/`, cobrindo estados `loading`, `empty`, `error` e sucesso.
   - **E2E Playwright:** spec em `apps/desktop/e2e/` com seletores semânticos (`getByRole`, `data-testid`), zero `waitForTimeout`.
   - **Pico C11 / Python:** suíte em `firmware/pico-p7501/tests/` (`-DPICO_HOST_TEST=1`) e/ou `scripts/tests/` (`mypy --strict`).
5. **Auditoria Anti-Flakiness:** Confirme ausência de diretórios temporários globais fixos, portas compartilhadas sem isolamento e condições de corrida.
6. **Fechamento nos Gates:** Rode `python scripts/validate.py` e documente se necessário.

## 3. Regras Invioláveis Anti-Flakiness & Anti-Teatro
1. **Diretórios Temporários Exclusivos:** Nunca use `std::env::temp_dir().join("panadiga_fixo_test")` nem apenas `process::id()` em funções concorrentes; inclua contador `AtomicU64` por chamada (ou `TempDir` com cleanup garantido) para evitar colisão entre threads/processos paralelos!
2. **Zero `sleep` / `waitForTimeout`:** Use polling condicional (`expect(...)`, eventos explícitos) e timers nativos apenas quando indispensável.
3. **Sem Teatro de Testes:** Todo teste que alega validar contrato entre Rust e TS/C11/Python deve realmente **ler/parsear ambos os lados** ou executar o executável gerado (nunca apenas checar um texto irrelevante!).
4. **Sem Vazamento de Mocks:** Mocks residem apenas em `apps/desktop/src/test/mocks/` ou atrás de `--mock` explícito em `scripts/bench/runner.py`.

## 4. Verificação
- Novo teste falha quando a lógica ou guarda é quebrada (prova de mutação/negativo), e passa em execuções repetidas em paralelo sem resíduos no `temp_dir()`.
