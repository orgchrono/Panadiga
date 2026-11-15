> [!NOTE]
> **Snapshot Histórico (`2026-11-11`):** Esta auditoria foi capturada antes da refatoração modular final em `crates/`, `firmware/pico-p7501/` e `scripts/`. Para o estado atual consolidado, consulte [`../../AUDITORIA_COMPLETA.md`](../../AUDITORIA_COMPLETA.md).

# 🔎 Auditoria Completa — Panadiga Engineering Suite

> **Data:** 2026-11-11 | **Escopo:** Build, Deploy, Run, Organização física, Arquitetura MVVM/FP, Libs alternativas

---

## 1. Raiz do Repositório — Inventário e Avaliação

### 1.1 Arquivos presentes na raiz

| Arquivo | Papel | Avaliação |
|---|---|---|
| `Cargo.toml` | Manifesto Rust único (bin + lib) | ✅ Correto |
| `Cargo.lock` | Lock file determinístico | ✅ Deve ser commitado |
| `Cargo.toml.bak` | **Backup manualmente criado** | ⚠️ REDUNDANTE — remover |
| `build.rs` | Build script Tauri | ✅ Correto |
| `build.cmd` | Pipeline build Windows | ✅ Principal |
| `build.ps1` | Pipeline build PowerShell | ⚠️ **Duplica** `build.cmd` (ver §3) |
| `build.sh` | Pipeline build Unix | ✅ Principal Unix |
| `run.cmd` | Launcher rápido Windows | ✅ Principal |
| `run.ps1` | Launcher PowerShell | ⚠️ **Desincronizado** com `run.cmd` (ver §3.2) |
| `run.sh` | Launcher Unix | ✅ Principal Unix |
| `validate_all.cmd` | 9 Quality Gates (CMD) | ✅ OK mas incompleto vs `.ps1` |
| `validate_all.ps1` | 9 Quality Gates (PowerShell) | ✅ Principal — mais completo |
| `validate_all.sh` | 9 Quality Gates (Unix) | ✅ Principal Unix |
| `generate_audit_bundle.cmd` | Wrapper para Python audit | ✅ Thin wrapper — OK |
| `generate_audit_bundle.ps1` | Wrapper para Python audit | ⚠️ **Duplica** o `.cmd` — desnecessário |
| `tauri.conf.json` | Configuração Tauri 2 | ⚠️ CSP `null` — risco de segurança (ver §6) |
| `package.json` | Workspace NPM root | ✅ Thin wrapper — OK |
| `.jscpd.json` | Config detector de duplicação | ✅ Bem configurado (threshold 0) |
| `.cargo/config.toml` | Config Cargo build | ✅ Incremental ativado |
| `GEMINI.md` | Diretrizes para AI assistants | ✅ Bem detalhado |
| `AGENTS.md` | Diretrizes para swarm agents | ✅ |
| `.cursorrules` | Regras Cursor AI | ✅ |
| `LICENSE` | Licença | ✅ |
| `README.md` / `README.pt-BR.md` | Documentação | ✅ Ambos mantidos em sincronia |

> [!WARNING]
> `Cargo.toml.bak` vaza informação de configuração intermediária e pode confundir IDEs e ferramentas. **Deletar** e usar Git para histórico.

---

## 2. Sistema de Build Multiplataforma

### 2.1 Estrutura atual — 3 scripts paralelos

```
build.cmd  ──►  Windows CMD (principal Windows)
build.ps1  ──►  Windows PowerShell (duplicata parcial)
build.sh   ──►  Unix bash (principal Linux/macOS)
```

**Pipeline de cada script:**

| Passo | build.cmd | build.ps1 | build.sh |
|---|---|---|---|
| npm ci + Vite build | ✅ | ✅ | ✅ |
| `cargo run ghidra export-all` | ✅ | ✅ | ✅ |
| `cargo build --release` | ✅ | ✅ | ✅ |
| Copy para `dist/` | ✅ | ✅ | ✅ |
| Copy UF2 firmware | ✅ src/pico primeiro | ✅ src/pico primeiro | ⚠️ só path legado `dwire32a_jtag_research/` |
| Copy `run.cmd`/`run.sh` | ✅ | ⚠️ copia run.ps1, não run.sh | ✅ |
| ZIP release | ✅ via PowerShell inline | ✅ | ✅ tar.gz |
| SHA256SUMS | ✅ | ✅ | ✅ |
| Android build | ✅ via `:build_android` label | ✅ via função `Build-Android` | ✅ via `--android` |
| CMake local search | ⚠️ só PATH | ⚠️ só PATH | ⚠️ só PATH |
| `CI` env sanitize | ✅ | ❌ ausente | ❌ ausente |

