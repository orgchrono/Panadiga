---
name: panadiga-typescript-frontend-dev
description: >-
  Desenvolvimento Frontend em Tauri 2 + React 18.3 + TypeScript 7 + Vite 8 + Tailwind CSS +
  Radix UI + Redux Toolkit + Zod 4 + Pino 10 + i18next 27 + Vitest 5 + Playwright 1.65.
  Enforça as 7 regras @orgchrono/oxlint-plugin (MVVM DAG, FP imutável, zero-hardcode,
  no-unsafe-type-cast, ux-guard 100/100), JSDoc canônico, mocks em src/test/mocks/ e
  isolamento Android --mode android. Use ao editar apps/desktop/src/.
---

# Desenvolvimento Frontend TypeScript (`Tauri 2` / `React 18.3` / `MVVM` / `JSDoc`)

Esta skill define as regras estritas de engenharia do workbench Desktop e Android APK em `apps/desktop/src/` (100% livre de mocks em produção).

## 1. Quando Usar & Handoffs
- Ative ao criar ou editar views, components, hooks, viewmodels, stores, services IPC, schemas Zod, dicionários i18n, testes Vitest ou specs Playwright E2E.
- Para decisões de decomposição arquitetural e anti-duplicação, combine com `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md`.
- Para comandos Rust Tauri novos no backend, combine com `.agents/skills/panadiga-rust-backend-dev/SKILL.md`.
- Tabela exata das 8 camadas MVVM, 7 regras Oxlint, padrões **JSDoc** e guia anti-flaky Vitest/Playwright em [ts-mvvm-jsdoc-and-testing.md](./references/ts-mvvm-jsdoc-and-testing.md).

## 2. Invariantes Não-Negociáveis (`@orgchrono/oxlint-plugin`)
1. **DAG MVVM Unidirecional (`panadiga/mvvm-strict-boundaries`):**
   - `models/`·`types/`·`constants/` -> `locales/`·`i18n/` -> `utils/`·`lib/`·`config/` -> **`services/`** (única camada que importa `@tauri-apps/*`) -> **`store/`** (única camada Redux/`localStorage`) -> `hooks/`·`context/`·`viewmodels/` -> **`components/`** (UI pura: proibido importar `services/`, `store/`, `viewmodels/`, `@tauri-apps/*`) -> `views/`·`App.tsx`.
2. **Estilo FP Imutável (`panadiga/no-let-or-mutation`):** Proibido `var`, loops (`for`, `for..of`, `while`), `++`/`--`, `delete`, `class`, `.push`/`.pop`/`.splice`/`.sort`/`.reverse` e `Object.assign`. `let` é permitido **exclusivamente** dentro de closure factory não-UI nomeada `create*` sem vazamento por referência.
3. **Zero Casts Inseguros (`panadiga/no-unsafe-type-cast`):** Proibido `any`, `<T>expr`, `as any`, `as unknown` e `as Type` em produção; use `as const`, `satisfies`, `zod` (`safeParseWithSchema`), `oneOfGuard` ou type predicates (`v is T`).
4. **Zero Hardcode, Zero `console.*` & Zero Supressões:** Proibido texto para usuário fora de `src/locales/{en-US,pt-BR,ir}/*.json` (paridade 1:1 verificada por `translations.test.ts`), proibido `t(k, 'fallback')`, números mágicos fora de `src/constants/`, `console.*` (use `logger` Pino/OTel em `src/utils/logger.ts`), e `eslint-disable`/`@ts-ignore`/`@ts-expect-error`.
5. **Isolamento Android (`--mode android`) & UX 100/100:** `IrCommanderView` e bridges CIR ficam tree-shaken fora do bundle desktop (`vite.config.ts`). Todo componente interativo exige `<button type="button|submit|reset">`, estilos via Tailwind/CSS vars `--*`, loading, empty state, error e feedback (`ux-guard` `100/100`).

## 3. Padrão JSDoc / TSDoc & Coerência de Camadas
- Documente com JSDoc (`/** ... */`): interfaces de domínio (`models/`), wrappers IPC (`services/`), hooks/viewmodels (`hooks/`/`viewmodels/`), factories `create*` e utilitários puros (`utils/`).
- Descreva **contrato da camada, pré-condições de segurança/Tauri e retorno**, com `@param`/`@returns` em APIs não-óbvias — nunca comente o trivial.

## 4. Workflow Canônico Frontend
```bash
# Dentro de apps/desktop (ou via npm --prefix apps/desktop run ...)
npm --prefix apps/desktop run lint       # oxlint src --deny-warnings && tsc --noEmit
npm --prefix apps/desktop run uxlint     # ux-guard scan src --fail-on info (100/100)
npm --prefix apps/desktop test           # Vitest (mocks isolados em src/test/mocks/)
npm --prefix apps/desktop run check      # lint + uxlint + jscpd + vitest
npm --prefix apps/desktop run test:e2e   # 12 specs Playwright Chromium headless
```

## 5. Armadilhas & Verificação
- **Armadilha:** Esquecer chave equivalente em `locales/pt-BR/` e `locales/en-US/` ou colocar arquivo mock fora de `src/test/mocks/`.
- **Verificação:** `npm run check:ui:all` (na raiz) verde com `0` warnings Oxlint, `0` erros `tsc`, `0` clones `jscpd`, `100/100` `ux-guard` e todos os specs Vitest + Playwright passando.
