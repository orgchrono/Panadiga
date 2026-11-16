/**
 * @file translations.ts
 * @brief Internationalization resources loaded directly from canonical JSON tables.
 *
 * Backed by locales/en-US.json and locales/pt-BR.json.
 */

import enUSJson from '../locales/en-US.json';
import ptBRJson from '../locales/pt-BR.json';
import irEnUSJson from '../locales/ir.en-US.json';
import irPtBRJson from '../locales/ir.pt-BR.json';

export type Locale = 'en-US' | 'pt-BR';

function mergeSection(base: unknown, extra: unknown): Record<string, unknown> {
  const a = base && typeof base === 'object' ? (base as Record<string, unknown>) : {};
  const b = extra && typeof extra === 'object' ? (extra as Record<string, unknown>) : {};
  return { ...a, ...b };
}

function mergeLocaleResources(
  core: Record<string, unknown>,
  ir: Record<string, unknown>,
): Record<string, unknown> {
  if (Object.keys(ir).length === 0) {
    return core;
  }
  const mergedShell = mergeSection(core.shell, ir.shell);
  mergedShell.viewTitles = mergeSection(
    (core.shell as Record<string, unknown> | undefined)?.viewTitles,
    (ir.shell as Record<string, unknown> | undefined)?.viewTitles,
  );
  return {
    ...core,
    ...ir,
    nav: mergeSection(core.nav, ir.nav),
    shell: mergedShell,
  };
}

export const mergedLocales = {
  'en-US': mergeLocaleResources(
    enUSJson as Record<string, unknown>,
    irEnUSJson as Record<string, unknown>,
  ),
  'pt-BR': mergeLocaleResources(
    ptBRJson as Record<string, unknown>,
    irPtBRJson as Record<string, unknown>,
  ),
} as const;

function flattenObject(obj: Record<string, unknown>, prefix = ''): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      Object.assign(result, flattenObject(value as Record<string, unknown>, fullKey));
    } else {
      result[fullKey] = String(value);
    }
  }
  return result;
}

export const translations = {
  'en-US': flattenObject(mergedLocales['en-US']),
  'pt-BR': flattenObject(mergedLocales['pt-BR']),
} as const;

export type TranslationKey = keyof typeof translations['en-US'];
