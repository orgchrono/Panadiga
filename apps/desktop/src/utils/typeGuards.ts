import { z } from 'zod';
import type {
  BgoMode,
  FrmPackMode,
  MainPassthroughMode,
  MatshitaBootScenario,
  PicoCommandCategory,
  PicoUf2Variant,
  Sc0BridgeInputMode,
} from '@/models';
import type { DynamicKeymapProfile } from '@/models/ir';

/** Narrows an unknown value to a plain string-keyed dictionary. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Reads an optional string property safely from an unknown record. */
export function readStringProp(record: Record<string, unknown>, key: string): string | undefined {
  const value = record[key];
  return typeof value === 'string' ? value : undefined;
}

/** Reads an optional finite number property safely from an unknown record. */
export function readNumberProp(record: Record<string, unknown>, key: string): number | undefined {
  const value = record[key];
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

/** Reads an optional boolean property safely from an unknown record. */
export function readBooleanProp(record: Record<string, unknown>, key: string): boolean | undefined {
  const value = record[key];
  return typeof value === 'boolean' ? value : undefined;
}

/** Parses JSON and validates its runtime shape using a strict predicate before returning. */
export function safeParseJson<T>(
  raw: string,
  guard: (value: unknown) => value is T,
  fallback: T,
): T {
  try {
    const parsed: unknown = JSON.parse(raw);
    return guard(parsed) ? parsed : fallback;
  } catch {
    return fallback;
  }
}

/** Parses JSON and validates via a canonical Zod schema, falling back safely on invalid input. */
export function safeParseWithSchema<T>(raw: string, schema: z.ZodType<T>, fallback: T): T {
  try {
    const parsed: unknown = JSON.parse(raw);
    const result = schema.safeParse(parsed);
    return result.success ? result.data : fallback;
  } catch {
    return fallback;
  }
}

/** Builds a runtime narrowing predicate for closed string literal unions (zero unsafe `as` casts). */
export function oneOfGuard<const T extends readonly string[]>(
  allowed: T,
): (value: unknown) => value is T[number] {
  return (value: unknown): value is T[number] =>
    typeof value === 'string' && allowed.some(item => item === value);
}

export const isFrmPackMode: (value: unknown) => value is FrmPackMode = oneOfGuard([
  'sd',
  'dvd',
  'digadiag',
] as const);

export const isPicoUf2Variant: (value: unknown) => value is PicoUf2Variant = oneOfGuard([
  'unified',
  'rp2040',
  'rp2350',
] as const);

export const isPicoCommandCategory: (value: unknown) => value is PicoCommandCategory = oneOfGuard([
  'electrical',
  'ocd_flash',
  'uart_fmu',
  'probe',
] as const);

export const isSc0BridgeInputMode: (value: unknown) => value is Sc0BridgeInputMode = oneOfGuard([
  'text',
  'line',
  'hex',
  'rsp',
] as const);

export const isMatshitaBootScenario: (value: unknown) => value is MatshitaBootScenario =
  oneOfGuard([
    'healthy_with_disc',
    'unlock_disc_frm',
    'tray_cannot_close_80045300',
    'laser_apc_failure_80040904',
  ] as const);

export const isBgoMode: (value: unknown) => value is BgoMode = oneOfGuard([
  'auto_format',
  'bypass',
  'uart_monitor',
  'factory',
] as const);

export const isMainPassthroughMode: (value: unknown) => value is MainPassthroughMode = oneOfGuard([
  'av_passthrough',
  'factory_original',
] as const);

export function isDynamicKeymapProfile(value: unknown): value is DynamicKeymapProfile {
  return (
    isRecord(value) &&
    typeof value.id === 'string' &&
    typeof value.name === 'string' &&
    Array.isArray(value.keys)
  );
}

export function isDynamicKeymapProfileArray(value: unknown): value is DynamicKeymapProfile[] {
  return Array.isArray(value) && value.every(isDynamicKeymapProfile);
}
