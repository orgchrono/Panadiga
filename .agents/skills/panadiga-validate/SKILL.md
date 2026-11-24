---
name: panadiga-validate
description: >-
  Executa os 9 (+1 opt-in) Quality Gates do ecossistema Panadiga (Rust 11 crates,
  Oxlint+TypeScript, Vitest, jscpd 0%, uxlint 100/100, Playwright E2E, SSOT + Ghidra export,
  Pico C11 host tests e Python mypy --strict / pytest paridade), governa a entrega de novas
  features do contrato ao teste nas 4 stacks e previne flakiness e teatro de testes. Use ao
  implementar novas features, endurecer suítes ou validar antes de encerrar qualquer tarefa.
---

# Validação Universal, Entrega de Features, Anti-Flakiness & 9 (+1) Quality Gates

Esta skill centraliza a entrega ponta a ponta de novas funcionalidades (do contrato SSOT aos testes determinísticos), erradicação de *flakiness*/teatro de testes e a execução dos portões automatizados de qualidade.

## 1. Quando Usar & Handoffs
- Ative ao planejar/implementar nova feature, corrigir bug com regressão, auditar flakiness/teatro de testes ou concluir mudanças em Rust, TypeScript, C11, Python, `profiles/` ou `docs/`.
- Para bloqueios físicos/AppSec e armadilhas `4 MiB` vs `8 MiB`, combine com `.agents/skills/panadiga-security-and-violation-guards/SKILL.md`, `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md` e `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md`.
- Para regras de linguagem e mapas SSOT/sincronia, combine com `.agents/skills/panadiga-rust-backend-dev/SKILL.md`, `.agents/skills/panadiga-typescript-frontend-dev/SKILL.md`, `.agents/skills/panadiga-device-onboarding/SKILL.md` e `.agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md`.
- Para bancada headless e mocks do Pico, combine com `.agents/skills/panadiga-bench-automation-and-headless/SKILL.md`.
- **Referências:**
  - Matriz dos 9+1 gates, pirâmide de testes universais e acoplamento em [universal-testing-and-gates-matrix.md](./references/universal-testing-and-gates-matrix.md).
  - Padrões por stack e catálogo anti-flakiness em [testing-patterns-and-anti-flakiness.md](./references/testing-patterns-and-anti-flakiness.md).

## 2. Fluxo de Entrega de Nova Feature & Regras Anti-Flakiness / Anti-Teatro
1. **Contrato & SSOT Primeiro:** Defina o dado/capacidade em `profiles/` ou `panadiga-core` (`CapabilityGate` fail-closed), rode `panadiga ssot export-all` / `ghidra export-all` e alinhe Zod + TS + i18n (`en-US` e `pt-BR`).
2. **Implementação FCIS/MVVM & Pareamento (Happy Path + Fail-Closed):**
   - **Rust:** testes unitários puros + integração rejeitando entradas inválidas (`E05`, `0xC000_100C`, path traversal, versões inválidas).
   - **Frontend TS & E2E:** Vitest cobrindo `loading`/`empty`/`error`/sucesso; specs Playwright em `apps/desktop/e2e/` com `getByRole`/`data-testid`.
   - **Pico C11 & Python:** suíte host `-DPICO_HOST_TEST=1` em `firmware/pico-p7501/tests/` e `scripts/tests/` (`mypy --strict`).
3. **Diretórios Temporários Exclusivos:** Nunca use `temp_dir().join("fixo")` nem apenas `process::id()` em testes concorrentes — inclua contador `AtomicU64` por chamada ou `TempDir` RAII.
4. **Zero `sleep` / `waitForTimeout` & Sem Vazamento de Mocks:** Use polling condicional/eventos explícitos; mocks ficam exclusivamente em `apps/desktop/src/test/mocks/` ou atrás de `--mock` explícito em `scripts/bench/runner.py`.
5. **Sem Teatro de Testes:** Todo teste que valida contrato entre Rust e TS/C11/Python deve realmente **ler/parsear ambos os lados** ou executar o binário gerado, e falhar quando a guarda é quebrada.

## 3. Execução dos 9 (+1) Portões (`scripts/validate.py`)
```bash
# Execução completa paralela (Windows, Linux, macOS)
python scripts/validate.py            # ou: npm run validate
python scripts/validate.py --sequential
python scripts/validate.py --jobs 2
python scripts/validate.py --ghidra-smoke   # Opt-in Gate 10 (PANADIGA_GHIDRA_PATCHED)

# Alvo Android ARM64 (quando aarch64-linux-android estiver instalado):
cargo clippy -p panadiga-core -p panadiga-desktop --lib --target aarch64-linux-android --no-deps -- -D warnings

# Atalhos focados durante iteração:
npm run check:ui      # lint + uxlint + jscpd + vitest (apps/desktop)
npm run check:ui:all  # + Playwright E2E
python -m pytest scripts/tests/test_docs_parity.py scripts/tests/test_profiles_ssot.py
```

## 4. Resumo dos 9 (+1) Portões (`scripts/validate_gates.py`) & Acoplamento
1. **Rust Workspace (`11 crates`):** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --no-deps -- -D warnings` & `cargo test --workspace`.
2. **Oxlint Strict + TypeScript (`apps/desktop`):** `oxlint src --deny-warnings && tsc --noEmit`.
3. **Frontend Vitest (`apps/desktop`):** `vitest run` (`src/test/mocks/` isolado).
4. **Copy/Paste Detector (`jscpd`):** `.jscpd.json` com **`threshold: 0`** (`0` clones em 7 diretórios raiz).
5. **UX Completeness (`uxlint`):** `ux-guard scan src --fail-on info` (`100/100`, `0` issues).
6. **Playwright E2E (`apps/desktop`):** Chromium headless (`vite :1420`).
7. **SSOT & Ghidra Native Generator:** `panadiga ssot export-all --root .` + `panadiga ghidra export-all --dir tools/ghidra`.
8. **Pico C11 Host Tests (+ ARM):** `-std=c11 -Wall -Wextra -Werror -DPICO_HOST_TEST=1` (ou MSVC `/W4 /WX`) + build ARM se SDK presente.
9. **Python Strict & Paridade:** `mypy --strict scripts`, `py_compile` (`scripts/` + `tools/ghidra/*.py`) & `pytest scripts/tests`.
10. **Opt-In (`--ghidra-smoke`):** `ghidra_fleet_orchestrator --ghidra-smoke` com `PANADIGA_GHIDRA_PATCHED`.

- **Regra de Acoplamento:** Mudança em Rust atualiza schema/DTO + `ssot export-all` + TS/Zod + i18n + testes determinísticos + `docs/`/`ledger.json`. Alinhe `#[cfg(...)]` multi-SO e evite links em docs apontando para caminhos no `.gitignore` (`test_docs_parity.py`).
- **Verificação:** Tarefa de código/SSOT só é concluída após `python scripts/validate.py` passar `9/9` (e `--ghidra-smoke` se alterar pipeline/SLEIGH Ghidra).
