/**
 * @file translations.ts
 * @brief Internationalization resources composed from modular domain JSON tables under src/locales/<lang>/<domain>.json.
 */

import { isRecord } from '@/utils/typeGuards';
import enCore from '../locales/en-US/core.json';
import enWorkspace from '../locales/en-US/workspace.json';
import enHdd from '../locales/en-US/hdd.json';
import enCrypto from '../locales/en-US/crypto.json';
import enPico from '../locales/en-US/pico.json';
import enSsot from '../locales/en-US/ssot.json';
import enErrors from '../locales/en-US/errors.json';
import enExpress from '../locales/en-US/express.json';
import enPipelines from '../locales/en-US/pipelines.json';
import ptCore from '../locales/pt-BR/core.json';
import ptWorkspace from '../locales/pt-BR/workspace.json';
import ptHdd from '../locales/pt-BR/hdd.json';
import ptCrypto from '../locales/pt-BR/crypto.json';
import ptPico from '../locales/pt-BR/pico.json';
import ptSsot from '../locales/pt-BR/ssot.json';
import ptErrors from '../locales/pt-BR/errors.json';
import ptExpress from '../locales/pt-BR/express.json';
import ptPipelines from '../locales/pt-BR/pipelines.json';
import irEnUSJson from '../locales/ir/en-US.json';
import irPtBRJson from '../locales/ir/pt-BR.json';

const enUSJson = {
  ...enCore,
  ...enWorkspace,
  ...enHdd,
  ...enCrypto,
  ...enPico,
  ...enSsot,
  ...enErrors,
  ...enExpress,
  ...enPipelines,
} as const;

const ptBRJson = {
  ...ptCore,
  ...ptWorkspace,
  ...ptHdd,
  ...ptCrypto,
  ...ptPico,
  ...ptSsot,
  ...ptErrors,
  ...ptExpress,
  ...ptPipelines,
} as const;

export type Locale = 'en-US' | 'pt-BR';

type JoinKey<K, P> = K extends string ? (P extends string ? `${K}.${P}` : never) : never;

type NestedKeyOf<T> = {
  [K in keyof T & string]: T[K] extends readonly unknown[]
    ? K
    : T[K] extends Record<string, unknown>
      ? JoinKey<K, NestedKeyOf<T[K]>>
      : K;
}[keyof T & string];

export type TranslationKey = NestedKeyOf<typeof enUSJson> | NestedKeyOf<typeof irEnUSJson>;

function mergeSection(base: unknown, extra: unknown): Record<string, unknown> {
  const a = isRecord(base) ? base : {};
  const b = isRecord(extra) ? extra : {};
  return { ...a, ...b };
}

function mergeLocaleResources(core: unknown, ir: unknown): Record<string, unknown> {
  const coreRecord = isRecord(core) ? core : {};
  const irRecord = isRecord(ir) ? ir : {};
  if (Object.keys(irRecord).length === 0) {
    return coreRecord;
  }
  const coreShell = isRecord(coreRecord.shell) ? coreRecord.shell : {};
  const irShell = isRecord(irRecord.shell) ? irRecord.shell : {};
  const mergedShell: Record<string, unknown> = {
    ...coreShell,
    ...irShell,
    viewTitles: mergeSection(coreShell.viewTitles, irShell.viewTitles),
  };
  return {
    ...coreRecord,
    ...irRecord,
    nav: mergeSection(coreRecord.nav, irRecord.nav),
    shell: mergedShell,
  };
}

export const mergedLocales = {
  'en-US': mergeLocaleResources(enUSJson, irEnUSJson),
  'pt-BR': mergeLocaleResources(ptBRJson, irPtBRJson),
} as const;

function flattenObject(obj: Record<string, unknown>, prefix = ''): Record<string, string> {
  const entries = Object.entries(obj).flatMap<readonly [string, string]>(([key, value]) => {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (isRecord(value)) {
      return Object.entries(flattenObject(value, fullKey));
    }
    return [[fullKey, String(value)]];
  });
  return Object.fromEntries(entries);
}

export const translations = {
  'en-US': flattenObject(mergedLocales['en-US']),
  'pt-BR': flattenObject(mergedLocales['pt-BR']),
} as const;
