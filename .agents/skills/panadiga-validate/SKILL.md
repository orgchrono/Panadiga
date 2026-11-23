---
name: panadiga-validate
description: >-
  Executa os 9 (+1 opt-in) Quality Gates do ecossistema Panadiga (Rust 11 crates,
  Oxlint+TypeScript, Vitest, jscpd 0%, uxlint 100/100, Playwright E2E, SSOT + Ghidra export,
  Pico C11 host tests e Python mypy --strict / pytest paridade), define a estratégia universal
  de testes e garante o acoplamento sincronizado entre código, SSOT, testes e documentação.
  Use sempre ao modificar código ou antes de encerrar tarefas.
---

# Validação Universal, Acoplamento & 9 (+1) Quality Gates

Esta skill centraliza a execução dos portões automatizados de qualidade e o protocolo obrigatório de acoplamento entre código, SSOT, testes universais e documentação.

## 1. Quando Usar & Handoffs
- Ative ao concluir mudanças em Rust, TypeScript, C11, Python, `profiles/` ou `docs/`, ou ao projetar suítes de testes em novos módulos.
- Para diagnóstico detalhado de violações e armadilhas de geometria Flash/BGO, consulte `.agents/skills/panadiga-security-and-violation-guards/SKILL.md`, `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md` e `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md`.
- Para construir mapas SSOT (`profiles/` + `ssot export-all`) e preparar sincronia/plano de commits sem tocar arquivos concorrentes (`--commit-plan --exclude`), combine com `.agents/skills/panadiga-ssot-builder-and-validation/SKILL.md` e `.agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md`.
- Para automatização headless em bancada e mocks do Pico, combine com `.agents/skills/panadiga-bench-automation-and-headless/SKILL.md`.
- Matriz completa dos 9+1 gates, pirâmide de testes universais e regras de acoplamento em [universal-testing-and-gates-matrix.md](./references/universal-testing-and-gates-matrix.md).

## 2. Como Executar os Portões (`scripts/validate.py`)
```bash
# Execução completa paralela (Windows, Linux, macOS)
python scripts/validate.py
# ou: npm run validate

# Modo sequencial com logs em tempo real (ideal se Playwright sofrer contenção de CPU) ou limitar workers:
python scripts/validate.py --sequential
python scripts/validate.py --jobs 2

# Opt-in Gate 10: Smoke real no Ghidra patchado (micro-fixture Traps #1..#8):
python scripts/validate.py --ghidra-smoke

# Alvo Android ARM64 (quando aarch64-linux-android estiver instalado via rustup):
cargo clippy -p panadiga-core -p panadiga-desktop --lib --target aarch64-linux-android --no-deps -- -D warnings

# Atalhos direcionados durante iteração:
npm run check:ui      # lint + uxlint + jscpd + vitest (apps/desktop)
npm run check:ui:all  # + Playwright E2E (12 specs)
python -m pytest scripts/tests/test_docs_parity.py scripts/tests/test_profiles_ssot.py
```

## 3. Resumo dos 9 (+1) Portões (`scripts/validate_gates.py`)
1. **Rust Workspace (`11 crates`):** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --no-deps -- -D warnings` & `cargo test --workspace`.
2. **Oxlint Strict + TypeScript (`apps/desktop`):** `oxlint src --deny-warnings && tsc --noEmit`.
3. **Frontend Vitest (`apps/desktop`):** `vitest run` (`src/test/mocks/` isolado).
4. **Copy/Paste Detector (`jscpd`):** `.jscpd.json` com **`threshold: 0`** (`0` clones em 7 diretórios raiz).
5. **UX Completeness (`uxlint`):** `ux-guard scan src --fail-on info` (`100/100`, `0` issues).
6. **Playwright E2E (`apps/desktop`):** `12` specs em Chromium headless (`vite :1420`).
7. **SSOT & Ghidra Native Generator:** `panadiga ssot export-all --root .` + `panadiga ghidra export-all --dir tools/ghidra`.
8. **Pico C11 Host Tests (+ ARM):** Compilação e execução de `53` arquivos C11 / `4` suítes host com `-std=c11 -Wall -Wextra -Werror -DPICO_HOST_TEST=1` (ou MSVC `/W4 /WX`) + build ARM se SDK presente.
9. **Python Strict & Paridade:** `mypy --strict scripts`, `py_compile` (`scripts/` + `tools/ghidra/*.py`) & `pytest scripts/tests` (`8` arquivos incluindo `test_docs_parity.py` e `test_profiles_ssot.py`).
10. **Opt-In (`--ghidra-smoke`):** `ghidra_fleet_orchestrator --ghidra-smoke` com `PANADIGA_GHIDRA_PATCHED`.

## 4. Regra de Acoplamento no Ecossistema
- **Nenhuma mudança está pronta isoladamente:** alteração de regra em Rust atualiza schema/DTO + `ssot export-all` + TS/Zod + i18n `en-US`/`pt-BR` + testes determinísticos + `docs/`/`ledger.json` (quando mudar fatos de bancada/CLI).
- **Resolução rápida de falhas comuns:**
  - `unused_imports`/`dead_code` em outro SO -> alinhe `#[cfg(...)]` exato no `use` e na função;
  - link quebrado só no CI -> `test_docs_parity.py` rejeita links para arquivos ignorados no `.gitignore`;
  - falha `jscpd` -> extraia helper único compartilhado entre módulos.

## 5. Verificação
- Tarefa de código/SSOT só é declarada concluída após `python scripts/validate.py` passar `9/9` (e `--ghidra-smoke` quando mexer no pipeline/SLEIGH Ghidra).
