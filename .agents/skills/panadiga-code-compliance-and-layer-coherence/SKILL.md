---
name: panadiga-code-compliance-and-layer-coherence
description: >-
  Compliance de código cross-stack e coerência 100% entre todas as camadas do Panadiga
  (profiles/ JSON SSOT, panadiga-profiles, panadiga-core, src-tauri IPC, apps/desktop/src
  TypeScript/Zod/locales, firmware C11 Pico, scripts Python e tools/ghidra). Use ao auditar
  contratos, eliminar duplicações JSON/constantes ou projetar codegen declarativo único.
---

# Compliance de Código & Coerência 100% Entre Camadas (SSOT Hardening)

Esta skill audita e impõe conformidade rigorosa entre todas as camadas do ecossistema, eliminando silos JSON paralelos, enums divergentes, constantes duplicadas e pontes IPC sem geração de tipos.

## 1. Quando Usar & Handoffs
- Ative ao alterar qualquer constante de hardware, modelo, schema JSON, comando IPC Tauri, dicionário i18n, tabela C11 do Pico ou `TypedDict` Python.
- Combine com `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md` para modularidade e `.agents/skills/panadiga-validate/SKILL.md` para gates automatizados.
- Mapa completo da arquitetura SSOT unificada (Compiled Hardware IR + IPC Codegen + Typed EventBus + Append-Only Journal) e dívidas reais encontradas em [cross-layer-contracts-and-ssot-coherence.md](./references/cross-layer-contracts-and-ssot-coherence.md).

## 2. Invariantes de Coerência Cross-Layer
1. **SSOT Declarativa Única (Sem Silos Órfãos):** Toda especificação estática de hardware, pinagem, limiar elétrico, mapa de memória e catálogo de comandos deve ter JSON Schema + origem única em `profiles/` (ou catálogo schemado associado) com geração automática para Rust, TS, C11, Python e Ghidra — **nunca** mantida à mão em múltiplos arquivos.
2. **Proibido Arquivo "Generated" Editado à Mão:** Se um arquivo vive em `*/generated/` (como `apps/desktop/src/models/generated/profiles.ts`), ele **DEVE** ser emitido por `panadiga ssot export-all` e verificado em CI (`git diff --exit-code` após `export-all`).
3. **Simetria de Segurança Entre C11, Rust e TS:** Faixas de segurança física (ex.: GPIOs livres de IR `GP6..9 | GP16..22`, teto de timeout NOR `185_000 ms`, `SA0..SA10` IPL RO) devem ser idênticas nas 3 camadas — jamais mais frouxas no Rust do que no C11.
4. **Isolamento de Idioma/UI vs Core:** O crate `panadiga-core` **não deve** depender de `apps/desktop/src/locales/pt-BR/` via `include_str!`; prose localizada pertence à camada de apresentação (`locales/`).
5. **Verificação de Versão & Hash de Contrato:** `ProfilesBundle.schema_version` deve ser validado em tempo de carga (`validate.rs`), e `P7501_SSOT_HASH` deve cobrir macros + tabelas e ser validado entre Host e Pico no handshake `IDENTIFY`.

## 3. Protocolo de Auditoria de Compliance Cross-Layer
```bash
# 1. Validar todos os relacionamentos e invariantes SSOT (796+ checks)
cargo run -q -p panadiga-cli -- profiles validate -j -

# 2. Regenerar projeções SSOT e inspecionar se há deriva não-commitada
cargo run -q -p panadiga-cli -- ssot export-all --root .
git diff --stat apps/desktop/src/models/generated/ firmware/pico-p7501/generated/ tools/ghidra/ profiles/schema/

# 3. Verificar compliance em todas as linguagens
cargo clippy --workspace --all-targets --no-deps -- -D warnings
npm --prefix apps/desktop run lint
npx --prefix apps/desktop jscpd --config .jscpd.json
python -m mypy --strict scripts
```

## 4. Armadilhas & Anti-Padrões
- Criar workarounds em runtime (como duplicar chaves `snake_case`/`camelCase` no `ipc.ts`) para mascarar falta de codegen de tipos IPC entre Rust e TypeScript.
- Ignorar `**/*.json` e `**/generated/**` no `jscpd` e achar que não existe duplicação semântica de constantes entre `.json`, `.rs`, `.ts` e `.h`.
- Copiar apenas subconjuntos de modelos (`TARGET_PROFILES_ORDER = 4/89`) quando a SSOT cresce para dezenas de aparelhos.

## 5. Verificação
- Zero deriva entre `profiles/`, `profiles/schema/`, código Rust, TS `models/generated/`, C11 `generated/` e `scripts/`; todos os checks de compliance passando sem warnings.
