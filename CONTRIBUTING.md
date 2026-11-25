# Contributing to Panadiga — Engineering Agreement & Architectural Standards

> **[AGENTS.md](AGENTS.md)** defines AI agent workflows and hardware protocols; **`CONTRIBUTING.md`** (this document) establishes the binding development contract, architectural boundaries, linter rules, business/safety invariants, and quality gates for **all** contributors across Rust, TypeScript/React, C11, and Python.

---

## 1. Core Architectural Philosophy (Rust-First Logic + Pure MVVM Frontend)

1. **Domain Logic Lives in Rust (`crates/panadiga-core`, `panadiga-rag`, `panadiga-mcp`, `apps/desktop/src-tauri`)**:
   - Parsing, cryptography, MEIHFS forensics, Spansion NOR safety bounds, runtime settings validation, and persistence reside in Rust.
   - TypeScript/React renders state cleanly via Tauri IPC adapters and MUST NOT duplicate business rules, physics calculations, or hardware safety logic.
2. **Unidirectional Frontend Clean Architecture / MVVM DAG (`apps/desktop/src/`)**:
   - Strictly enforced by [`@orgchrono/oxlint-plugin`](packages/oxlint-plugin-panadiga/README.md) (`panadiga/mvvm-strict-boundaries`):
     - `models/` · `types/` · `constants/` → leaf contracts & SSOT hardware constants (UI-framework-free).
     - `locales/` · `i18n/` → modular domain dictionaries & translation hooks.
     - `utils/` · `lib/` · `config/` → pure FP utilities, Zod schemas, Pino/OTel logging facade (UI-framework-free).
     - `services/` → **sole layer permitted to import `@tauri-apps/*` or call `invokeCommand`/`listenEvent`/`emitLocalEvent`**.
     - `store/` → **sole layer permitted to import `react-redux` / `@reduxjs/toolkit`** (plus `localStorage`/DOM adapters in `store/storage.ts`).
     - `hooks/` · `context/` · `viewmodels/` → reactive orchestration hooks composing `services/` & `store/`.
     - `components/` → pure presentational UI (MUST NOT import `services/`, `store/`, `context/`, `viewmodels/`, or `@tauri-apps/*`).
     - `views/` · `App.tsx` → compose ViewModels and Components.
3. **Android-Only Feature Isolation**:
   - `IrCommanderView`, `panasonic_ir_catalog`, and `__PANADIGA_IR_BRIDGE__` are compiled strictly for `--mode android` builds and tree-shaken out of desktop builds (`vite.config.ts` + CI check).

---

## 2. Non-Negotiable Hardware & Forensic Safety Invariants

These parameters are **physical invariants**, never user-editable runtime settings:

| Subsystem | Hard Invariant | Enforcement Layer |
| :--- | :--- | :--- |
| **`P7501` (`VEP79138` UART `115200 8N1`)** | `SW_UART` / `XRESET` (Open-Drain) / `UARTM2P_T` / `UARTP2M_T` operate at **3.3V LVTTL**. Pin 5 (`NSW5V` / `GP3`, `~5.15V` upstream via `D7510`) MUST stay **`Hi-Z Input` without pulls (NEVER push-pull)**. | Pico C11 firmware, `panadiga-core::settings` (`RESERVED_P7501_MAX_GPIO = 5`, rejecting `GP0..GP5` for user I/O), RAG `safety.rs` |
| **Probe Power (`P7501 TO JIGU`)** | NEVER power the Raspberry Pi Pico from `P7501` Pin 1 (`< 10 mA` sense). Always power externally via USB or `VSYS`. | Bench schematic + RAG shield |
| **Spansion `S29JL064H` (`IC5001` 8 MB)** | Physical bus `0x80000000..0x807FFFFF`. Critical boot IPL (`SA0..SA10`) is **permanently Read-Only (`E05`)**. Routine NVRAM surgery targets `SA134..SA141` (`0x807F0000..0x807FFFFF`, file offset `0x7F0000..0x7FFFFF`) via Copy-on-Write. | `panadiga-core::nvram`, Pico firmware, Tauri `E05` error taxonomy |
| **HDD Forensics (`MEIHFS`)** | Raw disk dumps (`hdd_100.bin`) are immutable evidence: never mutate in place. `MBR` (`"SH"`) at LBA `0`, mirror at LBA `57344` (`28 MB`), journal at `8 MB` (LBA `16384`). Validate SHA-256 against manifests. | `panadiga-core::hdd` |

