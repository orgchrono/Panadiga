---
name: panadiga-rust-backend-dev
description: >-
  Desenvolvimento idiomático e seguro nos 11 crates Rust Edition 2024 do workspace Panadiga
  (panadiga-constants, panadiga-bench, panadiga-vault, panadiga-crypto, panadiga-ir,
  panadiga-profiles, panadiga-core, panadiga-rag, panadiga-mcp, panadiga-cli e
  apps/desktop/src-tauri / panadiga-desktop, incluindo alvo Android aarch64-linux-android).
  Enforça #![deny(unsafe_code)], tracing via stderr, Clippy -D warnings, --json - limpo
  e Rustdoc. Use ao editar ou criar código Rust.
---

# Desenvolvimento Backend Rust (`11 Crates`, Edition 2024 & Android FFI)

Esta skill padroniza o desenvolvimento em Rust nos **11 crates** acíclicos do workspace (`Cargo.toml`), preservando segurança de memória, pureza funcional no núcleo e compatibilidade cross-OS + Android APK.

## 1. Quando Usar & Handoffs
- Ative ao modificar ou criar módulos em `crates/panadiga-*` ou `apps/desktop/src-tauri/`.
- Para contratos Plano A/Plano B (`panadiga-bench`, `panadiga-vault`), Zero-Trust IPC e mapa completo dos 11 crates, consulte `.agents/skills/panadiga-contracts-stream-and-journal/SKILL.md`.
- Para fronteiras FCIS/SSOT/anti-god-modules, consulte `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md`; para refatoração funcional, álgebra de intervalos, detectores de padrões e provas diferenciais, consulte `.agents/skills/panadiga-functional-math-and-patterns/SKILL.md`.
- Se alterar schemas/tools MCP ou camadas RAG, consulte `.agents/skills/panadiga-rag-and-mcp-engineering/SKILL.md`.
- Guia de padrões, multi-SO `#[cfg(...)]` e Android em [rust-workspace-patterns.md](./references/rust-workspace-patterns.md).

## 2. Invariantes Não-Negociáveis do Workspace Rust
1. **Zero `unsafe` onde Aplicável (`#![deny(unsafe_code)]`):** Módulos não-FFI rejeitam código `unsafe`; chamadas Win32/Unix de disco ficam isoladas atrás de abstrações pequenas e auditadas (`hdd/writer/win_ioctl/`).
2. **Banimento de `print_stdout` / `print_stderr` (`clippy`):** `panadiga-core`, `panadiga-rag`, `panadiga-mcp` e `panadiga-desktop` proíbem `println!`/`eprintln!` em produção para não corromper JSON-RPC `stdio` do MCP nem `--json -` da CLI. Use `tracing` (`stderr` + `opentelemetry`).
3. **Erros Tipados & Documentação:** Bibliotecas usam `thiserror`; CLI/orquestração usa `anyhow::Context`. Toda API pública leva Rustdoc `///` explicando contratos e invariantes.
4. **Alvo Android Permanente (`aarch64-linux-android`):** `apps/desktop/src-tauri/Cargo.toml` mantém `[lib] name = "panadiga"` permanente (sem mutar `Cargo.toml` entre builds desktop/Android).
5. **Paridade Multi-SO (`#[cfg(...)]`):** Itens compartilhados entre `windows`, `linux`, `macos` e `android` devem carregar o mesmo `#[cfg(...)]` no `use` e no helper para evitar `unused_imports`/`dead_code` no CI multi-plataforma.

## 3. Workflow Canônico de Desenvolvimento Rust
```bash
# 1. Formatação e análise estrita em todo o workspace (6 crates, todas as targets)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --no-deps -- -D warnings

# 2. Suíte de testes Rust (337+ testes unitários e de integração)
cargo test --workspace

# 3. Se o target Android estiver instalado via rustup:
cargo clippy -p panadiga-core -p panadiga-desktop --lib --target aarch64-linux-android --no-deps -- -D warnings

# 4. Se alterou profiles/ ou constantes SSOT / Ghidra:
cargo run -q -p panadiga-cli -- ssot export-all --root .
cargo run -q -p panadiga-cli -- ghidra export-all --dir tools/ghidra
```

## 4. Armadilhas & Anti-Padrões
- Emitir logs ou texto solto em `stdout` quando `--json -` estiver ativo (`crates/panadiga-cli/src/main.rs`).
- Criar dependências de IO/terminal/filesystem dentro de funções puras de parsing em `panadiga-core` que deveriam rodar iguais em Desktop, CLI, Android e testes determinísticos.
- Esquecer de rodar `cargo clippy --all-targets -- -D warnings` (warnings em `tests/` ou exemplos quebram o Gate 1).

## 5. Verificação
- Gate 1 (`cargo fmt --check`, `cargo clippy --workspace --all-targets --no-deps -- -D warnings`, `cargo test --workspace`) + Gate 4 (`jscpd 0%`) + Gate 7 (`ssot`/`ghidra export-all`) verdes em `python scripts/validate.py`.
