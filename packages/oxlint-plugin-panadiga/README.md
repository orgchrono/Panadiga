# @orgchrono/oxlint-plugin

Architectural Oxlint JS plugin enforcing Panadiga frontend invariants:

- `panadiga/no-unsafe-type-cast`: Disallows `any`, `<T>expr`, `as unknown`, `as any`, chained assertions, and production narrowing casts (permitting `as const` and `satisfies`).
- `panadiga/no-let-or-mutation`: Enforces functional immutability (disallows `var`, loops, `++`/`--`, mutating array/collection methods, OOP classes, and external reassignments), permitting `let` **only** when lexically encapsulated inside an explicit non-UI `create*` closure factory whose direct binding never leaks.
- `panadiga/no-global-state-or-side-effects`: Disallows `declare global`, top-level side effects, module-scope mutable maps, and direct browser globals outside storage/platform/pointer hook adapters.
- `panadiga/mvvm-strict-boundaries`: Enforces the unidirectional Clean Architecture / MVVM DAG and isolates `@tauri-apps/*` and IPC calls strictly inside `src/services/`.
- `panadiga/zero-hardcode`: Bans hardcoded error literals, inline `t(k, fallback)` strings, user-facing prose outside `src/locales/`, magic hardware constants outside `src/constants/`, and raw `console.*`.
- `panadiga/no-lint-suppression`: Disallows inline `eslint-disable` / `@ts-ignore` / `@ts-expect-error` directives and custom `.d.ts` files outside `vite-env.d.ts`.

## Consumption & GitHub Packages

Inside this monorepo, `apps/desktop` consumes `@orgchrono/oxlint-plugin` offline via `"file:../../packages/oxlint-plugin-panadiga"`. To publish externally to GitHub Packages:

```bash
npm publish --workspace=packages/oxlint-plugin-panadiga
```
