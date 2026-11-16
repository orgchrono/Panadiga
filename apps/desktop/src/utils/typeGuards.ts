import type {
  MatshitaBootScenario,
  PicoCommandCategory,
  PicoUf2Variant,
  Sc0BridgeInputMode,
} from '@/models';
import type { FrmPackMode } from '@/components/crypto/FrmPackFormFields';
import type { BgoMode, MainPassthroughMode } from '@/components/unbrick/unbrickDomain';

/** Builds a runtime narrowing predicate for closed string literal unions (zero unsafe `as` casts). */
export function oneOfGuard<const T extends readonly string[]>(
  allowed: T
): (value: string) => value is T[number] {
  return (value: string): value is T[number] =>
    (allowed as readonly string[]).includes(value);
}

export const isFrmPackMode: (value: string) => value is FrmPackMode = oneOfGuard([
  'sd',
  'dvd',
  'digadiag',
] as const);

export const isPicoUf2Variant: (value: string) => value is PicoUf2Variant = oneOfGuard([
  'unified',
  'rp2040',
  'rp2350',
] as const);

export const isPicoCommandCategory: (value: string) => value is PicoCommandCategory = oneOfGuard([
  'electrical',
  'ocd_flash',
  'uart_fmu',
  'probe',
] as const);

export const isSc0BridgeInputMode: (value: string) => value is Sc0BridgeInputMode = oneOfGuard([
  'text',
  'line',
  'hex',
  'rsp',
] as const);

export const isMatshitaBootScenario: (value: string) => value is MatshitaBootScenario =
  oneOfGuard([
    'healthy_with_disc',
    'unlock_disc_frm',
    'tray_cannot_close_80045300',
    'laser_apc_failure_80040904',
  ] as const);

export const isBgoMode: (value: string) => value is BgoMode = oneOfGuard([
  'auto_format',
  'bypass',
  'uart_monitor',
  'factory',
] as const);

export const isMainPassthroughMode: (value: string) => value is MainPassthroughMode = oneOfGuard([
  'av_passthrough',
  'factory_original',
] as const);
