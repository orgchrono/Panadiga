---
name: panadiga-typescript-frontend-dev
description: >-
  Desenvolvimento Frontend e UX/UI em Tauri 2 + React 18.3 + TypeScript 7 + Vite 8 +
  Tailwind CSS + Radix UI + Redux Toolkit + Zod 4 + Pino 10 + i18next 27 + Vitest 5 +
  Playwright 1.65. Enforça as 7 regras @orgchrono/oxlint-plugin (MVVM DAG, FP imutável,
  zero-hardcode, no-unsafe-type-cast), layout responsivo h-dvh/enableEdgeToEdge, a11y,
  ux-guard 100/100, JSDoc, mocks em src/test/mocks/ e segurança WebView Tauri 2 / Android.
  Use ao editar apps/desktop/src/ ou projetar telas, a11y e bridges IPC/Android.
---

# Frontend TypeScript, UX/UI Responsiva & Segurança WebView (`Tauri 2` / `React 18.3`)

Esta skill define as regras de engenharia, design de interface, acessibilidade (`ux-guard` + `jsx-a11y`) e segurança de renderização WebView do workbench Desktop e Android APK em `apps/desktop/src/` (100% livre de mocks em produção).

## 1. Quando Usar & Handoffs
- Ative ao criar ou editar views (`ExpressView`, `IrCommanderView`), components, hooks, viewmodels, stores, services IPC, schemas Zod, dicionários i18n, layouts responsivos (`h-dvh`, drawers, console inferior), temas/a11y, testes Vitest ou specs Playwright E2E.
- Para decomposição arquitetural e DRY, combine com `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md`.
- Para comandos Rust Tauri novos no backend ou segurança global de hardware/AppSec, combine com `.agents/skills/panadiga-rust-backend-dev/SKILL.md` e `.agents/skills/panadiga-security-and-violation-guards/SKILL.md`.
- **Referências:**
  - 8 camadas MVVM, 7 regras Oxlint, padrões **JSDoc** e guia anti-flaky Vitest/Playwright em [ts-mvvm-jsdoc-and-testing.md](./references/ts-mvvm-jsdoc-and-testing.md).
  - Design System (4 temas), bugs reais de layout/`ux-guard` e matriz de segurança da WebView em [ux-layout-a11y-and-webview-security.md](./references/ux-layout-a11y-and-webview-security.md).

## 2. Invariantes de Código & Arquitetura (`@orgchrono/oxlint-plugin`)
1. **DAG MVVM Unidirecional (`panadiga/mvvm-strict-boundaries`):** `models/`·`types/`·`constants/` -> `locales/`·`i18n/` -> `utils/`·`lib/`·`config/` -> **`services/`** (única camada que importa `@tauri-apps/*`) -> **`store/`** (única camada Redux/`localStorage`) -> `hooks/`·`context/`·`viewmodels/` -> **`components/`** (UI pura: proibido importar `services/`, `store/`, `viewmodels/`, `@tauri-apps/*`) -> `views/`·`App.tsx`.
2. **Estilo FP Imutável (`panadiga/no-let-or-mutation`):** Proibido `var`, loops (`for`, `for..of`, `while`), `++`/`--`, `delete`, `class`, `.push`/`.pop`/`.splice`/`.sort`/`.reverse` e `Object.assign`. `let` é permitido **exclusivamente** dentro de closure factory não-UI nomeada `create*` sem vazamento por referência.
3. **Zero Casts Inseguros (`panadiga/no-unsafe-type-cast`):** Proibido `any`, `<T>expr`, `as any`, `as unknown` e `as Type` em produção; use `as const`, `satisfies`, `zod` (`safeParseWithSchema`), `oneOfGuard` ou type predicates (`v is T`).
4. **Zero Hardcode, Zero `console.*` & Zero Supressões:** Proibido texto fora de `src/locales/{en-US,pt-BR,ir}/*.json` (`translations.test.ts`), `t(k, 'fallback')`, números mágicos fora de `src/constants/`, `console.*` (use `logger` Pino/OTel em `src/utils/logger.ts`), e `eslint-disable`/`@ts-ignore`/`@ts-expect-error`.
5. **Padrão JSDoc / TSDoc:** Documente interfaces de domínio (`models/`), wrappers IPC (`services/`), hooks/viewmodels, factories `create*` e utilitários com contrato de camada, pré-condições de segurança/Tauri e `@param`/`@returns`.

## 3. Invariantes de UX/UI, Layout Responsivo & Segurança WebView
1. **Fluxo Guiado Padrão & Isolamento Android (`--mode android`):** Novos usuários iniciam em `ExpressView` (`StrictWorkflowStepper`). `IrCommanderView` e bridges CIR ficam tree-shaken fora do bundle desktop (`vite.config.ts`).
2. **Viewport Mobile Real (`h-dvh`, NUNCA `h-screen`):** Com `enableEdgeToEdge()` no Android (`MainActivity.kt`) e `env(safe-area-inset-*)` no `index.css`, `h-screen` (`100vh`) corta a `WorkbenchStatusBar` na base — use sempre `h-dvh` e `max-h-[45dvh]` no console inferior.
3. **Semântica, A11y & `ux-guard` (`100/100`):** Radix UI, `role="log"`/`aria-live`, `<button type="button|submit|reset">` obrigatório (`panadiga/ux-workbench-integrity`), `role="group"` em `<div>` (evite `<fieldset>` sem `<legend>`), controles visíveis em `<640px`. Em `apps/desktop`, use `ux-guard.config.*` com chaves camelCase (`loadingState`, `emptyState`, `errorState`, `formValidation`, `disabledSubmit`, `successFeedback`, `"exclude"`) — não `uxlint.config.json`.
4. **WebView Como Sandbox Não-Confiável (`Tauri 2` & Kotlin):** Toda trava real (confirmação física, lockouts OS/NVMe, `SA0..SA10`, `canonicalize()` de paths) **DEVE residir no backend Rust/Kotlin/Pico**. `tauri.conf.json` mantém `"withGlobalTauri": false`, CSP `"script-src 'self'"` sem `'unsafe-inline'`, zero `tauri_plugin_shell` sem uso; no `AndroidIrExecutor.kt` (`@JavascriptInterface`), valide `sysfsPayload` por regex estrita antes de tocar `sysfs`/`ProcessBuilder` — **jamais** interpole strings JS em `su`.

## 4. Workflow Canônico & Verificação
```bash
npm --prefix apps/desktop run lint       # oxlint src --deny-warnings && tsc --noEmit
npm --prefix apps/desktop run uxlint     # ux-guard scan src --fail-on info (100/100)
npm --prefix apps/desktop test           # Vitest (mocks isolados em src/test/mocks/)
npm --prefix apps/desktop run check      # lint + uxlint + jscpd + vitest
npm --prefix apps/desktop run test:e2e   # Playwright Chromium headless
```
- **Armadilhas:** Esquecer paridade `locales/{en-US,pt-BR}/`, colocar mock fora de `src/test/mocks/`, ou usar `h-screen` em layouts Android.
- **Verificação:** `npm run check:ui:all` verde (`0` warnings Oxlint, `0` erros `tsc`, `0` clones `jscpd`, `100/100` `ux-guard`, desktop `1440x900` e mobile `390x844` sem overflow vertical, Vitest + Playwright 100% PASS).
