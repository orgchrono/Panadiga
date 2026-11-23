# Invariantes do Portão de Qualidade (Panadiga)

Qualquer alteração submetida deve satisfazer os **9 Quality Gates** (`npm run validate` / `python scripts/validate.py`, além do Gate 10 opt-in `--ghidra-smoke` quando aplicável) sem exceção:

1. **Rust Check, Clippy & Tests (`11 crates`):** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --no-deps -- -D warnings` e `cargo test --workspace` devem passar com `0` falhas (`#![deny(unsafe_code)]` nos módulos não-FFI; `#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]`; validar também `cargo clippy -p panadiga-core -p panadiga-desktop --lib --target aarch64-linux-android --no-deps -- -D warnings` quando o target Android estiver instalado).
2. **Oxlint + TypeScript Strict (`apps/desktop`):** `npm run lint` (`oxlint src --deny-warnings && tsc --noEmit`) com `@orgchrono/oxlint-plugin` e `0` avisos/erros.
3. **Vitest Unit (`apps/desktop`):** `npm test` com `100%` de sucesso e mocks estritamente isolados em `apps/desktop/src/test/mocks/`.
4. **Detector de Duplicação (`jscpd`):** `npx --prefix apps/desktop jscpd --config .jscpd.json` com `threshold: 0` (`0` clones tolerados).
5. **Linter UX (`uxlint`):** `npm run uxlint` em `apps/desktop` (`ux-guard scan src --fail-on info`, score `100/100`, `0` issues).
6. **Playwright E2E (`apps/desktop`):** `npm run test:e2e` com os `12` specs passando em Chromium headless.
7. **SSOT & Ghidra Native Generator:** `panadiga ssot export-all --root .` e `panadiga ghidra export-all --dir tools/ghidra` mantendo a SSOT em Rust sincronizada com TS, C11, Schemas e Ghidra.
8. **Pico C11 Host Tests (+ ARM):** Compilação e execução dos `53` arquivos / `4` suítes host (`test_uart_codec.c`, `test_pana_logic.c`, `test_pana_diag_drv.c`, `test_gdb_rsp_host.c`) com `-std=c11 -Wall -Wextra -Werror -DPICO_HOST_TEST=1` (ou MSVC `/W4 /WX`), mais checagem ARM RP2040/RP2350 quando SDK estiver presente.
9. **Python Strict, Paridade & SSOT:** `python -m mypy --strict scripts`, `python -m py_compile` nos alvos Python/Jython e `python -m pytest scripts/tests` (incluindo `test_docs_parity.py` com `git check-ignore` + `ledger.json` e `test_profiles_ssot.py` com higiene SSOT/`SA`).
10. **Opt-In Ghidra Smoke (`--ghidra-smoke`):** `python scripts/validate.py --ghidra-smoke` executa `ghidra_fleet_orchestrator` na micro-fixture de Armadilhas MN103S quando `PANADIGA_GHIDRA_PATCHED` estiver configurado.
