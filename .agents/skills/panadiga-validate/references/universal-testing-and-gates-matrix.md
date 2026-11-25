# Estrutura Universal de Testes, Acoplamento & Matriz dos 9+1 Gates

## 1. Pirâmide Universal de Testes Panadiga

| Camada | Escopo & Localização | Política Anti-Flakiness & Mocks |
| :--- | :--- | :--- |
| **Nível 1 — Unitário Puro & Tipagem** | `cargo test`, `vitest` (`apps/desktop/src/**/*.test.*`), suítes C11 host (`firmware/picofw/tests/` + `firmware/digaos/tests/`), `mypy --strict` + `pytest scripts/tests/` | Sem acesso a hardware/rede; mocks TS isolados estritamente em `src/test/mocks/`; `MockBenchTransport` em Python |
| **Nível 2 — Contrato, SSOT & Paridade** | `panadiga profiles validate` (`796` checagens), `test_profiles_ssot.py`, `test_docs_parity.py`, `translations.test.ts` | Falha em tokens banidos, esquemas inválidos, hashes quebrados ou falta de chave `en-US`↔`pt-BR` |
| **Nível 3 — Emulação & Análise Estática** | `panadiga re verify-patches` (`Mn103Vm`), `matshita-emu`, `oxlint` (`7` regras), `jscpd` (`0` clones), `ux-guard` (`100/100`) | Zero `sleep` fixo; prova determinística de opcodes e limites `E05` |
| **Nível 4 — Headless End-to-End** | Playwright (`12` specs em `apps/desktop/e2e/`), `scripts.bench.runner --mock`, `--ghidra-smoke` | Retries apenas em CI (`2`), seletores semânticos, `vite` gerenciado automaticamente |
| **Nível 5 — Bancada Passiva / Silício Real** | `scripts.bench.runner --port COM*`, `pico bench-wizard`, `davis-probe`, `bgo-live-apply` | Sempre com `ISOLATE` RAII, isolamento SMPS/`ADuM`, backup SHA-256 e `ledger.json` |

## 2. Matriz de Acoplamento Obrigatório por Tipo de Mudança
| Se você alterou... | Sincronize obrigatoriamente... |
| :--- | :--- |
| `profiles/*.json` ou `panadiga-profiles` | `panadiga ssot export-all`, `profiles validate`, `apps/desktop/src/models/generated/{profiles,ssot,digaos}.ts`, `firmware/picofw/generated/`, `firmware/digaos/generated/` |
| `crates/panadiga-core` (IPC / comandos / erros) | `crates/panadiga-mcp/src/tools/`, `apps/desktop/src-tauri`, `services/`, `locales/{en-US,pt-BR}/`, `src/test/mocks/` |
| `crates/panadiga-mcp` (nova tool) | `tools/mod.rs`, testes unitários, contadores em `test_docs_parity.py`, `README*`, `AGENTS.md`, `.github/RELEASE_NOTES.md` |
| `docs/pt-BR/` ou `docs/en-US/` ou `docs/debug_logs/` | Par bilíngue 1:1, `docs/translation_pairs.json`, `docs/INDEX.md`, `docs/STATUS.md`, `docs/debug_logs/README.md`, `docs/evidence/ledger.json`, verificar `.gitignore` |
| `.agents/skills/` | `.agents/skills/README.md`, `panadiga-skill-orchestrator`, `.agents/agents/*.md`, rodar verificação YAML/links/higiene |

## 3. Os 8 Arquivos de Teste em `scripts/tests/`
- `test_docs_parity.py`: 40 pares bilíngues, 28 logs + SHA-256, `ledger.json`, stale hash, `[PROVADO EM HARDWARE REAL]` -> `N4_HARDWARE`, `git check-ignore`.
- `test_profiles_ssot.py`: 12 JSON Schemas, 9 pastas SSOT, rejeição de overlay `5V` e proibição de tokens `SA` obsoletos em `.agents/rules/AGENTS.md`, `AGENTS.md`, `GEMINI.md`, `RELEASE_NOTES.md`, `profiles/`.
- `test_bench_protocol.py`: `BenchSession` `ISOLATE`, `SAFE_BENCH_COMMANDS`, `MockBenchTransport`.
- `test_re_discovery_scripts.py`: `fleet_decryptor`, `frm_train_harvester`, `nor_intake_analyzer`, `manual_spec_extractor` (quarentena), `ghidra_fleet_orchestrator`.
- `test_build_and_validate.py`, `test_build_cache.py`, `test_android_verify.py`, `test_re_ghidra_tools.py`: DAG de build, cache SHA-256, assinatura APK e paridade Python/Rust/Ghidra.
