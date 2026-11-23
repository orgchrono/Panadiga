# Padrões do Workspace Rust (`6 Crates`, Edition 2024 & Android)

## 1. Mapa dos 6 Crates (`Cargo.toml`)
| Crate | Caminho | Papel Arquitetural |
| :--- | :--- | :--- |
| **`panadiga-profiles`** | `crates/panadiga-profiles` | SSOT relacional (`profiles/`), 12 JSON Schemas Draft 2020-12, `EvidenceTier`, `CapabilityGate` `C0..C5`, codegen TS/C11/Ghidra |
| **`panadiga-core`** | `crates/panadiga-core` | Núcleo puro determinístico: `hdd`, `crypto`, `nvram`, `pico`, `debugger`, `ghidra`, `re` (`Mn103Vm`, `prewrite_gate`, `sig_locator`, `verify_loop`, `bgo_unbrick`, `region_compat`), `vault`, `ingest`, `ssot`, `settings` |
| **`panadiga-rag`** | `crates/panadiga-rag` | Engine Zero-Error RAG de 6 camadas + `lopdf` + `HardwareSafetyValidator` |
| **`panadiga-mcp`** | `crates/panadiga-mcp` | Servidor JSON-RPC 2.0 `stdio` (25 tools, 8 famílias `panadiga://`, 4 prompts) |
| **`panadiga-cli`** | `crates/panadiga-cli` | Binário `panadiga` / `panadiga.exe` (Clap, `--json -`, TUI `-i`, launcher Desktop) |
| **`panadiga-desktop`** | `apps/desktop/src-tauri` | Shell Tauri 2 (Windows/Linux/macOS + Android APK `aarch64-linux-android` com `[lib] name = "panadiga"` permanente) |

## 2. Regras de Implementação & Observabilidade
- **Logs & Telemetria:** Use `tracing::{info, warn, error, debug}` + `tracing-subscriber` (`stderr`) + `opentelemetry`. Nunca use `println!`/`eprintln!` em produção dentro dos crates com `deny(clippy::print_stdout, clippy::print_stderr)`.
- **Erros:** Use `thiserror::Error` com variantes descritivas e códigos estruturados (`E05`, `BenchSafetyError`, etc.) nos crates de biblioteca; converta com `.with_context(...)` (`anyhow`) na CLI.
- **Rustdoc (`///`):** Documente structs públicos, campos críticos de segurança, pré-condições de alinhamento/endereço e invariantes (`E05`, `SA0..SA10`) — evitando comentários óbvios sobre o "o quê".
- **Dependências Compartilhadas:** Declare versões comuns em `[workspace.dependencies]` no `Cargo.toml` raiz (`serde`, `sha2`, `thiserror`, `tracing`, `clap`, `lopdf`, etc.) sem duplicar versões em subcrates.

## 3. Portabilidade Multi-SO (`Windows / Linux / macOS / Android`)
- Mantenha implementações específicas de SO simétricas sob módulos separados (`discovery_win.rs`, `win_ioctl/{win_impl,stub}.rs`, HAL Android USB-OTG / BLE NUS) com atributos `#[cfg(...)]` consistentes tanto nos `use` quanto nas funções auxiliares.
- Verifique `cargo clippy -p panadiga-core -p panadiga-desktop --lib --target aarch64-linux-android --no-deps -- -D warnings` sempre que mexer em serial/USB/BLE/Tauri.
