---
name: panadiga-contracts-stream-and-journal
description: >-
  Contratos arquiteturais Plano A (IR Rust, codegen declarativo único e SSOT_CONTRACT_HASH),
  compliance 100% cross-layer (profiles/, panadiga-profiles, panadiga-core, src-tauri IPC,
  TypeScript/Zod/locales, C11 Pico, Python e tools/ghidra), Plano B (stream panadiga-bench e
  journal append-only panadiga-vault), Zero-Trust WebView/IPC Tauri 2, 11 crates Rust e
  profiles/symbols/. Use ao editar IPC, streams, journal, constantes ou contratos SSOT.
---

# Contratos Cross-Stack, Compliance SSOT, Stream Unidirecional & Operation Journal

Esta skill consolida os contratos estáticos (**Plano A** + compliance cross-layer eliminando silos e constantes duplicadas), o estado dinâmico de bancada (**Plano B**), a blindagem da fronteira WebView/IPC no Tauri 2 e o escudo de callgraphs nos **11 crates Rust** acíclicos.

## 1. Quando Usar & Handoffs
- Ative ao alterar DTOs/IPC em `apps/desktop/src-tauri/`, schemas/constantes de hardware, dicionários i18n, tabelas C11 do Pico, `TypedDict` Python, telemetria/nonce em `crates/panadiga-bench/`, trilha forense em `crates/panadiga-vault/` ou projeções `ssot export-all`.
- Para regras gerais de desacoplamento, FCIS/MVVM e anti-god-module, combine com `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md`.
- Para gates automatizados completos, use `.agents/skills/panadiga-validate/SKILL.md`.
- **Referências:**
  - Payloads, constantes e contratos Plano A/Plano B/IPC em [plano-a-plano-b-and-zero-trust-ipc.md](./references/plano-a-plano-b-and-zero-trust-ipc.md).
  - Mapa da arquitetura SSOT unificada e dívidas cross-layer em [cross-layer-contracts-and-ssot-coherence.md](./references/cross-layer-contracts-and-ssot-coherence.md).

## 2. Plano A, Compliance Cross-Layer & Simetria SSOT
1. **SSOT Declarativa Única & Codegen (Sem Silos Órfãos):** `panadiga-constants`, `panadiga-profiles` e `panadiga-core::ssot` compilam a IR estática para TypeScript (`apps/desktop/src/models/generated/{profiles,ssot,digaos}.ts` + `SSOT_CONTRACT_HASH_STR`), Python (`scripts/profiles_types.py`), Ghidra (`tools/ghidra/`) e C11 (`firmware/picofw/generated/picofw_ssot_tables.*` + `firmware/digaos/generated/digaos_ssot_tables.*`) via `panadiga ssot export-all`. **Nunca** mantenha especificações paralelas à mão nem edite arquivos em `*/generated/`.
2. **Simetria de Segurança Entre C11, Rust & TS:** Faixas de segurança física (GPIOs livres de IR `GP6..9 | GP16..22`, teto de timeout NOR `185_000 ms`, `SA0..SA10` IPL RO) devem ser idênticas nas 3 camadas — jamais mais frouxas no Rust do que no C11.
3. **Isolamento de UI/Idioma & Verificação de Contrato:** `panadiga-core` **não deve** depender de `apps/desktop/src/locales/pt-BR/` via `include_str!` (prose pertence a `locales/`). Toda camada valida `ProfilesBundle.schema_version` (`validate.rs`), `SSOT_CONTRACT_HASH` e `P7501_SSOT_HASH` (macros + tabelas no handshake `IDENTIFY`).

## 3. Plano B — Stream Unidirecional (`panadiga-bench`) & Operation Journal (`panadiga-vault`)
1. **Stream de Telemetria (`panadiga://bench-event`):** Pull inicial via `get_bench_snapshot` (`BenchSnapshot` com `seq`, `state`, `latest_telemetry`, `active_progress`, `incomplete_op`) + deltas unidirecionais Rust → UI (`BenchPublisher` com coalescência e batching).
2. **Proibição de Controle no EventBus:** Gravar NOR, resetar target ou alterar pinos do Pico **NUNCA** trafega pelo stream; permanece **RPC Request → Response** explícito protegido por lock no Rust.
3. **Operation Journal (`vault_dir/journal/ops.jsonl`):** Log forense **append-only** encadeado por SHA-256 (`GENESIS_PREV_HASH` + `compute_journal_entry_hash`) com `OP_BEGIN` + `fsync` antes da I/O física e `OP_END` depois. **Nunca** faça Event Sourcing: blobs continuam imutáveis por SHA-256 e o índice rápido continua em `manifest.json` atômico protegido por `VaultLock`.

## 4. Fronteira Zero-Trust WebView / IPC & Os 11 Crates Acíclicos
- **WebView Confinada (`tauri.conf.json`):** `withGlobalTauri: false`, CSP restrita (`default-src 'self'; script-src 'self'`), zero `tauri_plugin_shell` e zero strings seriais livres sem validação estrita no backend Rust.
- **Nonces Efêmeros (`OperationNonceStore`):** `request_operation_nonce` emite `OperationNonceChallenge` (`DEFAULT_NONCE_TTL_MS = 120_000`) ligado a `(op_kind, target, input_sha256, ssot_contract_hash)`.
- **Escudo de Símbolos (`profiles/symbols/`):** `panadiga re index-symbols` gera índices determinísticos; `panadiga_symbol_lookup`, `panadiga://symbols/<train_id>/<module>` e `panadiga-rag` bloqueiam chamadas destrutivas a mutadores BGO (`0x50409DF2`, `0x50409E65`).
- **DAG Acíclico (`Cargo.toml`):** Base L1 (`panadiga-constants`, `panadiga-bench`, `panadiga-vault`, `panadiga-crypto`, `panadiga-ir`, `panadiga-profiles`) -> L2/L3 (`panadiga-core` -> `panadiga-rag` -> `panadiga-mcp` -> `panadiga-cli` + `panadiga-desktop`), todos com `#![deny(unsafe_code)]`.

## 5. Armadilhas & Verificação Obrigatória
- **Armadilhas:** Criar workarounds de casing em `ipc.ts` em vez de codegen único; ignorar `**/*.json` e `**/generated/**` ao auditar duplicação semântica; copiar apenas subconjunto de modelos em vez de todos os 89 perfis.
```bash
cargo run -q -p panadiga-cli -- profiles validate -j -
cargo run -q -p panadiga-cli -- ssot export-all --root .
git diff --stat apps/desktop/src/models/generated/ firmware/picofw/generated/ firmware/digaos/generated/ tools/ghidra/ profiles/schema/
cargo test --workspace && cargo clippy --workspace --all-targets --no-deps -- -D warnings
npm --prefix apps/desktop run lint && npx --prefix apps/desktop jscpd --config .jscpd.json
python -m mypy --strict scripts && python scripts/repo_sanitize_and_sync.py --check
```
