---
name: panadiga-ux-ui-and-layout
description: >-
  Arquitetura UX/UI, layout responsivo Desktop + Android enableEdgeToEdge, Design System
  Tailwind 3.4 + Radix UI + 4 temas (dark-lab, high-contrast AAA, daylight-bench, amber-crt),
  flags de acessibilidade, ux-guard 100/100, ExpressView vs Engineering Mode e segurança de
  renderização WebView Tauri 2 (CSP, withGlobalTauri, confinamento IPC e bridges Android).
  Use ao projetar layouts, corrigir responsividade/a11y ou auditar segurança da WebView.
---

# UX/UI, Layout Responsivo, Acessibilidade & Segurança da WebView

Esta skill cobre o design de interface do Panadiga Workbench (Desktop e Android), acessibilidade real (`ux-guard` + `jsx-a11y`) e a postura de segurança obrigatória ao renderizar ferramentas críticas via Tauri 2 WebView.

## 1. Quando Usar & Handoffs
- Ative ao desenhar novas telas/cards, ajustar responsividade (`h-dvh`, drawers, console inferior), configurar temas/a11y ou endurecer a fronteira WebView <-> Rust/Kotlin.
- Para regras de TypeScript/MVVM DAG, Oxlint e JSDoc, combine com `.agents/skills/panadiga-typescript-frontend-dev/SKILL.md`.
- Para checagem de segurança global e bloqueios de hardware no backend, combine com `.agents/skills/panadiga-security-and-violation-guards/SKILL.md`.
- Design System, bugs reais encontrados no layout/config `uxlint` e matriz completa de segurança da WebView em [ux-layout-a11y-and-webview-security.md](./references/ux-layout-a11y-and-webview-security.md).

## 2. Invariantes de UX/UI & Layout
1. **Fluxo Guiado Primeiro (`ExpressView` Padrão):** Novos usuários iniciam em `ExpressView` (4 missões guiadas + `StrictWorkflowStepper`); o modo avançado (`Engineering`) expõe 9 views desktop + `IrCommanderView` (`--mode android` apenas).
2. **Viewport Real em Mobile/Android (`h-dvh`, NUNCA `h-screen`):** Com `enableEdgeToEdge()` no Android (`MainActivity.kt`) e `env(safe-area-inset-*)` no `index.css`, usar `h-screen` (`100vh`) soma paddings e corta a `WorkbenchStatusBar` na base! Use `h-dvh` e limites como `max-h-[45dvh]` no console inferior.
3. **Semântica & Acessibilidade Real:** Use Radix UI, regiões vivas (`role="log"`, `aria-live`), `<button type="...">` obrigatório (`panadiga/ux-workbench-integrity`), `role="group"` em `<div>` (evite `<fieldset>` sem `<legend>`) e mantenha controles essenciais visíveis em `<640px`.
4. **Regra `ux-guard` (`100/100`) & Config Real:** `apps/desktop/package.json` executa `ux-guard scan src --fail-on info`. **Atenção:** `ux-guard-core` procura `ux-guard.config.{json,js,...}` com chaves camelCase (`loadingState`, `emptyState`, `errorState`, `formValidation`, `disabledSubmit`, `successFeedback`, `"exclude"`) — não o antigo `uxlint.config.json` ignorado!

## 3. Invariantes de Segurança da Renderização WebView (Tauri 2)
1. **Tratar a WebView Como Sandbox Não-Confiável:** A UI pode prevenir erros acidentais do operador, **mas toda trava de segurança real** (confirmação física, lockouts OS/NVMe, bloqueio `SA0..SA10`, confinamento de paths `canonicalize()`, sanitização Kotlin `@JavascriptInterface`) **DEVE residir no backend Rust/Kotlin/Pico**.
2. **CSP Estrita & Sem Global Leaked:** `tauri.conf.json` deve manter `"withGlobalTauri": false` (usar imports ESM `@tauri-apps/api/*`) e `"script-src 'self'"` **sem `'unsafe-inline'`** em scripts.
3. **Mínimo Privilégio de Capabilities & Plugins:** Nunca habilite plugins não utilizados (`@tauri-apps/plugin-shell` sem uso) nem abra paths arbitrários do filesystem a partir de argumentos brutos da WebView.
4. **Bridges Android `@JavascriptInterface`:** Todo payload recebido de JS (`sysfsPayload` no `AndroidIrExecutor.kt`) deve ser validado por regex estrita antes de tocar `sysfs` ou `ProcessBuilder` — **jamais** interpolar strings de JS em shell `su`.

## 4. Verificação
```bash
npm --prefix apps/desktop run uxlint
npm --prefix apps/desktop run lint
npm --prefix apps/desktop run test:e2e
```
Confirme `100/100` em `ux-guard`, `0` avisos `jsx-a11y` e layout desktop/mobile (`390x844` / `1440x900`) sem overflow vertical.
