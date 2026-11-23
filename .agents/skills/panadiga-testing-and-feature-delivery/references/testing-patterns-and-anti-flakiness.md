# Padrões de Criação de Testes & Erradicação de Flakiness

## 1. Matriz de Criação de Testes por Stack

| Stack | Local & Comando | Padrão Obrigatório |
| :--- | :--- | :--- |
| **Rust** | `crates/*/src/**` (`#[cfg(test)]`) & `crates/*/tests/` · `cargo test --workspace` | Helpers de temp dir com `format!("panadiga_<escopo>_{}_{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed))` + RAII drop guard |
| **C11 Pico** | `firmware/pico-p7501/tests/` (`test_*.c`) · Gate 8 `validate_c11.py` | Compila com `-std=c11 -Wall -Wextra -Werror -DPICO_HOST_TEST=1`; usar stubs `hal/` deterministicos |
| **Vitest TS** | `apps/desktop/src/**/*.test.{ts,tsx}` · `npm --prefix apps/desktop test` | Mocks exclusivamente em `src/test/mocks/`; testar estados `isLoading`, vazio, erro e paridade `translations.test.ts` |
| **Playwright E2E** | `apps/desktop/e2e/*.spec.ts` · `npm --prefix apps/desktop run test:e2e` | `getByRole`/`getByTestId`; `webServer` gerenciado (`reuseExistingServer` atento a porta ocupada); **0 `waitForTimeout`** |
| **Python** | `scripts/tests/test_*.py` · `python -m pytest scripts/tests` + `mypy --strict scripts` | `tmp_path` fixture nativa do pytest; `MockBenchTransport` em bancada; nunca deixar subprocessos órfãos |

## 2. Pontos Críticos de Flakiness Detectados no Repositório
1. **Corrida de Escrita no Gate 7 (`scripts/validate_gates.py`):** Executar `ssot export-all` em paralelo com `oxlint`, `vitest`, `jscpd` e C11 gera corrida nos arquivos gerados (`ssot.ts`, `p7501_ssot.*`); sincronize antes ou rode exportação em fase anterior/posterior às leituras.
2. **Diretórios Temporários Fixos / Sem Sequência Atômica:** Pastas fixas em `temp_dir()` (ex.: `paths.rs`, `frm_pack`, `sd_creator`, `rebuild.rs`, `pdf.rs`) quebram sob testes paralelos ou chamadas concorrentes no Tauri — sempre adicione `AtomicU64` + cleanup.
3. **`reuseExistingServer: !process.env.CI` em `:1420`:** Em máquina local de desenvolvimento, se houver outro processo na porta `1420`, o Playwright testa o servidor errado; confirme ou encerre processos na porta antes de rodar E2E.
4. **Falso Teste de Contrato:** Em `hardening_audit_tests.rs`, sempre garanta que o teste realmente leia e compare o arquivo TS/C11/Python alvo em vez de apenas comentar o nome do arquivo.
