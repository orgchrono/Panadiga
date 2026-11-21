/**
 * Functional Programming (FP) Pure Formatting Utilities
 * All functions are pure, deterministic, and free of side effects.
 */

import {
  MAX_U32_VALUE,
  PICO_MAX_VDD_CEILING_VOLTS,
  PICO_MIN_VDD_THRESHOLD_VOLTS,
  PICO_SAFE_OPERATING_MIN_VOLTS,
} from '@/constants';

const HEX_RADIX = 16;
const DECIMAL_RADIX = 10;
const HEX32_PAD_WIDTH = 8;
const ID_RADIX = 36;
const UUID_SEGMENT_LEN = 8;
const USEC_PER_MS = 1000;

/**
 * Pure function: formats a 32-bit unsigned number into 8-digit uppercase hex representation with 0x prefix.
 */
export const formatHex32 = (val: number | undefined | null, defaultVal = '0x00000000'): string => {
  if (val === undefined || val === null || isNaN(val)) {
    return defaultVal;
  }
  return `0x${(val >>> 0).toString(HEX_RADIX).padStart(HEX32_PAD_WIDTH, '0').toUpperCase()}`;
};

/**
 * Pure function: formats a float voltage into a 2-decimal string.
 */
export const formatVoltage = (volts: number | undefined | null): string => {
  if (volts === undefined || volts === null || isNaN(volts)) {
    return '0.00';
  }
  return volts.toFixed(2);
};

/**
 * Pure predicate: determines whether target reference voltage indicates a powered board.
 */
export const isTargetPowered = (
  volts: number | undefined | null,
  threshold = PICO_MIN_VDD_THRESHOLD_VOLTS,
): boolean => {
  if (volts === undefined || volts === null || isNaN(volts)) {
    return false;
  }
  return volts >= threshold;
};

/**
 * Pure predicate: determines whether measured VDD rail lies within nominal 3.3V LVTTL tolerance.
 */
export const isTargetVoltageNominal = (
  volts: number | undefined | null,
  minVolts = PICO_SAFE_OPERATING_MIN_VOLTS,
  maxVolts = PICO_MAX_VDD_CEILING_VOLTS,
): boolean => {
  if (volts === undefined || volts === null || isNaN(volts)) {
    return false;
  }
  return volts >= minVolts && volts <= maxVolts;
};

/**
 * Pure function: formats millisecond duration into human-readable latency.
 */
export const formatElapsedMs = (ms: number | undefined | null): string => {
  if (!ms || ms < 0) return '0ms';
  return `${ms.toLocaleString('en-US')}ms`;
};

function generateStatelessToken(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID().slice(0, UUID_SEGMENT_LEN);
  }
  const stamp = Date.now().toString(ID_RADIX);
  const perf =
    typeof performance !== 'undefined'
      ? Math.floor(performance.now() * USEC_PER_MS).toString(ID_RADIX)
      : '0';
  return `${stamp}-${perf}`;
}

/**
 * Stateless closure factory producing prefix-scoped unique IDs without mutable globals.
 */
export const createIdGenerator = (prefix = 'id'): (() => string) => () =>
  `${prefix}-${generateStatelessToken()}`;

/**
 * Pure helper: resolves live target rail voltage from hardware status or latest diagnostic fallback.
 */
export const resolveEffectiveVddVolts = (
  statusVdd?: number | null,
  diagVdd?: number | null,
): number => {
  if (typeof statusVdd === 'number' && statusVdd > 0) {
    return statusVdd;
  }
  return typeof diagVdd === 'number' && diagVdd > 0 ? diagVdd : 0;
};

/**
 * Pure helper: classifies Pico bench transport flags (BLE, Android USB-OTG, RP2040/RP2350 CDC).
 */
export const detectPicoTransportFlags = (
  transportMode?: string | null,
  selectedPort?: string | null,
): { isBle: boolean; isOtg: boolean; isPicoPort: boolean } => {
  const port = selectedPort ?? '';
  return {
    isBle: transportMode === 'ble' || port.startsWith('BLE_'),
    isOtg: transportMode === 'usb-otg' || port.startsWith('ANDROID-USB-OTG'),
    isPicoPort: /VID:2E8A|RP2040|RP2350|Pico|PANADIGA|P7501/i.test(port),
  };
};

/**
 * Stateless prefix-scoped unique ID generator free of mutable counters.
 */
export const nextUniqueId = (prefix = 'id'): string => `${prefix}-${generateStatelessToken()}`;

/**
 * Pure FP helper: decorates items with guaranteed-unique semantic React keys without index keys.
 */
export function withUniqueKeys<T>(
  items: readonly T[],
  getBaseKey: (item: T) => string,
): ReadonlyArray<{ readonly item: T; readonly key: string }> {
  const result = items.reduce<{
    readonly rows: ReadonlyArray<{ readonly item: T; readonly key: string }>;
    readonly counts: Readonly<Record<string, number>>;
  }>(
    (state, item) => {
      const base = getBaseKey(item);
      const bucket = `k:${base}`;
      const seen = (state.counts[bucket] ?? 0) + 1;
      const key = seen === 1 ? base : `${base}#${seen}`;
      return {
        rows: [...state.rows, { item, key }],
        counts: { ...state.counts, [bucket]: seen },
      };
    },
    { rows: [], counts: {} },
  );
  return result.rows;
}

/**
 * Pure parser: converts a hex ('0x...') or decimal NVRAM offset input into a validated u32 integer.
 * Returns null for NaN, negative, fractional, malformed, or > 0xFFFFFFFF inputs.
 */
export function parseNvramOffsetInput(raw: string | number): number | null {
  if (typeof raw === 'number') {
    return Number.isInteger(raw) && raw >= 0 && raw <= MAX_U32_VALUE ? raw : null;
  }
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const isHex = /^0x[0-9a-f]+$/i.test(trimmed);
  const isDec = /^\d+$/.test(trimmed);
  if (!isHex && !isDec) return null;
  const parsed = Number.parseInt(trimmed, isHex ? HEX_RADIX : DECIMAL_RADIX);
  return Number.isInteger(parsed) && parsed >= 0 && parsed <= MAX_U32_VALUE ? parsed : null;
}

/**
 * Pure error-to-string formatter shared cleanly across MVVM ViewModels.
 */
export const formatErrorMessage = (err: unknown): string =>
  err instanceof Error ? err.message : String(err);
