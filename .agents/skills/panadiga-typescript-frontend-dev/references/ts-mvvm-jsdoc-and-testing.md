# Frontend TypeScript (`apps/desktop`): MVVM DAG, Oxlint, JSDoc & Testes

## 1. Hierarquia Estrita das 8 Camadas MVVM (`apps/desktop/src/`)
1. `models/` · `types/` · `constants/`: contratos puros, DTOs gerados (`models/generated/profiles.ts`), constantes de hardware (sem dependências de UI/DOM/Tauri).
2. `locales/` · `i18n/`: dicionários modulares `en-US/*.json`, `pt-BR/*.json` (`core`, `workspace`, `hdd`, `crypto`, `pico`, `ssot`, `errors`) + `locales/ir/{en-US,pt-BR}.json`.
3. `utils/` · `lib/` · `config/`: utilitários FP puros, `typeGuards.ts`, `runtimeSettingsSchema.ts` (**Zod 4**), `logger.ts` (**Pino 10** + **OTel**).
4. `services/`: **única camada autorizada** a importar `@tauri-apps/*` e chamar `invokeCommand`/`listenEvent`/`emitLocalEvent`.
5. `store/`: Redux Toolkit + persistência isolada em `store/storage.ts`.
6. `hooks/` · `context/` · `viewmodels/`: composição reativa (`services/` + `store/`), sem JSX apresentacional acoplado.
7. `components/`: componentes presentacionais puros (`Radix UI` + `Tailwind CSS`) — **PROIBIDO** importar `services/`, `store/`, `context/`, `viewmodels/` ou `@tauri-apps/*`.
8. `views/` · `App.tsx`: páginas (`ExpressView`, `SettingsView`, `IrCommanderView`, etc.) conectando ViewModels a Components.

## 2. As 7 Regras Customizadas do `@orgchrono/oxlint-plugin` (`apps/desktop/.oxlintrc.json`)
- `panadiga/no-unsafe-type-cast`: bane `any`, `<T>expr`, `as any`, `as unknown`, `as Type` em produção (aceita `as const`, `satisfies`, `zod`, `oneOfGuard`, type predicates).
- `panadiga/no-let-or-mutation`: bane `var`, `for`/`while`, `++`/`--`, `delete`, `class`, `.push`/`.pop`/`.splice`/`.sort`/`.reverse`, `Object.assign`; aceita `let` só dentro de closure factory não-UI `create*` sem vazamento.
- `panadiga/no-global-state-or-side-effects`: bane `window`/`document`/`localStorage`/`navigator` fora de `services/`, `store/storage.ts`, `config/platform.ts`, `hooks/usePointerDrag.ts` e `main.tsx`.
- `panadiga/mvvm-strict-boundaries`: impõe a DAG unidirecional acima.
- `panadiga/zero-hardcode`: bane literais de erro em `throw new Error(...)`, `t('k', 'fallback')`, prose em JSX/services/stores/viewmodels, magic numbers (exceto `-1, 0, 1, 2, 10, 16, 100`) e `console.*`.
- `panadiga/no-lint-suppression`: bane `eslint-disable`, `oxlint-disable`, `@ts-ignore`, `@ts-expect-error`, `@ts-nocheck` e `.d.ts` extras fora `vite-env.d.ts`.
- `panadiga/ux-workbench-integrity`: exige `<button type="button|submit|reset">` e `style={{ '--var': ... }}` apenas para custom properties CSS.

## 3. Padrão Canônico JSDoc / TSDoc
```ts
/**
 * Executa pré-auditoria read-only da região NVRAM via IPC Tauri sem mutar o dump original.
 *
 * @param imagePath Caminho absoluto ou de sessão para o dump Spansion de 8 MiB.
 * @returns Resumo validado por schema Zod dos 142 setores auditados.
 * @throws Erro traduzido via `errors.*` caso a camada IPC rejeite o arquivo.
 */
```
- **Obrigatório em:** interfaces de contrato (`models/`), wrappers IPC (`services/`), hooks/viewmodels (`hooks/`/`viewmodels/`), fábricas `create*` e validadores/parsers (`utils/`). Inclua invariantes de camada (ex.: "camada `services`: nunca chamar direto de `components/`").

## 4. Vitest & Playwright Anti-Flaky (`src/test/mocks/` + `apps/desktop/e2e/`)
- **Isolamento de Mocks:** Mocks de teste residem **exclusivamente** em `apps/desktop/src/test/mocks/` (jamais importados em produção).
- **Paridade i18n:** `translations.test.ts` falha se faltar qualquer chave entre `en-US` e `pt-BR`.
- **Playwright (`12` specs em `e2e/`):** Configurado em `playwright.config.ts` (`baseURL: http://localhost:1420`, `viewport: 1440x900`, `expect.timeout: 25000`). Use seletores semânticos (`getByRole`, `getByLabel`, `data-testid`) e esperas por estado visível/IPC — nunca `waitForTimeout` fixo.
