# Referência Técnica: Plano A, Plano B, Zero-Trust IPC & DAG de 11 Crates

## 1. Mapa do DAG Acíclico de 11 Crates (`Cargo.toml`)

```text
  panadiga-constants ──┬──► panadiga-bench (Snapshot, Stream, NonceStore)
                       ├──► panadiga-vault (ops.jsonl SHA-256 chain + VaultLock)
                       ├──► panadiga-crypto (.FRM, Feistel, AES, EXTRHEAD)
                       ├──► panadiga-ir (NEC/Panasonic IR codesets Android/Pico)
                       └──► panadiga-profiles (89 modelos, 12 Schemas, T0..T4)
                                      │
                                      ▼
                                panadiga-core
                         ┌────────────┴────────────┐
                         ▼                         ▼
                   panadiga-rag           panadiga-desktop (Tauri 2)
                         │
                         ▼
                   panadiga-mcp (26 tools, 9 resource families)
                         │
                         ▼
                   panadiga-cli (binário unificado CLI/TUI)
```

- Regra de separação L1/L2: crates base (`bench`, `vault`, `crypto`, `ir`, `profiles`, `constants`) **não** dependem de `panadiga-core`, evitando acoplamento circular e permitindo compilar alvos leves (`android-ir` / `aarch64-linux-android`).

## 2. Contratos `panadiga-bench` (Snapshot + Stream + Nonces)

- **Canal Único:** `BENCH_EVENT_CHANNEL` (`"panadiga://bench-event"`).
- **Snapshot Inicial (`get_bench_snapshot`):** `BenchSnapshot` contém `seq`, `ssot_contract_hash`, `state: BenchState`, `latest_telemetry: Option<TelemetrySample>`, `active_progress: Option<OpProgressSample>` e `incomplete_op: Option<IncompleteOpNotice>`.
- **Backpressure na Fonte (`BenchPublisher`):** Constantes exportadas no crate (`TELEMETRY_FLUSH_INTERVAL_MS`, `PROGRESS_FLUSH_INTERVAL_MS`, `LOG_FLUSH_INTERVAL_MS`, `LOG_BATCH_MAX_LINES`, `LOG_BATCH_MAX_BYTES`, `LOG_BUFFER_CAP_LINES`) impedem saturação da WebView durante rajadas UART.
- **Nonces Efêmeros (`crates/panadiga-bench/src/nonce.rs`):**
  - `OperationNonceStore::with_contract_hash(ssot_contract_hash)` emite `OperationNonceChallenge` via `request_operation_nonce`.
  - Cada desafio vincula `nonce_id`, `op_kind: OpKind`, `target`, `input_sha256`, `ssot_contract_hash`, `issued_at_ms` e `expires_at_ms` (`DEFAULT_NONCE_TTL_MS = 120_000 ms`), sendo invalidado após uso único (`prune_expired` limpa expirados).

## 3. Contratos `panadiga-vault` (`ops.jsonl` + Advisory Locking)

- **Arquivo de Trilha (`vault_dir/journal/ops.jsonl`):** Schema `JOURNAL_SCHEMA_VERSION = 1`, diretório `JOURNAL_SUBDIR = "journal"`, nome `JOURNAL_FILENAME = "ops.jsonl"`.
- **Encadeamento SHA-256:** Cada registro começa a partir de `GENESIS_PREV_HASH` (`64` zeros) e calcula `compute_journal_entry_hash(v, seq, prev_hash, body_bytes)` (`SHA256("{v}|{seq}|{prev_hash}|" || body_bytes)`).
- **Tolerância a Crash em Bancada:** Se faltar `OpEnd` correspondente a um `OpBegin` no EOF após queda de energia ou corte USB, o replay forense sinaliza `IncompleteOpNotice` (`JournalAlertKind`) sem tentar reconstruir estado de blobs via Event Sourcing.
- **Concorrência Multi-Processo:** `VaultLock` (`VAULT_LOCK_FILENAME`, `VaultLockMode`) serializa acessos entre Tauri Desktop, `panadiga-cli` e `panadiga-mcp`.

## 4. Segurança WebView Tauri 2 (`apps/desktop/src-tauri/`)

- `tauri.conf.json`: `"withGlobalTauri": false`, CSP explícita `"default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self';"`.
- `src-tauri/src/lib.rs`: `#![deny(unsafe_code)]`, `#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]`, inicialização de `attach_bench_stream_sink` e `start_port_mutation_observer` durante `.setup(...)`.
- Toda execução destrutiva exige handshake via `request_operation_nonce` + validação de alvo/hash/SSOT em Rust antes de abrir porta ou gravar mídia/Flash.
