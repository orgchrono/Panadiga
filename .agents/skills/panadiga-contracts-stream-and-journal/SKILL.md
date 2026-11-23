---
name: panadiga-contracts-stream-and-journal
description: >-
  Guia os contratos arquiteturais Plano A (IR Rust + codegen + SSOT_CONTRACT_HASH),
  Plano B (stream unidirecional panadiga-bench e journal append-only panadiga-vault),
  fronteira Zero-Trust WebView/IPC Tauri 2 (nonces efêmeros e sem plugin shell),
  os 11 crates Rust acíclicos e o dicionário de símbolos/callers em profiles/symbols/.
  Use ao editar IPC Tauri, streams de bancada, journal do vault ou contratos SSOT.
---

# Contratos Cross-Stack, Stream Unidirecional, Operation Journal & Zero-Trust IPC

Esta skill consolida a arquitetura de contratos estáticos (**Plano A**), estado dinâmico de bancada (**Plano B**), blindagem da fronteira WebView/IPC no Tauri 2 e proteção de callgraphs no ecossistema de **11 crates Rust**.

## 1. Quando Usar & Handoffs
- Ative ao alterar DTOs/IPC em `apps/desktop/src-tauri/`, código de telemetria/nonce em `crates/panadiga-bench/`, trilha forense em `crates/panadiga-vault/`, ou projeções `ssot export-all`.
- Para regras gerais de desacoplamento e MVVM, combine com `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md`.
- Para checagem de coerência cross-layer (Zod, i18n, JSON Schema), use `.agents/skills/panadiga-code-compliance-and-layer-coherence/SKILL.md`.
- Detalhes de payloads, constantes e contratos em [plano-a-plano-b-and-zero-trust-ipc.md](./references/plano-a-plano-b-and-zero-trust-ipc.md).

## 2. Plano A vs. Plano B (Limites Não Negociáveis)
1. **Plano A — Contratos Estáticos (IR Compilada + Codegen):**
   - `panadiga-constants`, `panadiga-profiles` e `panadiga-core::ssot` compilam a IR estática para TypeScript (`SSOT_CONTRACT_HASH_STR`), Python (`scripts/profiles_types.py`) e C11 (`p7501_ssot_tables.*`) via `panadiga ssot export-all`.
   - Toda camada valida `SSOT_CONTRACT_HASH` antes de aceitar operação física.
2. **Plano B — Estado Dinâmico (`panadiga-bench` + `panadiga-vault`):**
   - **Stream de Telemetria (`panadiga://bench-event`):** Pull inicial via `get_bench_snapshot` (`BenchSnapshot` com `seq`, `state`, `latest_telemetry`, `active_progress`, `incomplete_op`) + deltas unidirecionais Rust → UI (`BenchPublisher` com coalescência e batching).
   - **Proibição de Controle no EventBus:** Gravar NOR, resetar target ou alterar pinos do Pico **NUNCA** trafega pelo stream; permanece **RPC Request → Response** explícito protegido por lock no Rust.
   - **Operation Journal (`vault_dir/journal/ops.jsonl`):** Log forense **append-only** encadeado por SHA-256 (`GENESIS_PREV_HASH` + `compute_journal_entry_hash`) com `OP_BEGIN` + `fsync` antes da I/O física e `OP_END` depois. **Nunca** faça Event Sourcing: blobs continuam imutáveis por SHA-256 e o índice rápido continua em `manifest.json` atômico protegido por `VaultLock`.

## 3. Fronteira Zero-Trust WebView / IPC & Símbolos BGO
- **WebView Confinada (`tauri.conf.json`):** `withGlobalTauri: false`, CSP restrita (`default-src 'self'; script-src 'self'`), zero `tauri_plugin_shell` e zero strings seriais livres sem validação estrita no backend Rust.
- **Nonces Efêmeros (`OperationNonceStore`):** Tokens estáticos do tipo `CONFIRM_*` dão lugar a `request_operation_nonce` emitindo `OperationNonceChallenge` (`DEFAULT_NONCE_TTL_MS = 120_000`) ligado a `(op_kind, target, input_sha256, ssot_contract_hash)`.
- **Escudo de Símbolos (`profiles/symbols/`):** `panadiga re index-symbols` gera índices determinísticos por `(firmware_train_id, module, sha256, base_va)`. Tanto a tool MCP `panadiga_symbol_lookup` / resource `panadiga://symbols/<train_id>/<module>` quanto o guardrail do `panadiga-rag` bloqueiam recomendações/chamadas destrutivas a mutadores BGO (`0x50409DF2`, `0x50409E65`).

## 4. Os 11 Crates Rust Acíclicos (`Cargo.toml`)
- **Base L1:** `panadiga-constants`, `panadiga-bench`, `panadiga-vault`, `panadiga-crypto`, `panadiga-ir`, `panadiga-profiles`
- **Núcleo & Serviços L2/L3:** `panadiga-core` → `panadiga-rag` → `panadiga-mcp` → `panadiga-cli` + `panadiga-desktop` (`apps/desktop/src-tauri`, lib Android `panadiga`).
- Todos respeitam `#![deny(unsafe_code)]`, zero ciclos no DAG de dependências e sincronia obrigatória via `panadiga ssot export-all`.

## 5. Verificação Obrigatória
```bash
cargo test --workspace
cargo clippy --workspace --all-targets --no-deps -- -D warnings
cargo run -q -p panadiga-cli -- ssot export-all --root .
python scripts/repo_sanitize_and_sync.py --check
```
