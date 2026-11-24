# Coerência Cross-Layer & Evolução Arquitetural da SSOT

## 1. Diagnóstico Arquitetural: Por que a SSOT JSON Atual Soa "Frouxa"?
Hoje não há um único SSOT, e sim `profiles/` + **4 JSONs sem schema** (`hardware_catalog.json`, `ghidra_maps_ssot.json`, `ir_macros_ssot.json`, `firmware_error_dict.json`) + espelhos manuais em TS (`profiles.ts`, `physicalConstants.ts`, `spansionS29jl064hTopology.ts`, `regionPresets.ts`), C11 (`p7501_types.h`), Python (`scripts/profiles_types.py` morto) e Tauri (`commands/constants.rs`), gerando bugs reais como `DIAG_MAX_TIMEOUT_MS = 125_000` vs `185_000`, `GP26` aceito em `settings.rs`, `ProfilesBundle.schema_version` não validado e `PanaRebuildRecipe` ausente em `generated/profiles.ts`.

## 2. Solução Arquitetural Recomendada (Separação em 2 Planos)

| Problema | Mecanismo Errado | Solução Arquitetural Correta |
| :--- | :--- | :--- |
| **Verdade Estática (Hardware, Pinagem, Limiares, IPC, Comandos, Schemas)** | EventBus / Subscriptions em runtime (adiciona complexidade assíncrona a dados imutáveis) | **1. Compiled Hardware IR + Codegen Total:** Unificar os 4 JSONs órfãos sob JSON Schema + emitir `profiles.ts`, `ssot.ts`, `p7501_ssot.h` (com macros via IR), `profiles_types.py` e contratos IPC tipados (`ts-rs` / `tauri-specta`), travando tudo com **1 `SSOT_CONTRACT_HASH`** checado no CI (`git diff --exit-code`) e no `IDENTIFY` do Pico |
| **Estado Dinâmico (Bancada, Serial, Progresso Unbrick, Vault)** | Mutação in-place de manifesto ou `Arc<Atomic*>` isolado sem trilha | **2. Typed EventBus + Append-Only Audit Journal:** Unificar catálogo de eventos Tauri↔TS em um único contrato gerado + gravar operações críticas em **Journal append-only com CRC32/SHA-256 e `schema_version`** (mesmo padrão que o Pico já usa no `P4_BLACKBOX`!) |

## 3. Passos de Migração Incremental Sem Quebrar o Build
1. Validar `bundle.schema_version` no `panadiga-profiles` e gerar `generated/profiles.ts` + `scripts/profiles_types.py` automaticamente via `ssot export-all`.
2. Remover duplicatas manuais em `physicalConstants.ts`, `spansionS29jl064hTopology.ts`, `p7501_types.h` e `src-tauri/commands/constants.rs`, importando do código gerado.
3. Remover `include_str!(".../locales/pt-BR/ssot.json")` de `panadiga-core` (mantendo `core` neutro de idioma e traduzindo chaves na UI).
4. Adicionar `git diff --exit-code` no Gate 7 do CI após `ssot export-all` e validar `12` JSON Schemas contra todos os JSONs de `profiles/` no `test_profiles_ssot.py`.