---

## 3. Linter, Type Safety & Functional Programming Rules

Enforced in `apps/desktop/.oxlintrc.json` (`--deny-warnings`) via `@orgchrono/oxlint-plugin` (`packages/oxlint-plugin-panadiga/`):

1. **Zero Unsafe Type Casts (`panadiga/no-unsafe-type-cast`)**:
   - `any`, `<T>expr`, `as any`, `as unknown`, `as unknown as T`, and production narrowing `as Type` casts are **forbidden**.
   - Permitted narrowing patterns: `as const`, `satisfies`, `oneOfGuard(...)`, `safeParseWithSchema(..., zodSchema, fallback)`, and explicit type predicates (`v is T`).
   - Custom ambient `.d.ts` files (outside `vite-env.d.ts`) and `declare global` / `declare module` are **forbidden**.
2. **Functional Immutability & Surgical Closure Factory Decision (`panadiga/no-let-or-mutation`)**:
   - `var`, imperative loops (`for`, `for..in`, `for..of`, `while`, `do..while`), `++`/`--`, `delete`, OOP `class`, `.push`/`.pop`/`.splice`/`.sort`/`.reverse`, and `Object.assign(target, ...)` are **forbidden**.
   - **Architectural Decision on `let` in Closed Factory Closures**:
     - `let` and lexical reassignment (`=`) are permitted **strictly** inside an explicit non-UI closure factory named `create*` (`.ts` files outside `components/`, `views/`, `viewmodels/`, and never at module scope or inside loops), provided the binding never leaks by reference and member assignment (`obj.prop = ...`) stays forbidden.
     - Everywhere else (including all `.tsx` UI files and ViewModels), bindings MUST be immutable `const`.
3. **Zero Global State & Browser Adapter Isolation (`panadiga/no-global-state-or-side-effects`)**:
   - Direct `window`, `document`, `localStorage`, `sessionStorage`, `navigator`, `location`, and `history` access is allowed only inside `services/`, `store/storage.ts`, `config/platform.ts`, `hooks/usePointerDrag.ts`, and `main.tsx`.
   - Module top-level side effects are banned (except `main.tsx` and `i18n/i18n.ts`).
4. **Zero Hardcode & Zero Suppress (`panadiga/zero-hardcode`, `panadiga/no-lint-suppression`)**:
   - No hardcoded strings in `throw new Error(...)` (use `translate('errors....')`).
   - No inline fallback strings `t('key', 'fallback')`.
   - No user-facing prose in JSX text, JSX prose attributes, services, stores, or ViewModels.
   - Hardware magic addresses/bauds/sizes belong in `src/constants/`. Use named constants (`no-magic-numbers` permits only `[-1, 0, 1, 2, 10, 16, 100]`).
   - Inline linter/TypeScript suppressions (`eslint-disable`, `oxlint-disable`, `@ts-ignore`, `@ts-expect-error`, `@ts-nocheck`) are **forbidden**.
5. **UX Completeness (`ux-guard`) & Zero Duplication (`jscpd`)**:
   - `ux-guard scan src --fail-on warn` must score `100/100` (`isLoading`, empty array guards `.length === 0`, error handling, success feedback).
   - `jscpd --threshold 0` enforces **0% clones** across `apps/desktop/src`, `apps/desktop/src-tauri/src`, `packages/oxlint-plugin-panadiga/src`, `crates`, `firmware/picofw`, `firmware/digaos`, `tools`, and `scripts`.

---

