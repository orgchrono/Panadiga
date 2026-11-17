/**
 * @file constants/hardwareSpecs/picoCommandBlueprints.ts
 * @brief Data-driven CLI command blueprint loader over shared Rust/TS `hardware_catalog.json`.
 */

import ptBRJson from '@/locales/pt-BR/ssot.json';
import type { PicoCommandCategory, PicoCommandSpec } from '@/models';
import hardwareCatalog from '../../../../../crates/panadiga-core/src/ssot/hardware_catalog.json';
import { resolveCatalogTemplate } from './catalogTokens';

export interface PicoCommandLocaleEntry {
  readonly label: string;
  readonly description: string;
  readonly arg_label?: string;
  readonly arg_placeholder?: string;
  readonly options?: Readonly<Record<string, string>>;
}

export interface CommandBlueprint {
  readonly id: string;
  readonly category: PicoCommandSpec['category'];
  readonly syntax: string;
  readonly default_arg: string | null;
  readonly optionPairs?: ReadonlyArray<readonly [string, string]>;
  readonly dangerous?: boolean;
  readonly timeout_ms: number;
}

const VALID_COMMAND_CATEGORIES: Readonly<Record<string, PicoCommandCategory>> = Object.freeze({
  electrical: 'electrical',
  ocd_flash: 'ocd_flash',
  uart_fmu: 'uart_fmu',
  probe: 'probe',
});

function parseCommandCategory(raw: string, id: string): PicoCommandCategory {
  const category = VALID_COMMAND_CATEGORIES[raw];
  if (!category) {
    throw new Error(`Invalid SSOT command category '${raw}' for '${id}'`);
  }
  return category;
}

function assertCommandLocaleKey(raw: string): string {
  if (!Object.prototype.hasOwnProperty.call(ptBRJson.ssot.commands, raw)) {
    throw new Error(`Missing locale entry ssot.commands.${raw}`);
  }
  return raw;
}

function resolveCatalogTimeoutMs(key: string, id: string): number {
  const table: Readonly<Record<string, number>> = hardwareCatalog.timeouts_ms;
  const value = table[key];
  if (typeof value !== 'number') {
    throw new Error(`Missing timeout key '${key}' in hardware_catalog.json for '${id}'`);
  }
  return value;
}

function extractOptionPairs(entry: object): ReadonlyArray<readonly [string, string]> | undefined {
  if (!('options' in entry) || !Array.isArray(entry.options)) {
    return undefined;
  }
  return Object.freeze(
    entry.options.flatMap(pair => {
      if (
        Array.isArray(pair) &&
        pair.length >= 2 &&
        typeof pair[0] === 'string' &&
        typeof pair[1] === 'string'
      ) {
        return [Object.freeze([pair[0], resolveCatalogTemplate(pair[1])] as const)];
      }
      return [];
    }),
  );
}

export const PICO_COMMAND_BLUEPRINTS: readonly CommandBlueprint[] = Object.freeze(
  hardwareCatalog.commands.map(entry => {
    const id = assertCommandLocaleKey(entry.id);
    const defaultArg =
      'default_arg' in entry && typeof entry.default_arg === 'string'
        ? resolveCatalogTemplate(entry.default_arg)
        : null;
    const dangerous = 'dangerous' in entry && entry.dangerous === true;

    return Object.freeze({
      id,
      category: parseCommandCategory(entry.category, entry.id),
      syntax: resolveCatalogTemplate(entry.syntax),
      default_arg: defaultArg,
      optionPairs: extractOptionPairs(entry),
      dangerous,
      timeout_ms: resolveCatalogTimeoutMs(entry.timeout, entry.id),
    });
  }),
);
