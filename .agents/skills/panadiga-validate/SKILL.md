---
name: panadiga-validate
description: >-
  Executa a suíte completa de 9 Quality Gates do ecossistema Panadiga
  (Rust, Oxlint+TypeScript, Vitest, jscpd 0%, uxlint, Playwright E2E, Ghidra export, C11 host tests e Python mypy/pytest).
  Use esta skill sempre que modificar código ou antes de finalizar qualquer tarefa.
---

# Procedimento de Validação Universal — 9 Quality Gates

Esta skill orienta a execução da suíte obrigatória de qualidade do Panadiga em **Windows**, **Linux** e **macOS**.

## Como Executar
Na raiz do repositório:
```bash
npm run validate
# ou diretamente via Python 3:
python scripts/validate.py
# modo sequencial com logs em tempo real (ideal se Playwright sofrer contenção de CPU):
python scripts/validate.py --sequential
# ou limitando workers paralelos:
python scripts/validate.py --jobs 2
```

Para validar o target **Android ARM64** quando `aarch64-linux-android` estiver instalado via `rustup`:
```bash
cargo clippy -p panadiga-core -p panadiga-desktop --lib --target aarch64-linux-android --no-deps -- -D warnings
```

## Os 9 Portões Validados (`scripts/validate_gates.py`)
1. **Rust Workspace (5 crates)**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --no-deps -- -D warnings` e `cargo test --workspace`.
2. **Oxlint Strict + TypeScript (`apps/desktop`)**: `npm run lint` (`oxlint src --deny-warnings && tsc --noEmit`).
3. **Frontend Vitest (`apps/desktop`)**: `npm test` (`vitest run`).
4. **Copy/Paste Detector (`jscpd`)**: `npx --prefix apps/desktop jscpd --config .jscpd.json` (**0% / 0 clones** em `apps/desktop/src`, `apps/desktop/src-tauri/src`, `packages/oxlint-plugin-panadiga/src`, `crates`, `firmware/pico-p7501`, `tools`, `scripts`).
5. **UX Completeness (`uxlint`)**: `npm run uxlint` em `apps/desktop` (`ux-guard scan src --fail-on info`, score 100/100).
6. **Playwright E2E (`apps/desktop`)**: `npm run test:e2e` (Chromium headless).
7. **Ghidra Native Generator**: `cargo run -p panadiga-cli -- ghidra export-all --dir tools/ghidra`.
8. **Pico C11 Host Tests**: Compilação e execução em diretório temporário com `gcc`/`clang` (`-std=c11 -Wall -Wextra -Werror -DPICO_HOST_TEST=1`) ou MSVC `cl` (`/std:c11 /W4 /WX /DPICO_HOST_TEST=1`) cobrindo `test_uart_codec.c`, `test_pana_logic.c`, `test_pana_diag_drv.c` e `test_gdb_rsp_host.c`.
9. **Python Strict & Paridade**: `python -m mypy --strict scripts`, `python -m py_compile` nos módulos Python/Jython (`tools/ghidra/*.py` + `scripts/`) e `python -m pytest scripts/tests` (inclui `test_docs_parity.py` com verificação `git check-ignore`).

## Resolução de Falhas Multi-SO Comuns
- **`unused_imports` / `dead_code` em outro SO**: Itens compartilhados entre `#[cfg(windows)]`, `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]` ou `#[cfg(target_os = "android")]` devem usar o `#[cfg(...)]` exato no `use` e no item auxiliar para passar em `clippy --all-targets -D warnings`.
- **Links quebrados apenas no GitHub Actions CI**: `scripts/tests/test_docs_parity.py` valida com `git check-ignore --stdin` que links em `docs/INDEX.md`, `docs/STATUS.md`, `AUDITORIA_COMPLETA.md`, `COMPLETE_AUDIT.md` e `docs/evidence/ledger.json` nunca apontem para caminhos ignorados pelo `.gitignore`.
- **Falha em `jscpd`**: Extraia helpers compartilhados entre `crates/`, `apps/desktop/src-tauri/src/`, `apps/desktop/src/` ou `scripts/` (0 clones tolerados).
- **Falha em `mypy --strict`**: Adicione anotações completas de parâmetros e retorno (`-> None`, `Mapping`, etc.).