### 2.2 Problemas identificados no build

| ID | Sev | Localização | Problema |
|---|---|---|---|
| BD-01 | **ALTO** | `build.sh:118-123` | Cópia UF2 usa apenas path legado `dwire32a_jtag_research/`, ignora `src/pico/firmware_bins/` (path correto) |
| BD-02 | **ALTO** | `build.ps1:68` | `Compress-Archive` não tem guard para `WebView2Loader.dll` inexistente — falha silenciosa |
| BD-03 | **MÉDIO** | `build.cmd:64-73` / `build.sh` / `build.ps1` | CMake local em `dwire32a_jtag_research/` não é detectado; script cheats no PATH global |
| BD-04 | **MÉDIO** | `build.sh`, `build.ps1` | Ausência de sanitização de `CI` env — pode causar `--ci 1 ` (trailing space) no Tauri CLI |
| BD-05 | **MÉDIO** | `build.ps1:68` | `Compress-Archive` lista itens absolutos sem verificar existência de cada um (falha em máquinas sem `dist\WebView2Loader.dll`) |
| BD-06 | **BAIXO** | `Cargo.toml` | `strip = true` no release remove símbolos de debug — dificulta análise de crashes em produção; considerar `strip = "debuginfo"` |
| BD-07 | **BAIXO** | `Cargo.toml` | `codegen-units = 20` no release não é o valor mais agressivo para LTO thin; `codegen-units = 1` maximiza otimizações |
| BD-08 | **INFO** | `build.cmd:172` | Mutação do `Cargo.toml` (crate-type patch) durante Android build cria janela de risco se o processo for interrompido — `Cargo.toml.bak` sugere que já ocorreu antes |

### 2.3 Lib alternativa — sistema de build flexível

