# Invariantes do Portão de Qualidade (Panadiga)

Qualquer alteração submetida deve satisfazer os **9 Quality Gates** (`npm run validate` / `python scripts/validate.py`) sem exceção:

1. **Rust Check, Clippy & Tests (5 crates)**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --no-deps -- -D warnings` e `cargo test --workspace` devem passar com 0 falhas (`#![deny(unsafe_code)]` nos módulos não-FFI; validar também `cargo clippy -p panadiga-core -p panadiga-desktop --lib --target aarch64-linux-android --no-deps -- -D warnings` quando o target Android estiver instalado).
2. **Oxlint + TypeScript Strict**: `npm run lint` (`oxlint src --deny-warnings && tsc --noEmit`) em `apps/desktop` com 0 avisos/erros.
3. **Vitest Unit**: `npm test` em `apps/desktop` com 100% de sucesso.
4. **Detector de Duplicação (`jscpd`)**: `npx --prefix apps/desktop jscpd --config .jscpd.json` com `threshold: 0` (0 clones tolerados).
5. **Linter UX (`uxlint`)**: `npm run uxlint` em `apps/desktop` pontuação `100/100` (`0` issues).
6. **Playwright E2E**: `npm run test:e2e` em `apps/desktop` com todos os cenários passando em Chromium headless.
7. **Ghidra Native Generator**: `cargo run -p panadiga-cli -- ghidra export-all --dir tools/ghidra` executando sem erros e mantendo a SSOT em Rust sincronizada.
8. **Pico C11 Host Test**: Compilação e execução em diretório temporário via GCC/Clang (`-std=c11 -Wall -Wextra -Werror -DPICO_HOST_TEST=1`) ou MSVC (`/std:c11 /W4 /WX /DPICO_HOST_TEST=1`).
9. **Python Strict & Paridade**: `python -m mypy --strict scripts`, `python -m py_compile` nos alvos Python/Jython e `python -m pytest scripts/tests` (incluindo paridade documental e `git check-ignore`).
