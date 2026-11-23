# Design System, Layout Audit & Segurança de Renderização WebView (Tauri 2)

## 1. Design System & Temas (`apps/desktop/vite.config.ts` & `src/index.css`)
- **Tokens Tailwind Inline (`vite.config.ts:7-39`):** Paletas `bench-{950,900,850,800}` e `diga-*` + stack monospace `JetBrains Mono` / `Fira Code` (sem `tailwind.config.*` separado).
- **4 Temas via `data-theme` no `<html>` (`index.css:22-145`):** `dark-lab` (padrão), `high-contrast` (WCAG AAA), `daylight-bench` (claro para bancada iluminada), `amber-crt`; flags `data-colorblind="true"`, `data-reduced-motion="true"` (`<MotionConfig>`), `data-font-scale="large"` (`17.5px`).
- **Modularidade:** Zero componente TSX de produção acima de 350 linhas (maior: `SdKitBuilderCard.tsx` com 332 L).

## 2. Achados de Layout & Config (`apps/desktop/`)
- **Config Morta de `uxlint`:** `apps/desktop/uxlint.config.json` usa nome errado e chaves snake_case (`missing_loading_state`); `ux-guard-core` lê `ux-guard.config.json` (camelCase + `"exclude"`).
- **Overflow em Android Edge-to-Edge:** `App.tsx:154` e `ExpressView.tsx:32` usando `h-screen` somam com `env(safe-area-inset-*)` e cortam a barra inferior; usar `h-dvh` e cap `max-h-[45dvh]` no console (`workbench-bottom-console.tsx:70`).
- **Modo Toggle Oculto em `<640px` (`Header.tsx:209`):** `hidden sm:inline-flex` esconde alternância rápida no celular retrato.
- **Copy Obsoleta de UI:** `workbench-right-dock.tsx:41` corrigido de badge legado para `'SA0..SA10 RO'`.

## 3. Segurança da WebView Tauri 2 — Veredito Técnico
**Renderizar via WebView Tauri 2 É seguro para este domínio se — e somente se — as 5 camadas abaixo estiverem fechadas:**
1. **CSP & Superfície JS:** `withGlobalTauri: false`, `script-src 'self'` (sem `'unsafe-inline'`), zero `eval`/`dangerouslySetInnerHTML`, `window.open` restrito a `https?:/mailto:` com `noopener,noreferrer`.
2. **Confinamento de Filesystem no Rust:** Comandos IPC de leitura/escrita (`read_hex_dump`, `resolve_read_path`, `resolve_write_path`) devem restringir paths via `canonicalize()` dentro de diretórios autorizados (workspace, vault, temp, arquivos abertos explicitamente), em vez de aceitar qualquer path absoluto.
3. **Separação de Autoridade (Sem Tokens Forjáveis para Operações Irreversíveis):** Tokens determinísticos `CONFIRM-<dev>` protegem contra clique acidental, não contra XSS; operações live sensíveis (`pico_send_serial_command` arbitrário, `pico_panamon1_clean_nvram_command`) não devem depender de fluxo irrestrito da WebView.
4. **Bridges Android:** Validar `^[0-9]{1,7}(,[0-9]{1,7}){0,2047}$` antes de gravar `sysfs` IR (`AndroidIrExecutor.kt`); usar `Intent(...).setPackage(packageName)` em `PendingIntent.FLAG_MUTABLE` no Android 14+ (`AndroidPicoBridge.kt:495`); definir `android:allowBackup="false"` em produção.
5. **Alternativa Air-Gapped:** Para flashes irreversíveis ou ambiente hostil, preferir `panadiga` CLI/TUI nativo sem WebView.