## 4. Standard Libraries, Observability & Print/Console Ban

Prefer battle-tested libraries over custom infrastructure:

- **Rust Telemetry & Printing Ban**:
  - Diagnostic logging uses **`tracing`** + **`tracing-subscriber`** (streaming to **`stderr`** with runtime-reloadable `EnvFilter`) + **`opentelemetry`** / **`opentelemetry_sdk`** / **`tracing-opentelemetry`**.
  - `panadiga-core`, `panadiga-rag`, `panadiga-mcp`, and `panadiga-desktop` enforce `#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]` so ad-hoc `println!`/`eprintln!` cannot corrupt MCP stdio JSON-RPC framing or CLI `--json -` output.
- **TypeScript Telemetry & `console.*` Ban**:
  - Frontend uses **`pino`** + **`@opentelemetry/api`** behind [`src/utils/logger.ts`](apps/desktop/src/utils/logger.ts), forwarding UI logs in Tauri runtime via [`settingsService.ts`](apps/desktop/src/services/settingsService.ts) (`log_frontend_event`) into Rust `tracing`.
  - Raw `console.*` calls are **banned** everywhere in `apps/desktop/src/` (`no-console: error` + `panadiga/zero-hardcode` with **no** file overrides).
- **Schema Validation**:
  - Use **`zod`** for runtime contract validation (`src/utils/typeGuards.ts`, `src/utils/runtimeSettingsSchema.ts`).

---

## 5. Modular i18n Dictionaries & Runtime Operational Settings

1. **Modular Localization (`apps/desktop/src/locales/`)**:
   - Dictionaries are split by domain under `src/locales/en-US/*.json` and `src/locales/pt-BR/*.json` (`core`, `workspace`, `hdd`, `crypto`, `pico`, `ssot`, `errors`) plus `src/locales/ir/{en-US,pt-BR}.json`.
   - Every key in `en-US` MUST have exact parity in `pt-BR` (verified by `translations.test.ts`).
2. **Runtime Defaults vs Physical Invariants (`crates/panadiga-core/src/settings.rs`)**:
   - Operational defaults (serial baud rate, GDB listen socket, diagnostic timeout, telemetry poll interval, default HDD capacity GB, NOR block size, hex page bytes, forensic keywords, safe user IR GPIO `GP6..GP28`, search limit, split ratios, log level) are validated and atomically persisted by Rust via **`dirs`** (`<config_dir>/panadiga/runtime_settings.json`) and exposed in **Settings** (`SettingsView`).

---

## 6. Custom Oxlint Plugin Packaging (`packages/oxlint-plugin-panadiga/`)

- The architectural Oxlint rules live in [`packages/oxlint-plugin-panadiga/`](packages/oxlint-plugin-panadiga/package.json) as `@orgchrono/oxlint-plugin`.
- `apps/desktop` consumes it offline via `"file:../../packages/oxlint-plugin-panadiga"` so `npm ci --prefer-offline` and local development require zero network tokens, while `publishConfig.registry` (`https://npm.pkg.github.com`) enables publishing to **GitHub Packages** when desired.

---

## 7. Mandatory Pre-Commit Verification (9 Quality Gates)

Before considering any change complete, run from repository root:

```bash
python scripts/validate.py
# or: npm run validate
```