> [!TIP]
> Em vez de manter 3 scripts paralelos (CMD, PS1, sh), considere:
> - **[`just`](https://github.com/casey/just)** — `Justfile` declarativo multiplataforma; um único arquivo com receitas para todas as plataformas; substituiria `build.cmd`, `build.ps1`, `build.sh` com um único `justfile`.
> - **[`cargo-make`](https://github.com/sagiegurari/cargo-make)** — `Makefile.toml` em TOML; integrado ao ecosistema Rust; excelente para projetos Tauri.

**Exemplo Justfile:**
```just
# Runs on Windows, Linux, macOS
build:
    npm --prefix apps/desktop ci
    cargo build --release -p panadiga
    just _package

_package:
    mkdir -p dist/firmware dist/releases
    cp target/release/panadiga{{EXE}} dist/

android:
    just build
    npx @tauri-apps/cli android build --apk --target aarch64
```

---

## 3. Launchers (run.*) — Redundâncias e Bugs

### 3.1 Comparação run.cmd vs run.ps1

| Aspecto | run.cmd | run.ps1 | run.sh |
|---|---|---|---|
| Binary lookup | `dist\panadiga.exe` → `target\release\` | `dist_bin\panadiga-desktop.exe` (**path diferente!**) | `dist/panadiga` → `target/release/` |
| Fallback build | `build.cmd` | `build_all.ps1` (**arquivo que não existe!**) | `build.sh` |
| CLI mode | `dist\panadiga.exe %*` | `dist_bin\panadiga-cli.exe` (binário separado que não existe) | `dist/panadiga $@` |
| Subcommands | `nvram hdd frm headers scan manuals pico ghidra rag` | Mesmo conjunto | Mesmo conjunto |

> [!CAUTION]
> `run.ps1` **está completamente desatualizado**: procura binários em `dist_bin\` (que não existe), chama `build_all.ps1` (que não existe), e referencia `panadiga-cli.exe` como binário separado — arquitetura abandonada. **Este arquivo está quebrado e nunca funciona.**

### 3.2 Fix recomendado para run.ps1

Alinhar com `run.cmd`: procurar em `dist\panadiga.exe` e `target\release\panadiga.exe`, usar `build.ps1` como fallback.

---

## 4. Arquivos Auxiliares de Build — Redundâncias

### 4.1 Duplicação scripts de qualidade

| Conjunto | Arquivos | Status |
|---|---|---|
| Validation | `validate_all.cmd`, `validate_all.ps1`, `validate_all.sh` | Necessário (3 plataformas) |
| Build | `build.cmd`, `build.ps1`, `build.sh` | Parcialmente necessário — `build.ps1` **duplica** `build.cmd` com diferenças sutis |
| Run | `run.cmd`, `run.ps1`, `run.sh` | `run.ps1` **quebrado** e redundante com `run.cmd` |
| Audit | `generate_audit_bundle.cmd`, `generate_audit_bundle.ps1` | Um thin wrapper CMD basta — `.ps1` redundante |

### 4.2 Arquivos `.jscpd.json` duplicados

```
.jscpd.json                          ← config raiz
apps/desktop/.jscpd.json             ← config frontend (separada)
```

O `validate_all` usa a da raiz; `package.json` do desktop define `jscpd src --threshold 5` (threshold **5**, diferente de 0 da raiz). **Inconsistência**: o limiar real de aceitação varia conforme qual config é carregada.

### 4.3 Duplicação Python ↔ Rust — RAG Engine

```
src/rag/*.rs          ← Motor RAG em Rust (integrado ao binário)
tools/rag_engine/*.py ← Motor RAG em Python (standalone CLI)
```

Ambos implementam `indexer`, `retriever`, `generator`, `critic`, `guardrails`, `pipeline`. São intencionalmente **duplicados em duas linguagens**, sem partilhar contratos formais (schema JSON, protobuf, etc.). Isso cria risco de drift semântico.

> [!NOTE]
> Se a intenção é ter ambos: documentar isso explicitamente e criar testes de contrato (input/output idênticos para o mesmo dado). Se a intenção é ter apenas um: o Rust é o SSOT natural e o Python pode ser wrapper.

### 4.4 Duplicação Ghidra scripts

```
tools/ghidra_conectar_debugger.py    ← PT-BR
tools/ghidra_connect_debugger.py     ← EN
tools/ghidra_importar_simbolos_panadiga.py  ← PT-BR
tools/ghidra_import_symbols_panadiga.py     ← EN
```

**4 scripts são 2 scripts com dois idiomas.** Manter apenas a versão EN (padrão de engenharia) com comentários PT-BR inline, ou usar i18n para Ghidra scripts.

---

## 5. Organização Física do Acervo

### 5.1 Estrutura atual

```
Panadiga/
├── apps/desktop/          ← Frontend React/Tauri (TypeScript)
├── src/                   ← Backend Rust (lib + bin)
│   ├── cli/               ← Clap CLI entrypoint
│   ├── crypto/            ← Análise criptográfica FRM
│   ├── desktop/           ← Comandos Tauri IPC + pico serial
│   ├── hdd/               ← Forense MEIHFS
│   ├── nvram/             ← Unbrick NVRAM U99
│   ├── pico/              ← Flash + diagnóstico RP2040/RP2350
│   │   └── firmware_bins/ ← UF2 pré-compilados (SSOT)
│   └── rag/               ← Motor RAG Rust
├── dwire32a_jtag_research/ ← Firmware C11 + CMake + GDB RSP
├── tools/                 ← Python CLI, RAG, Ghidra scripts
│   └── rag_engine/        ← Motor RAG Python
├── tests/                 ← Testes de integração Rust
├── dist/                  ← Artefatos de deploy (gerado)
└── gen/android/           ← Projeto Android gerado pelo Tauri
```

### 5.2 Problemas de organização

| ID | Sev | Localização | Problema |
|---|---|---|---|
| ORG-01 | **MÉDIO** | `src/desktop/pico/` e `src/pico/` | **Dois módulos pico** no Rust: `src/pico/` (flash/diagnóstico CLI) e `src/desktop/pico/` (serial/GDB IPC). Nomes confusos; deveria ser `src/pico/cli/` e `src/pico/ipc/` |
| ORG-02 | **MÉDIO** | `tools/rag_engine/` | RAG duplicado em Python e Rust sem contrato explícito |
| ORG-03 | **BAIXO** | `tools/` | Mix de: Ghidra scripts, CLI, MCP server, HxD binário, scripts .cmd — deveria ser segmentado em `tools/ghidra/`, `tools/cli/`, `tools/mcp/`, `tools/bin/` |
| ORG-04 | **BAIXO** | Raiz | `Cargo.toml.bak` — deletar |
| ORG-05 | **BAIXO** | Raiz | `.mypy_cache/` commitado (deveria estar no `.gitignore`) |
| ORG-06 | **BAIXO** | Raiz | `.pytest_cache/` commitado (deveria estar no `.gitignore`) |
| ORG-07 | **INFO** | `dwire32a_jtag_research/` | O nome é descritivo mas longuíssimo para um diretório de primeiro nível; considerar `firmware/` ou `pico-fw/` |

### 5.3 Estrutura recomendada

```
Panadiga/
├── apps/desktop/          ← Frontend (sem mudança)
├── src/                   ← Backend Rust
│   ├── cli/               ← (sem mudança)
│   ├── crypto/            ← (sem mudança)
│   ├── desktop/commands/  ← IPC Tauri
│   ├── hdd/               ← (sem mudança)
│   ├── nvram/             ← (sem mudança)
│   ├── pico/
│   │   ├── cli/           ← (era src/pico/)  flash, diagnostics
│   │   ├── ipc/           ← (era src/desktop/pico/) serial, GDB bridge
│   │   └── firmware_bins/ ← UF2
│   └── rag/               ← (sem mudança)
├── firmware/              ← (era dwire32a_jtag_research/)
│   ├── CMakeLists.txt
│   ├── pico_sdk_import.cmake
│   └── ...
├── tools/
│   ├── ghidra/            ← scripts Ghidra (consolidar PT/EN)
│   ├── mcp/               ← panadiga_mcp_server.py
│   ├── rag/               ← rag_engine Python
│   ├── bin/               ← HxD, executables
│   └── cli.py             ← panadiga_cli.py
└── tests/                 ← (sem mudança)
```

---

## 6. Erros Possíveis no Build/Deploy

| ID | Sev | Plataforma | Cenário | Mitigação |
|---|---|---|---|---|
| ERR-01 | 🔴 **CRÍTICO** | Windows | `Compress-Archive` falha se qualquer arquivo listado não existir (sem `WebView2Loader.dll` em CI limpo) | Guard `if (Test-Path ...)` para cada item antes do array |
| ERR-02 | 🔴 **CRÍTICO** | Todos | Interrupção durante Android build deixa `Cargo.toml` com `crate-type = ["staticlib","cdylib","rlib"]` — desktop build seguinte falha | Usar `trap` / `try/finally` (já em `build.ps1`, faltando em `build.sh`) |
| ERR-03 | 🟠 **ALTO** | Windows CI | `CI` env com trailing space causa `--ci 1 ` para clap (analisado antes) — sanificação presente só em `build.cmd`, faltando em `build.ps1` e CI scripts | Adicionar sanitização em `build.ps1` |
| ERR-04 | 🟠 **ALTO** | Todos | `run.ps1` referencia `dist_bin\`, `build_all.ps1` que não existem — crash garantido | Reescrever `run.ps1` |
| ERR-05 | 🟠 **ALTO** | Windows | `tauri.conf.json` tem `"csp": null` — desabilita Content Security Policy totalmente | Definir CSP restritiva: `"default-src 'self'; script-src 'self'"` |
| ERR-06 | 🟡 **MÉDIO** | Linux/macOS | `build.sh` copia UF2 do path legado `dwire32a_jtag_research/`, não do SSOT `src/pico/firmware_bins/` — UF2 pode estar desatualizado | Adicionar check de path primário igual ao `build.cmd` |
| ERR-07 | 🟡 **MÉDIO** | Todos | `validate_all.cmd` Gate 8 usa `gcc -std=c11` mas não tem fallback para MSVC/Clang como o `.ps1` | Usar PowerShell Gate 8 como referência |
| ERR-08 | 🟡 **MÉDIO** | Android | `build.cmd` Android muta `Cargo.toml` via regex string — se a string não casar exatamente, restauração silenciosamente falha | Usar arquivo separado `Cargo.android.toml` ou features Cargo |
| ERR-09 | 🟡 **MÉDIO** | Todos | `Cargo.toml` não especifica `rust-version` mínima — builds podem falhar em Rust antigo sem mensagem clara | Adicionar `rust-version = "1.80"` |
| ERR-10 | 🟡 **MÉDIO** | Windows | `validate_all.cmd` Gate 4 usa `npx --prefix apps\desktop jscpd --config .jscpd.json` — a flag `--prefix` em `npx` não garante que o config da raiz seja usado; pode variar | Usar `npx jscpd` a partir da raiz explicitamente |
| ERR-11 | 🔵 **BAIXO** | Todos | `flasher.rs` `RP2040_FAMILY_ID = 0xE4849208` (spec Microsoft) não bate com IDs reais do SDK (`0xE48BFF56`/`0xE48BFF57`) — log retorna "Pico-Compatible" em vez de "RP2040" | Adicionar ambos os IDs no match |

---

## 7. Arquitetura MVVM / FP — Validação

### 7.1 Camadas identificadas

```
┌─────────────────────────────────────────────────────┐
│  View (React TSX)                                   │
│  apps/desktop/src/views/                            │
│  apps/desktop/src/components/                       │
├─────────────────────────────────────────────────────┤
│  ViewModel (Custom Hooks)                           │
│  apps/desktop/src/viewmodels/use*.ts                │
├─────────────────────────────────────────────────────┤
│  Service / IPC Layer                                │
│  apps/desktop/src/services/ipc.ts                   │
│  apps/desktop/src/services/mockDispatcher.ts        │
├─────────────────────────────────────────────────────┤
│  Model (TypeScript interfaces)                      │
│  apps/desktop/src/models/                           │
│  apps/desktop/src/types/domain.ts                   │
├─────────────────────────────────────────────────────┤
│  Rust Backend (Commands)                            │
│  src/desktop/commands/*.rs                          │
│  src/desktop/commands/dto.rs (DTO = Model Rust)     │
└─────────────────────────────────────────────────────┘
```

### 7.2 Conformidade MVVM

| Regra MVVM | Conformidade | Notas |
|---|---|---|
| View não contém lógica de negócio | ✅ | Views delegam para ViewModels |
| ViewModel expõe apenas estado observável | ✅ | `useState` + retorno de objeto plano |
| ViewModel não importa componentes View | ✅ | Sem imports circulares |
| Separação Model/ViewModel | ✅ | `models/` separa interfaces de hooks |
| IPC centralizado | ✅ | `services/ipc.ts` como único ponto |
| Mock transparente | ✅ | `isTauri()` guard + `mockDispatcher` |
| ViewModel testável sem DOM | ✅ | Vitest testa hooks diretamente |

### 7.3 Conformidade Programação Funcional (Rust)

| Princípio FP | Conformidade | Notas |
|---|---|---|
| Imutabilidade por padrão | ✅ | `let` imutável padrão; `mut` explícito quando necessário |
| Funções puras | ✅ | `headers.rs`, `crypto/`, `hdd/` são majoritariamente puros |
| Sem efeitos colaterais ocultos | ⚠️ | `backup.rs:52-55` salva no CWD sem path absoluto (bug FW-03) |
| Error handling explícito | ✅ | `anyhow::Result<T>` e `thiserror` em toda a codebase |
| Composição de funções | ✅ | Pipeline de análise é composicional |
| Sem estado global mutável | ⚠️ | `ipc.ts` usa `let demoModeActive = false` como singleton mutável — no Rust não há equivalente, mas TS tem |

### 7.4 Conformidade FP — Python (tools/)

| Princípio | Conformidade | Notas |
|---|---|---|
| Mypy strict | ✅ | Validado no Gate 9 |
| Funções tipadas | ✅ | Typer + anotações completas |
| Sem mutação de estado global | ⚠️ | RAG indexer mantém estado em disco (normal para RAG, mas não documentado) |

### 7.5 Problemas de arquitetura

| ID | Sev | Localização | Problema |
|---|---|---|---|
| ARCH-01 | **MÉDIO** | `src/pico/` vs `src/desktop/pico/` | Dois módulos Rust com nome `pico` em paths diferentes — confunde tooling e contribuidores |
| ARCH-02 | **MÉDIO** | `services/ipc.ts` | `demoModeActive` é singleton mutável global em módulo ES — em ambiente de testes paralelos pode causar vazamento de estado entre testes |
| ARCH-03 | **BAIXO** | `tools/rag_engine/` | RAG Python e RAG Rust sem contrato formal compartilhado (protobuf, JSON schema, OpenAPI) |
| ARCH-04 | **BAIXO** | `src/desktop/commands/dto.rs` | DTOs Rust não geram automaticamente tipos TypeScript — qualquer mudança exige atualização manual dos `models/` TS |

---

## 8. Libs Recomendadas — Substituições para Custom Code

### 8.1 Build System

| Código Custom | Lib Recomendada | Vantagem |
|---|---|---|
| `build.cmd` + `build.ps1` + `build.sh` (3 scripts paralelos) | [`just`](https://github.com/casey/just) ou [`cargo-make`](https://github.com/sagiegurari/cargo-make) | Um único arquivo, multiplataforma real, sem duplicação |
| Zip + SHA256 inline em PowerShell | Nativo no `just`/`cargo-make` via `tar` e `sha256sum` | Já incluído |

### 8.2 IPC / DTO Rust ↔ TypeScript

| Código Custom | Lib Recomendada | Vantagem |
|---|---|---|
| `dto.rs` manual + `models/*.ts` manual | [`specta`](https://github.com/oscartbeaumont/specta) + [`tauri-specta`](https://github.com/oscartbeaumont/tauri-specta) | Gera tipos TypeScript **automaticamente** a partir de structs Rust; elimina drift ARCH-04 |

```rust
// Com tauri-specta:
#[tauri::command]
#[specta::specta]
pub fn get_system_overview() -> SystemOverview { ... }
// → gera automaticamente SystemOverview.ts
```

### 8.3 UF2 Validation / Flashing

| Código Custom | Lib Recomendada | Vantagem |
|---|---|---|
| `flasher.rs` (parse UF2 manual) | [`uf2-rs`](https://crates.io/crates/uf2) | Parser UF2 auditado, suporta todos Family IDs corretos incluindo `0xE48BFF56` |

### 8.4 Serial Port + Protocol

| Código Custom | Lib Recomendada | Vantagem |
|---|---|---|
| `serial.rs` read loop manual | [`tokio-serial`](https://crates.io/crates/tokio-serial) | Async, timeout nativo, cancelação limpa |
| `gdb_bridge.rs` GDB RSP manual | [`probe-rs`](https://probe.rs/) | Implementação GDB RSP completa e testada para RP2040/RP2350; suporta DWire/SWD |

### 8.5 RAG Engine

| Código Custom | Lib Recomendada | Vantagem |
|---|---|---|
| `tools/rag_engine/` (Python custom) | [`llama-index`](https://github.com/run-llama/llama_index) ou [`haystack`](https://haystack.deepset.ai/) | Indexers, retrievers e pipelines prontos com LLM integrado |
| `src/rag/` (Rust custom) | [`candle`](https://github.com/huggingface/candle) + [`fastembed-rs`](https://crates.io/crates/fastembed) | Embeddings e retrieval em Rust puro sem dependência Python |

### 8.6 Cargo Toml Mutation para Android

| Código Custom | Alternativa | Vantagem |
|---|---|---|
| Regex mutation de `Cargo.toml` em runtime | Cargo `[features]` + `cfg(target_os = "android")` | Sem mutation, sem risco de Cargo.toml corrompido |

```toml
# Cargo.toml — sem mutation:
[lib]
crate-type = ["rlib"]  # desktop padrão

[target.'cfg(target_os = "android")'.lib]
crate-type = ["cdylib"]  # Android automático via target cfg
```

### 8.7 Validação Duplicação de Código

| Código Custom | Lib Recomendada | Vantagem |
|---|---|---|
| `jscpd` (já em uso) | ✅ Manter — bem configurado | — |
| `uxlint` (já em uso) | ✅ Manter | — |

---

## 9. Resumo Executivo — Prioridades

### 🔴 Crítico (bloqueia build/deploy)

1. **ERR-01** — `build.ps1` `Compress-Archive` falha sem `WebView2Loader.dll`
2. **ERR-04** — `run.ps1` completamente quebrado (referencia artefatos inexistentes)
3. **ERR-05** — CSP `null` no `tauri.conf.json` — vulnerabilidade de segurança XSS

### 🟠 Alto (degradação funcional)

4. **BD-01** — `build.sh` copia UF2 do path errado (legado)
5. **BD-03** — CMake local em `dwire32a_jtag_research/` não detectado
6. **ERR-03** — Sanitização `CI` env ausente em `build.ps1`
7. **ERR-08** — Mutation Android `Cargo.toml` sem recuperação robusta

### 🟡 Médio (qualidade/manutenção)

8. **ORG-01** — Dois módulos `pico` em paths conflitantes
9. **ARCH-04** — DTOs Rust/TS sem geração automática (usar `tauri-specta`)
10. **BD-08** — `Cargo.toml.bak` deletar
11. **ORG-05/06** — `.mypy_cache/` e `.pytest_cache/` no `.gitignore`

### 🔵 Baixo / Recomendações

12. Consolidar scripts com `just` ou `cargo-make`
13. Usar `uf2-rs` para validação UF2 (Fix FW-01 FamilyID)
14. Usar `tauri-specta` para tipos TypeScript automáticos
15. Consolidar scripts Ghidra PT-BR/EN
16. Documentar contrato formal RAG Rust ↔ Python

---

*Auditoria gerada automaticamente pela análise do repositório em 2026-11-11.*
