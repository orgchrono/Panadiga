/**
 * Android Native Infrared HAL Client.
 * Wraps `window.__PANADIGA_IR_BRIDGE__` via `resolveAndroidIrBridge()` + strict runtime guards.
 */

import { translate } from '@/i18n/useTranslation';
import type {
  AndroidIrCapabilities,
  AndroidIrStoredSettings,
  AndroidIrTransmitResult,
  IrDeviceInfo,
  IrFrequencyRange,
  IrTimingOverride,
} from '@/models/ir';
import { isRecord, oneOfGuard, safeParseJson } from '@/utils/typeGuards';
import { resolveAndroidIrBridge } from './hostBridge';

const isIrTimingOverride: (value: unknown) => value is IrTimingOverride = oneOfGuard([
  'AUTO',
  'MICROSECONDS',
  'CARRIER_CYCLES',
  'PERIOD_SCALED',
] as const);

function isIrFrequencyRange(value: unknown): value is IrFrequencyRange {
  return isRecord(value) && typeof value.min_hz === 'number' && typeof value.max_hz === 'number';
}

function isIrDeviceInfo(value: unknown): value is IrDeviceInfo {
  return (
    isRecord(value) &&
    typeof value.manufacturer === 'string' &&
    typeof value.model === 'string' &&
    typeof value.hardware === 'string' &&
    typeof value.board === 'string' &&
    typeof value.product === 'string' &&
    typeof value.device === 'string' &&
    typeof value.android_release === 'string' &&
    typeof value.sdk_int === 'number'
  );
}

function isAndroidIrCapabilities(value: unknown): value is AndroidIrCapabilities {
  return (
    isRecord(value) &&
    typeof value.hasEmitter === 'boolean' &&
    typeof value.serviceFound === 'boolean' &&
    typeof value.hasSecIrNode === 'boolean' &&
    typeof value.hasFpgaNode === 'boolean' &&
    typeof value.suAvailable === 'boolean' &&
    Array.isArray(value.ranges) &&
    value.ranges.every(isIrFrequencyRange) &&
    isIrDeviceInfo(value.device)
  );
}

function isAndroidIrStoredSettings(value: unknown): value is AndroidIrStoredSettings {
  return (
    isRecord(value) &&
    isIrTimingOverride(value.timingUnitOverride) &&
    typeof value.repeatCount === 'number' &&
    typeof value.customProfilesJson === 'string'
  );
}

function isSaveSettingsAck(value: unknown): value is { readonly ok: boolean } {
  return isRecord(value) && typeof value.ok === 'boolean';
}

function isAndroidIrTransmitResult(value: unknown): value is AndroidIrTransmitResult {
  return (
    isRecord(value) &&
    typeof value.ok === 'boolean' &&
    (value.backendUsed === undefined || typeof value.backendUsed === 'string') &&
    (value.error === undefined || typeof value.error === 'string')
  );
}

export function isAndroidIrBridgeAvailable(): boolean {
  return resolveAndroidIrBridge()?.isAvailable() ?? false;
}

export function getAndroidIrCapabilities(): AndroidIrCapabilities | null {
  const bridge = resolveAndroidIrBridge();
  if (!bridge) return null;
  return safeParseJson<AndroidIrCapabilities | null>(
    bridge.getCapabilitiesJson(),
    (value): value is AndroidIrCapabilities | null => isAndroidIrCapabilities(value),
    null,
  );
}

export function getAndroidIrSettings(): AndroidIrStoredSettings | null {
  const bridge = resolveAndroidIrBridge();
  if (!bridge) return null;
  return safeParseJson<AndroidIrStoredSettings | null>(
    bridge.getSettingsJson(),
    (value): value is AndroidIrStoredSettings | null => isAndroidIrStoredSettings(value),
    null,
  );
}

export function saveAndroidIrSettings(
  timingUnitOverride: IrTimingOverride,
  repeatCount: number,
): boolean {
  const bridge = resolveAndroidIrBridge();
  if (!bridge) return false;
  const res = safeParseJson(
    bridge.saveSettings(timingUnitOverride, repeatCount),
    isSaveSettingsAck,
    { ok: false },
  );
  return res.ok;
}

export function saveAndroidCustomProfilesJson(profilesJson: string): boolean {
  return resolveAndroidIrBridge()?.saveCustomProfilesJson(profilesJson) ?? false;
}

export function transmitViaAndroidIrHal(
  carrierHz: number,
  encodedPattern: readonly number[],
  sysfsPayload: string,
  repeatCount: number,
): AndroidIrTransmitResult {
  const bridge = resolveAndroidIrBridge();
  if (!bridge) {
    return {
      ok: false,
      error: translate('errors.androidIr.unavailable'),
    };
  }
  return safeParseJson(
    bridge.transmitPreparedJson(
      carrierHz,
      JSON.stringify(encodedPattern),
      sysfsPayload,
      repeatCount,
    ),
    isAndroidIrTransmitResult,
    { ok: false, error: translate('errors.androidIr.invalidHalJson') },
  );
}