All 9 gates must pass cleanly:
1. **Rust Workspace**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --no-deps -- -D warnings`, `cargo test --workspace`
2. **Oxlint + TypeScript**: `npm run lint` (`oxlint src --deny-warnings && tsc --noEmit`)
3. **Vitest Suite**: `npm test`
4. **Copy/Paste Detector**: `jscpd --config .jscpd.json` (`threshold: 0`, `0` clones)
5. **UX Completeness**: `npm run uxlint` (`ux-guard scan src --fail-on warn`)
6. **Playwright E2E**: `npm run test:e2e`
7. **Ghidra Native Export**: `cargo run -p panadiga-cli -- ghidra export-all`
8. **Pico C11 Host Tests**: strict `-Wall -Wextra -Werror` / `/W4 /WX`
9. **Python Strict Typing & Tests**: `mypy --strict scripts`, `py_compile`, `pytest scripts/tests`

---

## 8. New Device Onboarding, Discovery Scripts & PR Checklist

When integrating a newly dumped or serviced Panasonic DIGA model into Panadiga, follow the 7-stage end-to-end onboarding guide ([`docs/en-US/WHATS_NEXT_NEW_DEVICE_ONBOARDING.md`](docs/en-US/WHATS_NEXT_NEW_DEVICE_ONBOARDING.md) · [`docs/pt-BR/WHATS_NEXT_ONBOARDING_NOVOS_APARELHOS.md`](docs/pt-BR/WHATS_NEXT_ONBOARDING_NOVOS_APARELHOS.md)):

1. **SMPS Galvanic Isolation & Operational Hardware Tuple**:
   - Use an AC isolation transformer or `ADuM3160`/`ADuM4160` USB galvanic isolator before probing SMPS-powered DIGA chassis (`~60–110 VAC` Y-capacitor leakage risk), discharge bulk capacitors, and verify rail voltage (`3.3V`/`1.8V`/`5V`) by multimeter prior to connecting Pico probes.
   - Always identify targets via the 4-tuple **`(model, region, board_rev, flash_jedec_id)`** (never rely on a bare marketing model string alone) and keep zero product-hardcoded logic in `crates/panadiga-core`, `apps/desktop/src`, `firmware/picofw`, or `firmware/digaos`.
2. **Physical Capture Integrity & Runtime `< T3` Gating**:
   - Archive raw UART bench logs in `docs/debug_logs/` and update `docs/debug_logs/README.md` (`SHA256SUMS`).
   - Screen NOR dumps via `panadiga profiles discover-nor` or `python -m scripts.re.nor_intake_analyzer --offline-only` to catch 16-bit byte-swapping, floating-address-pin mirroring, `all-0xFF` reads, and stale `EXTRHEAD` magic in erased FTL blocks before claiming `T3`.
   - Runtime kit builders (`prepare-sd`, `rebuild-from-dump`, `retarget`) fail closed on `< T3` profiles unless `--allow-unverified-profile` (and `--allow-unverified-mix` for mismatched rebuilds) is explicitly supplied.
3. **Quarantined `.FRM` / Service Manual Hydration (No Direct PDF-to-SSOT Writes)**:
   - Run `python -m scripts.re.frm_train_harvester --unpack-missing --apply` for `T2` `.FRM` metadata (`T3`/`T4` verified entries are never downgraded).
   - Run `python -m scripts.re.manual_spec_extractor --apply`, which emits quarantined JSON proposals with `page_citations` into `profiles/staging/manual_proposals/` for human verification — never auto-mutating `profiles/boards/` or `profiles/eeprom_chips/`.
4. **Ghidra Reachable-Code Coverage, Honest Firmware Framing & SSOT Sync**:
   - Analyze reachable executable functions (never blind `0 ??` across literal pools/jump tables), export plain-text bundles via `tools/ghidra/ghidra_export_pseudocode_bundle.py`, and run `panadiga ssot export-all`, `panadiga profiles export-schema --out-dir profiles/schema`, and `panadiga ghidra export-all`.
   - Keep `firmware/digaos` (`NOT LEGALLY CLEAN-ROOM`) honestly documented as an open-source hardware-interoperability operating system prototype (not clean-room) and treat `Mn103Vm` strictly as an ISA/bounds check rather than real-silicon CFI/IRQ/WDT proof.
5. **Audit & Pull Request Verification**:
   - Verify `panadiga profiles completeness` and `panadiga profiles validate` pass cleanly, then run the full 9-Gate suite (`python scripts/validate.py`) confirming `0` `jscpd` clones, `100/100` `uxlint`, bilingual doc parity, and all Rust/Vitest/Playwright/C11/Pytest tests passing before opening your PR.
